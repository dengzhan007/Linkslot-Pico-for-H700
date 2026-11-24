#ifndef H700_SUNXI_MMC_BLOCK_DEVICE_H
#define H700_SUNXI_MMC_BLOCK_DEVICE_H

#include <stdint.h>

#include "h700_gpt_readonly.h"
#include "sunxi_mmc_clock.h"
#include "sunxi_mmc_pio_read.h"
#include "sunxi_sd_card_init.h"

struct h700_sunxi_mmc_block_timeouts {
    uint32_t command_timeout_ms;
    uint32_t fifo_timeout_ms;
    uint32_t data_timeout_ms;
    uint32_t recovery_timeout_ms;
};

struct h700_sunxi_mmc_block_device {
    struct sunxi_mmc_readonly_host host;
};

/*
 * Initialize a read-only GPT block-reader adapter from a successfully
 * selected SD card. Card capacity/addressing comes from the CSD/OCR result.
 *
 * The resulting reader exposes only 512-byte sector reads through CMD17.
 * It has no write, erase, flush, or arbitrary-command method. The `io` table
 * and `io->context` are retained by pointer in device->host and must remain
 * valid for the lifetime of the block device.
 */
int h700_sunxi_mmc_block_device_init(
    struct h700_sunxi_mmc_block_device *device,
    struct h700_block_reader *reader,
    const struct sunxi_mmc_io *io,
    const struct sunxi_sd_card_info *card,
    const struct h700_sunxi_mmc_block_timeouts *timeouts);

#endif /* H700_SUNXI_MMC_BLOCK_DEVICE_H */
