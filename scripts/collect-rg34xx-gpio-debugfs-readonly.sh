#!/bin/sh
# Bounded read-only sampling of Linux gpiolib's existing debugfs GPIO view.
# Does not mount debugfs, export/request GPIOs, or alter pin configuration.
set -eu
umask 077

usage() {
    echo "Usage: $0 [OUTPUT_DIR [SAMPLES]]" >&2
    echo "Default: /tmp/rg34xx-gpio-debugfs-<timestamp>, 600 samples at ~100 ms" >&2
    exit 2
}

[ "$#" -le 2 ] || usage
OUTDIR="${1:-}"
SAMPLES="${2:-600}"

case "$SAMPLES" in
    ''|*[!0-9]*) echo "error: SAMPLES must be an integer" >&2; exit 2 ;;
esac
[ "$SAMPLES" -ge 1 ] && [ "$SAMPLES" -le 3600 ] || {
    echo "error: SAMPLES must be between 1 and 3600" >&2
    exit 2
}

if [ -z "$OUTDIR" ]; then
    STAMP="$(date +%Y%m%d-%H%M%S 2>/dev/null || echo run)"
    OUTDIR="/tmp/rg34xx-gpio-debugfs-$STAMP"
fi
[ ! -e "$OUTDIR" ] || {
    echo "error: output path already exists: $OUTDIR" >&2
    exit 1
}

command -v id >/dev/null 2>&1 || { echo "error: id not found" >&2; exit 1; }
command -v cat >/dev/null 2>&1 || { echo "error: cat not found" >&2; exit 1; }
command -v sleep >/dev/null 2>&1 || { echo "error: sleep not found" >&2; exit 1; }
[ -r /proc/uptime ] || { echo "error: /proc/uptime unavailable" >&2; exit 1; }

UID_VALUE="$(id -u)"
[ "$UID_VALUE" = "0" ] || {
    echo "error: run from a root ADB shell; no reads were attempted" >&2
    exit 1
}

GPIO_DEBUG="/sys/kernel/debug/gpio"
[ -r "$GPIO_DEBUG" ] || {
    echo "error: $GPIO_DEBUG is absent or unreadable; collector will not mount debugfs" >&2
    exit 1
}

mkdir -p "$OUTDIR"
{
    echo "collector=rg34xx-gpio-debugfs-readonly-v1"
    echo "captured_at=$(date 2>/dev/null || echo unknown)"
    echo "kernel=$(uname -a 2>&1 || echo unknown)"
    echo "uid=$UID_VALUE"
    echo "source=$GPIO_DEBUG"
    echo "samples_requested=$SAMPLES"
    echo "nominal_sleep_seconds=0.1"
    echo "actual_sample_times=recorded from /proc/uptime"
    echo "access=read-only cat of existing gpiolib debugfs file"
    echo "no_debugfs_mount=1"
    echo "no_gpio_export_or_request=1"
    echo "no_pinmux_pinconf_gpio_eint_gic_writes=1"
} > "$OUTDIR/summary.txt"

# A one-shot preflight proves the debugfs show callback can be read before
# opening the timed capture. This file is kept separately from timed samples.
if ! cat "$GPIO_DEBUG" > "$OUTDIR/preflight-gpio.txt"; then
    echo "error: initial read of $GPIO_DEBUG failed" >&2
    exit 1
fi

echo "Capture running: press and release one physical gamepad key at a time." >&2
echo "Start with idle, then test each key individually. Do not press Power." >&2
echo "Each sample re-reads only $GPIO_DEBUG through the existing kernel GPIO driver." >&2

SAMPLE_FILE="$OUTDIR/gpio-debugfs-samples.txt"
: > "$SAMPLE_FILE"

sample=0
while [ "$sample" -lt "$SAMPLES" ]; do
    uptime_value=""
    IFS=' ' read -r uptime_value _ < /proc/uptime
    printf '\n=== sample=%s uptime_s=%s ===\n' "$sample" "$uptime_value" >> "$SAMPLE_FILE"
    if ! cat "$GPIO_DEBUG" >> "$SAMPLE_FILE"; then
        echo "error: read of $GPIO_DEBUG failed at sample $sample" >&2
        exit 1
    fi
    sample=$((sample + 1))
    [ "$sample" -ge "$SAMPLES" ] || sleep 0.1
done

{
    echo "samples_written=$sample"
    echo "preflight=preflight-gpio.txt"
    echo "samples=gpio-debugfs-samples.txt"
    echo "sample_sleep_seconds=0.1 (nominal; cat overhead adds latency)"
} >> "$OUTDIR/summary.txt"

echo "Capture complete: $OUTDIR" >&2
