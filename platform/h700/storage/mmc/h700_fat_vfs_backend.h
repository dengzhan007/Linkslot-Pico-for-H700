#ifndef H700_FAT_VFS_BACKEND_H
#define H700_FAT_VFS_BACKEND_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "fat_ro.h"
#include "h700_sunxi_mmc_fat.h"
#include "h700_sunxi_sdc0_readonly_scan.h"
#include "linkslot_vfs.h"

#define H700_FAT_VFS_MAX_FILES 16u
#define H700_FAT_VFS_MAX_DIRS 8u
#define H700_FAT_VFS_PATH_CAPACITY 512u
#define H700_FAT_VFS_MOUNT_CAPACITY 96u

struct h700_fat_vfs_file_slot {
    uint32_t magic;
    uint8_t active;
    uint32_t position;
    fat_file file;
    char path[H700_FAT_VFS_PATH_CAPACITY];
};

struct h700_fat_vfs_dir_slot {
    uint32_t magic;
    uint8_t active;
    uint8_t virtual_root;
    uint8_t current_valid;
    uint8_t root_scan_done;
    uint32_t root_index;
    fat_dir directory;
    fat_dirent current;
    char path[H700_FAT_VFS_PATH_CAPACITY];
};

/*
 * The provider is static/process-lifetime storage. It owns the mounted FAT
 * volume and fixed VFS handle tables; no allocator is needed in the callbacks.
 *
 * The MMC device, the `sunxi_mmc_io` callback table stored in device->host.io,
 * and that table's context must also remain valid for the whole VFS lifetime.
 * The IO table is referenced, not copied. Calls are expected to be serialized
 * by the frontend; this provider adds no SMP lock around the shared MMC FIFO.
 */
struct h700_fat_vfs {
    uint32_t magic;
    char mount_prefix[H700_FAT_VFS_MOUNT_CAPACITY];
    fat_vol volume;
    struct h700_fat_vfs_file_slot files[H700_FAT_VFS_MAX_FILES];
    struct h700_fat_vfs_dir_slot directories[H700_FAT_VFS_MAX_DIRS];
};

/*
 * Mount a validated GPT partition through the H700 CMD17/FAT adapter, then
 * install the read-only Libretro VFS v3 backend with linkslot_vfs_install().
 *
 * mount_prefix is an absolute lexical prefix such as "/mnt/sdcard". Only
 * Games, BIOS, Saves, States, System and linkSlot are exposed. Relative paths,
 * traversal, sibling mount prefixes and write/update access are rejected.
 *
 * The provider, MMC device and mounted volume are process-lifetime state.
 * Installation is one-shot and must occur before retro_init().
 */
enum h700_fat_vfs_bootstrap_result {
    H700_FAT_VFS_BOOTSTRAP_OK = 0,
    H700_FAT_VFS_BOOTSTRAP_INVALID = -1,
    H700_FAT_VFS_BOOTSTRAP_SCAN_FAILED = -2,
    H700_FAT_VFS_BOOTSTRAP_PARTITION_NOT_FOUND = -3,
    H700_FAT_VFS_BOOTSTRAP_BLOCK_DEVICE_FAILED = -4,
    H700_FAT_VFS_BOOTSTRAP_INSTALL_FAILED = -5
};

/*
 * Complete read-only startup helper:
 *   sdc0 pin/clock/card initialization -> primary GPT scan -> exact GPT-name
 *   lookup -> persistent CMD17 block device -> bounded FAT mount -> VFS install.
 *
 * The caller must provide process-lifetime provider/device objects and a
 * GPT scratch array with H700_GPT_MAX_ENTRIES elements. The scratch array is
 * used synchronously and only needs to remain valid for this call.
 * "primary" is the expected RG34XX p7 label; callers may provide another
 * exact GPT name where the board layout differs. The scan result preserves
 * the card/GPT diagnostics. PIO/APB1, card power and MMIO mappings remain
 * explicit platform preconditions; no guessed regulator setup is performed.
 */
enum h700_fat_vfs_bootstrap_result h700_fat_vfs_scan_partition_and_install(
    struct h700_fat_vfs *provider,
    struct h700_sunxi_mmc_block_device *device,
    const struct h700_sunxi_sdc0_scan_config *scan_config,
    struct h700_gpt_partition *partitions,
    size_t partition_capacity,
    const char *partition_name,
    const char *mount_prefix,
    struct h700_sunxi_sdc0_scan_result *scan_result);

int h700_fat_vfs_mount_and_install(
    struct h700_fat_vfs *provider,
    struct h700_sunxi_mmc_block_device *device,
    const struct h700_gpt_partition *partition,
    const char *mount_prefix);

#endif /* H700_FAT_VFS_BACKEND_H */
