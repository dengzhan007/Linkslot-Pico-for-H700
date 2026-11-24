#ifndef H700_SUNXI_SD_CARD_INIT_H
#define H700_SUNXI_SD_CARD_INIT_H

#include <stdbool.h>
#include <stdint.h>

#include "sunxi_mmc_clock.h"
#include "sunxi_sd_csd.h"

#define SUNXI_SD_IDENTIFICATION_CLOCK_HZ 400000u
#define SUNXI_SD_OCR_READY               (1u << 31)
#define SUNXI_SD_OCR_CCS                 (1u << 30)

struct sunxi_sd_init_host {
    const struct sunxi_mmc_io *io;

    /*
     * Platform callback must configure the H616/H700 MMC module clock and
     * card output clock together. It must use the measured/effective PLL
     * parent rate and the common MASK_DATA0 clock-update path.
     *
     * Pinmux, card power, CCU bus gate/reset and MMIO mapping are platform
     * preconditions; this function does not guess or program their addresses.
     */
    void *clock_context;
    int (*set_clock_hz)(void *context, uint32_t target_hz,
                        uint32_t update_timeout_ms);

    uint32_t controller_reset_timeout_ms;
    uint32_t command_timeout_ms;
    uint32_t ocr_timeout_ms;
    uint32_t busy_timeout_ms;
    uint32_t clock_update_timeout_ms;
};

struct sunxi_sd_card_info {
    uint32_t ocr;
    uint32_t rca;
    uint32_t cid[4]; /* R2 order: bits 127..96 first. */
    uint32_t csd[4]; /* R2 order consumed by sunxi_sd_csd_parse(). */
    struct sunxi_sd_csd_info capacity;
};

enum sunxi_sd_init_result {
    SUNXI_SD_INIT_OK = 0,
    SUNXI_SD_INIT_INVALID = -1,
    SUNXI_SD_INIT_CONTROLLER_TIMEOUT = -2,
    SUNXI_SD_INIT_CLOCK_FAILED = -3,
    SUNXI_SD_INIT_COMMAND_TIMEOUT = -4,
    SUNXI_SD_INIT_CONTROLLER_ERROR = -5,
    SUNXI_SD_INIT_UNSUPPORTED_CARD = -6,
    SUNXI_SD_INIT_CARD_REJECTED = -7,
    SUNXI_SD_INIT_OCR_TIMEOUT = -8,
    SUNXI_SD_INIT_BUSY_TIMEOUT = -9,
    SUNXI_SD_INIT_CSD_ERROR = -10
};

/*
 * Identify and select an SD memory card using only non-writing commands:
 * CMD0, CMD8, CMD55/ACMD41, CMD2, CMD3, CMD9, CMD7 and, for SDSC, CMD16.
 *
 * SD v2.0 or later is required (CMD8 must echo 0x1AA). SDHC/SDXC and
 * byte-addressed SDSC are supported. SDUC and MMC/eMMC are rejected.
 *
 * The host must already have a mapped sdc0 register block, powered card,
 * configured PF0-PF5 pins, and enabled CCU bus/reset resources. The callback
 * establishes a 400 kHz identification clock. No block read/write or
 * filesystem access is performed here.
 */
enum sunxi_sd_init_result sunxi_sd_card_identify(
    const struct sunxi_sd_init_host *host,
    struct sunxi_sd_card_info *card);

#endif /* H700_SUNXI_SD_CARD_INIT_H */
