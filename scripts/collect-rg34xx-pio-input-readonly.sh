#!/bin/sh
# Bounded, read-only RG34XX/H700 PA_DAT and PE_DAT capture.
# Never writes /dev/mem, GPIO, pinctrl, sysfs, or any hardware register.
set -eu
umask 077

usage() {
    echo "Usage: $0 [OUTPUT_DIR [SAMPLES]]" >&2
    echo "Default: /tmp/rg34xx-pio-input-<timestamp>, 240 samples at ~100 ms" >&2
    exit 2
}

[ "$#" -le 2 ] || usage
OUTDIR="${1:-}"
SAMPLES="${2:-240}"

case "$SAMPLES" in
    ''|*[!0-9]*) echo "error: SAMPLES must be an integer" >&2; exit 2 ;;
esac
[ "$SAMPLES" -ge 1 ] && [ "$SAMPLES" -le 5000 ] || {
    echo "error: SAMPLES must be between 1 and 5000" >&2
    exit 2
}

if [ -z "$OUTDIR" ]; then
    STAMP="$(date +%Y%m%d-%H%M%S 2>/dev/null || echo run)"
    OUTDIR="/tmp/rg34xx-pio-input-$STAMP"
fi
[ ! -e "$OUTDIR" ] || {
    echo "error: output path already exists: $OUTDIR" >&2
    exit 1
}

command -v id >/dev/null 2>&1 || { echo "error: id not found" >&2; exit 1; }
command -v grep >/dev/null 2>&1 || { echo "error: grep not found" >&2; exit 1; }
command -v dd >/dev/null 2>&1 || { echo "error: dd not found" >&2; exit 1; }
command -v od >/dev/null 2>&1 || { echo "error: od not found" >&2; exit 1; }
command -v wc >/dev/null 2>&1 || { echo "error: wc not found" >&2; exit 1; }
command -v tr >/dev/null 2>&1 || { echo "error: tr not found" >&2; exit 1; }
command -v sleep >/dev/null 2>&1 || { echo "error: sleep not found" >&2; exit 1; }

UID_VALUE="$(id -u)"
[ "$UID_VALUE" = "0" ] || {
    echo "error: run from a root ADB shell; no read attempt was made" >&2
    exit 1
}
[ -c /dev/mem ] && [ -r /dev/mem ] || {
    echo "error: readable character device /dev/mem is unavailable" >&2
    exit 1
}
[ -r /proc/iomem ] || {
    echo "error: /proc/iomem is unavailable; refusing MMIO read" >&2
    exit 1
}

# This is intentionally specific to the RG34XX live PIO resource observed
# in reports/rg34xx-inventory-20261125-163152/iomem.txt. A missing or changed
# resource name/range fails closed instead of probing an arbitrary address.
PIO_RESOURCE_PATTERN='^0300b000-0300b3ff[[:space:]]*:[[:space:]].*pinctrl@0300b000([[:space:]]|$)'
if ! grep -Eiq "$PIO_RESOURCE_PATTERN" /proc/iomem; then
    echo "error: /proc/iomem does not expose the expected 0x0300b000 PIO resource; refusing MMIO read" >&2
    exit 1
fi

mkdir -p "$OUTDIR"
trap 'rm -f "$OUTDIR/.pa-dat.tmp" "$OUTDIR/.pe-dat.tmp"' 0 HUP INT TERM

{
    echo "collector=rg34xx-pio-input-readonly-v1"
    echo "captured_at=$(date 2>/dev/null || echo unknown)"
    echo "uid=$UID_VALUE"
    echo "kernel=$(uname -a 2>&1 || echo unknown)"
    echo "samples_requested=$SAMPLES"
    echo "sample_sleep_seconds=0.1 (actual intervals are recorded from /proc/uptime)"
    echo "pio_base=0x0300b000"
    echo "pa_dat_address=0x0300b010"
    echo "pe_dat_address=0x0300b0a0"
    echo "read_method=dd if=/dev/mem; output files are report-local temporary files"
    echo "safety=read-only; no register, pinmux, pinconf, GPIO, EINT or GIC writes"
    echo
    echo "[matching /proc/iomem resource]"
    grep -Ei "$PIO_RESOURCE_PATTERN" /proc/iomem
} > "$OUTDIR/summary.txt"

PA_DAT_ADDRESS=0x0300b010
PE_DAT_ADDRESS=0x0300b0a0
PA_WORD_INDEX=$((PA_DAT_ADDRESS / 4))
PE_WORD_INDEX=$((PE_DAT_ADDRESS / 4))

read_pio_word() {
    word_index="$1"
    tmp="$2"
    error_file="$3"

    # The input is the fixed /dev/mem character device. The only output is a
    # four-byte temporary file inside OUTDIR; no device is ever an output.
    if ! dd if=/dev/mem of="$tmp" bs=4 skip="$word_index" count=1 2>"$error_file"; then
        echo "error: /dev/mem read failed at word index $word_index; see $error_file" >&2
        return 1
    fi
    byte_count="$(wc -c < "$tmp" | tr -d '[:space:]')"
    [ "$byte_count" = "4" ] || {
        echo "error: expected exactly 4 bytes from /dev/mem, got $byte_count" >&2
        return 1
    }
    value="$(od -An -tx4 "$tmp" | tr -d '[:space:]')"
    [ "${#value}" -eq 8 ] || {
        echo "error: invalid 32-bit register readback" >&2
        return 1
    }
    printf '%s' "$value"
}

echo "sample,uptime_s,pa_dat,pe_dat" > "$OUTDIR/pio-samples.csv"
echo "Capture running: press and release one physical gamepad key at a time." >&2
echo "Do not press Power. Hold each key briefly, release, then move to the next." >&2

sample=0
while [ "$sample" -lt "$SAMPLES" ]; do
    pa_value="$(read_pio_word "$PA_WORD_INDEX" "$OUTDIR/.pa-dat.tmp" "$OUTDIR/pa-read-error.txt")"
    pe_value="$(read_pio_word "$PE_WORD_INDEX" "$OUTDIR/.pe-dat.tmp" "$OUTDIR/pe-read-error.txt")"
    uptime_value=""
    IFS=' ' read -r uptime_value _ < /proc/uptime
    printf '%s,%s,0x%s,0x%s\n' "$sample" "$uptime_value" "$pa_value" "$pe_value" >> "$OUTDIR/pio-samples.csv"
    sample=$((sample + 1))
    [ "$sample" -ge "$SAMPLES" ] || sleep 0.1
done

{
    echo "samples_written=$sample"
    echo "csv=pio-samples.csv"
    echo "pa_word_index=$PA_WORD_INDEX"
    echo "pe_word_index=$PE_WORD_INDEX"
    echo "register_accesses_per_sample=2 (PA_DAT and PE_DAT only)"
} >> "$OUTDIR/summary.txt"
rm -f "$OUTDIR/.pa-dat.tmp" "$OUTDIR/.pe-dat.tmp"
echo "Capture complete: $OUTDIR" >&2
