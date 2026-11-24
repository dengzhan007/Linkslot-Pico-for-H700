#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>

#include "h700_sunxi_mmc_fat.h"
#include "sunxi_mmc_regs.h"

#define FIFO_WORDS 128u
#define TEST_CARD_SECTORS 100000ull
#define CHECK(c, m) do { \
    if (!(c)) { fprintf(stderr, "FAIL: %s (line %d)\n", m, __LINE__); return 0; } \
} while (0)

struct fake_card {
    uint32_t regs[129];
    uint8_t media_sector[512];
    uint32_t fifo_index;
    uint32_t command_count;
    uint32_t write_command_count;
    uint32_t last_argument;
    uint64_t now_ms;
};

static uint32_t fake_read32(void *context, uint32_t offset)
{
    struct fake_card *fake = (struct fake_card *)context;
    if ((offset & 3u) != 0u || offset >= sizeof(fake->regs))
        return 0u;

    if (offset == SUNXI_MMC_REG_STATUS) {
        uint32_t remaining;
        if (fake->fifo_index >= FIFO_WORDS)
            return SUNXI_MMC_STATUS_FIFO_EMPTY;
        remaining = FIFO_WORDS - fake->fifo_index;
        if (remaining > 32u)
            remaining = 32u;
        return remaining << 17;
    }

    if (offset == SUNXI_MMC_REG_FIFO) {
        uint32_t index;
        if (fake->fifo_index >= FIFO_WORDS)
            return 0u;
        index = fake->fifo_index++ * 4u;
        return (uint32_t)fake->media_sector[index] |
               ((uint32_t)fake->media_sector[index + 1u] << 8) |
               ((uint32_t)fake->media_sector[index + 2u] << 16) |
               ((uint32_t)fake->media_sector[index + 3u] << 24);
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
        if ((value & SUNXI_MMC_CMD_WRITE) != 0u)
            ++fake->write_command_count;
        fake->last_argument = fake->regs[SUNXI_MMC_REG_ARG / 4u];
        fake->fifo_index = 0u;
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

static void put16(uint8_t *p, uint16_t value)
{
    p[0] = (uint8_t)value;
    p[1] = (uint8_t)(value >> 8);
}

static void put32(uint8_t *p, uint32_t value)
{
    p[0] = (uint8_t)value;
    p[1] = (uint8_t)(value >> 8);
    p[2] = (uint8_t)(value >> 16);
    p[3] = (uint8_t)(value >> 24);
}

static void make_fat32_vbr(uint8_t sector[512])
{
    memset(sector, 0, 512u);
    sector[0] = 0xebu;
    sector[1] = 0x58u;
    sector[2] = 0x90u;
    memcpy(sector + 3u, "MSWIN4.1", 8u);
    put16(sector + 0x0bu, 512u);
    sector[0x0du] = 1u;
    put16(sector + 0x0eu, 32u);
    sector[0x10u] = 2u;
    put16(sector + 0x11u, 0u);
    put16(sector + 0x13u, 0u);
    sector[0x15u] = 0xf8u;
    put16(sector + 0x16u, 0u);
    put32(sector + 0x20u, 70000u);
    put32(sector + 0x24u, 600u);
    put32(sector + 0x2cu, 2u);
    sector[510] = 0x55u;
    sector[511] = 0xaau;
}

static void init_device(struct fake_card *fake,
                        struct h700_sunxi_mmc_block_device *device,
                        struct sunxi_mmc_io *io)
{
    struct sunxi_sd_card_info card;
    struct h700_sunxi_mmc_block_timeouts timeouts;

    memset(&card, 0, sizeof(card));
    card.capacity.sector_count = TEST_CARD_SECTORS;
    card.capacity.block_addressed = true;

    timeouts.command_timeout_ms = 20u;
    timeouts.fifo_timeout_ms = 20u;
    timeouts.data_timeout_ms = 20u;
    timeouts.recovery_timeout_ms = 20u;

    *io = make_io(fake);
    (void)h700_sunxi_mmc_block_device_init(device, &(struct h700_block_reader){0},
                                           io, &card, &timeouts);
}

static int test_unaligned_multi_sector_read(void)
{
    struct fake_card fake;
    struct h700_sunxi_mmc_block_device device;
    struct sunxi_mmc_io io;
    uint8_t storage[1026];
    uint8_t *unaligned = storage + 1u;
    uint32_t i;

    memset(&fake, 0, sizeof(fake));
    for (i = 0; i < sizeof(fake.media_sector); ++i)
        fake.media_sector[i] = (uint8_t)(i ^ 0x5au);
    init_device(&fake, &device, &io);

    CHECK(((uintptr_t)unaligned & 3u) != 0u,
          "test destination is deliberately not 4-byte aligned");
    CHECK(h700_sunxi_mmc_fat_read(&device, 37u, 2u, unaligned) == 2u,
          "two sectors are read through aligned CMD17 bounce storage");
    CHECK(fake.command_count == 2u && fake.last_argument == 38u,
          "multi-sector callback issues sequential single-block CMD17 reads");
    CHECK(memcmp(unaligned, fake.media_sector, 512u) == 0 &&
          memcmp(unaligned + 512u, fake.media_sector, 512u) == 0,
          "both sector payloads are copied to an unaligned destination");

    CHECK(h700_sunxi_mmc_fat_read(&device, 99999u, 2u, unaligned) == 0u,
          "out-of-card-range read is rejected before CMD17");
    CHECK(fake.command_count == 2u,
          "out-of-range callback does not issue an SD command");
    CHECK(fake.write_command_count == 0u,
          "FAT callback never issues an SD write command");
    return 1;
}

static int test_gpt_partition_fat32_mount_is_bounded_and_readonly(void)
{
    struct fake_card fake;
    struct h700_sunxi_mmc_block_device device;
    struct sunxi_mmc_io io;
    struct h700_gpt_partition partition;
    fat_vol volume;

    memset(&fake, 0, sizeof(fake));
    make_fat32_vbr(fake.media_sector);
    init_device(&fake, &device, &io);

    memset(&partition, 0, sizeof(partition));
    partition.first_lba = 100u;
    partition.last_lba = 70099u;
    partition.number = 7u;
    memcpy(partition.name, "primary", sizeof("primary"));

    CHECK(h700_sunxi_mmc_fat_mount_gpt_partition(
              &device, &volume, &partition) == 0,
          "GPT p7 mounts as FAT32 through CMD17");
    CHECK(volume.is_fat32 && volume.part_lba == 100u,
          "mounted FAT32 partition start is preserved");
    CHECK(volume.partition_sectors == 70000u &&
          volume.volume_sectors == 70000u,
          "GPT extent and FAT BPB length are both retained");
    CHECK(volume.wr == 0,
          "GPT partition mount remains read-only");
    CHECK(fake.command_count == 1u && fake.last_argument == 100u,
          "mount reads exactly the partition VBR sector");
    CHECK(fake.write_command_count == 0u,
          "GPT-to-FAT mount issues no SD write command");

    partition.last_lba = TEST_CARD_SECTORS;
    CHECK(h700_sunxi_mmc_fat_mount_gpt_partition(
              &device, &volume, &partition) != 0,
          "GPT partition ending at or beyond card capacity is rejected");
    CHECK(fake.command_count == 1u,
          "invalid GPT extent is rejected before another SD command");

    partition.first_lba = 0x100000000ull;
    partition.last_lba = 0x100000010ull;
    CHECK(h700_sunxi_mmc_fat_mount_gpt_partition(
              &device, &volume, &partition) != 0,
          "GPT LBAs outside the 32-bit FAT API are rejected");
    return 1;
}

int main(void)
{
    if (!test_unaligned_multi_sector_read() ||
        !test_gpt_partition_fat32_mount_is_bounded_and_readonly())
        return EXIT_FAILURE;

    puts("PASS: H700 CMD17 multi-sector FAT adapter and GPT partition mount tests");
    return EXIT_SUCCESS;
}
