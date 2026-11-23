#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>

#include "../h700_sunxi_mmc_block_device.h"
#include "../sunxi_mmc_pio_read.h"
#include "../sunxi_mmc_regs.h"

#define FIFO_WORDS 128u
#define TEST_SECTORS 4096ull
#define CHECK(condition, message) do { \
    if (!(condition)) { fprintf(stderr, "FAIL: %s (line %d)\n", message, __LINE__); return 0; } \
} while (0)

struct fake_card {
    uint32_t regs[129];
    uint32_t words[FIFO_WORDS];
    uint32_t fifo_index;
    uint32_t argument;
    uint32_t command_count;
    uint64_t now_ms;
    int fifo_enabled;
};

static uint32_t fake_read32(void *context, uint32_t offset)
{
    struct fake_card *fake = (struct fake_card *)context;
    if ((offset & 3u) != 0u || offset >= sizeof(fake->regs))
        return 0u;

    if (offset == SUNXI_MMC_REG_STATUS) {
        uint32_t remaining;
        if (!fake->fifo_enabled || fake->fifo_index >= FIFO_WORDS)
            return SUNXI_MMC_STATUS_FIFO_EMPTY;
        remaining = FIFO_WORDS - fake->fifo_index;
        if (remaining > 32u)
            remaining = 32u;
        return remaining << 17;
    }
    if (offset == SUNXI_MMC_REG_FIFO) {
        if (fake->fifo_index >= FIFO_WORDS)
            return 0u;
        return fake->words[fake->fifo_index++];
    }
    return fake->regs[offset / 4u];
}

static void fake_write32(void *context, uint32_t offset, uint32_t value)
{
    struct fake_card *fake = (struct fake_card *)context;
    uint32_t index;
    if ((offset & 3u) != 0u || offset >= sizeof(fake->regs))
        return;
    index = offset / 4u;

    if (offset == SUNXI_MMC_REG_RINT) {
        fake->regs[index] &= ~value;
        return;
    }
    if (offset == SUNXI_MMC_REG_CMD) {
        ++fake->command_count;
        fake->argument = fake->regs[SUNXI_MMC_REG_ARG / 4u];
        fake->fifo_index = 0u;
        fake->fifo_enabled = 1;
        fake->regs[index] = value & ~SUNXI_MMC_CMD_START;
        fake->regs[SUNXI_MMC_REG_RINT / 4u] |=
            SUNXI_MMC_RINT_COMMAND_DONE | SUNXI_MMC_RINT_DATA_OVER;
        fake->regs[SUNXI_MMC_REG_RESP0 / 4u] = 0x00000900u;
        return;
    }
    fake->regs[index] = value;
}

static uint64_t fake_time_ms(void *context)
{
    struct fake_card *fake = (struct fake_card *)context;
    return fake->now_ms++;
}

static void fake_delay_us(void *context, uint32_t delay)
{
    struct fake_card *fake = (struct fake_card *)context;
    fake->now_ms += delay / 1000u;
}

static void fake_read_barrier(void *context)
{
    (void)context;
}

static void fake_init(struct fake_card *fake)
{
    uint32_t i;
    memset(fake, 0, sizeof(*fake));
    for (i = 0u; i < FIFO_WORDS; ++i)
        fake->words[i] = 0x5a000000u + i;
}

static struct sunxi_mmc_io make_io(struct fake_card *fake)
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

static int test_reader_metadata_and_sdhc_read(void)
{
    struct fake_card fake;
    struct sunxi_mmc_io io;
    struct sunxi_sd_card_info card;
    struct h700_sunxi_mmc_block_device device;
    struct h700_block_reader reader;
    struct h700_sunxi_mmc_block_timeouts timeouts;
    _Alignas(4) uint8_t sector[512];
    uint32_t i;

    fake_init(&fake);
    io = make_io(&fake);
    memset(&card, 0, sizeof(card));
    card.capacity.sector_count = TEST_SECTORS;
    card.capacity.block_addressed = true;

    timeouts.command_timeout_ms = 20u;
    timeouts.fifo_timeout_ms = 20u;
    timeouts.data_timeout_ms = 20u;
    timeouts.recovery_timeout_ms = 20u;

    CHECK(h700_sunxi_mmc_block_device_init(
              &device, &reader, &io, &card, &timeouts) == 0,
          "block-reader adapter initializes");
    CHECK(reader.sector_count == TEST_SECTORS &&
          reader.sector_size == 512u,
          "reader metadata comes from SD card CSD result");
    CHECK(reader.read_sector(reader.context, 7u, sector) == 0,
          "GPT block-reader callback reads one sector with CMD17");
    CHECK(fake.command_count == 1u && fake.argument == 7u,
          "SDHC LBA is passed to the controller without conversion");
    for (i = 0u; i < FIFO_WORDS; ++i) {
        uint32_t word = 0x5a000000u + i;
        CHECK(sector[i * 4u] == (uint8_t)word &&
              sector[i * 4u + 1u] == (uint8_t)(word >> 8) &&
              sector[i * 4u + 2u] == (uint8_t)(word >> 16) &&
              sector[i * 4u + 3u] == (uint8_t)(word >> 24),
              "sector bytes are forwarded from PIO FIFO");
    }
    return 1;
}

static int test_sdsc_byte_address_and_range(void)
{
    struct fake_card fake;
    struct sunxi_mmc_io io;
    struct sunxi_sd_card_info card;
    struct h700_sunxi_mmc_block_device device;
    struct h700_block_reader reader;
    struct h700_sunxi_mmc_block_timeouts timeouts;
    _Alignas(4) uint8_t sector[512];

    fake_init(&fake);
    io = make_io(&fake);
    memset(&card, 0, sizeof(card));
    card.capacity.sector_count = TEST_SECTORS;
    card.capacity.block_addressed = false;
    timeouts.command_timeout_ms = 20u;
    timeouts.fifo_timeout_ms = 20u;
    timeouts.data_timeout_ms = 20u;
    timeouts.recovery_timeout_ms = 20u;

    CHECK(h700_sunxi_mmc_block_device_init(
              &device, &reader, &io, &card, &timeouts) == 0,
          "SDSC block-reader adapter initializes");
    CHECK(reader.read_sector(reader.context, 2u, sector) == 0,
          "SDSC sector read through GPT callback succeeds");
    CHECK(fake.argument == 1024u,
          "SDSC LBA is converted to a byte address");
    CHECK(reader.read_sector(reader.context, TEST_SECTORS, sector) != 0,
          "out-of-range GPT block read fails");
    CHECK(fake.command_count == 1u,
          "out-of-range request does not issue another SD command");
    return 1;
}

static int test_invalid_initialization_arguments(void)
{
    struct fake_card fake;
    struct sunxi_mmc_io io;
    struct sunxi_sd_card_info card;
    struct h700_sunxi_mmc_block_device device;
    struct h700_block_reader reader;
    struct h700_sunxi_mmc_block_timeouts timeouts;

    fake_init(&fake);
    io = make_io(&fake);
    memset(&card, 0, sizeof(card));
    card.capacity.sector_count = 0u;
    timeouts.command_timeout_ms = 20u;
    timeouts.fifo_timeout_ms = 20u;
    timeouts.data_timeout_ms = 20u;
    timeouts.recovery_timeout_ms = 20u;

    CHECK(h700_sunxi_mmc_block_device_init(
              &device, &reader, &io, &card, &timeouts) != 0,
          "zero-capacity card is rejected");
    CHECK(h700_sunxi_mmc_block_device_init(
              NULL, &reader, &io, &card, &timeouts) != 0,
          "null device is rejected");
    return 1;
}

int main(void)
{
    if (!test_reader_metadata_and_sdhc_read() ||
        !test_sdsc_byte_address_and_range() ||
        !test_invalid_initialization_arguments())
        return EXIT_FAILURE;

    puts("PASS: H700 SD CMD17 to GPT block-reader adapter tests");
    return EXIT_SUCCESS;
}
