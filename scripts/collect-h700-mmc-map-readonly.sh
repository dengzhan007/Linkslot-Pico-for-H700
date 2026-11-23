#!/bin/sh
# Read-only H700 MMC host <-> block-device <-> live-DT mapping collector.
# Writes only text files beneath OUTDIR; never opens a block device for I/O.
set -eu
umask 077

STAMP="$(date +%Y%m%d-%H%M%S 2>/dev/null || echo run)"
OUTDIR="${1:-/tmp/h700-mmc-map-$STAMP}"
[ ! -e "$OUTDIR" ] || { echo "error: output path exists: $OUTDIR" >&2; exit 1; }
mkdir -p "$OUTDIR"

DT=/sys/firmware/devicetree/base
[ -d "$DT" ] || DT=/proc/device-tree
[ -d "$DT" ] || DT=""

readlink_path() {
    if command -v readlink >/dev/null 2>&1; then
        readlink -f "$1" 2>/dev/null || readlink "$1" 2>/dev/null || echo "UNRESOLVED"
    else
        echo "readlink unavailable"
    fi
}
dump_prop() {
    file="$1"
    if [ -r "$file" ]; then
        printf '  %s = ' "${file##*/}"
        if command -v od >/dev/null 2>&1; then
            od -An -v -tx1 "$file" 2>/dev/null | tr '\n' ' '
            echo
        else
            echo "PRESENT (od unavailable; bytes not decoded)"
        fi
    else
        echo "  ${file##*/}: NOT_PRESENT"
    fi
}
dump_attrs() {
    base="$1"
    shift
    for attr in "$@"; do
        if [ -r "$base/$attr" ]; then
            echo "[$attr]"
            cat "$base/$attr" 2>&1 || true
        fi
    done
}

{
    echo "collector=h700-mmc-map-readonly-v1"
    echo "captured_at=$(date 2>/dev/null || echo unknown)"
    echo "kernel=$(uname -a 2>&1 || true)"
    echo "dt_root=${DT:-NOT_PRESENT}"
    echo "cmdline=$(cat /proc/cmdline 2>/dev/null || echo NOT_PRESENT)"
    echo
    echo "== device tree model / compatible =="
    if [ -n "$DT" ]; then
        dump_prop "$DT/model"
        dump_prop "$DT/compatible"
    else
        echo "Device tree filesystem not exported"
    fi
    echo
    echo "== aliases =="
    if [ -n "$DT" ] && [ -d "$DT/aliases" ]; then
        for a in mmc0 mmc1 mmc2; do
            dump_prop "$DT/aliases/$a"
        done
    else
        echo "NOT_PRESENT"
    fi
    echo
    echo "== block devices =="
    for node in /sys/class/block/mmcblk*; do
        [ -e "$node" ] || continue
        echo "-- $node --"
        echo "device=$(readlink_path "$node/device")"
        echo "device_of_node=$(readlink_path "$node/device/../../../of_node")"
        dump_attrs "$node" dev size start partition ro removable uevent
    done
    echo
    echo "== MMC hosts =="
    for host in /sys/class/mmc_host/mmc*; do
        [ -d "$host" ] || continue
        echo "-- $host --"
        echo "host_device=$(readlink_path "$host/device")"
        echo "host_driver=$(readlink_path "$host/device/driver")"
        echo "host_of_node=$(readlink_path "$host/device/of_node")"
        dump_attrs "$host" uevent
        dump_attrs "$host/device" uevent modalias power/control power/runtime_status
    done
    echo
    echo "== platform MMC devices and live DT properties =="
    for dev in /sys/bus/platform/devices/*mmc*; do
        [ -e "$dev" ] || continue
        echo "-- $dev --"
        echo "platform_device=$(readlink_path "$dev")"
        echo "driver=$(readlink_path "$dev/driver")"
        NODE="$dev/of_node"
        echo "of_node=$(readlink_path "$NODE")"
        for prop in compatible status reg clocks clock-names resets reset-names \
                    pinctrl-0 pinctrl-names bus-width cd-gpios vmmc-supply \
                    vqmmc-supply no-1-8-v disable-wp cap-sd-highspeed; do
            dump_prop "$NODE/$prop"
        done
        dump_attrs "$dev" uevent modalias power/control power/runtime_status
    done
    echo
    echo "== regulators =="
    for reg in /sys/class/regulator/regulator.*; do
        [ -d "$reg" ] || continue
        echo "-- $reg --"
        dump_attrs "$reg" name state status type microvolts min_microvolts \
                   max_microvolts num_users
    done
    echo
    echo "== debugfs clock / pinctrl / regulator summaries (if exported) =="
    for file in /sys/kernel/debug/clk/clk_summary \
                /sys/kernel/debug/pinctrl/pinctrl-handles \
                /sys/kernel/debug/pinctrl/pinctrl-maps \
                /sys/kernel/debug/regulator/regulator_summary; do
        if [ -r "$file" ]; then
            echo "-- $file --"
            cat "$file" 2>&1 || true
        else
            echo "NOT_PRESENT: $file"
        fi
    done
    echo
    echo "== MMC kernel log lines =="
    if command -v dmesg >/dev/null 2>&1; then
        dmesg 2>/dev/null | grep -iE 'mmc|sunxi.mmc|sdhci|card detect|card insert|card remove' || true
    else
        echo "dmesg unavailable"
    fi
} > "$OUTDIR/mmc-map.txt"

{
    echo "Read-only MMC mapping capture completed."
    echo "Output directory: $OUTDIR"
    echo "No block-device read/write command was issued by this collector."
    echo "All captured MMC CID/CSD identity fields are intentionally omitted."
} > "$OUTDIR/README.txt"

if command -v tar >/dev/null 2>&1; then
    tar -czf "$OUTDIR.tar.gz" -C "$OUTDIR" .
    echo "Archive: $OUTDIR.tar.gz"
else
    echo "tar unavailable; pull the output directory directly: $OUTDIR"
fi
