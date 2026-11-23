#include "sunxi_mmc_pio_read.h"

#include "sunxi_mmc_regs.h"

#define SD_CMD_READ_SINGLE_BLOCK 17u
#define SD_BLOCK_SIZE            512u
#define SD_BLOCK_WORDS           (SD_BLOCK_SIZE / 4u)
#define SD_FIFO_FALLBACK_WORDS   32u
#define SD_POLL_DELAY_US         10u
#define SD_R1_ERROR_MASK ( \
    (1u << 31) | (1u << 30) | (1u << 29) | (1u << 28) | \
    (1u << 27) | (1u << 26) | (1u << 25) | (1u << 24) | \
    (1u << 23) | (1u << 22) | (1u << 21) | (1u << 20) | \
    (1u << 19) | (1u << 16) | (1u << 15) | (1u << 7))

static int host_valid(const struct sunxi_mmc_readonly_host *host)
{
    const struct sunxi_mmc_io *io;

    if (host == 0 || host->io == 0 || host->sector_count == 0u ||
        host->command_timeout_ms == 0u || host->fifo_timeout_ms == 0u ||
        host->data_timeout_ms == 0u || host->recovery_timeout_ms == 0u)
        return 0;

    io = host->io;
    return io->read32 != 0 && io->write32 != 0 &&
           io->time_ms != 0 && io->delay_us != 0 &&
           io->read_barrier != 0;
}

static void poll_delay(const struct sunxi_mmc_io *io)
{
    io->delay_us(io->context, SD_POLL_DELAY_US);
}

static enum sunxi_mmc_read_result wait_interrupt(
    const struct sunxi_mmc_io *io,
    uint32_t done_mask,
    uint32_t timeout_ms)
{
    uint64_t start_ms = io->time_ms(io->context);

    for (;;) {
        uint32_t status = io->read32(io->context, SUNXI_MMC_REG_RINT);

        if ((status & SUNXI_MMC_RINT_ERROR_MASK) != 0u)
            return SUNXI_MMC_READ_CONTROLLER_ERROR;
        if ((status & done_mask) == done_mask)
            return SUNXI_MMC_READ_OK;
        if ((io->time_ms(io->context) - start_ms) >= timeout_ms)
            return SUNXI_MMC_READ_TIMEOUT;

        poll_delay(io);
    }
}

static enum sunxi_mmc_read_result drain_read_fifo(
    const struct sunxi_mmc_io *io,
    uint8_t *buffer,
    uint32_t timeout_ms)
{
    uint32_t words_read = 0u;
    uint64_t start_ms = io->time_ms(io->context);

    while (words_read < SD_BLOCK_WORDS) {
        uint32_t interrupt_status;
        uint32_t fifo_status;
        uint32_t fifo_words;
        uint32_t i;
        uint32_t remaining;

        interrupt_status = io->read32(io->context, SUNXI_MMC_REG_RINT);
        if ((interrupt_status & SUNXI_MMC_RINT_ERROR_MASK) != 0u)
            return SUNXI_MMC_READ_CONTROLLER_ERROR;

        fifo_status = io->read32(io->context, SUNXI_MMC_REG_STATUS);
        if ((fifo_status & SUNXI_MMC_STATUS_FIFO_EMPTY) != 0u) {
            if ((io->time_ms(io->context) - start_ms) >= timeout_ms)
                return SUNXI_MMC_READ_TIMEOUT;
            poll_delay(io);
            continue;
        }

        fifo_words = SUNXI_MMC_STATUS_FIFO_LEVEL(fifo_status);
        if (fifo_words == 0u &&
            (fifo_status & SUNXI_MMC_STATUS_FIFO_FULL) != 0u)
            fifo_words = SD_FIFO_FALLBACK_WORDS;

        if (fifo_words == 0u) {
            if ((io->time_ms(io->context) - start_ms) >= timeout_ms)
                return SUNXI_MMC_READ_TIMEOUT;
            poll_delay(io);
            continue;
        }

        remaining = SD_BLOCK_WORDS - words_read;
        if (fifo_words > remaining)
            fifo_words = remaining;

        for (i = 0u; i < fifo_words; ++i) {
            uint32_t word = io->read32(io->context, SUNXI_MMC_REG_FIFO);
            uint32_t offset = (words_read + i) * 4u;

            buffer[offset + 0u] = (uint8_t)word;
            buffer[offset + 1u] = (uint8_t)(word >> 8);
            buffer[offset + 2u] = (uint8_t)(word >> 16);
            buffer[offset + 3u] = (uint8_t)(word >> 24);
        }

        words_read += fifo_words;
        io->read_barrier(io->context);
    }

    return SUNXI_MMC_READ_OK;
}

/*
 * Reset the controller, then issue an UPCLK_ONLY update with H616
 * MASK_DATA0 asserted. The DATA0 mask is part of the common clock-update
 * protocol for this controller family.
 */
static int recover_controller(const struct sunxi_mmc_io *io,
                              uint32_t timeout_ms)
{
    uint64_t start_ms;
    uint32_t command;
    uint32_t interrupt_status;

    io->write32(io->context, SUNXI_MMC_REG_GCTRL,
                SUNXI_MMC_GCTRL_RESET_MASK);
    io->delay_us(io->context, 1000u);

    io->write32(io->context, SUNXI_MMC_REG_CLKCR,
                io->read32(io->context, SUNXI_MMC_REG_CLKCR) |
                SUNXI_MMC_CLKCR_MASK_DATA0);

    command = SUNXI_MMC_CMD_START |
              SUNXI_MMC_CMD_UPCLK_ONLY |
              SUNXI_MMC_CMD_WAIT_PRE_OVER;
    io->write32(io->context, SUNXI_MMC_REG_CMD, command);

    start_ms = io->time_ms(io->context);
    while ((io->read32(io->context, SUNXI_MMC_REG_CMD) &
            SUNXI_MMC_CMD_START) != 0u) {
        if ((io->time_ms(io->context) - start_ms) >= timeout_ms) {
            io->write32(io->context, SUNXI_MMC_REG_CLKCR,
                        io->read32(io->context, SUNXI_MMC_REG_CLKCR) &
                        ~SUNXI_MMC_CLKCR_MASK_DATA0);
            return -1;
        }
        poll_delay(io);
    }

    io->write32(io->context, SUNXI_MMC_REG_CLKCR,
                io->read32(io->context, SUNXI_MMC_REG_CLKCR) &
                ~SUNXI_MMC_CLKCR_MASK_DATA0);

    interrupt_status = io->read32(io->context, SUNXI_MMC_REG_RINT);
    io->write32(io->context, SUNXI_MMC_REG_RINT, interrupt_status);
    io->write32(io->context, SUNXI_MMC_REG_RINT, 0xffffffffu);
    io->write32(io->context, SUNXI_MMC_REG_GCTRL,
                io->read32(io->context, SUNXI_MMC_REG_GCTRL) |
                SUNXI_MMC_GCTRL_FIFO_RESET);

    return 0;
}

static void finish_success(const struct sunxi_mmc_io *io)
{
    io->write32(io->context, SUNXI_MMC_REG_RINT, 0xffffffffu);
    io->write32(io->context, SUNXI_MMC_REG_GCTRL,
                io->read32(io->context, SUNXI_MMC_REG_GCTRL) |
                SUNXI_MMC_GCTRL_FIFO_RESET);
}

enum sunxi_mmc_read_result sunxi_mmc_read_sector(
    const struct sunxi_mmc_readonly_host *host,
    uint64_t sector_lba,
    void *buffer,
    size_t buffer_capacity,
    uint32_t *response)
{
    const struct sunxi_mmc_io *io;
    uint64_t command_argument;
    uint32_t command;
    uint32_t response_word;
    enum sunxi_mmc_read_result result;

    if (!host_valid(host) || buffer == 0 ||
        buffer_capacity < SUNXI_MMC_PIO_SECTOR_SIZE)
        return SUNXI_MMC_READ_INVALID;

    if (((uintptr_t)buffer & 3u) != 0u)
        return SUNXI_MMC_READ_ALIGNMENT;

    if (sector_lba >= host->sector_count)
        return SUNXI_MMC_READ_RANGE;

    if (host->block_addressed) {
        if (sector_lba > 0xffffffffu)
            return SUNXI_MMC_READ_RANGE;
        command_argument = sector_lba;
    } else {
        if (sector_lba > (0xffffffffu / SD_BLOCK_SIZE))
            return SUNXI_MMC_READ_RANGE;
        command_argument = sector_lba * SD_BLOCK_SIZE;
    }

    io = host->io;

    /* CPU PIO uses the AHB FIFO path; IDMA remains disabled. */
    io->write32(io->context, SUNXI_MMC_REG_RINT, 0xffffffffu);
    io->write32(io->context, SUNXI_MMC_REG_GCTRL,
                (io->read32(io->context, SUNXI_MMC_REG_GCTRL) &
                 ~SUNXI_MMC_GCTRL_DMA_ENABLE) |
                SUNXI_MMC_GCTRL_ACCESS_AHB);
    io->write32(io->context, SUNXI_MMC_REG_BLKSZ, SD_BLOCK_SIZE);
    io->write32(io->context, SUNXI_MMC_REG_BYTECNT, SD_BLOCK_SIZE);
    io->write32(io->context, SUNXI_MMC_REG_ARG, (uint32_t)command_argument);

    command = SUNXI_MMC_CMD_START |
              SUNXI_MMC_CMD_RESP_EXPIRE |
              SUNXI_MMC_CMD_CHK_RESPONSE_CRC |
              SUNXI_MMC_CMD_DATA_EXPIRE |
              SUNXI_MMC_CMD_WAIT_PRE_OVER |
              SD_CMD_READ_SINGLE_BLOCK;
    io->write32(io->context, SUNXI_MMC_REG_CMD, command);

    result = drain_read_fifo(io, (uint8_t *)buffer, host->fifo_timeout_ms);
    if (result == SUNXI_MMC_READ_OK)
        result = wait_interrupt(io, SUNXI_MMC_RINT_COMMAND_DONE,
                                host->command_timeout_ms);
    if (result == SUNXI_MMC_READ_OK)
        result = wait_interrupt(io, SUNXI_MMC_RINT_DATA_OVER,
                                host->data_timeout_ms);

    if (result != SUNXI_MMC_READ_OK) {
        if (recover_controller(io, host->recovery_timeout_ms) != 0)
            return SUNXI_MMC_READ_RECOVERY_FAILED;
        return result;
    }

    response_word = io->read32(io->context, SUNXI_MMC_REG_RESP0);
    if (response != 0)
        *response = response_word;

    /* A completed controller transfer may still carry a card-side R1 error. */
    if ((response_word & SD_R1_ERROR_MASK) != 0u) {
        if (recover_controller(io, host->recovery_timeout_ms) != 0)
            return SUNXI_MMC_READ_RECOVERY_FAILED;
        return SUNXI_MMC_READ_CARD_ERROR;
    }

    finish_success(io);
    return SUNXI_MMC_READ_OK;
}
