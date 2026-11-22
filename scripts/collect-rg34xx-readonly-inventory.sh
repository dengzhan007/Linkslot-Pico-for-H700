#!/bin/sh
# Read-only RG34XX/H700 boot inventory. The source block device is never an output.
set -eu
umask 077

usage() {
    echo "Usage: $0 [BLOCK_DEVICE [OUTPUT_DIR]]" >&2
    echo "Example: $0 /dev/mmcblk0 /tmp/rg34xx-readonly-inventory" >&2
    exit 2
}

if [ "${1:-}" = "-h" ] || [ "${1:-}" = "--help" ]; then
    usage
fi

DISK="${1:-}"
OUTDIR="${2:-}"
ROOTDEV=""

if [ -z "$DISK" ]; then
    for arg in $(cat /proc/cmdline 2>/dev/null || true); do
        case "$arg" in
            root=/dev/*) ROOTDEV="${arg#root=}" ;;
        esac
    done
    case "$ROOTDEV" in
        /dev/mmcblk*p[0-9]*) DISK="${ROOTDEV%p*}" ;;
        /dev/nvme*n*p[0-9]*) DISK="${ROOTDEV%p*}" ;;
        /dev/sd[a-z][0-9]*) DISK="${ROOTDEV%[0-9]*}" ;;
        *)
            echo "error: cannot derive the SD block device from root=$ROOTDEV" >&2
            echo "Pass the whole-disk device explicitly, for example /dev/mmcblk0." >&2
            exit 1
            ;;
    esac
fi

[ -b "$DISK" ] || {
    echo "error: '$DISK' is not a block device" >&2
    exit 1
}
command -v dd >/dev/null 2>&1 || { echo "error: dd not found" >&2; exit 1; }
command -v sha256sum >/dev/null 2>&1 || { echo "error: sha256sum not found" >&2; exit 1; }
command -v od >/dev/null 2>&1 || { echo "error: od not found" >&2; exit 1; }

if [ -z "$OUTDIR" ]; then
    STAMP="$(date +%Y%m%d-%H%M%S 2>/dev/null || echo run)"
    OUTDIR="/tmp/rg34xx-readonly-inventory-$STAMP"
fi
[ ! -e "$OUTDIR" ] || {
    echo "error: output path already exists: $OUTDIR" >&2
    exit 1
}
mkdir -p "$OUTDIR"

BASE="${DISK##*/}"
{
    echo "collector=rg34xx-readonly-inventory-v1"
    echo "disk=$DISK"
    echo "root_device_from_cmdline=$ROOTDEV"
    echo "captured_at=$(date 2>/dev/null || echo unknown)"
    echo "block_device_bytes:"
    if command -v blockdev >/dev/null 2>&1; then
        blockdev --getsize64 "$DISK" 2>&1 || true
        echo "logical_sector_size:"
        blockdev --getss "$DISK" 2>&1 || true
        echo "read_only_flag:"
        blockdev --getro "$DISK" 2>&1 || true
    else
        echo "blockdev utility unavailable"
    fi
} > "$OUTDIR/summary.txt"

capture_proc() {
    name="$1"
    path="$2"
    if [ -r "$path" ]; then
        cat "$path" > "$OUTDIR/$name"
    else
        echo "NOT_PRESENT: $path" > "$OUTDIR/$name"
    fi
}

capture_proc "cmdline.txt" "/proc/cmdline"
capture_proc "cpuinfo.txt" "/proc/cpuinfo"
capture_proc "meminfo.txt" "/proc/meminfo"
capture_proc "iomem.txt" "/proc/iomem"
capture_proc "partitions.txt" "/proc/partitions"
capture_proc "mounts.txt" "/proc/mounts"
capture_proc "interrupts.txt" "/proc/interrupts"

{
    echo "== kernel =="
    uname -a 2>&1 || true
    echo
    echo "== device-tree model / compatible =="
    for prop in model compatible; do
        path="/sys/firmware/devicetree/base/$prop"
        if [ -r "$path" ]; then
            printf '%s: ' "$prop"
            tr '\000' '\n' < "$path"
        else
            echo "$prop: NOT_PRESENT"
        fi
    done
    echo
    echo "== block sysfs =="
    for node in "/sys/class/block/$BASE" /sys/class/block/"$BASE"p*; do
        [ -e "$node" ] || continue
        echo "-- $node --"
        for attr in dev size start partition ro removable uevent; do
            if [ -r "$node/$attr" ]; then
                echo "[$attr]"
                cat "$node/$attr"
            fi
        done
    done
    echo
    echo "== partition labels =="
    if [ -d /dev/disk/by-partlabel ]; then
        ls -l /dev/disk/by-partlabel 2>&1 || true
    else
        echo "NOT_PRESENT"
    fi
} > "$OUTDIR/device-and-block-layout.txt"

if [ -r /sys/firmware/fdt ]; then
    cat /sys/firmware/fdt > "$OUTDIR/live-fdt.dtb"
    echo "live_fdt=present" >> "$OUTDIR/summary.txt"
else
    echo "live_fdt=not-exported-by-kernel" >> "$OUTDIR/summary.txt"
fi

if command -v sfdisk >/dev/null 2>&1; then
    sfdisk --dump "$DISK" > "$OUTDIR/gpt-sfdisk.txt" 2>&1 || true
elif command -v fdisk >/dev/null 2>&1; then
    fdisk -l "$DISK" > "$OUTDIR/gpt-fdisk.txt" 2>&1 || true
else
    echo "Neither sfdisk nor fdisk is installed." > "$OUTDIR/gpt-tool-unavailable.txt"
fi

if command -v dmesg >/dev/null 2>&1; then
    dmesg 2>/dev/null | grep -iE 'sunxi|dram|memory|lcd|panel|backlight|axp|pmic|mmc|boot|uart|serial|reserved' \
        > "$OUTDIR/dmesg-hardware-lines.txt" || true
fi

# Read-only source offsets used by the BaseOS H700 firmware investigation:
# Boot0 copies at LBA 16 and 512; TOC1 at LBA 32800.
read_exact() {
    label="$1"
    lba="$2"
    sectors="$3"
    output="$4"
    expected=$((sectors * 512))
    dd if="$DISK" of="$OUTDIR/$output" bs=512 skip="$lba" count="$sectors" 2>"$OUTDIR/$label-dd.txt"
    actual="$(wc -c < "$OUTDIR/$output" | tr -d ' ')"
    [ "$actual" -eq "$expected" ] || {
        echo "error: short read for $label: got $actual bytes, expected $expected" >&2
        exit 1
    }
}

read_exact "boot0-primary" 16 128 "boot0-primary.bin"
read_exact "boot0-secondary" 512 128 "boot0-secondary.bin"
read_exact "toc1-package" 32800 2560 "toc1-package.bin"

# The U-Boot environment is a separate GPT partition on BaseOS cards. Capture
# only its first 64 KiB when a by-partlabel link is available; never guess p3.
ENV_PART="/dev/disk/by-partlabel/env"
if [ -b "$ENV_PART" ]; then
    dd if="$ENV_PART" of="$OUTDIR/env-partition-first64k.bin" bs=512 count=128 2>"$OUTDIR/env-dd.txt"
    actual="$(wc -c < "$OUTDIR/env-partition-first64k.bin" | tr -d ' ')"
    [ "$actual" -eq 65536 ] || {
        echo "error: short read from $ENV_PART" >&2
        exit 1
    }
    echo "env_partition_capture=first_64KiB" >> "$OUTDIR/summary.txt"
else
    echo "env_partition_capture=skipped_no_by-partlabel-env" >> "$OUTDIR/summary.txt"
fi

{
    echo "== SHA-256 of raw boot-chain captures =="
    sha256sum "$OUTDIR"/boot0-primary.bin "$OUTDIR"/boot0-secondary.bin "$OUTDIR"/toc1-package.bin
    if [ -f "$OUTDIR/env-partition-first64k.bin" ]; then
        sha256sum "$OUTDIR/env-partition-first64k.bin"
    fi
    if [ -f "$OUTDIR/live-fdt.dtb" ]; then
        sha256sum "$OUTDIR/live-fdt.dtb"
    fi
    echo
    echo "== Boot0 copy comparison =="
    if command -v cmp >/dev/null 2>&1; then
        if cmp -s "$OUTDIR/boot0-primary.bin" "$OUTDIR/boot0-secondary.bin"; then
            echo "identical"
        else
            echo "different (may include documented training/header mutations)"
        fi
    else
        echo "cmp utility unavailable"
    fi
} > "$OUTDIR/raw-sha256.txt"

echo "Read-only inventory completed."
echo "Output: $OUTDIR"
echo "No write was issued to $DISK."
