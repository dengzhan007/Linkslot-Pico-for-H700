#include "sunxi_sd_card_init.h"

#include <stddef.h>
#include <string.h>

#include "sunxi_mmc_regs.h"

#define SD_CMD_GO_IDLE_STATE          0u
#define SD_CMD_SEND_IF_COND           8u
#define SD_CMD_SEND_CSD               9u
#define SD_CMD_SEND_CID               2u
#define SD_CMD_SET_RELATIVE_ADDR       3u
#define SD_CMD_SELECT_CARD             7u
#define SD_CMD_APP_CMD                55u
#define SD_ACMD_SD_SEND_OP_COND        41u
#define SD_CMD_SET_BLOCKLEN            16u

#define SD_CMD_SEND_INIT_SEQ           (1u << 15)
#define SD_CMD_LONG_RESPONSE           (1u << 7)
#define SD_CMD_RSP_PRESENT             SUNXI_MMC_CMD_RESP_EXPIRE
#define SD_CMD_RSP_CRC                 SUNXI_MMC_CMD_CHK_RESPONSE_CRC

#define SD_R1_APP_CMD                  (1u << 5)
#define SD_R1_ERROR_MASK ( \
    (1u << 31) | (1u << 30) | (1u << 29) | (1u << 28) | \
    (1u << 27) | (1u << 26) | (1u << 25) | (1u << 24) | \
    (1u << 23) | (1u << 22) | (1u << 21) | (1u << 20) | \
    (1u << 19) | (1u << 16) | (1u << 15) | (1u << 7))
#define SD_R6_ERROR_MASK               (0x7u << 13)

#define SD_OCR_VOLTAGE_WINDOW           0x00ff8000u
#define SD_OCR_HOST_CAPACITY_SUPPORT    (1u << 30)
#define SD_IF_COND_ARGUMENT             0x000001aau
#define SD_BLOCK_LENGTH                  512u

#define SD_THLDC_READ_ENABLE             (1u << 0)
#define SD_THLDC_WRITE_ENABLE            (1u << 2)
#define SD_THLDC_READ_THRESHOLD(value)   (((value) & 0x0fffu) << 16)

static int host_valid(const struct sunxi_sd_init_host *host)
{
    const struct sunxi_mmc_io *io;

    if (host == NULL || host->io == NULL || host->set_clock_hz == NULL ||
        host->controller_reset_timeout_ms == 0u ||
        host->command_timeout_ms == 0u || host->ocr_timeout_ms == 0u ||
        host->busy_timeout_ms == 0u || host->clock_update_timeout_ms == 0u)
        return 0;

    io = host->io;
    return io->read32 != NULL && io->write32 != NULL &&
           io->time_ms != NULL && io->delay_us != NULL;
}

static void clear_interrupts(const struct sunxi_mmc_io *io)
{
    io->write32(io->context, SUNXI_MMC_REG_RINT, 0xffffffffu);
}

static enum sunxi_sd_init_result wait_command_done(
    const struct sunxi_mmc_io *io,
    uint32_t timeout_ms)
{
    uint64_t start = io->time_ms(io->context);

    for (;;) {
        uint32_t status = io->read32(io->context, SUNXI_MMC_REG_RINT);

        if ((status & SUNXI_MMC_RINT_ERROR_MASK) != 0u)
            return SUNXI_SD_INIT_CONTROLLER_ERROR;
        if ((status & SUNXI_MMC_RINT_COMMAND_DONE) != 0u)
            return SUNXI_SD_INIT_OK;
        if ((io->time_ms(io->context) - start) >= timeout_ms)
            return SUNXI_SD_INIT_COMMAND_TIMEOUT;

        io->delay_us(io->context, 10u);
    }
}

static enum sunxi_sd_init_result controller_prepare(
    const struct sunxi_sd_init_host *host)
{
    const struct sunxi_mmc_io *io = host->io;
    uint64_t start;

    io->write32(io->context, SUNXI_MMC_REG_GCTRL,
                SUNXI_MMC_GCTRL_RESET_MASK);

    start = io->time_ms(io->context);
    while ((io->read32(io->context, SUNXI_MMC_REG_GCTRL) &
            SUNXI_MMC_GCTRL_RESET_MASK) != 0u) {
        if ((io->time_ms(io->context) - start) >=
            host->controller_reset_timeout_ms)
            return SUNXI_SD_INIT_CONTROLLER_TIMEOUT;
        io->delay_us(io->context, 10u);
    }

    /* CPU/AHB mode only: IDMA stays disabled throughout this read-only path. */
    io->write32(io->context, SUNXI_MMC_REG_GCTRL, SUNXI_MMC_GCTRL_ACCESS_AHB);
    io->write32(io->context, SUNXI_MMC_REG_TMOUT, 0xffffffffu);
    io->write32(io->context, SUNXI_MMC_REG_WIDTH, 0u); /* Identification in 1-bit mode. */
    io->write32(io->context, SUNXI_MMC_REG_BLKSZ, SD_BLOCK_LENGTH);
    io->write32(io->context, SUNXI_MMC_REG_BYTECNT, 0u);

    /* H616 requires the read/write FIFO threshold controls to be initialized. */
    io->write32(io->context, SUNXI_MMC_REG_THLDC,
                SD_THLDC_READ_THRESHOLD(SD_BLOCK_LENGTH) |
                SD_THLDC_WRITE_ENABLE |
                SD_THLDC_READ_ENABLE);

    clear_interrupts(io);
    return SUNXI_SD_INIT_OK;
}

/*
 * This private command executor is intentionally not exported. Its callers
 * below are a closed list of non-writing SD identification commands.
 */
static enum sunxi_sd_init_result send_command(
    const struct sunxi_sd_init_host *host,
    uint32_t command_index,
    uint32_t argument,
    bool response_present,
    bool response_crc,
    bool long_response,
    bool send_initialization_sequence,
    uint32_t response[4])
{
    const struct sunxi_mmc_io *io = host->io;
    uint32_t command = SUNXI_MMC_CMD_START | command_index;
    enum sunxi_sd_init_result result;

    if (command_index > 63u)
        return SUNXI_SD_INIT_INVALID;

    clear_interrupts(io);
    io->write32(io->context, SUNXI_MMC_REG_ARG, argument);

    if (response_present)
        command |= SD_CMD_RSP_PRESENT;
    if (response_crc)
        command |= SD_CMD_RSP_CRC;
    if (long_response)
        command |= SD_CMD_LONG_RESPONSE;
    if (send_initialization_sequence)
        command |= SD_CMD_SEND_INIT_SEQ;

    /* No data phase, write bit, or arbitrary caller-supplied flags. */
    io->write32(io->context, SUNXI_MMC_REG_CMD, command);

    result = wait_command_done(io, host->command_timeout_ms);
    if (result != SUNXI_SD_INIT_OK)
        return result;

    if (response_present && response != NULL) {
        if (long_response) {
            /* Synopsys response registers are read in RESP3..RESP0 order. */
            response[0] = io->read32(io->context, SUNXI_MMC_REG_RESP3);
            response[1] = io->read32(io->context, SUNXI_MMC_REG_RESP2);
            response[2] = io->read32(io->context, SUNXI_MMC_REG_RESP1);
            response[3] = io->read32(io->context, SUNXI_MMC_REG_RESP0);
        } else {
            response[0] = io->read32(io->context, SUNXI_MMC_REG_RESP0);
            response[1] = 0u;
            response[2] = 0u;
            response[3] = 0u;
        }
    }

    clear_interrupts(io);
    return SUNXI_SD_INIT_OK;
}

static enum sunxi_sd_init_result send_app_command(
    const struct sunxi_sd_init_host *host,
    uint32_t rca,
    uint32_t *response)
{
    uint32_t r1[4];
    enum sunxi_sd_init_result result;

    result = send_command(host, SD_CMD_APP_CMD, rca << 16,
                          true, true, false, false, r1);
    if (result != SUNXI_SD_INIT_OK)
        return result;

    if ((r1[0] & SD_R1_ERROR_MASK) != 0u ||
        (r1[0] & SD_R1_APP_CMD) == 0u)
        return SUNXI_SD_INIT_CARD_REJECTED;

    *response = r1[0];
    return SUNXI_SD_INIT_OK;
}

static enum sunxi_sd_init_result negotiate_ocr(
    const struct sunxi_sd_init_host *host,
    uint32_t *ocr_out)
{
    uint64_t start = host->io->time_ms(host->io->context);

    for (;;) {
        uint32_t app_response;
        uint32_t ocr[4];
        enum sunxi_sd_init_result result;

        result = send_app_command(host, 0u, &app_response);
        if (result != SUNXI_SD_INIT_OK)
            return result;

        result = send_command(host, SD_ACMD_SD_SEND_OP_COND,
                              SD_OCR_VOLTAGE_WINDOW |
                                  SD_OCR_HOST_CAPACITY_SUPPORT,
                              true, false, false, false, ocr);
        if (result != SUNXI_SD_INIT_OK)
            return result;

        if ((ocr[0] & SUNXI_SD_OCR_READY) != 0u) {
            if ((ocr[0] & SD_OCR_VOLTAGE_WINDOW) == 0u)
                return SUNXI_SD_INIT_UNSUPPORTED_CARD;
            *ocr_out = ocr[0];
            return SUNXI_SD_INIT_OK;
        }

        if ((host->io->time_ms(host->io->context) - start) >=
            host->ocr_timeout_ms)
            return SUNXI_SD_INIT_OCR_TIMEOUT;

        host->io->delay_us(host->io->context, 1000u);
    }
}

static enum sunxi_sd_init_result wait_card_not_busy(
    const struct sunxi_sd_init_host *host)
{
    uint64_t start = host->io->time_ms(host->io->context);

    while ((host->io->read32(host->io->context, SUNXI_MMC_REG_STATUS) &
            SUNXI_MMC_STATUS_CARD_DATA_BUSY) != 0u) {
        if ((host->io->time_ms(host->io->context) - start) >=
            host->busy_timeout_ms)
            return SUNXI_SD_INIT_BUSY_TIMEOUT;
        host->io->delay_us(host->io->context, 100u);
    }

    return SUNXI_SD_INIT_OK;
}

static enum sunxi_sd_init_result validate_r1(uint32_t response)
{
    return (response & SD_R1_ERROR_MASK) == 0u
               ? SUNXI_SD_INIT_OK
               : SUNXI_SD_INIT_CARD_REJECTED;
}

enum sunxi_sd_init_result sunxi_sd_card_identify(
    const struct sunxi_sd_init_host *host,
    struct sunxi_sd_card_info *card)
{
    uint32_t response[4];
    uint32_t rca;
    bool block_addressed;
    enum sunxi_sd_init_result result;

    if (card != NULL)
        memset(card, 0, sizeof(*card));
    if (!host_valid(host) || card == NULL)
        return SUNXI_SD_INIT_INVALID;

    result = controller_prepare(host);
    if (result != SUNXI_SD_INIT_OK)
        return result;

    result = (host->set_clock_hz)(host->clock_context,
                                  SUNXI_SD_IDENTIFICATION_CLOCK_HZ,
                                  host->clock_update_timeout_ms) == 0
                 ? SUNXI_SD_INIT_OK
                 : SUNXI_SD_INIT_CLOCK_FAILED;
    if (result != SUNXI_SD_INIT_OK)
        return result;

    /*
     * SD requires at least 74 clock cycles after power-up before CMD0.
     * At 400 kHz, a 1 ms delay provides at least 400 card-clock cycles.
     */
    host->io->delay_us(host->io->context, 1000u);

    /* 1) Reset the card state machine. CMD0 has no response. */
    result = send_command(host, SD_CMD_GO_IDLE_STATE, 0u,
                         false, false, false, true, NULL);
    if (result != SUNXI_SD_INIT_OK)
        return result;

    /* 2) Require SD v2 interface-condition support; validate voltage/check pattern. */
    result = send_command(host, SD_CMD_SEND_IF_COND, SD_IF_COND_ARGUMENT,
                          true, true, false, false, response);
    if (result != SUNXI_SD_INIT_OK)
        return result;
    if ((response[0] & 0xfffu) != SD_IF_COND_ARGUMENT)
        return SUNXI_SD_INIT_UNSUPPORTED_CARD;

    /* 3) Poll CMD55/ACMD41 until the card exits power-up. */
    result = negotiate_ocr(host, &card->ocr);
    if (result != SUNXI_SD_INIT_OK)
        return result;
    block_addressed = (card->ocr & SUNXI_SD_OCR_CCS) != 0u;

    /* 4) Read CID and obtain the card-relative address. */
    result = send_command(host, SD_CMD_SEND_CID, 0u,
                          true, true, true, false, card->cid);
    if (result != SUNXI_SD_INIT_OK)
        return result;

    result = send_command(host, SD_CMD_SET_RELATIVE_ADDR, 0u,
                          true, true, false, false, response);
    if (result != SUNXI_SD_INIT_OK)
        return result;
    rca = response[0] >> 16;
    if (rca == 0u || (response[0] & SD_R6_ERROR_MASK) != 0u)
        return SUNXI_SD_INIT_CARD_REJECTED;
    card->rca = rca;

    /* 5) Read and validate CSD capacity/addressing against ACMD41.CCS. */
    result = send_command(host, SD_CMD_SEND_CSD, rca << 16,
                          true, true, true, false, card->csd);
    if (result != SUNXI_SD_INIT_OK)
        return result;

    if (sunxi_sd_csd_parse(card->csd, block_addressed,
                           &card->capacity) != SUNXI_SD_CSD_OK)
        return SUNXI_SD_INIT_CSD_ERROR;

    /* 6) Select the card and wait until the DAT0 busy indication clears. */
    result = send_command(host, SD_CMD_SELECT_CARD, rca << 16,
                          true, true, false, false, response);
    if (result != SUNXI_SD_INIT_OK)
        return result;
    result = validate_r1(response[0]);
    if (result != SUNXI_SD_INIT_OK)
        return result;

    result = wait_card_not_busy(host);
    if (result != SUNXI_SD_INIT_OK)
        return result;

    /* SDSC uses byte addressing and must be configured for 512-byte blocks. */
    if (!block_addressed) {
        result = send_command(host, SD_CMD_SET_BLOCKLEN, SD_BLOCK_LENGTH,
                              true, true, false, false, response);
        if (result != SUNXI_SD_INIT_OK)
            return result;
        result = validate_r1(response[0]);
        if (result != SUNXI_SD_INIT_OK)
            return result;
    }

    return SUNXI_SD_INIT_OK;
}
