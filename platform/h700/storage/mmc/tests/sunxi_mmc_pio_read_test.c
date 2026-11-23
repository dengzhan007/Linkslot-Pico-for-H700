#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>

#include "../sunxi_mmc_pio_read.h"
#include "../sunxi_mmc_regs.h"

#define FIFO_WORDS 128u
#define TEST_SECTORS 4096u
#define CMD_INDEX_MASK 0x3fu
#define CHECK(condition, message) do { \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s (line %d)\n", message, __LINE__); \
        return 0; \
    } \
} while (0)

struct fake_mmc {
    uint32_t regs[129];
    uint32_t fifo[FIFO_WORDS];
    uint32_t fifo_index;
    uint32_t command_word;
    uint32_t read_command_count;
    uint32_t recovery_command_count;
    uint32_t barrier_count;
    uint32_t gctrl_write_count;
    uint64_t now_ms;
    int fifo_available;
    int inject_controller_error;
    int inject_card_r1_error;
    int omit_data_over;
    int hold_recovery;
    int data0_mask_seen;
};

static uint32_t fake_read32(void *context, uint32_t offset)
{
    struct fake_mmc *fake = (struct fake_mmc *)context;

    if ((offset & 3u) != 0u || offset >= sizeof(fake->regs))
        return 0u;

    if (offset == SUNXI_MMC_REG_RINT) {
        if (fake->inject_controller_error)
            return fake->regs[offset / 4u] | SUNXI_MMC_RINT_RESP_TIMEOUT;
        return fake->regs[offset / 4u];
    }

    if (offset == SUNXI_MMC_REG_STATUS) {
        uint32_t remaining;
        uint32_t available;

        if (!fake->fifo_available || fake->fifo_index >= FIFO_WORDS)
            return SUNXI_MMC_STATUS_FIFO_EMPTY;

        remaining = FIFO_WORDS - fake->fifo_index;
        available = remaining > 16u ? 16u : remaining;
        return available << 17;
    }

    if (offset == SUNXI_MMC_REG_FIFO) {
        if (fake->fifo_index >= FIFO_WORDS)
            return 0u;
        return fake->fifo[fake->fifo_index++];
    }

    return fake->regs[offset / 4u];
}

static void fake_write32(void *context, uint32_t offset, uint32_t value)
{
    struct fake_mmc *fake = (struct fake_mmc *)context;

    if ((offset & 3u) != 0u || offset >= sizeof(fake->regs))
        return;

    if (offset == SUNXI_MMC_REG_RINT) {
        fake->regs[offset / 4u] &= ~value;
        return;
    }

    if (offset == SUNXI_MMC_REG_GCTRL) {
        ++fake->gctrl_write_count;
        fake->regs[offset / 4u] = value;
        return;
    }

    if (offset == SUNXI_MMC_REG_CLKCR) {
        if ((value & SUNXI_MMC_CLKCR_MASK_DATA0) != 0u)
            fake->data0_mask_seen = 1;
        fake->regs[offset / 4u] = value;
        return;
    }

    if (offset == SUNXI_MMC_REG_CMD) {
        fake->regs[offset / 4u] = value;
        if ((value & SUNXI_MMC_CMD_UPCLK_ONLY) != 0u) {
            ++fake->recovery_command_count;
            if (!fake->hold_recovery)
                fake->regs[offset / 4u] &= ~SUNXI_MMC_CMD_START;
        } else {
            ++fake->read_command_count;
            fake->command_word = value;
            fake->regs[offset / 4u] &= ~SUNXI_MMC_CMD_START;
            fake->regs[SUNXI_MMC_REG_RINT / 4u] |=
                SUNXI_MMC_RINT_COMMAND_DONE;
            fake->regs[SUNXI_MMC_REG_RESP0 / 4u] =
                fake->inject_card_r1_error ? (1u << 31) : 0x00000900u;
            if (!fake->omit_data_over)
                fake->regs[SUNXI_MMC_REG_RINT / 4u] |=
                    SUNXI_MMC_RINT_DATA_OVER;
        }
        return;
    }

    fake->regs[offset / 4u] = value;
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

static void fake_read_barrier(void *context)
{
    struct fake_mmc *fake = (struct fake_mmc *)context;
    ++fake->barrier_count;
}

static struct sunxi_mmc_io make_io(struct fake_mmc *fake)
{
    struct sunxi_mmc_io io;
    io.context = fake;
    io.read32 = fake_read32;
    io.write32 = fake_write32;
    io.time_ms = fake_time_ms;
    io.delay_us = fake_delay_us;
    io.read_barrier = fake_read_barrier;
    return io;
}

static struct sunxi_mmc_readonly_host make_host(
    struct sunxi_mmc_io *io, uint64_t sectors, int block_addressed)
{
    struct sunxi_mmc_readonly_host host;
    host.io = io;
    host.sector_count = sectors;
    host.block_addressed = block_addressed != 0;
    host.command_timeout_ms = 20u;
    host.fifo_timeout_ms = 20u;
    host.data_timeout_ms = 20u;
    host.recovery_timeout_ms = 20u;
    return host;
}

static void fake_init(struct fake_mmc *fake, int fifo_available)
{
    uint32_t i;
    memset(fake, 0, sizeof(*fake));
    fake->fifo_available = fifo_available;
    fake->regs[SUNXI_MMC_REG_RESP0 / 4u] = 0x00000900u;
    for (i = 0u; i < FIFO_WORDS; ++i)
        fake->fifo[i] = 0x03020100u + i * 0x04040404u;
}

static int test_sdhc_cmd17_reads_one_sector(void)
{
    struct fake_mmc fake;
    struct sunxi_mmc_io io;
    struct sunxi_mmc_readonly_host host;
    _Alignas(4) uint8_t buffer[512];
    uint32_t response = 0u;
    uint32_t i;

    fake_init(&fake, 1);
    io = make_io(&fake);
    host = make_host(&io, TEST_SECTORS, 1);

    CHECK(sunxi_mmc_read_sector(&host, 3u, buffer, sizeof(buffer),
                                &response) == SUNXI_MMC_READ_OK,
          "SDHC single-sector read succeeds");
    CHECK(fake.read_command_count == 1u, "exactly one card read command issued");
    CHECK((fake.command_word & CMD_INDEX_MASK) == 17u,
          "CMD17 single-block read selected");
    CHECK((fake.command_word & SUNXI_MMC_CMD_RESP_EXPIRE) != 0u,
          "response enabled");
    CHECK((fake.command_word & SUNXI_MMC_CMD_CHK_RESPONSE_CRC) != 0u,
          "response CRC checking enabled");
    CHECK((fake.command_word & SUNXI_MMC_CMD_DATA_EXPIRE) != 0u,
          "data phase enabled");
    CHECK((fake.command_word & SUNXI_MMC_CMD_WRITE) == 0u,
          "write direction bit is not set");
    CHECK((fake.command_word & SUNXI_MMC_CMD_AUTO_STOP) == 0u,
          "single-block command does not request auto-stop");
    CHECK(fake.regs[SUNXI_MMC_REG_ARG / 4u] == 3u,
          "SDHC argument is sector-addressed");
    CHECK(fake.regs[SUNXI_MMC_REG_BLKSZ / 4u] == 512u,
          "block size is 512 bytes");
    CHECK(fake.regs[SUNXI_MMC_REG_BYTECNT / 4u] == 512u,
          "byte count is 512 bytes");
    CHECK((fake.regs[SUNXI_MMC_REG_GCTRL / 4u] &
           SUNXI_MMC_GCTRL_ACCESS_AHB) != 0u,
          "AHB/PIO access selected");
    CHECK((fake.regs[SUNXI_MMC_REG_GCTRL / 4u] &
           SUNXI_MMC_GCTRL_DMA_ENABLE) == 0u,
          "IDMA remains disabled");
    CHECK(response == 0x00000900u, "R1 response returned");
    CHECK(fake.fifo_index == FIFO_WORDS, "exactly 128 FIFO words consumed");
    CHECK(fake.barrier_count != 0u, "read barrier used after FIFO batches");

    for (i = 0u; i < FIFO_WORDS; ++i) {
        uint32_t word = 0x03020100u + i * 0x04040404u;
        uint32_t offset = i * 4u;
        CHECK(buffer[offset] == (uint8_t)word, "FIFO byte 0 copied");
        CHECK(buffer[offset + 1u] == (uint8_t)(word >> 8), "FIFO byte 1 copied");
        CHECK(buffer[offset + 2u] == (uint8_t)(word >> 16), "FIFO byte 2 copied");
        CHECK(buffer[offset + 3u] == (uint8_t)(word >> 24), "FIFO byte 3 copied");
    }
    return 1;
}

static int test_sdsc_byte_addressing(void)
{
    struct fake_mmc fake;
    struct sunxi_mmc_io io;
    struct sunxi_mmc_readonly_host host;
    _Alignas(4) uint8_t buffer[512];

    fake_init(&fake, 1);
    io = make_io(&fake);
    host = make_host(&io, TEST_SECTORS, 0);
    CHECK(sunxi_mmc_read_sector(&host, 2u, buffer, sizeof(buffer), NULL) ==
              SUNXI_MMC_READ_OK,
          "SDSC sector read succeeds");
    CHECK(fake.regs[SUNXI_MMC_REG_ARG / 4u] == 1024u,
          "SDSC argument is byte-addressed");
    return 1;
}

static int test_invalid_buffer_and_lba(void)
{
    struct fake_mmc fake;
    struct sunxi_mmc_io io;
    struct sunxi_mmc_readonly_host host;
    _Alignas(4) uint8_t buffer[513];

    fake_init(&fake, 1);
    io = make_io(&fake);
    host = make_host(&io, TEST_SECTORS, 1);

    CHECK(sunxi_mmc_read_sector(&host, TEST_SECTORS, buffer,
                                512u, NULL) == SUNXI_MMC_READ_RANGE,
          "LBA equal to sector count rejected");
    CHECK(sunxi_mmc_read_sector(&host, 0u, buffer,
                                511u, NULL) == SUNXI_MMC_READ_INVALID,
          "undersized buffer rejected");
    CHECK(sunxi_mmc_read_sector(&host, 0u, buffer + 1u,
                                512u, NULL) == SUNXI_MMC_READ_ALIGNMENT,
          "unaligned buffer rejected");
    CHECK(fake.read_command_count == 0u,
          "invalid requests do not issue a card command");

    host.sector_count = 0x100000001ull;
    CHECK(sunxi_mmc_read_sector(&host, 0x100000000ull, buffer,
                                512u, NULL) == SUNXI_MMC_READ_RANGE,
          "SDHC command argument overflow rejected");
    host.block_addressed = false;
    CHECK(sunxi_mmc_read_sector(&host, 0x00800000ull, buffer,
                                512u, NULL) == SUNXI_MMC_READ_RANGE,
          "SDSC byte-address overflow rejected");

    host.command_timeout_ms = 0u;
    CHECK(sunxi_mmc_read_sector(&host, 0u, buffer,
                                512u, NULL) == SUNXI_MMC_READ_INVALID,
          "zero timeout rejected");
    return 1;
}

static int test_fifo_timeout_recovers_controller(void)
{
    struct fake_mmc fake;
    struct sunxi_mmc_io io;
    struct sunxi_mmc_readonly_host host;
    _Alignas(4) uint8_t buffer[512];

    fake_init(&fake, 0);
    io = make_io(&fake);
    host = make_host(&io, TEST_SECTORS, 1);
    host.fifo_timeout_ms = 3u;

    CHECK(sunxi_mmc_read_sector(&host, 1u, buffer, sizeof(buffer), NULL) ==
              SUNXI_MMC_READ_TIMEOUT,
          "empty FIFO returns timeout");
    CHECK(fake.recovery_command_count == 1u,
          "controller recovery issues one UPCLK_ONLY command");
    CHECK(fake.read_command_count == 1u,
          "recovery does not issue another card read");
    CHECK((fake.regs[SUNXI_MMC_REG_CMD / 4u] &
           SUNXI_MMC_CMD_START) == 0u,
          "recovery clock update completes");
    CHECK(fake.data0_mask_seen,
          "H616 recovery clock update asserts MASK_DATA0");
    CHECK((fake.regs[SUNXI_MMC_REG_CLKCR / 4u] &
           SUNXI_MMC_CLKCR_MASK_DATA0) == 0u,
          "recovery clears MASK_DATA0 after clock update");
    return 1;
}

static int test_controller_error_recovers(void)
{
    struct fake_mmc fake;
    struct sunxi_mmc_io io;
    struct sunxi_mmc_readonly_host host;
    _Alignas(4) uint8_t buffer[512];

    fake_init(&fake, 1);
    fake.inject_controller_error = 1;
    io = make_io(&fake);
    host = make_host(&io, TEST_SECTORS, 1);

    CHECK(sunxi_mmc_read_sector(&host, 1u, buffer, sizeof(buffer), NULL) ==
              SUNXI_MMC_READ_CONTROLLER_ERROR,
          "controller response-timeout error propagated");
    CHECK(fake.recovery_command_count == 1u,
          "controller error triggers reset and clock recovery");
    return 1;
}

static int test_card_r1_error_is_not_reported_as_success(void)
{
    struct fake_mmc fake;
    struct sunxi_mmc_io io;
    struct sunxi_mmc_readonly_host host;
    _Alignas(4) uint8_t buffer[512];
    uint32_t response = 0u;

    fake_init(&fake, 1);
    fake.inject_card_r1_error = 1;
    io = make_io(&fake);
    host = make_host(&io, TEST_SECTORS, 1);

    CHECK(sunxi_mmc_read_sector(&host, 1u, buffer, sizeof(buffer), &response) ==
              SUNXI_MMC_READ_CARD_ERROR,
          "card-side R1 error is returned separately");
    CHECK((response & (1u << 31)) != 0u,
          "card R1 status is preserved for diagnostics");
    CHECK(fake.recovery_command_count == 1u,
          "card-side R1 error triggers controller recovery");
    return 1;
}

static int test_missing_data_over_times_out(void)
{
    struct fake_mmc fake;
    struct sunxi_mmc_io io;
    struct sunxi_mmc_readonly_host host;
    _Alignas(4) uint8_t buffer[512];

    fake_init(&fake, 1);
    fake.omit_data_over = 1;
    io = make_io(&fake);
    host = make_host(&io, TEST_SECTORS, 1);
    host.data_timeout_ms = 3u;

    CHECK(sunxi_mmc_read_sector(&host, 1u, buffer, sizeof(buffer), NULL) ==
              SUNXI_MMC_READ_TIMEOUT,
          "missing DATA_OVER event returns timeout");
    CHECK(fake.recovery_command_count == 1u,
          "data timeout triggers controller recovery");
    return 1;
}

static int test_recovery_timeout_is_reported(void)
{
    struct fake_mmc fake;
    struct sunxi_mmc_io io;
    struct sunxi_mmc_readonly_host host;
    _Alignas(4) uint8_t buffer[512];

    fake_init(&fake, 0);
    fake.hold_recovery = 1;
    io = make_io(&fake);
    host = make_host(&io, TEST_SECTORS, 1);
    host.fifo_timeout_ms = 2u;
    host.recovery_timeout_ms = 3u;

    CHECK(sunxi_mmc_read_sector(&host, 1u, buffer, sizeof(buffer), NULL) ==
              SUNXI_MMC_READ_RECOVERY_FAILED,
          "stalled recovery is reported separately");
    return 1;
}

int main(void)
{
    if (!test_sdhc_cmd17_reads_one_sector() ||
        !test_sdsc_byte_addressing() ||
        !test_invalid_buffer_and_lba() ||
        !test_fifo_timeout_recovers_controller() ||
        !test_controller_error_recovers() ||
        !test_card_r1_error_is_not_reported_as_success() ||
        !test_missing_data_over_times_out() ||
        !test_recovery_timeout_is_reported())
        return EXIT_FAILURE;

    puts("PASS: H700 read-only CMD17 PIO sector-read tests");
    return EXIT_SUCCESS;
}
