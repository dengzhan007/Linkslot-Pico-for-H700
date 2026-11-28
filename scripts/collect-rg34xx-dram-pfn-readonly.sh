#!/system/bin/sh
# Read-only metadata probe for the page containing stock BL31 DRAM source 0x40000000.
# This reads only /proc page metadata; it never reads/writes the DRAM page itself.
set -u

PFN_DEC=262144
EXPECTED_PAGE_SIZE=4096
KERNEL_RELEASE="$(uname -r 2>/dev/null || echo unknown)"

echo "RG34XX fixed DRAM-source PFN metadata probe"
echo "kernel_release=$KERNEL_RELEASE"
echo "physical_source_window=[0x40000000,0x40000140)"
echo "target_page=[0x40000000,0x40001000)"
echo "target_pfn=0x40000 decimal=$PFN_DEC"
echo "entry_bytes=8"
echo "note=raw metadata only; no DRAM contents, SMC, MMIO, or suspend operation"

UID_VALUE="$(id -u 2>/dev/null || echo unknown)"
echo "uid=$UID_VALUE"
if [ "$UID_VALUE" != "0" ]; then
    echo "probe_status=BLOCKED_ROOT_REQUIRED"
    exit 0
fi

PAGE_SIZE=""
if command -v getconf >/dev/null 2>&1; then
    PAGE_SIZE="$(getconf PAGESIZE 2>/dev/null || getconf PAGE_SIZE 2>/dev/null || true)"
fi
case "$PAGE_SIZE" in
    ''|*[!0-9]*) PAGE_SIZE="" ;;
esac
if [ -z "$PAGE_SIZE" ] && [ -r /proc/self/smaps ]; then
    PAGE_SIZE_KB="$(awk '/^KernelPageSize:/ { print $2; exit }' /proc/self/smaps 2>/dev/null || true)"
    case "$PAGE_SIZE_KB" in
        ''|*[!0-9]*) PAGE_SIZE="" ;;
        *) PAGE_SIZE=$((PAGE_SIZE_KB * 1024)) ;;
    esac
fi
echo "page_size_bytes=${PAGE_SIZE:-unknown}"
if [ "$PAGE_SIZE" != "$EXPECTED_PAGE_SIZE" ]; then
    echo "probe_status=BLOCKED_UNEXPECTED_OR_UNKNOWN_PAGE_SIZE"
    exit 0
fi
printf 'target_page_start=0x%08x\n' "$((PFN_DEC * PAGE_SIZE))"
printf 'kpage_metadata_file_offset_bytes=%u\n' "$((PFN_DEC * 8))"

read_page_metadata() {
    label="$1"
    path="$2"
    echo "[$label]"
    echo "path=$path"
    if [ ! -e "$path" ]; then
        echo "status=NOT_PRESENT"
        return
    fi
    if [ ! -r "$path" ]; then
        echo "status=NOT_READABLE"
        return
    fi
    if ! command -v dd >/dev/null 2>&1 || ! command -v od >/dev/null 2>&1; then
        echo "status=REQUIRED_DD_OR_OD_MISSING"
        return
    fi
    raw_hex="$(dd if="$path" bs=8 skip="$PFN_DEC" count=1 2>/dev/null | od -An -tx1 2>/dev/null | tr -d '[:space:]')"
    case "$raw_hex" in
        ''|*[!0-9a-fA-F]*)
            echo "status=READ_FAILED_OR_NON_HEX"
            return
            ;;
    esac
    if [ "${#raw_hex}" -ne 16 ]; then
        echo "status=SHORT_READ"
        echo "returned_hex_chars=${#raw_hex}"
        echo "returned_hex=$raw_hex"
        return
    fi
    echo "status=READ_OK"
    echo "raw_8_bytes_hex=$raw_hex"
    echo "decode=deferred_until_matching_kernel_4.9.170_flag_definitions_are_checked"
}

read_page_metadata "kpageflags" "/proc/kpageflags"
read_page_metadata "kpagecount" "/proc/kpagecount"

# The relocated BL31 retention analysis also identifies a second candidate
# source window at 0x60000000. Capture only its PFN metadata, never page bytes.
SECOND_PFN_DEC=393216
echo
echo "secondary_physical_source_window=[0x60000000,0x60000140)"
echo "secondary_target_page=[0x60000000,0x60001000)"
echo "secondary_target_pfn=0x60000 decimal=$SECOND_PFN_DEC"
echo "secondary_kpage_metadata_file_offset_bytes=$((SECOND_PFN_DEC * 8))"
read_secondary_metadata() {
    label="$1"
    path="$2"
    echo "[${label}_pfn_60000]"
    echo "path=$path"
    if [ ! -e "$path" ]; then
        echo "status=NOT_PRESENT"
        return
    fi
    if [ ! -r "$path" ] || ! command -v dd >/dev/null 2>&1 || ! command -v od >/dev/null 2>&1; then
        echo "status=NOT_READABLE_OR_REQUIRED_TOOL_MISSING"
        return
    fi
    raw_hex="$(dd if="$path" bs=8 skip="$SECOND_PFN_DEC" count=1 2>/dev/null | od -An -tx1 2>/dev/null | tr -d '[:space:]')"
    case "$raw_hex" in
        ''|*[!0-9a-fA-F]*) echo "status=READ_FAILED_OR_NON_HEX"; return ;;
    esac
    if [ "${#raw_hex}" -ne 16 ]; then
        echo "status=SHORT_READ"
        echo "returned_hex_chars=${#raw_hex}"
        return
    fi
    echo "status=READ_OK"
    echo "raw_8_bytes_hex=$raw_hex"
}
read_secondary_metadata "kpageflags" "/proc/kpageflags"
read_secondary_metadata "kpagecount" "/proc/kpagecount"

# Kernel ownership context only; these are metadata interfaces, not DRAM reads.
for proc_path in /proc/iomem /proc/zoneinfo /proc/buddyinfo /proc/pagetypeinfo /proc/meminfo; do
    section="$(basename "$proc_path")"
    echo
    echo "[kernel_memory_${section}]"
    if [ -r "$proc_path" ]; then
        echo "status=READ_OK"
        cat "$proc_path"
    elif [ -e "$proc_path" ]; then
        echo "status=NOT_READABLE"
    else
        echo "status=NOT_PRESENT"
    fi
done

echo "[page_owner_availability]"
if [ -r /sys/kernel/debug/page_owner ]; then
    echo "status=READABLE_NOT_DUMPED"
elif [ -e /sys/kernel/debug/page_owner ]; then
    echo "status=PRESENT_NOT_READABLE"
else
    echo "status=NOT_PRESENT"
fi
if [ -r /sys/kernel/debug/page_owner_stacks/show_stacks ]; then
    echo "page_owner_stacks=READABLE_NOT_DUMPED"
else
    echo "page_owner_stacks=NOT_PRESENT_OR_NOT_READABLE"
fi

echo "[kernel_config]"
CONFIG_FOUND=0
for config in "/proc/config.gz" "/boot/config-$KERNEL_RELEASE" "/lib/modules/$KERNEL_RELEASE/build/.config"; do
    [ -r "$config" ] || continue
    CONFIG_FOUND=1
    echo "config_source=$config"
    case "$config" in
        *.gz)
            if command -v gzip >/dev/null 2>&1; then
                gzip -dc "$config" 2>/dev/null | grep -E '^(CONFIG_PROC_PAGE_MONITOR|CONFIG_PAGE_OWNER|CONFIG_PAGE_EXTENSION|CONFIG_DEBUG_VM)=' || true
            else
                echo "gzip=NOT_PRESENT"
            fi
            ;;
        *)
            grep -E '^(CONFIG_PROC_PAGE_MONITOR|CONFIG_PAGE_OWNER|CONFIG_PAGE_EXTENSION|CONFIG_DEBUG_VM)=' "$config" || true
            ;;
    esac
done
if [ "$CONFIG_FOUND" -eq 0 ]; then
    echo "status=NO_READABLE_KERNEL_CONFIG"
fi

echo "probe_status=COMPLETE_READ_ONLY_METADATA_ONLY"
echo "safety=H700_MEMORY_LAYOUT_APPROVED remains unset; no ownership approval implied"
