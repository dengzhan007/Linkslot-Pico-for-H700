#ifndef H700_FAT_VFS_BACKEND_H
#define H700_FAT_VFS_BACKEND_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "fat_ro.h"
#include "h700_sunxi_mmc_fat.h"
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
 * The MMC device supplied at mount time must also remain alive for the whole
 * VFS lifetime. Calls are expected to be serialized by the frontend; this
 * provider does not add an SMP lock around the shared MMC FIFO controller.
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
int h700_fat_vfs_mount_and_install(
    struct h700_fat_vfs *provider,
    struct h700_sunxi_mmc_block_device *device,
    const struct h700_gpt_partition *partition,
    const char *mount_prefix);

#endif /* H700_FAT_VFS_BACKEND_H */
