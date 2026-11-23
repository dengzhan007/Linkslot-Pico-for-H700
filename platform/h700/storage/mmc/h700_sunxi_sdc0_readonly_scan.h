#ifndef H700_SUNXI_SDC0_READONLY_SCAN_H
#define H700_SUNXI_SDC0_READONLY_SCAN_H

#include <stddef.h>
#include <stdint.h>

#include "h700_gpt_readonly.h"
#include "sunxi_h616_sdc0_platform.h"
#include "sunxi_h616_sdc0_pins.h"
#include "sunxi_sd_card_init.h"

struct h700_sunxi_sdc0_scan_config {
    const struct sunxi_h616_pio_io *pio;
    const struct sunxi_h616_sdc0_platform *platform;

    uint32_t controller_reset_timeout_ms;
    uint32_t command_timeout_ms;
    uint32_t ocr_timeout_ms;
    uint32_t card_busy_timeout_ms;
    uint32_t clock_update_timeout_ms;
    uint32_t fifo_timeout_ms;
    uint32_t data_timeout_ms;
    uint32_t recovery_timeout_ms;
};

struct h700_sunxi_sdc0_scan_result {
    struct sunxi_sd_card_info card;
    struct h700_gpt_table gpt;
    enum sunxi_sd_init_result card_error;
    enum h700_gpt_result gpt_error;
};

enum h700_sunxi_sdc0_scan_status {
    H700_SDC0_SCAN_OK = 0,
    H700_SDC0_SCAN_INVALID = -1,
    H700_SDC0_SCAN_PIO_CLOCK_FAILED = -2,
    H700_SDC0_SCAN_PIN_CONFIG_FAILED = -3,
    H700_SDC0_SCAN_BUS_ENABLE_FAILED = -4,
    H700_SDC0_SCAN_CARD_IDENTIFY_FAILED = -5,
    H700_SDC0_SCAN_BLOCK_ADAPTER_FAILED = -6,
    H700_SDC0_SCAN_GPT_FAILED = -7
};

/*
 * Run the complete read-only sdc0 bring-up and primary-GPT scan:
 *   APB1 PIO clock -> PF0-PF5 reference pin configuration -> MMC0 gate/reset ->
 *   400 kHz clock + SD v2 identification -> CMD17 block reader -> GPT parser.
 *
 * Board preconditions:
 * - The caller has mapped the H616 PIO, CCU and sdc0 register blocks and
 *   supplied valid offset-based callbacks.
 * - PIO, CCU and MMC register blocks are mapped. This routine configures APB1
 *   to 24 MHz from OSC24M before applying the PF0-PF5 pin group.
 * - The SD card is powered and stable.
 * - The initial identification path uses the confirmed 24 MHz HOSC and does not require a PLL-rate value. The platform wrapper rejects rates above 24 MHz until the sun50iw9p1 `pll_periph1x2` path is validated.
 *
 * This routine only changes controller/clock/pin registers. Its storage
 * protocol sends identification and CMD17 read commands only. It has no
 * SD write/erase or boot-area access path.
 */
enum h700_sunxi_sdc0_scan_status h700_sunxi_sdc0_scan_primary_gpt(
    const struct h700_sunxi_sdc0_scan_config *config,
    struct h700_gpt_partition *partitions,
    size_t partition_capacity,
    struct h700_sunxi_sdc0_scan_result *result);

#endif /* H700_SUNXI_SDC0_READONLY_SCAN_H */
