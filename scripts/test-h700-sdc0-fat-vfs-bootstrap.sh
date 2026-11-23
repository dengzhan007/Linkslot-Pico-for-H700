#!/bin/sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
OUT="/tmp/h700-sdc0-fat-vfs-bootstrap-$$"
trap 'rm -rf "$OUT"' EXIT HUP INT TERM
mkdir -p "$OUT"

SOURCES="
$ROOT/platform/h700/storage/mmc/h700_fat_vfs_backend.c
$ROOT/platform/h700/storage/mmc/h700_sunxi_mmc_fat.c
$ROOT/platform/h700/storage/mmc/h700_sunxi_mmc_block_device.c
$ROOT/platform/h700/storage/mmc/h700_sunxi_sdc0_readonly_scan.c
$ROOT/platform/h700/storage/mmc/sunxi_h616_sdc0_pins.c
$ROOT/platform/h700/storage/mmc/sunxi_h616_sdc0_platform.c
$ROOT/platform/h700/storage/mmc/sunxi_mmc_clock.c
$ROOT/platform/h700/storage/mmc/sunxi_mmc_module_clock.c
$ROOT/platform/h700/storage/mmc/sunxi_mmc_pio_read.c
$ROOT/platform/h700/storage/mmc/sunxi_sd_card_init.c
$ROOT/platform/h700/storage/mmc/sunxi_sd_csd.c
$ROOT/platform/h700/storage/gpt/h700_gpt_readonly.c
$ROOT/platform/h700/storage/fat/fat_ro.c
$ROOT/platform/h700/storage/fat/fat_exfat.c
$ROOT/platform/h700/storage/mmc/tests/h700_sdc0_fat_vfs_bootstrap_test.c
"

build_variant()
{
    variant="$1"
    if [ "$variant" = "sanitized" ]; then
        flags="-O1 -g -fsanitize=address,undefined -fno-omit-frame-pointer"
    else
        flags="-O2"
    fi

    object_dir="$OUT/$variant"
    mkdir -p "$object_dir"

    # Compile each C translation unit separately. Keep the object list explicit
    # at link time instead of relying on a flattened source-list expansion.
    for source in $SOURCES; do
        filename="${source##*/}"
        object="$object_dir/${filename%.c}.o"
        # shellcheck disable=SC2086
        cc -std=c11 $flags -Wall -Wextra -Werror \
            -I"$ROOT/platform/h700/runtime/include" \
            -I"$ROOT/platform/h700/storage/mmc" \
            -I"$ROOT/platform/h700/storage/gpt" \
            -I"$ROOT/platform/h700/storage/fat" \
            -c "$source" -o "$object"
    done

    # Verify the two formerly unresolved dependencies are exported from their
    # dedicated objects before attempting the final executable link.
    nm -g "$object_dir/h700_sunxi_sdc0_readonly_scan.o" |
        grep -Eq '[[:space:]]T[[:space:]]h700_sunxi_sdc0_scan_primary_gpt$'
    nm -g "$object_dir/h700_gpt_readonly.o" |
        grep -Eq '[[:space:]]T[[:space:]]h700_gpt_find_unique_name$'

    set -- "$object_dir"/*.o
    # shellcheck disable=SC2086
    cc $flags "$@" -o "$object_dir/test-h700-sdc0-fat-vfs-bootstrap"
}

build_variant plain
"$OUT/plain/test-h700-sdc0-fat-vfs-bootstrap"

if build_variant sanitized; then
    ASAN_OPTIONS=detect_leaks=0 \
        "$OUT/sanitized/test-h700-sdc0-fat-vfs-bootstrap"
else
    echo "SKIP: sanitized build unavailable"
fi
