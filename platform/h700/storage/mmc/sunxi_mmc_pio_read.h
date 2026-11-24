#ifndef H700_SUNXI_MMC_PIO_READ_H
#define H700_SUNXI_MMC_PIO_READ_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "sunxi_mmc_clock.h"

#define SUNXI_MMC_PIO_SECTOR_SIZE 512u

struct sunxi_mmc_readonly_host {
    const struct sunxi_mmc_io *io;
    uint64_t sector_count;
    bool block_addressed; /* true for SDHC/SDXC, false for SDSC. */
    uint32_t command_timeout_ms;
    uint32_t fifo_timeout_ms;
    uint32_t data_timeout_ms;
    uint32_t recovery_timeout_ms;
};

enum sunxi_mmc_read_result {
    SUNXI_MMC_READ_OK = 0,
    SUNXI_MMC_READ_INVALID = -1,
    SUNXI_MMC_READ_RANGE = -2,
    SUNXI_MMC_READ_ALIGNMENT = -3,
    SUNXI_MMC_READ_TIMEOUT = -4,
    SUNXI_MMC_READ_CONTROLLER_ERROR = -5,
    SUNXI_MMC_READ_RECOVERY_FAILED = -6,
    SUNXI_MMC_READ_CARD_ERROR = -7
};

/*
 * Read one 512-byte sector with SD CMD17 and the AHB/PIO FIFO.
 *
 * Preconditions: controller and card initialization, RCA selection, card
 * clock, and bus width are already configured by the caller. The API has
 * no write, erase, multi-block-write, or arbitrary-command operation.
 *
 * The buffer must be at least 512 bytes and 4-byte aligned. response may be
 * NULL; otherwise it receives the CMD17 R1 response on success.
 */
enum sunxi_mmc_read_result sunxi_mmc_read_sector(
    const struct sunxi_mmc_readonly_host *host,
    uint64_t sector_lba,
    void *buffer,
    size_t buffer_capacity,
    uint32_t *response);

#endif /* H700_SUNXI_MMC_PIO_READ_H */
