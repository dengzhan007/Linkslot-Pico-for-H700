#ifndef H700_SUNXI_H616_SDC0_PINS_H
#define H700_SUNXI_H616_SDC0_PINS_H

#include <stdint.h>

/*
 * PIO callbacks use offsets relative to the H616 PIO base 0x0300b000.
 * The PIO/APB1 clock must already be enabled by the platform.
 */
struct sunxi_h616_pio_io {
    void *context;
    uint32_t (*read32)(void *context, uint32_t offset);
    void (*write32)(void *context, uint32_t offset, uint32_t value);
};

/*
 * Configure RG34XX PF0-PF5 for sdc0 using the live sun50iw9p1 pinconf
 * capture: ALT2, 40 mA drive, weak pull-up. The vendor packed pinconf
 * values 0x00002809 / 0x00000005 decode as DRIVE_STRENGTH(40 mA) /
 * BIAS_PULL_UP using the sun50iw9p1 PIO driver encoding.
 *
 * Only PF0-PF5 fields are changed; all unrelated register fields remain
 * preserved. This helper remains host-tested and is not yet a live MMIO test.
 */
int sunxi_h616_sdc0_configure_pins(
    const struct sunxi_h616_pio_io *pio);

#endif /* H700_SUNXI_H616_SDC0_PINS_H */
