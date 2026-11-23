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
 * Configure PF0-PF5 for MMC0 using the pinned H616 upstream reference:
 * ALT2, 30 mA drive, weak pull-up. Only fields for PF0-PF5 are changed;
 * all other pins in those register words are preserved.
 *
 * This is not a decode or replay of the RG34XX vendor raw pinconf words.
 */
int sunxi_h616_sdc0_configure_pins(
    const struct sunxi_h616_pio_io *pio);

#endif /* H700_SUNXI_H616_SDC0_PINS_H */
