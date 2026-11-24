#ifndef H700_SUNXI_MMC_FAT_H
#define H700_SUNXI_MMC_FAT_H

#include <stdint.h>

#include "fat_ro.h"
#include "h700_gpt_readonly.h"
#include "h700_sunxi_mmc_block_device.h"

/*
 * Read count 512-byte sectors through CMD17.
 *
 * The callback uses an aligned internal sector buffer, so it is safe for the
 * byte-oriented FAT reader's stack buffers, which are not required by C to be
 * 4-byte aligned. Returns the number of complete sectors copied.
 */
unsigned h700_sunxi_mmc_fat_read(
    void *context,
    uint32_t lba,
    uint32_t count,
    void *buffer);

/*
 * Mount one validated GPT partition as FAT16/FAT32/exFAT.
 *
 * The partition's first/last LBAs must fit the current 32-bit FAT block API
 * and remain inside the identified SD card capacity. The resulting fat_vol
 * is read-only (fat_mount_partition clears its write callback).
 */
int h700_sunxi_mmc_fat_mount_gpt_partition(
    struct h700_sunxi_mmc_block_device *device,
    fat_vol *volume,
    const struct h700_gpt_partition *partition);

#endif /* H700_SUNXI_MMC_FAT_H */
