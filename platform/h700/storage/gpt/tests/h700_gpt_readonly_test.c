#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "../h700_gpt_readonly.h"

#define DISK_SECTORS 30375936ull
#define GPT_HEADER_LBA 1u
#define ENTRY_ARRAY_LBA 2u
#define ENTRY_COUNT 8u
#define ENTRY_SIZE 128u
#define ENTRY_BYTES (ENTRY_COUNT * ENTRY_SIZE)
#define CHECK(c, m) do { if (!(c)) { fprintf(stderr, "FAIL: %s (line %d)\n", (m), __LINE__); return 0; } } while (0)

struct fake_disk {
    uint8_t sectors[4][H700_GPT_SECTOR_SIZE];
    uint64_t sector_count;
    uint64_t fail_lba;
    unsigned int read_count;
};

static uint32_t crc32_bytes(const uint8_t *data, size_t length)
{
    uint32_t crc = 0xFFFFFFFFu;
    size_t i;
    unsigned int bit;
    for (i = 0; i < length; ++i) {
        crc ^= data[i];
        for (bit = 0; bit < 8u; ++bit)
            crc = (crc >> 1) ^ (0xEDB88320u & (uint32_t)-(int32_t)(crc & 1u));
    }
    return ~crc;
}

static void put_le16(uint8_t *p, uint16_t v)
{
    p[0] = (uint8_t)v;
    p[1] = (uint8_t)(v >> 8);
}

static void put_le32(uint8_t *p, uint32_t v)
{
    p[0] = (uint8_t)v;
    p[1] = (uint8_t)(v >> 8);
    p[2] = (uint8_t)(v >> 16);
    p[3] = (uint8_t)(v >> 24);
}

static void put_le64(uint8_t *p, uint64_t v)
{
    put_le32(p, (uint32_t)v);
    put_le32(p + 4, (uint32_t)(v >> 32));
}

static uint8_t *entry_at(struct fake_disk *disk, unsigned int index)
{
    return &disk->sectors[ENTRY_ARRAY_LBA + (index * ENTRY_SIZE) / H700_GPT_SECTOR_SIZE]
                         [(index * ENTRY_SIZE) % H700_GPT_SECTOR_SIZE];
}

static void set_name(uint8_t *entry, const char *name)
{
    size_t i;
    memset(entry + 56, 0, 72);
    for (i = 0; name[i] != '\0' && i < H700_GPT_NAME_CHARS; ++i)
        put_le16(entry + 56 + i * 2u, (uint8_t)name[i]);
}

static int fake_read_sector(void *context, uint64_t lba, void *buffer)
{
    struct fake_disk *disk = (struct fake_disk *)context;
    ++disk->read_count;
    if (lba == disk->fail_lba || lba >= disk->sector_count || lba >= 4u)
        return -1;
    memcpy(buffer, disk->sectors[lba], H700_GPT_SECTOR_SIZE);
    return 0;
}

static void seal_fixture(struct fake_disk *disk)
{
    uint8_t *header = disk->sectors[GPT_HEADER_LBA];
    uint32_t crc;
    {
        uint8_t entries[ENTRY_BYTES];
        memcpy(entries, disk->sectors[ENTRY_ARRAY_LBA], H700_GPT_SECTOR_SIZE);
        memcpy(entries + H700_GPT_SECTOR_SIZE,
               disk->sectors[ENTRY_ARRAY_LBA + 1u], ENTRY_BYTES - H700_GPT_SECTOR_SIZE);
        crc = crc32_bytes(entries, sizeof(entries));
    }
    put_le32(header + 88, crc);
    put_le32(header + 16, 0u);
    put_le32(header + 16, crc32_bytes(header, 92u));
}

static void fixture_init(struct fake_disk *disk)
{
    static const struct {
        uint64_t first;
        uint64_t last;
        const char *name;
    } partitions[] = {
        { 73728u, 204799u, "special" },
        { 204800u, 270335u, "boot-resource" },
        { 270336u, 303103u, "env" },
        { 303104u, 434175u, "boot" },
        { 1482752u, 2531327u, "rootfs" },
        { 2531328u, 2793471u, "UDISK" },
        { 2793472u, 30375932u, "primary" },
    };
    uint8_t *mbr;
    uint8_t *header;
    size_t i;

    memset(disk, 0, sizeof(*disk));
    disk->sector_count = DISK_SECTORS;
    disk->fail_lba = UINT64_MAX;

    mbr = disk->sectors[0];
    mbr[446u + 4u] = 0xEEu;
    put_le32(mbr + 446u + 8u, 1u);
    put_le32(mbr + 446u + 12u, (uint32_t)(DISK_SECTORS - 1u));
    mbr[510] = 0x55u;
    mbr[511] = 0xAAu;

    header = disk->sectors[GPT_HEADER_LBA];
    memcpy(header, "EFI PART", 8u);
    put_le32(header + 8u, 0x00010000u);
    put_le32(header + 12u, 92u);
    put_le64(header + 24u, 1u);
    put_le64(header + 32u, DISK_SECTORS - 1u);
    put_le64(header + 40u, 4u);
    put_le64(header + 48u, DISK_SECTORS - 4u);
    put_le64(header + 72u, ENTRY_ARRAY_LBA);
    put_le32(header + 80u, ENTRY_COUNT);
    put_le32(header + 84u, ENTRY_SIZE);

    for (i = 0; i < sizeof(partitions) / sizeof(partitions[0]); ++i) {
        uint8_t *entry = entry_at(disk, (unsigned int)i);
        entry[0] = (uint8_t)(0xA0u + i);
        entry[16] = (uint8_t)(0x10u + i);
        put_le64(entry + 32u, partitions[i].first);
        put_le64(entry + 40u, partitions[i].last);
        set_name(entry, partitions[i].name);
    }
    seal_fixture(disk);
}

static enum h700_gpt_result run_parse(struct fake_disk *disk,
                                      struct h700_gpt_partition *parts,
                                      size_t capacity,
                                      struct h700_gpt_table *table)
{
    struct h700_block_reader reader;
    reader.context = disk;
    reader.sector_count = disk->sector_count;
    reader.sector_size = H700_GPT_SECTOR_SIZE;
    reader.read_sector = fake_read_sector;
    return h700_gpt_read_primary(&reader, parts, capacity, table);
}

static int test_real_layout_and_lookup(void)
{
    struct fake_disk disk;
    struct h700_gpt_partition parts[ENTRY_COUNT];
    struct h700_gpt_table table;
    const struct h700_gpt_partition *found;
    fixture_init(&disk);
    CHECK(run_parse(&disk, parts, ENTRY_COUNT, &table) == H700_GPT_OK,
          "captured RG34XX GPT layout parses");
    CHECK(table.partition_count == 7u, "seven populated partitions returned");
    CHECK(table.entry_count == 8u && table.entry_size == 128u,
          "GPT entry geometry preserved");
    CHECK(parts[5].number == 6u && strcmp(parts[5].name, "UDISK") == 0,
          "GPT slot 6 is named UDISK");
    CHECK(parts[6].number == 7u && strcmp(parts[6].name, "primary") == 0,
          "GPT slot 7 is named primary");
    CHECK(parts[6].first_lba == 2793472u && parts[6].last_lba == 30375932u,
          "partition 7 bounds match captured disk");
    CHECK(h700_gpt_find_unique_name(parts, table.partition_count, "primary", &found) == H700_GPT_OK,
          "primary name is found uniquely");
    CHECK(found == &parts[6], "primary lookup returns GPT slot 7");
    CHECK(h700_gpt_find_unique_name(parts, table.partition_count, "UDISK", &found) == H700_GPT_OK,
          "UDISK name is found uniquely");
    CHECK(found == &parts[5], "UDISK lookup returns GPT slot 6");
    return 1;
}

static int test_header_crc(void)
{
    struct fake_disk disk;
    struct h700_gpt_partition parts[ENTRY_COUNT];
    struct h700_gpt_table table;
    fixture_init(&disk);
    disk.sectors[1][16] ^= 0x01u;
    CHECK(run_parse(&disk, parts, ENTRY_COUNT, &table) == H700_GPT_ERR_HEADER_CRC,
          "header CRC corruption rejected");
    CHECK(table.partition_count == 0u, "failed parse publishes no partitions");
    return 1;
}

static int test_entry_crc(void)
{
    struct fake_disk disk;
    struct h700_gpt_partition parts[ENTRY_COUNT];
    struct h700_gpt_table table;
    fixture_init(&disk);
    disk.sectors[2][100] ^= 0x01u;
    CHECK(run_parse(&disk, parts, ENTRY_COUNT, &table) == H700_GPT_ERR_ENTRY_CRC,
          "entry-array CRC corruption rejected");
    return 1;
}

static int test_protective_mbr(void)
{
    struct fake_disk disk;
    struct h700_gpt_partition parts[ENTRY_COUNT];
    struct h700_gpt_table table;
    fixture_init(&disk);
    disk.sectors[0][510] = 0u;
    CHECK(run_parse(&disk, parts, ENTRY_COUNT, &table) == H700_GPT_ERR_PROTECTIVE_MBR,
          "missing MBR signature rejected");
    return 1;
}

static int test_capacity_and_io(void)
{
    struct fake_disk disk;
    struct h700_gpt_partition parts[ENTRY_COUNT];
    struct h700_gpt_table table;
    fixture_init(&disk);
    CHECK(run_parse(&disk, parts, 7u, &table) == H700_GPT_ERR_CAPACITY,
          "insufficient output capacity rejected");
    fixture_init(&disk);
    disk.fail_lba = 3u;
    CHECK(run_parse(&disk, parts, ENTRY_COUNT, &table) == H700_GPT_ERR_IO,
          "sector read failure propagated");
    return 1;
}

static int test_partition_bounds(void)
{
    struct fake_disk disk;
    struct h700_gpt_partition parts[ENTRY_COUNT];
    struct h700_gpt_table table;
    fixture_init(&disk);
    put_le64(entry_at(&disk, 6u) + 32u, 3u);
    seal_fixture(&disk);
    CHECK(run_parse(&disk, parts, ENTRY_COUNT, &table) == H700_GPT_ERR_PARTITION_BOUNDS,
          "partition before first usable LBA rejected");
    return 1;
}

static int test_overlap_and_duplicate_guid(void)
{
    struct fake_disk disk;
    struct h700_gpt_partition parts[ENTRY_COUNT];
    struct h700_gpt_table table;
    fixture_init(&disk);
    put_le64(entry_at(&disk, 6u) + 32u, 2531328u);
    seal_fixture(&disk);
    CHECK(run_parse(&disk, parts, ENTRY_COUNT, &table) == H700_GPT_ERR_OVERLAP,
          "overlapping partitions rejected");

    fixture_init(&disk);
    memcpy(entry_at(&disk, 6u) + 16u, entry_at(&disk, 5u) + 16u, 16u);
    seal_fixture(&disk);
    CHECK(run_parse(&disk, parts, ENTRY_COUNT, &table) == H700_GPT_ERR_DUPLICATE_GUID,
          "duplicate unique GUID rejected");
    return 1;
}

static int test_duplicate_name_is_ambiguous(void)
{
    struct fake_disk disk;
    struct h700_gpt_partition parts[ENTRY_COUNT];
    struct h700_gpt_table table;
    const struct h700_gpt_partition *found;
    fixture_init(&disk);
    set_name(entry_at(&disk, 6u), "UDISK");
    seal_fixture(&disk);
    CHECK(run_parse(&disk, parts, ENTRY_COUNT, &table) == H700_GPT_OK,
          "duplicate labels do not invalidate otherwise valid GPT");
    CHECK(h700_gpt_find_unique_name(parts, table.partition_count, "UDISK", &found) ==
              H700_GPT_ERR_AMBIGUOUS_NAME,
          "duplicate name lookup fails closed");
    return 1;
}

static void reseal_header_only(struct fake_disk *disk)
{
    uint8_t *header = disk->sectors[GPT_HEADER_LBA];
    put_le32(header + 16u, 0u);
    put_le32(header + 16u, crc32_bytes(header, 92u));
}

static int test_signature_revision_and_entry_layout(void)
{
    struct fake_disk disk;
    struct h700_gpt_partition parts[ENTRY_COUNT];
    struct h700_gpt_table table;
    fixture_init(&disk);
    memcpy(disk.sectors[1], "NOT GPT!", 8u);
    CHECK(run_parse(&disk, parts, ENTRY_COUNT, &table) == H700_GPT_ERR_SIGNATURE,
          "invalid GPT signature rejected");

    fixture_init(&disk);
    put_le32(disk.sectors[1] + 8u, 0x00020000u);
    reseal_header_only(&disk);
    CHECK(run_parse(&disk, parts, ENTRY_COUNT, &table) == H700_GPT_ERR_UNSUPPORTED_REVISION,
          "unsupported GPT revision rejected");

    fixture_init(&disk);
    put_le32(disk.sectors[1] + 84u, 136u);
    reseal_header_only(&disk);
    CHECK(run_parse(&disk, parts, ENTRY_COUNT, &table) == H700_GPT_ERR_ENTRY_LAYOUT,
          "entry records that cross sector boundaries are rejected");

    fixture_init(&disk);
    put_le64(disk.sectors[1] + 32u, DISK_SECTORS - 2u);
    reseal_header_only(&disk);
    CHECK(run_parse(&disk, parts, ENTRY_COUNT, &table) == H700_GPT_ERR_HEADER,
          "backup header pointer must identify the final disk sector");
    return 1;
}

static int test_non_ascii_partition_name_is_sanitized(void)
{
    struct fake_disk disk;
    struct h700_gpt_partition parts[ENTRY_COUNT];
    struct h700_gpt_table table;
    fixture_init(&disk);
    memset(entry_at(&disk, 6u) + 56u, 0, 72u);
    put_le16(entry_at(&disk, 6u) + 56u, 0x4E2Du);
    seal_fixture(&disk);
    CHECK(run_parse(&disk, parts, ENTRY_COUNT, &table) == H700_GPT_OK,
          "non-ASCII partition label does not invalidate the table");
    CHECK(parts[6].name[0] == '?' && parts[6].name[1] == '\0',
          "non-ASCII UTF-16 code unit is converted to question mark");
    return 1;
}

int main(void)
{
    struct { const char *name; int (*run)(void); } tests[] = {
        { "real RG34XX GPT layout and unique name lookup", test_real_layout_and_lookup },
        { "header CRC", test_header_crc },
        { "entry-array CRC", test_entry_crc },
        { "protective MBR", test_protective_mbr },
        { "capacity and I/O handling", test_capacity_and_io },
        { "partition bounds", test_partition_bounds },
        { "overlap and duplicate GUID", test_overlap_and_duplicate_guid },
        { "ambiguous name rejection", test_duplicate_name_is_ambiguous },
        { "signature, revision, and entry-layout validation", test_signature_revision_and_entry_layout },
        { "non-ASCII partition-name sanitization", test_non_ascii_partition_name_is_sanitized },
    };
    size_t i;
    for (i = 0; i < sizeof(tests) / sizeof(tests[0]); ++i) {
        if (!tests[i].run())
            return EXIT_FAILURE;
        printf("PASS: %s\n", tests[i].name);
    }
    printf("All %zu H700 GPT tests passed.\n", sizeof(tests) / sizeof(tests[0]));
    return EXIT_SUCCESS;
}
