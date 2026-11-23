#include "h700_sunxi_sdc0_readonly_scan.h"

#include <string.h>

#include "h700_sunxi_mmc_block_device.h"

enum h700_sunxi_sdc0_scan_status h700_sunxi_sdc0_scan_primary_gpt(
    const struct h700_sunxi_sdc0_scan_config *config,
    struct h700_gpt_partition *partitions,
    size_t partition_capacity,
    struct h700_sunxi_sdc0_scan_result *result)
{
    struct sunxi_sd_init_host sd_host;
    struct h700_sunxi_mmc_block_device block_device;
    struct h700_block_reader reader;
    struct h700_sunxi_mmc_block_timeouts block_timeouts;
    enum sunxi_sd_init_result card_status;
    enum h700_gpt_result gpt_status;
    size_t clear_count;

    if (result != NULL)
        memset(result, 0, sizeof(*result));
    if (partitions != NULL) {
        clear_count = partition_capacity;
        if (clear_count > H700_GPT_MAX_ENTRIES)
            clear_count = H700_GPT_MAX_ENTRIES;
        memset(partitions, 0, clear_count * sizeof(*partitions));
    }

    if (config == NULL || config->pio == NULL ||
        config->platform == NULL || config->platform->mmc_io == NULL ||
        config->platform->mmc_io->read32 == NULL ||
        config->platform->mmc_io->write32 == NULL ||
        config->platform->mmc_io->time_ms == NULL ||
        config->platform->mmc_io->delay_us == NULL ||
        config->platform->mmc_io->read_barrier == NULL ||
        config->platform->ccu.read32 == NULL ||
        config->platform->ccu.write32 == NULL ||
        config->platform->osc24m_hz == 0u ||
        config->platform->pll6_effective_hz == 0u ||
        partitions == NULL || partition_capacity < H700_GPT_MAX_ENTRIES ||
        result == NULL ||
        config->controller_reset_timeout_ms == 0u ||
        config->command_timeout_ms == 0u ||
        config->ocr_timeout_ms == 0u ||
        config->card_busy_timeout_ms == 0u ||
        config->clock_update_timeout_ms == 0u ||
        config->fifo_timeout_ms == 0u ||
        config->data_timeout_ms == 0u ||
        config->recovery_timeout_ms == 0u)
        return H700_SDC0_SCAN_INVALID;

    /* PIO/APB1 and card power are explicit platform preconditions. */
    if (sunxi_h616_sdc0_configure_pins(config->pio) != 0)
        return H700_SDC0_SCAN_PIN_CONFIG_FAILED;

    if (sunxi_h616_sdc0_enable_bus(config->platform) != 0)
        return H700_SDC0_SCAN_BUS_ENABLE_FAILED;

    memset(&sd_host, 0, sizeof(sd_host));
    sd_host.io = config->platform->mmc_io;
    sd_host.clock_context = (void *)config->platform;
    sd_host.set_clock_hz = sunxi_h616_sdc0_set_clock_hz;
    sd_host.controller_reset_timeout_ms = config->controller_reset_timeout_ms;
    sd_host.command_timeout_ms = config->command_timeout_ms;
    sd_host.ocr_timeout_ms = config->ocr_timeout_ms;
    sd_host.busy_timeout_ms = config->card_busy_timeout_ms;
    sd_host.clock_update_timeout_ms = config->clock_update_timeout_ms;

    card_status = sunxi_sd_card_identify(&sd_host, &result->card);
    result->card_error = card_status;
    if (card_status != SUNXI_SD_INIT_OK)
        return H700_SDC0_SCAN_CARD_IDENTIFY_FAILED;

    block_timeouts.command_timeout_ms = config->command_timeout_ms;
    block_timeouts.fifo_timeout_ms = config->fifo_timeout_ms;
    block_timeouts.data_timeout_ms = config->data_timeout_ms;
    block_timeouts.recovery_timeout_ms = config->recovery_timeout_ms;

    if (h700_sunxi_mmc_block_device_init(
            &block_device, &reader, config->platform->mmc_io,
            &result->card, &block_timeouts) != 0)
        return H700_SDC0_SCAN_BLOCK_ADAPTER_FAILED;

    gpt_status = h700_gpt_read_primary(
        &reader, partitions, partition_capacity, &result->gpt);
    result->gpt_error = gpt_status;
    if (gpt_status != H700_GPT_OK)
        return H700_SDC0_SCAN_GPT_FAILED;

    return H700_SDC0_SCAN_OK;
}
