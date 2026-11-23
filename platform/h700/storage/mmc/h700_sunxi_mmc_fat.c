#include "h700_sunxi_mmc_fat.h"

#include "sunxi_mmc_pio_read.h"

static void copy_bytes(uint8_t *destination,
                       const uint8_t *source,
                       uint32_t length)
{
    uint32_t i;
    for (i = 0u; i < length; ++i)
        destination[i] = source[i];
}

unsigned h700_sunxi_mmc_fat_read(
    void *context,
    uint32_t lba,
    uint32_t count,
    void *buffer)
{
    struct h700_sunxi_mmc_block_device *device =
        (struct h700_sunxi_mmc_block_device *)context;
    uint8_t *destination = (uint8_t *)buffer;
    uint32_t completed = 0u;

    if (count == 0u)
        return 0u;
    if (device == 0 || buffer == 0 || device->host.io == 0 ||
        device->host.sector_count == 0u)
        return 0u;

    /* lba/count are 32-bit in fat_read_fn; reject wrap and capacity overrun. */
    if ((uint64_t)lba + count > device->host.sector_count ||
        (uint64_t)lba + count > 0x100000000ull)
        return 0u;

    while (completed < count) {
        _Alignas(4) uint8_t sector[SUNXI_MMC_PIO_SECTOR_SIZE];
        enum sunxi_mmc_read_result result;

        result = sunxi_mmc_read_sector(
            &device->host, (uint64_t)lba + completed,
            sector, sizeof(sector), 0);
        if (result != SUNXI_MMC_READ_OK)
            break;

        copy_bytes(destination + completed * SUNXI_MMC_PIO_SECTOR_SIZE,
                   sector, SUNXI_MMC_PIO_SECTOR_SIZE);
        ++completed;
    }

    return completed;
}

int h700_sunxi_mmc_fat_mount_gpt_partition(
    struct h700_sunxi_mmc_block_device *device,
    fat_vol *volume,
    const struct h700_gpt_partition *partition)
{
    uint64_t partition_sectors;

    if (device == 0 || volume == 0 || partition == 0 ||
        device->host.sector_count == 0u ||
        partition->first_lba > 0xffffffffull ||
        partition->last_lba > 0xffffffffull ||
        partition->last_lba < partition->first_lba ||
        partition->last_lba >= device->host.sector_count)
        return -1;

    partition_sectors = partition->last_lba - partition->first_lba + 1u;
    if (partition_sectors == 0u || partition_sectors > 0xffffffffull)
        return -1;

    return fat_mount_partition(
        volume, h700_sunxi_mmc_fat_read, device,
        (uint32_t)partition->first_lba, (uint32_t)partition_sectors);
}
