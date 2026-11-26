#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>

#include "h700_fat_vfs_backend.h"
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
static const linkslot_vfs_backend *g_installed_backend;

struct fake_card {
    uint32_t regs[129];
    uint32_t fifo_index;
    uint64_t selected_lba;
    uint64_t now_ms;
    uint32_t command_count;
    uint32_t write_command_count;
};

bool linkslot_vfs_install(const linkslot_vfs_backend *backend)
{
    if (backend == 0 || g_installed_backend != 0 ||
        backend->get_path == 0 || backend->open == 0 ||
        backend->close == 0 || backend->size == 0 ||
        backend->tell == 0 || backend->seek == 0 ||
        backend->read == 0 || backend->write == 0 ||
        backend->flush == 0 || backend->remove == 0 ||
        backend->rename == 0 || backend->truncate == 0 ||
        backend->stat == 0 || backend->mkdir == 0 ||
        backend->opendir == 0 || backend->readdir == 0 ||
        backend->dirent_get_name == 0 ||
        backend->dirent_is_dir == 0 || backend->closedir == 0)
        return false;
    g_installed_backend = backend;
    return true;
}

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
    uint8_t *games = g_media[DATA_START + 1u];
    uint8_t *gba = g_media[DATA_START + 2u];

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

    put32(fat0 + 2u * 4u, 0x0fffffffu);
    put32(fat0 + 3u * 4u, 0x0fffffffu);
    put32(fat0 + 4u * 4u, 0x0fffffffu);
    put32(fat0 + 5u * 4u, 0x0fffffffu);
    put32(fat0 + 6u * 4u, 0x0fffffffu);

    make_short_entry(root, "GAMES      ", 0x10u, 3u, 0u);
    make_short_entry(root + 32u, "BIOS    BIN", 0x20u, 6u, 4u);
    make_short_entry(games, "GBA        ", 0x10u, 4u, 0u);
    make_short_entry(gba, "TITLE   GBA", 0x20u, 5u, 4u);

    memcpy(g_media[DATA_START + 3u], "GAME", 4u);
    memcpy(g_media[DATA_START + 4u], "BIOS", 4u);
}

static int make_device(struct fake_card *fake,
                       struct h700_sunxi_mmc_block_device *device,
                       struct sunxi_mmc_io *io)
{
    struct sunxi_sd_card_info card;
    struct h700_sunxi_mmc_block_timeouts timeouts;
    struct h700_block_reader unused_reader;

    memset(&card, 0, sizeof(card));
    card.capacity.sector_count = TEST_CARD_SECTORS;
    card.capacity.block_addressed = true;
    timeouts.command_timeout_ms = 20u;
    timeouts.fifo_timeout_ms = 20u;
    timeouts.data_timeout_ms = 20u;
    timeouts.recovery_timeout_ms = 20u;
    *io = make_io(fake);
    return h700_sunxi_mmc_block_device_init(
        device, &unused_reader, io, &card, &timeouts);
}

static int test_readonly_libretro_vfs_backend(void)
{
    static struct h700_fat_vfs provider;
    struct h700_sunxi_mmc_block_device device;
    struct sunxi_mmc_io io;
    struct h700_gpt_partition partition;
    struct fake_card fake;
    retro_vfs_file_handle *file;
    retro_vfs_file_handle *second_file;
    retro_vfs_file_handle *files[H700_FAT_VFS_MAX_FILES];
    retro_vfs_dir_handle *directory;
    retro_vfs_dir_handle *directories[H700_FAT_VFS_MAX_DIRS];
    char bytes[8] = {0};
    int32_t file_size = -1;
    int flags;
    int64_t result;
    uint32_t before_commands;
    size_t i;
    size_t j;

    memset(&fake, 0, sizeof(fake));
    make_fat32_media();
    CHECK(make_device(&fake, &device, &io) == 0,
          "H700 CMD17 device initializes");

    memset(&partition, 0, sizeof(partition));
    partition.first_lba = PARTITION_START;
    partition.last_lba = PARTITION_START + PARTITION_SECTORS - 1u;
    partition.number = 7u;
    memcpy(partition.name, "primary", sizeof("primary"));

    CHECK(h700_fat_vfs_mount_and_install(
              &provider, &device, &partition, "/mnt/sdcard") == 0,
          "GPT p7 mounts and read-only VFS backend installs");
    CHECK(g_installed_backend != 0,
          "C VFS backend is registered with Linkslot VFS bridge");

    flags = g_installed_backend->stat("/mnt/sdcard", &file_size);
    CHECK(flags == (LINKSLOT_RETRO_VFS_STAT_IS_VALID |
                    LINKSLOT_RETRO_VFS_STAT_IS_DIRECTORY),
          "virtual mount root is reported as a directory");

    directory = g_installed_backend->opendir("/mnt/sdcard", false);
    CHECK(directory != 0, "virtual mount root can be opened");
    CHECK(g_installed_backend->readdir(directory),
          "root exposes first real allowed directory");
    CHECK(strcmp(g_installed_backend->dirent_get_name(directory), "GAMES") == 0 &&
          g_installed_backend->dirent_is_dir(directory),
          "root entry is restricted to Games");
    CHECK(!g_installed_backend->readdir(directory),
          "unapproved BIOS file is not exposed in the virtual root");
    CHECK(g_installed_backend->closedir(directory) == 0,
          "virtual root directory closes");

    file = g_installed_backend->open(
        "/mnt/sdcard/Games/GBA/TITLE.GBA",
        LINKSLOT_RETRO_VFS_FILE_ACCESS_READ, 0u);
    CHECK(file != 0, "ROM opens through the Libretro VFS callback");
    CHECK(g_installed_backend->size(file) == 4,
          "VFS reports ROM file size");
    CHECK(g_installed_backend->tell(file) == 0,
          "new VFS handle begins at offset zero");
    CHECK(strcmp(g_installed_backend->get_path(file),
                 "/mnt/sdcard/Games/GBA/TITLE.GBA") == 0,
          "VFS get_path returns the original absolute path");
    CHECK(g_installed_backend->read(file, bytes, 2u) == 2 &&
          memcmp(bytes, "GA", 2u) == 0,
          "VFS reads ROM bytes through FAT and CMD17");
    CHECK(g_installed_backend->tell(file) == 2,
          "VFS read advances file position");
    CHECK(g_installed_backend->seek(file, 0, LINKSLOT_RETRO_VFS_SEEK_POSITION_START) == 0,
          "VFS supports absolute seek");
    memset(bytes, 0, sizeof(bytes));
    CHECK(g_installed_backend->read(file, bytes, 4u) == 4 &&
          memcmp(bytes, "GAME", 4u) == 0,
          "VFS can reread ROM contents");
    CHECK(g_installed_backend->seek(file, -1, LINKSLOT_RETRO_VFS_SEEK_POSITION_END) == 3,
          "VFS supports negative seek from EOF");
    CHECK(g_installed_backend->read(file, bytes, 1u) == 1 &&
          bytes[0] == 'E',
          "VFS reads the final byte after end-relative seek");
    CHECK(g_installed_backend->close(file) == 0,
          "VFS file handle closes");

    CHECK(g_installed_backend->stat(
              "/mnt/sdcard/Games/GBA/TITLE.GBA", &file_size) ==
              LINKSLOT_RETRO_VFS_STAT_IS_VALID && file_size == 4,
          "VFS stat reports ROM file metadata");
    CHECK(g_installed_backend->stat(
              "/mnt/sdcard/Games", &file_size) ==
              (LINKSLOT_RETRO_VFS_STAT_IS_VALID |
               LINKSLOT_RETRO_VFS_STAT_IS_DIRECTORY),
          "VFS stat reports directory metadata");
    directory = g_installed_backend->opendir(
        "/mnt/sdcard/Games/GBA", false);
    CHECK(directory != 0,
          "VFS opens a nested game directory");
    CHECK(g_installed_backend->readdir(directory) &&
          strcmp(g_installed_backend->dirent_get_name(directory), "TITLE.GBA") == 0 &&
          !g_installed_backend->dirent_is_dir(directory),
          "nested VFS directory enumerates the ROM file");
    CHECK(g_installed_backend->closedir(directory) == 0,
          "nested game directory closes");

    CHECK(g_installed_backend->open(
              "/mnt/sdcard/Games/GBA/TITLE.GBA",
              LINKSLOT_RETRO_VFS_FILE_ACCESS_WRITE, 0u) == 0,
          "VFS refuses write-only open mode");
    CHECK(g_installed_backend->open(
              "/mnt/sdcard/Games/GBA/TITLE.GBA",
              LINKSLOT_RETRO_VFS_FILE_ACCESS_READ_WRITE, 0u) == 0,
          "VFS refuses read-write open mode");
    CHECK(g_installed_backend->open(
              "/mnt/sdcard2/Games/GBA/TITLE.GBA",
              LINKSLOT_RETRO_VFS_FILE_ACCESS_READ, 0u) == 0,
          "VFS rejects a sibling mount prefix");
    CHECK(g_installed_backend->open(
              "/mnt/sdcard/Games/../BIOS/BIOS.BIN",
              LINKSLOT_RETRO_VFS_FILE_ACCESS_READ, 0u) == 0,
          "VFS rejects path traversal");
    CHECK(g_installed_backend->open(
              "/etc/passwd", LINKSLOT_RETRO_VFS_FILE_ACCESS_READ, 0u) == 0,
          "VFS rejects paths outside the mount");
    CHECK(g_installed_backend->open(
              "Games/GBA/TITLE.GBA",
              LINKSLOT_RETRO_VFS_FILE_ACCESS_READ, 0u) == 0,
          "VFS rejects relative paths without an explicit base");
    CHECK(g_installed_backend->open(
              "/mnt/sdcard/Games//GBA/TITLE.GBA",
              LINKSLOT_RETRO_VFS_FILE_ACCESS_READ, 0u) == 0,
          "VFS rejects empty path components");
    CHECK(g_installed_backend->open(
              "/mnt/sdcard/private/secret.bin",
              LINKSLOT_RETRO_VFS_FILE_ACCESS_READ, 0u) == 0,
          "VFS rejects unapproved top-level roots");

    before_commands = fake.command_count;
    CHECK(g_installed_backend->write(0, "X", 1u) < 0 &&
          g_installed_backend->flush(0) < 0 &&
          g_installed_backend->remove("/mnt/sdcard/Games/a") < 0 &&
          g_installed_backend->rename("/mnt/sdcard/Games/a",
                                      "/mnt/sdcard/Games/b") < 0 &&
          g_installed_backend->truncate(0, 0) < 0 &&
          g_installed_backend->mkdir("/mnt/sdcard/Games/new") < 0,
          "all VFS mutation callbacks fail closed");
    CHECK(fake.command_count == before_commands,
          "denied mutations do not issue SD commands");
    CHECK(fake.write_command_count == 0u && provider.volume.wr == 0,
          "mounted volume has no write callback and SD issues no writes");

    result = g_installed_backend->seek(0, 0, 99);
    CHECK(result < 0, "invalid seek whence is rejected");

    /* Two live file handles must keep independent positions and path state. */
    file = g_installed_backend->open(
        "/mnt/sdcard/Games/GBA/TITLE.GBA",
        LINKSLOT_RETRO_VFS_FILE_ACCESS_READ, 0u);
    second_file = g_installed_backend->open(
        "/mnt/sdcard/Games/GBA/TITLE.GBA",
        LINKSLOT_RETRO_VFS_FILE_ACCESS_READ, 0u);
    CHECK(file != 0 && second_file != 0 && file != second_file,
          "same ROM can be opened through two distinct live handles");
    CHECK(g_installed_backend->read(file, bytes, 1u) == 1 &&
          bytes[0] == 'G',
          "first live handle reads its first byte");
    CHECK(g_installed_backend->tell(file) == 1 &&
          g_installed_backend->tell(second_file) == 0,
          "second file handle retains its independent zero position");
    CHECK(g_installed_backend->read(second_file, bytes, 2u) == 2 &&
          memcmp(bytes, "GA", 2u) == 0,
          "second live handle reads independently from offset zero");
    CHECK(g_installed_backend->tell(file) == 1 &&
          g_installed_backend->tell(second_file) == 2,
          "interleaved reads preserve per-handle positions");
    CHECK(strcmp(g_installed_backend->get_path(file),
                 "/mnt/sdcard/Games/GBA/TITLE.GBA") == 0 &&
          strcmp(g_installed_backend->get_path(second_file),
                 "/mnt/sdcard/Games/GBA/TITLE.GBA") == 0,
          "each live file handle retains its original path");
    CHECK(g_installed_backend->close(file) == 0 &&
          g_installed_backend->close(second_file) == 0,
          "both independent file handles close");

    /* Exercise the entire fixed file table, overflow, stale-handle rejection,
       and slot reuse without allocating any target-side heap memory. */
    for (i = 0u; i < H700_FAT_VFS_MAX_FILES; ++i) {
        files[i] = g_installed_backend->open(
            "/mnt/sdcard/Games/GBA/TITLE.GBA",
            LINKSLOT_RETRO_VFS_FILE_ACCESS_READ, 0u);
        CHECK(files[i] != 0, "every configured file-handle slot can be allocated");
        for (j = 0u; j < i; ++j)
            CHECK(files[i] != files[j], "simultaneous file handles use distinct slots");
    }
    CHECK(g_installed_backend->open(
              "/mnt/sdcard/Games/GBA/TITLE.GBA",
              LINKSLOT_RETRO_VFS_FILE_ACCESS_READ, 0u) == 0,
          "file-handle table exhaustion fails closed");
    CHECK(g_installed_backend->tell(files[0]) == 0 &&
          g_installed_backend->tell(files[H700_FAT_VFS_MAX_FILES - 1u]) == 0,
          "new file-handle slots begin with independent positions");
    CHECK(g_installed_backend->read(files[0], bytes, 1u) == 1 &&
          bytes[0] == 'G' && g_installed_backend->tell(files[0]) == 1 &&
          g_installed_backend->tell(files[1]) == 0,
          "one file handle can advance without moving another");
    CHECK(g_installed_backend->close(files[1]) == 0,
          "a file slot can be released while other handles remain open");
    CHECK(g_installed_backend->size(files[1]) < 0 &&
          g_installed_backend->get_path(files[1]) == 0,
          "closed file handles are rejected before slot reuse");
    files[1] = g_installed_backend->open(
        "/mnt/sdcard/Games/GBA/TITLE.GBA",
        LINKSLOT_RETRO_VFS_FILE_ACCESS_READ, 0u);
    CHECK(files[1] != 0,
          "released file-handle capacity can be reused");
    for (i = 0u; i < H700_FAT_VFS_MAX_FILES; ++i)
        CHECK(g_installed_backend->close(files[i]) == 0,
              "all file-handle slots close cleanly");

    /* Directory handles have independent enumeration cursors and limits. */
    for (i = 0u; i < H700_FAT_VFS_MAX_DIRS; ++i) {
        directories[i] = g_installed_backend->opendir(
            "/mnt/sdcard/Games/GBA", false);
        CHECK(directories[i] != 0,
              "every configured directory-handle slot can be allocated");
        for (j = 0u; j < i; ++j)
            CHECK(directories[i] != directories[j],
                  "simultaneous directory handles use distinct slots");
    }
    CHECK(g_installed_backend->opendir(
              "/mnt/sdcard/Games/GBA", false) == 0,
          "directory-handle table exhaustion fails closed");
    CHECK(g_installed_backend->readdir(directories[0]) &&
          g_installed_backend->readdir(directories[1]),
          "two open directory handles enumerate independently");
    CHECK(strcmp(g_installed_backend->dirent_get_name(directories[0]), "TITLE.GBA") == 0 &&
          strcmp(g_installed_backend->dirent_get_name(directories[1]), "TITLE.GBA") == 0,
          "each directory handle owns its current directory entry");
    CHECK(!g_installed_backend->readdir(directories[0]) &&
          g_installed_backend->dirent_get_name(directories[0]) == 0,
          "first directory cursor reaches EOF independently");
    CHECK(g_installed_backend->dirent_get_name(directories[1]) != 0,
          "second directory cursor is unaffected by first-handle EOF");
    CHECK(g_installed_backend->closedir(directories[0]) == 0,
          "one directory slot can close while other iterators remain open");
    CHECK(!g_installed_backend->readdir(directories[0]) &&
          g_installed_backend->closedir(directories[0]) < 0,
          "closed directory handles are rejected before slot reuse");
    directory = g_installed_backend->opendir(
        "/mnt/sdcard/Games/GBA", false);
    CHECK(directory != 0,
          "released directory-handle capacity can be reused");
    CHECK(g_installed_backend->closedir(directory) == 0,
          "reused directory handle closes");
    for (i = 1u; i < H700_FAT_VFS_MAX_DIRS; ++i)
        CHECK(g_installed_backend->closedir(directories[i]) == 0,
              "all remaining directory-handle slots close cleanly");

    return 1;
}

int main(void)
{
    if (!test_readonly_libretro_vfs_backend())
        return EXIT_FAILURE;
    puts("PASS: Linkslot Libretro VFS v3 read-only FAT/CMD17 backend tests");
    return EXIT_SUCCESS;
}
