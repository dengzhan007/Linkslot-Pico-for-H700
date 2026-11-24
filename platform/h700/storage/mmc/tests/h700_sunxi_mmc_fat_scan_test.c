#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>

#include "h700_sunxi_mmc_fat.h"
#include "sunxi_mmc_regs.h"

#define TEST_CARD_SECTORS 100000ull
#define FAKE_MEDIA_SECTORS 2048u
#define FIFO_WORDS 128u
#define PARTITION_START 100u
#define PARTITION_SECTORS 70000u
#define FAT_START (PARTITION_START + 32u)
#define DATA_START (FAT_START + 2u * 600u)
#define CHECK(c, m) do { \
    if (!(c)) { fprintf(stderr, "FAIL: %s (line %d)\n", m, __LINE__); return 0; } \
} while (0)

static uint8_t g_media[FAKE_MEDIA_SECTORS][512];

struct fake_card {
    uint32_t regs[129];
    uint32_t fifo_index;
    uint64_t selected_lba;
    uint64_t now_ms;
    uint32_t command_count;
    uint32_t write_command_count;
};

static uint32_t fake_read32(void *context, uint32_t offset)
{
    struct fake_card *fake = (struct fake_card *)context;
    if ((offset & 3u) != 0u || offset >= sizeof(fake->regs))
        return 0u;

    if (offset == SUNXI_MMC_REG_STATUS) {
        uint32_t remaining;
        if (fake->selected_lba >= FAKE_MEDIA_SECTORS ||
            fake->fifo_index >= FIFO_WORDS)
            return SUNXI_MMC_STATUS_FIFO_EMPTY;
        remaining = FIFO_WORDS - fake->fifo_index;
        if (remaining > 32u)
            remaining = 32u;
        return remaining << 17;
    }

    if (offset == SUNXI_MMC_REG_FIFO) {
        const uint8_t *sector;
        uint32_t index;
        if (fake->selected_lba >= FAKE_MEDIA_SECTORS ||
            fake->fifo_index >= FIFO_WORDS)
            return 0u;
        sector = g_media[fake->selected_lba];
        index = fake->fifo_index++ * 4u;
        return (uint32_t)sector[index] |
               ((uint32_t)sector[index + 1u] << 8) |
               ((uint32_t)sector[index + 2u] << 16) |
               ((uint32_t)sector[index + 3u] << 24);
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
        fake->selected_lba = fake->regs[SUNXI_MMC_REG_ARG / 4u];
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

static void make_short_entry(uint8_t *entry,
                             const char name[11],
                             uint8_t attributes,
                             uint32_t first_cluster,
                             uint32_t size)
{
    memset(entry, 0, 32u);
    memcpy(entry, name, 11u);
    entry[11] = attributes;
    put16(entry + 20u, (uint16_t)(first_cluster >> 16));
    put16(entry + 26u, (uint16_t)first_cluster);
    put32(entry + 28u, size);
}

static void make_fat32_media(void)
{
    uint8_t *vbr = g_media[PARTITION_START];
    uint8_t *fat0 = g_media[FAT_START];
    uint8_t *root = g_media[DATA_START];
    uint8_t *roms = g_media[DATA_START + 1u];

    memset(g_media, 0, sizeof(g_media));

    vbr[0] = 0xebu;
    vbr[1] = 0x58u;
    vbr[2] = 0x90u;
    memcpy(vbr + 3u, "MSWIN4.1", 8u);
    put16(vbr + 0x0bu, 512u);
    vbr[0x0du] = 1u;
    put16(vbr + 0x0eu, 32u);
    vbr[0x10u] = 2u;
    put16(vbr + 0x11u, 0u);
    put16(vbr + 0x13u, 0u);
    vbr[0x15u] = 0xf8u;
    put16(vbr + 0x16u, 0u);
    put32(vbr + 0x20u, PARTITION_SECTORS);
    put32(vbr + 0x24u, 600u);
    put32(vbr + 0x2cu, 2u);
    vbr[510] = 0x55u;
    vbr[511] = 0xaau;

    /* FAT32 entries 2..5 are all end-of-chain. */
    put32(fat0 + 2u * 4u, 0x0fffffffu);
    put32(fat0 + 3u * 4u, 0x0fffffffu);
    put32(fat0 + 4u * 4u, 0x0fffffffu);
    put32(fat0 + 5u * 4u, 0x0fffffffu);

    make_short_entry(root, "ROMS       ", 0x10u, 3u, 0u);
    make_short_entry(root + 32u, "BIOS    BIN", 0x20u, 4u, 4u);
    make_short_entry(roms, "TITLE   GBA", 0x20u, 5u, 4u);
    memcpy(g_media[DATA_START + 2u], "BIOS", 4u);
    memcpy(g_media[DATA_START + 3u], "GAME", 4u);
}

static int test_cmd17_fat32_mount_directory_and_file_read(void)
{
    struct fake_card fake;
    struct sunxi_mmc_io io;
    struct sunxi_sd_card_info card;
    struct h700_sunxi_mmc_block_device device;
    struct h700_sunxi_mmc_block_timeouts timeouts;
    struct h700_gpt_partition partition;
    fat_vol volume;
    fat_dir directory;
    fat_dirent entry;
    fat_file file;
    char content[8] = {0};
    uint32_t entries = 0u;

    memset(&fake, 0, sizeof(fake));
    make_fat32_media();
    io = make_io(&fake);

    memset(&card, 0, sizeof(card));
    card.capacity.sector_count = TEST_CARD_SECTORS;
    card.capacity.block_addressed = true;

    timeouts.command_timeout_ms = 20u;
    timeouts.fifo_timeout_ms = 20u;
    timeouts.data_timeout_ms = 20u;
    timeouts.recovery_timeout_ms = 20u;
    CHECK(h700_sunxi_mmc_block_device_init(
              &device, &(struct h700_block_reader){0},
              &io, &card, &timeouts) == 0,
          "CMD17 block device initializes");

    memset(&partition, 0, sizeof(partition));
    partition.first_lba = PARTITION_START;
    partition.last_lba = PARTITION_START + PARTITION_SECTORS - 1u;
    partition.number = 7u;
    memcpy(partition.name, "primary", sizeof("primary"));

    CHECK(h700_sunxi_mmc_fat_mount_gpt_partition(
              &device, &volume, &partition) == 0,
          "GPT p7 FAT32 filesystem mounts through CMD17");

    CHECK(fat_opendir(&volume, "/roms", &directory) == 0,
          "FAT32 root directory opens the ROMS subdirectory");
    while (fat_readdir(&directory, &entry)) {
        ++entries;
        if (!entry.is_dir && strcmp(entry.name, "TITLE.GBA") == 0)
            CHECK(entry.size == 4u, "ROM directory metadata is decoded");
    }
    CHECK(entries == 1u, "one game entry is enumerated");

    CHECK(fat_open(&volume, "/ROMS/TITLE.GBA", &file) == 0,
          "ROM file opens case-insensitively");
    CHECK(fat_read(&file, 0u, content, 4u) == 4u,
          "ROM file bytes are read through CMD17");
    CHECK(memcmp(content, "GAME", 4u) == 0,
          "ROM file content matches the synthetic FAT32 disk");

    CHECK(fat_open(&volume, "/bios.bin", &file) == 0,
          "root BIOS file opens");
    memset(content, 0, sizeof(content));
    CHECK(fat_read(&file, 0u, content, 4u) == 4u &&
          memcmp(content, "BIOS", 4u) == 0,
          "root BIOS content is returned correctly");

    CHECK(fake.command_count >= 6u,
          "mount, directory traversal and file reads all issue CMD17");
    CHECK(fake.write_command_count == 0u && volume.wr == 0,
          "FAT mount and file access remain read-only");

    return 1;
}

int main(void)
{
    if (!test_cmd17_fat32_mount_directory_and_file_read())
        return EXIT_FAILURE;

    puts("PASS: H700 GPT -> CMD17 -> FAT32 directory and file read integration");
    return EXIT_SUCCESS;
}
