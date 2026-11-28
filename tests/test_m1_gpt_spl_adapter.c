#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <blk.h>
#include "h700_gpt_readonly.h"
#include "m1_firmware_readonly.h"
#include "m1_gpt_spl_adapter.h"

#define DISK_SECTORS UINT64_C(30375936)
#define GPT_SECTORS 4u
#define GPT_ENTRY_COUNT 8u
#define GPT_ENTRY_SIZE 128u
#define GPT_ARRAY_LBA 2u
#define GPT_ARRAY_BYTES (GPT_ENTRY_COUNT * GPT_ENTRY_SIZE)
#define PACKAGE_LBA UINT64_C(385024)

int h700_m1_spl_preflight(struct blk_desc *, uint64_t, uint64_t, uint32_t,
                          void *, size_t);

static uint8_t gpt[GPT_SECTORS * H700_GPT_SECTOR_SIZE];

static uint32_t crc32_bytes(const uint8_t *data, size_t length)
{
    uint32_t crc = 0xffffffffu;
    for (size_t i = 0; i < length; ++i) {
        crc ^= data[i];
        for (unsigned bit = 0; bit < 8u; ++bit)
            crc = (crc >> 1) ^ (0xedb88320u & (uint32_t)-(int32_t)(crc & 1u));
    }
    return ~crc;
}

static void put_le16(uint8_t *p, uint16_t value)
{
    p[0] = (uint8_t)value;
    p[1] = (uint8_t)(value >> 8);
}

static void put_le32(uint8_t *p, uint32_t value)
{
    p[0] = (uint8_t)value;
    p[1] = (uint8_t)(value >> 8);
    p[2] = (uint8_t)(value >> 16);
    p[3] = (uint8_t)(value >> 24);
}

static void put_le64(uint8_t *p, uint64_t value)
{
    put_le32(p, (uint32_t)value);
    put_le32(p + 4, (uint32_t)(value >> 32));
}

static uint8_t *entry(unsigned index)
{
    return gpt + GPT_ARRAY_LBA * H700_GPT_SECTOR_SIZE + index * GPT_ENTRY_SIZE;
}

static void set_name(uint8_t *entry_bytes, const char *name)
{
    memset(entry_bytes + 56, 0, 72);
    for (size_t i = 0; name[i] && i < H700_GPT_NAME_CHARS; ++i)
        put_le16(entry_bytes + 56 + i * 2u, (uint8_t)name[i]);
}

static void seal_gpt(void)
{
    uint8_t *header = gpt + H700_GPT_SECTOR_SIZE;
    put_le32(header + 88, crc32_bytes(gpt + GPT_ARRAY_LBA * 512u, GPT_ARRAY_BYTES));
    put_le32(header + 16, 0u);
    put_le32(header + 16, crc32_bytes(header, 92u));
}

static void make_gpt(void)
{
    static const struct {
        uint64_t first;
        uint64_t last;
        const char *name;
    } parts[] = {
        {73728u, 204799u, "special"},
        {204800u, 270335u, "boot-resource"},
        {270336u, 303103u, "env"},
        {303104u, 434175u, "boot"},
        {1482752u, 2531327u, "rootfs"},
        {2531328u, 2793471u, "UDISK"},
        {2793472u, 30375932u, "primary"}
    };
    uint8_t *mbr;
    uint8_t *header;

    memset(gpt, 0, sizeof(gpt));
    mbr = gpt;
    mbr[446u + 4u] = 0xeeu;
    put_le32(mbr + 446u + 8u, 1u);
    put_le32(mbr + 446u + 12u, (uint32_t)(DISK_SECTORS - 1u));
    mbr[510] = 0x55u;
    mbr[511] = 0xaau;

    header = gpt + H700_GPT_SECTOR_SIZE;
    memcpy(header, "EFI PART", 8u);
    put_le32(header + 8u, 0x00010000u);
    put_le32(header + 12u, 92u);
    put_le64(header + 24u, 1u);
    put_le64(header + 32u, DISK_SECTORS - 1u);
    put_le64(header + 40u, 4u);
    put_le64(header + 48u, DISK_SECTORS - 4u);
    put_le64(header + 72u, GPT_ARRAY_LBA);
    put_le32(header + 80u, GPT_ENTRY_COUNT);
    put_le32(header + 84u, GPT_ENTRY_SIZE);

    for (unsigned i = 0; i < sizeof(parts) / sizeof(parts[0]); ++i) {
        uint8_t *p = entry(i);
        p[0] = (uint8_t)(0xa0u + i);
        p[16] = (uint8_t)(0x10u + i);
        put_le64(p + 32u, parts[i].first);
        put_le64(p + 40u, parts[i].last);
        set_name(p, parts[i].name);
    }
    seal_gpt();
}

lbaint_t blk_dread(struct blk_desc *desc, lbaint_t start,
                   lbaint_t count, void *destination)
{
    ++desc->reads;
    if (desc->fail_read || count == 0 || count > H700_M1_STREAM_CHUNK_SECTORS)
        return 0;
    if (desc->gpt_image && start < desc->gpt_sector_count &&
        count <= desc->gpt_sector_count - start) {
        memcpy(destination, desc->gpt_image + start * 512u, (size_t)count * 512u);
        return count;
    }
    if (start >= desc->first_lba &&
        start - desc->first_lba + count <= desc->sectors) {
        uint64_t offset = (start - desc->first_lba) * 512u;
        memcpy(destination, desc->image + offset, (size_t)count * 512u);
        return count;
    }
    return 0;
}

static uint8_t *make_package(void)
{
    uint8_t *image = calloc(1, H700_M1_PACKAGE_BYTES);
    assert(image);
    for (uint32_t i = 0; i < H700_M1_PACKAGE_BYTES; ++i)
        image[i] = (uint8_t)(i * 17u + 29u);
    memset(image + 0x1a2d0, 0, 0x1a400 - 0x1a2d0);
    memset(image + 0x1ad20, 0, 0x1ae00 - 0x1ad20);
    memset(image + 0x3d02e, 0, 0x3e200 - 0x3d02e);
    return image;
}

static struct blk_desc make_card(const uint8_t *package)
{
    struct blk_desc card = {0};
    card.image = package;
    card.first_lba = PACKAGE_LBA;
    card.sectors = H700_M1_PACKAGE_SECTORS;
    card.gpt_image = gpt;
    card.gpt_sector_count = GPT_SECTORS;
    return card;
}

static void test_partition_discovery_and_preflight(void)
{
    uint8_t *package = make_package();
    struct blk_desc card = make_card(package);
    uint64_t p4_first = 0, p4_blocks = 0;
    uint32_t digest = h700_m1_package_fnv1a(package, H700_M1_PACKAGE_BYTES);

    assert(h700_m1_spl_find_boot_partition(&card, DISK_SECTORS, 512u,
                                           &p4_first, &p4_blocks));
    assert(p4_first == 303104u);
    assert(p4_blocks == 131072u);
    assert(card.reads == 6u);
    card.reads = 0;
    assert(h700_m1_spl_preflight_gpt(&card, DISK_SECTORS, 512u, digest));
    assert(card.reads == 77u);
    assert(card.writes == 0u);
    free(package);
}

static void test_bad_gpt_and_ambiguous_boot_fail_closed(void)
{
    uint8_t *package = make_package();
    struct blk_desc card;
    uint64_t first = 0, blocks = 0;

    make_gpt();
    gpt[512u + 16u] ^= 1u;
    card = make_card(package);
    assert(!h700_m1_spl_find_boot_partition(&card, DISK_SECTORS, 512u,
                                             &first, &blocks));
    assert(!h700_m1_spl_preflight_gpt(&card, DISK_SECTORS, 512u, 0xaf0018cbu));
    assert(card.writes == 0u);

    make_gpt();
    set_name(entry(6u), "boot");
    seal_gpt();
    card = make_card(package);
    assert(!h700_m1_spl_find_boot_partition(&card, DISK_SECTORS, 512u,
                                             &first, &blocks));
    assert(!h700_m1_spl_preflight_gpt(&card, DISK_SECTORS, 512u, 0xaf0018cbu));
    assert(card.writes == 0u);
    free(package);
}

static void test_wrong_geometry_and_io_fail_closed(void)
{
    uint8_t *package = make_package();
    struct blk_desc card;
    uint64_t first = 0, blocks = 0;

    make_gpt();
    card = make_card(package);
    assert(!h700_m1_spl_find_boot_partition(&card, DISK_SECTORS, 4096u,
                                             &first, &blocks));
    assert(!h700_m1_spl_preflight_gpt(&card, DISK_SECTORS, 4096u, 0xaf0018cbu));
    card.fail_read = 1;
    assert(!h700_m1_spl_find_boot_partition(&card, DISK_SECTORS, 512u,
                                             &first, &blocks));
    assert(card.writes == 0u);
    free(package);
}

int main(void)
{
    make_gpt();
    test_partition_discovery_and_preflight();
    test_bad_gpt_and_ambiguous_boot_fail_closed();
    test_wrong_geometry_and_io_fail_closed();
    puts("PASS M1 SPL GPT adapter discovers captured p4 by validated GPT name");
    puts("PASS GPT CRC, duplicate-name, geometry and I/O failures fail closed");
    puts("PASS GPT discovery -> p4+40MiB -> 71 bounded firmware reads; zero writes");
    puts("LIMIT host fake-card integration; no U-Boot SPL or physical MMC execution");
    return 0;
}
