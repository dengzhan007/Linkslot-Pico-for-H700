#ifndef H700_SUNXI_H616_SDC0_PLATFORM_H
#define H700_SUNXI_H616_SDC0_PLATFORM_H

#include <stdint.h>

#include "sunxi_mmc_clock.h"

struct sunxi_h616_ccu_io {
    void *context;
    uint32_t (*read32)(void *context, uint32_t offset);
    void (*write32)(void *context, uint32_t offset, uint32_t value);
};

struct sunxi_h616_sdc0_platform {
    const struct sunxi_mmc_io *mmc_io;
    struct sunxi_h616_ccu_io ccu;
    uint32_t osc24m_hz;
    uint32_t pll6_effective_hz;
};

/*
 * Configure the RG34XX sun50iw9p1 APB1 parent/divider used by clk_pio.
 * The vendor clock driver maps the PIO clock to APB1; select the 24 MHz HOSC
 * directly. This pinctrl clock path has no separate PIO gate register.
 */
int sunxi_h616_pio_enable_apb1_24m(
    const struct sunxi_h616_sdc0_platform *platform);

/*
 * Enable the sdc0 AHB gate and deassert its bus reset.
 *
 * The caller maps the RG34XX sun50iw9p1 CCU at 0x03001000 and passes
 * callbacks that use CCU-relative offsets. No pinmux, regulator or
 * card-power change is made.
 */
int sunxi_h616_sdc0_enable_bus(
    const struct sunxi_h616_sdc0_platform *platform);

/*
 * Configure a conservative SD identification clock.
 *
 * This currently rejects rates above 24 MHz until sampling/output delay and
 * calibration policy have been validated on RG34XX. It programs the MMC0
 * module-clock register, clears the controller's internal divider, then
 * enables the card clock using the H616 MASK_DATA0 update protocol.
 *
 * Signature intentionally matches sunxi_sd_init_host.set_clock_hz().
 */
int sunxi_h616_sdc0_set_clock_hz(
    void *context,
    uint32_t target_hz,
    uint32_t update_timeout_ms);

#endif /* H700_SUNXI_H616_SDC0_PLATFORM_H */
