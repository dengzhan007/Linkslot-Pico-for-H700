#include "h700_sunxi_mmc_block_device.h"

#include "sunxi_mmc_pio_read.h"

static int read_sector_callback(void *context, uint64_t lba, void *buffer)
{
    struct h700_sunxi_mmc_block_device *device =
        (struct h700_sunxi_mmc_block_device *)context;
    enum sunxi_mmc_read_result result;

    if (device == 0 || buffer == 0)
        return -1;

    result = sunxi_mmc_read_sector(&device->host, lba, buffer,
                                   H700_GPT_SECTOR_SIZE, 0);
    return result == SUNXI_MMC_READ_OK ? 0 : -1;
}

int h700_sunxi_mmc_block_device_init(
    struct h700_sunxi_mmc_block_device *device,
    struct h700_block_reader *reader,
    const struct sunxi_mmc_io *io,
    const struct sunxi_sd_card_info *card,
    const struct h700_sunxi_mmc_block_timeouts *timeouts)
{
    if (device == 0 || reader == 0 || io == 0 || card == 0 ||
        timeouts == 0 || card->capacity.sector_count == 0u ||
        timeouts->command_timeout_ms == 0u ||
        timeouts->fifo_timeout_ms == 0u ||
        timeouts->data_timeout_ms == 0u ||
        timeouts->recovery_timeout_ms == 0u)
        return -1;

    device->host.io = io;
    device->host.sector_count = card->capacity.sector_count;
    device->host.block_addressed = card->capacity.block_addressed;
    device->host.command_timeout_ms = timeouts->command_timeout_ms;
    device->host.fifo_timeout_ms = timeouts->fifo_timeout_ms;
    device->host.data_timeout_ms = timeouts->data_timeout_ms;
    device->host.recovery_timeout_ms = timeouts->recovery_timeout_ms;

    reader->context = device;
    reader->sector_count = device->host.sector_count;
    reader->sector_size = SUNXI_MMC_PIO_SECTOR_SIZE;
    reader->read_sector = read_sector_callback;

    return 0;
}
