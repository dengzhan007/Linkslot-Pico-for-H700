#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>

#include "../sunxi_sd_card_init.h"
#include "../sunxi_mmc_regs.h"

#define MAX_COMMANDS 64u
#define SD_R1_APP_CMD (1u << 5)
#define OCR_READY (1u << 31)
#define OCR_CCS (1u << 30)
#define IF_COND 0x1aau
#define CMD_WRITE (1u << 10)
#define CMD_INIT_SEQ (1u << 15)
#define CHECK(condition, message) do { \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s (line %d)\n", message, __LINE__); \
        return 0; \
    } \
} while (0)

struct fake_mmc {
    uint32_t regs[129];
    uint32_t commands[MAX_COMMANDS];
    uint32_t command_count;
    uint32_t acmd41_count;
    uint32_t ready_after;
    uint32_t rca;
    uint32_t csd[4];
    uint64_t now_ms;
    uint32_t clock_hz;
    uint32_t clock_calls;
    int card_high_capacity;
    int unsupported_cmd8;
    int never_ready;
    int omit_command_done;
    int reject_app_command;
    int inject_r1_error;
    int busy_stuck;
    int write_command_seen;
};

static void set_csd_bits(uint32_t words[4],
                         uint32_t start,
                         uint32_t count,
                         uint32_t value)
{
    uint32_t i;
    for (i = 0u; i < count; ++i) {
        uint32_t bit = start + i;
        uint32_t word_index = 3u - bit / 32u;
        uint32_t shift = bit % 32u;
        if ((value & (1u << i)) != 0u)
            words[word_index] |= 1u << shift;
        else
            words[word_index] &= ~(1u << shift);
    }
}

static void make_sdhc_csd(uint32_t words[4], uint32_t c_size)
{
    memset(words, 0, 4u * sizeof(words[0]));
    set_csd_bits(words, 126u, 2u, 1u);
    set_csd_bits(words, 48u, 22u, c_size);
}

static void make_sdsc_csd(uint32_t words[4])
{
    memset(words, 0, 4u * sizeof(words[0]));
    set_csd_bits(words, 126u, 2u, 0u);
    set_csd_bits(words, 62u, 12u, 1023u);
    set_csd_bits(words, 47u, 3u, 3u);
    set_csd_bits(words, 80u, 4u, 9u);
}

static uint32_t fake_read32(void *context, uint32_t offset)
{
    struct fake_mmc *fake = (struct fake_mmc *)context;

    if ((offset & 3u) != 0u || offset >= sizeof(fake->regs))
        return 0u;
    if (offset == SUNXI_MMC_REG_STATUS)
        return fake->busy_stuck ? SUNXI_MMC_STATUS_CARD_DATA_BUSY : 0u;
    return fake->regs[offset / 4u];
}

static void fake_set_r2(struct fake_mmc *fake, const uint32_t response[4])
{
    fake->regs[SUNXI_MMC_REG_RESP3 / 4u] = response[0];
    fake->regs[SUNXI_MMC_REG_RESP2 / 4u] = response[1];
    fake->regs[SUNXI_MMC_REG_RESP1 / 4u] = response[2];
    fake->regs[SUNXI_MMC_REG_RESP0 / 4u] = response[3];
}

static void fake_write32(void *context, uint32_t offset, uint32_t value)
{
    struct fake_mmc *fake = (struct fake_mmc *)context;
    uint32_t index;

    if ((offset & 3u) != 0u || offset >= sizeof(fake->regs))
        return;

    index = offset / 4u;
    if (offset == SUNXI_MMC_REG_RINT) {
        fake->regs[index] &= ~value;
        return;
    }

    if (offset == SUNXI_MMC_REG_GCTRL) {
        /* Model self-clearing soft/FIFO/DMA reset bits. */
        fake->regs[index] = value & ~SUNXI_MMC_GCTRL_RESET_MASK;
        return;
    }

    if (offset == SUNXI_MMC_REG_CMD) {
        uint32_t cmd = value & 0x3fu;
        uint32_t response = 0u;

        if (fake->command_count < MAX_COMMANDS)
            fake->commands[fake->command_count++] = value;
        if ((value & CMD_WRITE) != 0u)
            fake->write_command_seen = 1;

        fake->regs[index] = value & ~SUNXI_MMC_CMD_START;

        switch (cmd) {
        case 0u: /* CMD0 */
            break;
        case 8u: /* CMD8 / R7 */
            response = fake->unsupported_cmd8 ? 0x000001abu : IF_COND;
            fake->regs[SUNXI_MMC_REG_RESP0 / 4u] = response;
            break;
        case 55u: /* CMD55 / R1 */
            response = fake->reject_app_command ? 0u : SD_R1_APP_CMD;
            fake->regs[SUNXI_MMC_REG_RESP0 / 4u] = response;
            break;
        case 41u: /* ACMD41 / R3 */
            ++fake->acmd41_count;
            response = fake->never_ready ? 0u :
                       (fake->acmd41_count >= fake->ready_after
                            ? OCR_READY |
                                  (fake->card_high_capacity ? OCR_CCS : 0u) |
                                  0x00ff8000u
                            : 0x00ff8000u);
            fake->regs[SUNXI_MMC_REG_RESP0 / 4u] = response;
            break;
        case 2u: /* CMD2 / R2 */
            fake_set_r2(fake, (uint32_t[4]){
                0x12345678u, 0x9abcdef0u, 0x11223344u, 0x55667788u
            });
            break;
        case 3u: /* CMD3 / R6 */
            fake->regs[SUNXI_MMC_REG_RESP0 / 4u] =
                (fake->rca << 16) | (fake->inject_r1_error ? (1u << 14) : 0u);
            break;
        case 9u: /* CMD9 / R2 */
            fake_set_r2(fake, fake->csd);
            break;
        case 7u: /* CMD7 / R1b */
        case 16u: /* CMD16 / R1 */
            fake->regs[SUNXI_MMC_REG_RESP0 / 4u] = 0u;
            break;
        default:
            fake->regs[SUNXI_MMC_REG_RESP0 / 4u] = 0u;
            break;
        }

        if (!fake->omit_command_done)
            fake->regs[SUNXI_MMC_REG_RINT / 4u] |=
                SUNXI_MMC_RINT_COMMAND_DONE;
        return;
    }

    fake->regs[index] = value;
}

static uint64_t fake_time_ms(void *context)
{
    struct fake_mmc *fake = (struct fake_mmc *)context;
    return fake->now_ms++;
}

static void fake_delay_us(void *context, uint32_t delay)
{
    struct fake_mmc *fake = (struct fake_mmc *)context;
    fake->now_ms += delay / 1000u;
}

static int fake_set_clock(void *context, uint32_t hz, uint32_t timeout_ms)
{
    struct fake_mmc *fake = (struct fake_mmc *)context;
    if (timeout_ms == 0u)
        return -1;
    fake->clock_hz = hz;
    ++fake->clock_calls;
    return 0;
}

static void fake_init(struct fake_mmc *fake, int high_capacity)
{
    memset(fake, 0, sizeof(*fake));
    fake->ready_after = 2u;
    fake->rca = 0x1234u;
    fake->card_high_capacity = high_capacity;
    if (high_capacity)
        make_sdhc_csd(fake->csd, 29663u);
    else
        make_sdsc_csd(fake->csd);
}

static struct sunxi_mmc_io make_io(struct fake_mmc *fake)
{
    struct sunxi_mmc_io io;
    io.context = fake;
    io.read32 = fake_read32;
    io.write32 = fake_write32;
    io.time_ms = fake_time_ms;
    io.delay_us = fake_delay_us;
    io.read_barrier = NULL;
    return io;
}

static struct sunxi_sd_init_host make_host(struct sunxi_mmc_io *io,
                                           struct fake_mmc *fake)
{
    struct sunxi_sd_init_host host;
    host.io = io;
    host.clock_context = fake;
    host.set_clock_hz = fake_set_clock;
    host.controller_reset_timeout_ms = 20u;
    host.command_timeout_ms = 20u;
    host.ocr_timeout_ms = 50u;
    host.busy_timeout_ms = 20u;
    host.clock_update_timeout_ms = 20u;
    return host;
}

static int test_sdhc_identification_sequence(void)
{
    struct fake_mmc fake;
    struct sunxi_mmc_io io;
    struct sunxi_sd_init_host host;
    struct sunxi_sd_card_info card;
    enum sunxi_sd_init_result result;
    uint32_t i;

    fake_init(&fake, 1);
    io = make_io(&fake);
    host = make_host(&io, &fake);

    result = sunxi_sd_card_identify(&host, &card);
    CHECK(result == SUNXI_SD_INIT_OK, "SDHC v2 identification succeeds");
    CHECK(fake.clock_calls == 1u &&
          fake.clock_hz == SUNXI_SD_IDENTIFICATION_CLOCK_HZ,
          "identification uses 400 kHz clock");
    CHECK(fake.acmd41_count == 2u,
          "OCR polling repeats CMD55/ACMD41 until ready");
    CHECK(card.rca == 0x1234u, "RCA is read from CMD3 response");
    CHECK(card.capacity.block_addressed, "CCS marks card block addressed");
    CHECK(card.capacity.sector_count == 30375936ull,
          "CSD capacity matches RG34XX live inventory");
    CHECK(card.cid[0] == 0x12345678u &&
          card.cid[3] == 0x55667788u,
          "CID R2 response ordering preserved");
    CHECK(fake.command_count == 10u,
          "expected SD identification command count");
    CHECK((fake.commands[0] & 0x3fu) == 0u &&
          (fake.commands[0] & CMD_INIT_SEQ) != 0u,
          "CMD0 sends initialization sequence");
    CHECK((fake.commands[1] & 0x3fu) == 8u,
          "CMD8 follows CMD0");
    CHECK((fake.commands[2] & 0x3fu) == 55u &&
          (fake.commands[3] & 0x3fu) == 41u,
          "ACMD41 is prefixed by CMD55");
    CHECK((fake.commands[6] & 0x3fu) == 2u &&
          (fake.commands[7] & 0x3fu) == 3u &&
          (fake.commands[8] & 0x3fu) == 9u &&
          (fake.commands[9] & 0x3fu) == 7u,
          "CID/RCA/CSD/select order is correct");
    for (i = 0u; i < fake.command_count; ++i)
        CHECK((fake.commands[i] & CMD_WRITE) == 0u,
              "no SD write command flag is emitted");
    CHECK(!fake.write_command_seen, "no write command issued");
    CHECK(fake.regs[SUNXI_MMC_REG_WIDTH / 4u] == 0u,
          "card identification remains in 1-bit mode");
    CHECK(fake.regs[SUNXI_MMC_REG_TMOUT / 4u] == 0xffffffffu,
          "controller timeout register initialized");
    CHECK(fake.regs[SUNXI_MMC_REG_THLDC / 4u] ==
              (512u << 16 | (1u << 2) | (1u << 0)),
          "H616 FIFO threshold initialized for 512-byte blocks");
    CHECK((fake.regs[SUNXI_MMC_REG_GCTRL / 4u] &
           SUNXI_MMC_GCTRL_DMA_ENABLE) == 0u,
          "IDMA remains disabled");
    return 1;
}

static int test_sdsc_byte_addressed_and_cmd16(void)
{
    struct fake_mmc fake;
    struct sunxi_mmc_io io;
    struct sunxi_sd_init_host host;
    struct sunxi_sd_card_info card;

    fake_init(&fake, 0);
    io = make_io(&fake);
    host = make_host(&io, &fake);

    CHECK(sunxi_sd_card_identify(&host, &card) == SUNXI_SD_INIT_OK,
          "SDSC structure-0 card identification succeeds");
    CHECK(!card.capacity.block_addressed &&
          card.capacity.sector_count == 32768ull,
          "SDSC capacity and addressing mode decoded");
    CHECK(fake.command_count == 11u &&
          (fake.commands[10] & 0x3fu) == 16u,
          "SDSC receives CMD16 SET_BLOCKLEN");
    CHECK(fake.regs[SUNXI_MMC_REG_ARG / 4u] == 512u,
          "CMD16 sets 512-byte block length");
    return 1;
}

static int test_rejects_non_v2_or_wrong_if_cond_echo(void)
{
    struct fake_mmc fake;
    struct sunxi_mmc_io io;
    struct sunxi_sd_init_host host;
    struct sunxi_sd_card_info card;

    fake_init(&fake, 1);
    fake.unsupported_cmd8 = 1;
    io = make_io(&fake);
    host = make_host(&io, &fake);

    CHECK(sunxi_sd_card_identify(&host, &card) ==
              SUNXI_SD_INIT_UNSUPPORTED_CARD,
          "wrong CMD8 check pattern is rejected");
    CHECK(fake.command_count == 2u,
          "no OCR negotiation after invalid CMD8");
    return 1;
}

static int test_ocr_timeout(void)
{
    struct fake_mmc fake;
    struct sunxi_mmc_io io;
    struct sunxi_sd_init_host host;
    struct sunxi_sd_card_info card;

    fake_init(&fake, 1);
    fake.never_ready = 1;
    io = make_io(&fake);
    host = make_host(&io, &fake);
    host.ocr_timeout_ms = 8u;

    CHECK(sunxi_sd_card_identify(&host, &card) == SUNXI_SD_INIT_OCR_TIMEOUT,
          "card that never leaves power-up returns OCR timeout");
    CHECK(fake.command_count > 4u,
          "OCR timeout follows repeated CMD55/ACMD41 pairs");
    CHECK(card.rca == 0u && card.capacity.sector_count == 0u,
          "failed initialization does not publish a selected card");
    return 1;
}

static int test_controller_command_timeout(void)
{
    struct fake_mmc fake;
    struct sunxi_mmc_io io;
    struct sunxi_sd_init_host host;
    struct sunxi_sd_card_info card;

    fake_init(&fake, 1);
    fake.omit_command_done = 1;
    io = make_io(&fake);
    host = make_host(&io, &fake);
    host.command_timeout_ms = 3u;

    CHECK(sunxi_sd_card_identify(&host, &card) ==
              SUNXI_SD_INIT_COMMAND_TIMEOUT,
          "missing COMMAND_DONE returns command timeout");
    return 1;
}

static int test_card_rejects_application_command(void)
{
    struct fake_mmc fake;
    struct sunxi_mmc_io io;
    struct sunxi_sd_init_host host;
    struct sunxi_sd_card_info card;

    fake_init(&fake, 1);
    fake.reject_app_command = 1;
    io = make_io(&fake);
    host = make_host(&io, &fake);

    CHECK(sunxi_sd_card_identify(&host, &card) ==
              SUNXI_SD_INIT_CARD_REJECTED,
          "card without APP_CMD support is rejected");
    CHECK(fake.command_count == 3u,
          "no ACMD41 issued without accepted CMD55");
    return 1;
}

static int test_busy_timeout_after_select(void)
{
    struct fake_mmc fake;
    struct sunxi_mmc_io io;
    struct sunxi_sd_init_host host;
    struct sunxi_sd_card_info card;

    fake_init(&fake, 1);
    fake.ready_after = 1u;
    fake.busy_stuck = 1;
    io = make_io(&fake);
    host = make_host(&io, &fake);
    host.busy_timeout_ms = 5u;

    CHECK(sunxi_sd_card_identify(&host, &card) == SUNXI_SD_INIT_BUSY_TIMEOUT,
          "card busy after CMD7 returns timeout");
    return 1;
}

int main(void)
{
    if (!test_sdhc_identification_sequence() ||
        !test_sdsc_byte_addressed_and_cmd16() ||
        !test_rejects_non_v2_or_wrong_if_cond_echo() ||
        !test_ocr_timeout() ||
        !test_controller_command_timeout() ||
        !test_card_rejects_application_command() ||
        !test_busy_timeout_after_select())
        return EXIT_FAILURE;

    puts("PASS: H700 read-only SD card identification state machine tests");
    return EXIT_SUCCESS;
}
