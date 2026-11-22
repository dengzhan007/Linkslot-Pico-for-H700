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
ROOTARG=""

# Always record the root= argument, even when the whole-disk device was
# supplied explicitly by the Windows ADB wrapper.
for arg in $(cat /proc/cmdline 2>/dev/null || true); do
    case "$arg" in
        root=*) ROOTARG="${arg#root=}" ;;
    esac
done
case "$ROOTARG" in
    /dev/*) ROOTDEV="$ROOTARG" ;;
esac

if [ -z "$DISK" ]; then
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
    echo "root_argument_from_cmdline=$ROOTARG"
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

# Capture the kernel page size so CMA PFNs can be converted to byte ranges.
# getconf is preferred; older Android userspaces may omit it, so fall back to
# the KernelPageSize reported for this process in /proc/self/smaps.
PAGE_SIZE=""
PAGE_SIZE_SOURCE=""
if command -v getconf >/dev/null 2>&1; then
    PAGE_SIZE="$(getconf PAGESIZE 2>/dev/null || getconf PAGE_SIZE 2>/dev/null || true)"
    case "$PAGE_SIZE" in
        ''|*[!0-9]*) PAGE_SIZE="" ;;
        *) PAGE_SIZE_SOURCE="getconf" ;;
    esac
fi
if [ -z "$PAGE_SIZE" ] && [ -r /proc/self/smaps ]; then
    PAGE_SIZE_KB="$(awk '/^KernelPageSize:/ { print $2; exit }' /proc/self/smaps 2>/dev/null || true)"
    case "$PAGE_SIZE_KB" in
        ''|*[!0-9]*) PAGE_SIZE="" ;;
        *)
            PAGE_SIZE=$((PAGE_SIZE_KB * 1024))
            PAGE_SIZE_SOURCE="proc-self-smaps"
            ;;
    esac
fi
case "$PAGE_SIZE" in
    ''|*[!0-9]*)
        echo "NOT_PRESENT: getconf and /proc/self/smaps unavailable" > "$OUTDIR/page-size.txt"
        echo "not-present" > "$OUTDIR/page-size-source.txt"
        ;;
    *)
        echo "$PAGE_SIZE" > "$OUTDIR/page-size.txt"
        echo "$PAGE_SIZE_SOURCE" > "$OUTDIR/page-size-source.txt"
        ;;
esac

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
    dmesg 2>/dev/null | grep -iE 'sunxi|dram|memory|lcd|panel|backlight|axp|pmic|mmc|boot|uart|serial|reserved|cma|memblock|framebuffer|boot_fb|disp_reserve|drm config service|drm_base|drm_size|tee_base|secure|tee|bl31' \
        > "$OUTDIR/dmesg-hardware-lines.txt" || true
fi

# Capture read-only runtime evidence for the vendor ION/TEE path. In
# particular, some Allwinner kernels populate the secure ION heap through a
# vendor SMC even when the standard OP-TEE API probe fails. Never invoke an
# ioctl, SMC, debugfs control file, or any write operation from this collector.
{
    echo "== kernel identity =="
    uname -a 2>&1 || true
    echo
    echo "== kernel configuration (when exported) =="
    CONFIG_CAPTURED=0
    for config in /proc/config.gz "/boot/config-$(uname -r 2>/dev/null)" /boot/config; do
        [ -r "$config" ] || continue
        echo "-- $config --"
        case "$config" in
            *.gz)
                if command -v gzip >/dev/null 2>&1; then
                    gzip -dc "$config" 2>/dev/null | grep -E '^(CONFIG_(TEE|OPTEE|ION|ION_SUNXI|ION_OF|CMA|DMA_CMA|OF_RESERVED_MEM)=|# CONFIG_(TEE|OPTEE|ION|ION_SUNXI|ION_OF|CMA|DMA_CMA|OF_RESERVED_MEM) is not set)' || true
                    CONFIG_CAPTURED=1
                else
                    echo "gzip utility unavailable; config not decoded"
                fi
                ;;
            *)
                grep -E '^(CONFIG_(TEE|OPTEE|SUNXI_ION|ION|CMA|DMA_CMA|OF_RESERVED_MEM)=|# CONFIG_(TEE|OPTEE|SUNXI_ION|ION|CMA|DMA_CMA|OF_RESERVED_MEM) is not set)' "$config" || true
                CONFIG_CAPTURED=1
                ;;
        esac
    done
    [ "$CONFIG_CAPTURED" -eq 1 ] || echo "NOT_PRESENT: no readable kernel config export"
    echo
    echo "== relevant built-in ION / DRM symbols (names only) =="
    if [ -r /proc/kallsyms ]; then
        awk '
            $3 == "sunxi_ion_probe" ||
            $3 == "optee_probe_drm_configure" ||
            $3 == "ion_secure_heap_create" {
                print "[address redacted] " $3
                found[$3] = 1
            }
            END {
                if (!found["sunxi_ion_probe"])
                    print "NOT_FOUND_OR_HIDDEN: sunxi_ion_probe"
                if (!found["optee_probe_drm_configure"])
                    print "NOT_FOUND_OR_HIDDEN: optee_probe_drm_configure"
                if (!found["ion_secure_heap_create"])
                    print "NOT_FOUND_OR_HIDDEN: ion_secure_heap_create"
            }
        ' /proc/kallsyms
    else
        echo "NOT_PRESENT: /proc/kallsyms"
    fi
    for module in ion_sunxi optee; do
        if [ -d "/sys/module/$module" ]; then
            echo "MODULE_SYSFS_PRESENT: $module"
        else
            echo "MODULE_SYSFS_NOT_PRESENT: $module"
        fi
    done
    echo
    echo "== ION device and runtime debugfs =="
    for path in /dev/ion /dev/tee0 /dev/teepriv0 /sys/class/misc/ion /sys/class/tee /sys/kernel/debug/ion /sys/kernel/debug/ion/heaps /sys/kernel/debug/ion/clients /sys/bus/platform/drivers/ion-sunxi; do
        if [ -e "$path" ]; then
            ls -ld "$path" 2>&1 || true
        else
            echo "NOT_PRESENT: $path"
        fi
    done
    for heap in sys_user cma secure; do
        path="/sys/kernel/debug/ion/heaps/$heap"
        if [ -r "$path" ]; then
            echo "-- $path --"
            cat "$path" 2>&1 || true
        else
            echo "NOT_READABLE_OR_ABSENT: $path"
        fi
    done
    echo
    echo "== loaded ION / TEE modules =="
    if [ -r /proc/modules ]; then
        grep -iE 'ion|tee|optee' /proc/modules || echo "NO_MATCHING_MODULES (drivers may be built-in)"
    else
        echo "NOT_PRESENT: /proc/modules"
    fi
    echo
    echo "== platform driver binding =="
    if [ -d /sys/bus/platform/drivers/ion-sunxi ]; then
        ls -la /sys/bus/platform/drivers/ion-sunxi 2>&1 || true
    else
        echo "NOT_PRESENT: /sys/bus/platform/drivers/ion-sunxi"
    fi
    echo
    echo "== ION / TEE / DRM / secure / CMA kernel log lines =="
    if command -v dmesg >/dev/null 2>&1; then
        dmesg 2>/dev/null | grep -iE 'Ion:|ion[-_ ]|drm config service|drm_base|drm_size|tee_base|optee|secure|cma|reserved|BL31|ATF|trusted|api uid' || true
    else
        echo "NOT_PRESENT: dmesg utility"
    fi
} > "$OUTDIR/tee-ion-runtime.txt"

# Capture physical CMA/memblock layout when the running kernel exposes it via
# debugfs. These are reads only; do not touch the CMA alloc/free control files.
{
    echo "== CMA debugfs =="
    if [ -d /sys/kernel/debug/cma ]; then
        for node in /sys/kernel/debug/cma/*; do
            [ -d "$node" ] || continue
            echo "-- $node --"
            for attr in base_pfn count order_per_bit used; do
                if [ -r "$node/$attr" ]; then
                    echo "[$attr]"
                    cat "$node/$attr"
                fi
            done
            for range in "$node"/ranges/*; do
                [ -d "$range" ] || continue
                echo "-- $range --"
                for attr in base_pfn count; do
                    if [ -r "$range/$attr" ]; then
                        echo "[$attr]"
                        cat "$range/$attr"
                    fi
                done
            done
        done
    else
        echo "NOT_PRESENT: /sys/kernel/debug/cma"
    fi
    echo
    echo "== memblock debugfs =="
    if [ -d /sys/kernel/debug/memblock ]; then
        for name in memory reserved; do
            file="/sys/kernel/debug/memblock/$name"
            if [ -r "$file" ]; then
                echo "[$name]"
                cat "$file"
            else
                echo "NOT_PRESENT: $file"
            fi
        done
    else
        echo "NOT_PRESENT: /sys/kernel/debug/memblock"
    fi
} > "$OUTDIR/debugfs-memory-reservations.txt"

# Read-only source offsets used by the BaseOS H700 firmware investigation:
# Boot0 copies at LBA 16 and 512; TOC1 at LBA 32800.
# Capture 128 KiB for each Boot0 copy: some H700 cards declare 0x10000 bytes,
# while others declare 0x20000. The inspector crops to the header-declared
# size before validating the checksum, so the extra tail is harmless.
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

read_exact "boot0-primary" 16 256 "boot0-primary.bin"
read_exact "boot0-secondary" 512 256 "boot0-secondary.bin"
read_exact "toc1-package" 32800 2560 "toc1-package.bin"

# Find the environment partition by its exact GPT PARTNAME, not by a
# hard-coded partition number. Older userspaces may omit /dev/disk/by-partlabel.
ENV_PART=""
ENV_SOURCE=""
ENV_MATCHES=0
ENV_PART_SIZE=0
for node in /sys/class/block/"$BASE"p*; do
    [ -r "$node/uevent" ] || continue
    if grep -qx 'PARTNAME=env' "$node/uevent"; then
        ENV_MATCHES=$((ENV_MATCHES + 1))
        ENV_DEVNAME="$(sed -n 's/^DEVNAME=//p' "$node/uevent")"
        ENV_PART_SIZE="$(cat "$node/size" 2>/dev/null || echo 0)"
        case "$ENV_DEVNAME" in
            "$BASE"p[0-9]*) ;;
            *)
                echo "error: env PARTNAME resolved to an unexpected device: $ENV_DEVNAME" >&2
                exit 1
                ;;
        esac
        ENV_PART="/dev/$ENV_DEVNAME"
        ENV_SOURCE="sysfs PARTNAME=env"
    fi
done

[ "$ENV_MATCHES" -le 1 ] || {
    echo "error: multiple partitions advertise PARTNAME=env" >&2
    exit 1
}

# Compatibility fallback for systems exposing the by-partlabel symlink but
# not partition uevent metadata. The link name is semantic; no pN is assumed.
if [ "$ENV_MATCHES" -eq 0 ] && [ -b /dev/disk/by-partlabel/env ]; then
    ENV_PART="/dev/disk/by-partlabel/env"
    ENV_SOURCE="by-partlabel/env"
    if command -v blockdev >/dev/null 2>&1; then
        ENV_PART_SIZE="$(blockdev --getsz "$ENV_PART" 2>/dev/null || echo 0)"
    fi
fi

if [ -n "$ENV_PART" ]; then
    [ -b "$ENV_PART" ] || {
        echo "error: discovered env partition is not a block device: $ENV_PART" >&2
        exit 1
    }
    case "$ENV_PART_SIZE" in
        ''|*[!0-9]*)
            echo "error: invalid env partition size: $ENV_PART_SIZE" >&2
            exit 1
            ;;
    esac
    [ "$ENV_PART_SIZE" -ge 128 ] || {
        echo "error: env partition is smaller than 64 KiB or its size is unknown" >&2
        exit 1
    }
    # Capture the complete environment partition. H700 firmware commonly
    # builds env images at 16 MiB; a 64 KiB prefix is not enough to validate
    # the CRC or establish the exact environment image size.
    ENV_MAX_SECTORS=131072
    [ "$ENV_PART_SIZE" -le "$ENV_MAX_SECTORS" ] || {
        echo "error: env partition is larger than the 64 MiB safety limit: $ENV_PART_SIZE sectors" >&2
        exit 1
    }
    ENV_BYTES=$((ENV_PART_SIZE * 512))
    dd if="$ENV_PART" of="$OUTDIR/env-partition.bin" bs=512 count="$ENV_PART_SIZE" 2>"$OUTDIR/env-dd.txt"
    actual="$(wc -c < "$OUTDIR/env-partition.bin" | tr -d ' ')"
    [ "$actual" -eq "$ENV_BYTES" ] || {
        echo "error: short read from $ENV_PART: got $actual bytes, expected $ENV_BYTES" >&2
        exit 1
    }
    echo "env_partition_capture=complete source=$ENV_SOURCE device=${ENV_PART##*/} bytes=$ENV_BYTES sectors=$ENV_PART_SIZE" >> "$OUTDIR/summary.txt"
else
    echo "env_partition_capture=skipped_no_PARTNAME_env" >> "$OUTDIR/summary.txt"
fi

{
    echo "== SHA-256 of raw boot-chain captures =="
    sha256sum "$OUTDIR"/boot0-primary.bin "$OUTDIR"/boot0-secondary.bin "$OUTDIR"/toc1-package.bin
    if [ -f "$OUTDIR/env-partition.bin" ]; then
        sha256sum "$OUTDIR/env-partition.bin"
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
