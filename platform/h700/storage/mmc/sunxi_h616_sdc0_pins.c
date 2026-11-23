#include "sunxi_h616_sdc0_pins.h"

#define PIO_BANK_STRIDE       0x24u
#define PIO_MUX_REGS_OFFSET   0x00u
#define PIO_DRV_REGS_OFFSET   0x14u
#define PIO_PULL_REGS_OFFSET  0x1cu
#define PIO_PORT_F_INDEX      5u
#define PIO_MMC0_PINS         6u
#define PIO_MUX_ALT2          2u
#define PIO_DRIVE_30MA        2u
#define PIO_PULL_UP           1u

#define PIO_PORT_F_BASE (PIO_PORT_F_INDEX * PIO_BANK_STRIDE)
#define PIO_MUX0_OFFSET (PIO_PORT_F_BASE + PIO_MUX_REGS_OFFSET)
#define PIO_DRV0_OFFSET (PIO_PORT_F_BASE + PIO_DRV_REGS_OFFSET)
#define PIO_PULL0_OFFSET (PIO_PORT_F_BASE + PIO_PULL_REGS_OFFSET)

#define PIO_MUX_MASK   0x00ffffffu
#define PIO_MUX_VALUE  0x00222222u
#define PIO_DRV_MASK   0x00000fffu
#define PIO_DRV_VALUE  0x00000aaau
#define PIO_PULL_MASK  0x00000fffu
#define PIO_PULL_VALUE 0x00000555u

static int update_fields(const struct sunxi_h616_pio_io *pio,
                         uint32_t offset,
                         uint32_t mask,
                         uint32_t value)
{
    uint32_t current = pio->read32(pio->context, offset);
    uint32_t updated = (current & ~mask) | (value & mask);

    pio->write32(pio->context, offset, updated);
    return (pio->read32(pio->context, offset) & mask) == (value & mask)
               ? 0
               : -1;
}

int sunxi_h616_sdc0_configure_pins(
    const struct sunxi_h616_pio_io *pio)
{
    if (pio == 0 || pio->read32 == 0 || pio->write32 == 0)
        return -1;

    /* Set electrical characteristics first; switch mux to MMC0 last. */
    if (update_fields(pio, PIO_DRV0_OFFSET,
                      PIO_DRV_MASK, PIO_DRV_VALUE) != 0)
        return -1;
    if (update_fields(pio, PIO_PULL0_OFFSET,
                      PIO_PULL_MASK, PIO_PULL_VALUE) != 0)
        return -1;
    if (update_fields(pio, PIO_MUX0_OFFSET,
                      PIO_MUX_MASK, PIO_MUX_VALUE) != 0)
        return -1;

    return 0;
}
