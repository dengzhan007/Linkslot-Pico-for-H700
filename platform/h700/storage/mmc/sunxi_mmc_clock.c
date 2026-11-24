#include "sunxi_mmc_clock.h"

#include "sunxi_mmc_regs.h"

static int io_valid(const struct sunxi_mmc_io *io)
{
    return io != 0 &&
           io->read32 != 0 &&
           io->write32 != 0 &&
           io->time_ms != 0;
}

static void clear_data0_mask(const struct sunxi_mmc_io *io)
{
    uint32_t clkcr;

    clkcr = io->read32(io->context, SUNXI_MMC_REG_CLKCR);
    clkcr &= ~SUNXI_MMC_CLKCR_MASK_DATA0;
    io->write32(io->context, SUNXI_MMC_REG_CLKCR, clkcr);
}

int sunxi_mmc_set_output_clock(const struct sunxi_mmc_io *io,
                               bool enable,
                               bool mask_data0,
                               uint32_t timeout_ms)
{
    uint32_t clkcr;
    uint32_t command;
    uint32_t interrupt_status;
    uint64_t start_ms;

    if (!io_valid(io) || timeout_ms == 0u)
        return SUNXI_MMC_CLOCK_INVALID;

    /*
     * Clear low-power and any stale DATA0 mask before starting a new update.
     * Keep divider bits and all unrelated CLKCR state intact.
     */
    clkcr = io->read32(io->context, SUNXI_MMC_REG_CLKCR);
    clkcr &= ~(SUNXI_MMC_CLKCR_CARD_CLOCK |
               SUNXI_MMC_CLKCR_LOW_POWER |
               SUNXI_MMC_CLKCR_MASK_DATA0);

    if (enable)
        clkcr |= SUNXI_MMC_CLKCR_CARD_CLOCK;
    if (mask_data0)
        clkcr |= SUNXI_MMC_CLKCR_MASK_DATA0;

    io->write32(io->context, SUNXI_MMC_REG_CLKCR, clkcr);

    command = SUNXI_MMC_CMD_START |
              SUNXI_MMC_CMD_UPCLK_ONLY |
              SUNXI_MMC_CMD_WAIT_PRE_OVER;
    io->write32(io->context, SUNXI_MMC_REG_CMD, command);

    start_ms = io->time_ms(io->context);
    for (;;) {
        uint32_t command_status;

        command_status = io->read32(io->context, SUNXI_MMC_REG_CMD);
        if ((command_status & SUNXI_MMC_CMD_START) == 0u)
            break;

        if ((io->time_ms(io->context) - start_ms) >= timeout_ms) {
            if (mask_data0)
                clear_data0_mask(io);
            return SUNXI_MMC_CLOCK_TIMEOUT;
        }

        if (io->delay_us != 0)
            io->delay_us(io->context, 10u);
    }

    /* Clock-update commands set raw interrupt status bits; W1C those bits. */
    interrupt_status = io->read32(io->context, SUNXI_MMC_REG_RINT);
    io->write32(io->context, SUNXI_MMC_REG_RINT, interrupt_status);

    if (mask_data0)
        clear_data0_mask(io);

    return SUNXI_MMC_CLOCK_OK;
}
