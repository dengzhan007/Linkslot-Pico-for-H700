#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>

#include "fat_ro.h"

#define DISK_SECTORS 80000u
#define CHECK(c, m) do { \
    if (!(c)) { fprintf(stderr, "FAIL: %s (line %d)\n", m, __LINE__); return 0; } \
} while (0)

struct fake_disk {
    uint32_t base_lba;
    uint8_t mbr[512];
    uint8_t vbr[512];
    uint8_t exfat_root[512];
    uint8_t gpt_header[512];
    uint8_t gpt_entries[1024];
    uint32_t root_lba;
    uint32_t reads;
    uint32_t root_reads;
    int gpt_enabled;
};

static void put16(uint8_t *p, uint16_t v)
{
    p[0] = (uint8_t)v;
    p[1] = (uint8_t)(v >> 8);
}

static void put32(uint8_t *p, uint32_t v)
{
    p[0] = (uint8_t)v;
    p[1] = (uint8_t)(v >> 8);
    p[2] = (uint8_t)(v >> 16);
    p[3] = (uint8_t)(v >> 24);
}

static unsigned fake_read(void *context, uint32_t lba,
                          uint32_t count, void *buffer)
{
    struct fake_disk *disk = context;
    uint32_t i;
    if (count == 0u)
        return 0u;
    for (i = 0; i < count; ++i) {
        uint32_t current = lba + i;
        uint8_t *out = (uint8_t *)buffer + i * 512u;
        ++disk->reads;
        if (current == 0u) {
            memcpy(out, disk->mbr, 512u);
        } else if (disk->gpt_enabled && current == 1u) {
            memcpy(out, disk->gpt_header, 512u);
        } else if (disk->gpt_enabled && (current == 2u || current == 3u)) {
            memcpy(out, disk->gpt_entries + (current - 2u) * 512u, 512u);
        } else if (current == disk->base_lba) {
            memcpy(out, disk->vbr, 512u);
        } else if (current == disk->root_lba && disk->root_lba != 0u) {
            memcpy(out, disk->exfat_root, 512u);
            ++disk->root_reads;
        } else {
            return i;
        }
    }
    return count;
}

static uint32_t crc32_bytes(const uint8_t *data, size_t length)
{
    uint32_t crc = 0xffffffffu;
    size_t i;
    unsigned int bit;
    for (i = 0; i < length; ++i) {
        crc ^= data[i];
        for (bit = 0; bit < 8u; ++bit)
            crc = (crc >> 1) ^
                (0xedb88320u & (uint32_t)-(int32_t)(crc & 1u));
    }
    return ~crc;
}

static void put64(uint8_t *p, uint64_t value)
{
    put32(p, (uint32_t)value);
    put32(p + 4u, (uint32_t)(value >> 32));
}

static void make_gpt_entry(struct fake_disk *disk, unsigned slot,
                           uint64_t first, uint64_t last)
{
    uint8_t *entry = disk->gpt_entries + slot * 128u;
    unsigned i;
    memset(entry, 0, 128u);
    entry[0] = 0x28u; /* nonzero type GUID */
    for (i = 0; i < 16u; ++i)
        entry[16u + i] = (uint8_t)(slot + i + 1u);
    put64(entry + 32u, first);
    put64(entry + 40u, last);
}

static void make_gpt(struct fake_disk *disk)
{
    uint8_t *mbr = disk->mbr;
    uint8_t *header = disk->gpt_header;
    memset(mbr, 0, sizeof(disk->mbr));
    memset(header, 0, sizeof(disk->gpt_header));
    memset(disk->gpt_entries, 0, sizeof(disk->gpt_entries));

    mbr[446u + 4u] = 0xeeu;
    put32(mbr + 446u + 8u, 1u);
    put32(mbr + 446u + 12u, DISK_SECTORS - 1u);
    mbr[510] = 0x55u; mbr[511] = 0xaau;

    /* p6 is an unrelated non-FAT candidate; p7 is the FAT volume under test. */
    make_gpt_entry(disk, 5u, 100u, 199u);
    make_gpt_entry(disk, 6u, 4u, 70003u);

    memcpy(header, "EFI PART", 8u);
    put32(header + 8u, 0x00010000u);
    put32(header + 12u, 92u);
    put64(header + 24u, 1u);
    put64(header + 32u, DISK_SECTORS - 1u);
    put64(header + 40u, 4u);
    put64(header + 48u, DISK_SECTORS - 2u);
    header[56u] = 0x42u; /* nonzero disk GUID */
    put64(header + 72u, 2u);
    put32(header + 80u, 8u);
    put32(header + 84u, 128u);
    put32(header + 88u, crc32_bytes(disk->gpt_entries, sizeof(disk->gpt_entries)));
    put32(header + 16u, 0u);
    put32(header + 16u, crc32_bytes(header, 92u));
    disk->base_lba = 4u;
    disk->gpt_enabled = 1;
}

static void make_fat32_vbr(struct fake_disk *disk,
                           uint32_t total_sectors,
                           uint32_t fat_sectors)
{
    uint8_t *b = disk->vbr;
    memset(b, 0, 512u);
    b[0] = 0xebu; b[1] = 0x58u; b[2] = 0x90u;
    memcpy(b + 3u, "MSWIN4.1", 8u);
    put16(b + 0x0bu, 512u);
    b[0x0du] = 1u;
    put16(b + 0x0eu, 32u);
    b[0x10u] = 2u;
    put16(b + 0x11u, 0u);
    put16(b + 0x13u, 0u);
    b[0x15u] = 0xf8u;
    put16(b + 0x16u, 0u);
    put32(b + 0x20u, total_sectors);
    put32(b + 0x24u, fat_sectors);
    put32(b + 0x2cu, 2u);
    b[510] = 0x55u; b[511] = 0xaau;
}

static void make_fat16_vbr(struct fake_disk *disk)
{
    uint8_t *b = disk->vbr;
    memset(b, 0, 512u);
    b[0] = 0xebu; b[1] = 0x3cu; b[2] = 0x90u;
    memcpy(b + 3u, "MSDOS5.0", 8u);
    put16(b + 0x0bu, 512u);
    b[0x0du] = 1u;
    put16(b + 0x0eu, 1u);
    b[0x10u] = 2u;
    put16(b + 0x11u, 512u);
    put16(b + 0x13u, 4169u);
    b[0x15u] = 0xf8u;
    put16(b + 0x16u, 20u);
    b[510] = 0x55u; b[511] = 0xaau;
}

static void make_exfat_vbr(struct fake_disk *disk, uint32_t volume_sectors)
{
    uint8_t *b = disk->vbr;
    memset(b, 0, 512u);
    b[0] = 0xebu; b[1] = 0x76u; b[2] = 0x90u;
    memcpy(b + 3u, "EXFAT   ", 8u);
    put32(b + 0x48u, volume_sectors);
    put32(b + 0x4cu, 0u);
    put32(b + 0x50u, 24u);       /* FAT begins after the reserved region. */
    put32(b + 0x54u, 32u);       /* 32 sectors * 128 FAT entries = 4096. */
    put32(b + 0x58u, 56u);       /* Cluster heap follows the FAT. */
    put32(b + 0x5cu, 4000u);
    put32(b + 0x60u, 2u);        /* Root directory cluster. */
    put16(b + 0x6au, 0u);
    b[0x6cu] = 9u;               /* 512-byte sectors. */
    b[0x6du] = 0u;               /* One sector per cluster. */
    b[0x6eu] = 1u;               /* One FAT. */
    b[510] = 0x55u; b[511] = 0xaau;

    memset(disk->exfat_root, 0, sizeof(disk->exfat_root));
    disk->root_lba = disk->base_lba + 56u;
    disk->exfat_root[0] = 0x81u; /* Allocation bitmap entry. */
    disk->exfat_root[1] = 0u;
    put32(disk->exfat_root + 0x14u, 3u);
    put32(disk->exfat_root + 0x18u, 500u); /* ceil(4000 / 8). */
}

static int test_fat32_partition_bounds(void)
{
    struct fake_disk disk;
    fat_vol volume;
    memset(&disk, 0, sizeof(disk));
    disk.base_lba = 100u;
    make_fat32_vbr(&disk, 70000u, 600u);

    CHECK(fat_mount_partition(&volume, fake_read, &disk,
                              disk.base_lba, 70000u) == 0,
          "FAT32 volume mounts when BPB length fits partition");
    CHECK(volume.is_fat32 && volume.part_lba == 100u,
          "FAT32 type and partition start are retained");
    CHECK(volume.partition_sectors == 70000u &&
          volume.volume_sectors == 70000u,
          "FAT32 partition and BPB lengths are recorded");

    CHECK(fat_mount_partition(&volume, fake_read, &disk,
                              disk.base_lba, 69999u) != 0,
          "FAT32 BPB length beyond partition is rejected");
    CHECK(fat_mount_partition(&volume, fake_read, &disk,
                              0xfffffff0u, 32u) != 0,
          "partition LBA range overflow is rejected");

    make_fat32_vbr(&disk, 70000u, 1u);
    CHECK(fat_mount_partition(&volume, fake_read, &disk,
                              disk.base_lba, 70000u) != 0,
          "FAT32 FAT too small for declared cluster count is rejected");

    make_fat32_vbr(&disk, 70000u, 600u);
    put32(disk.vbr + 0x2cu, 70000u);
    CHECK(fat_mount_partition(&volume, fake_read, &disk,
                              disk.base_lba, 70000u) != 0,
          "FAT32 root cluster outside data cluster range is rejected");
    return 1;
}

static int test_fat16_partition_bounds(void)
{
    struct fake_disk disk;
    fat_vol volume;
    memset(&disk, 0, sizeof(disk));
    disk.base_lba = 200u;
    make_fat16_vbr(&disk);

    CHECK(fat_mount_partition(&volume, fake_read, &disk,
                              disk.base_lba, 4169u) == 0,
          "FAT16 volume mounts inside its partition");
    CHECK(!volume.is_fat32 && volume.volume_sectors == 4169u,
          "FAT16 layout and length are recorded");
    CHECK(fat_mount_partition(&volume, fake_read, &disk,
                              disk.base_lba, 4168u) != 0,
          "FAT16 BPB length beyond partition is rejected");
    return 1;
}

static int test_exfat_partition_bounds(void)
{
    struct fake_disk disk;
    fat_vol volume;
    memset(&disk, 0, sizeof(disk));
    disk.base_lba = 300u;
    make_exfat_vbr(&disk, 4096u);

    CHECK(fat_mount_partition(&volume, fake_read, &disk,
                              disk.base_lba, 4096u) == 0,
          "exFAT volume mounts when metadata fits partition");
    CHECK(volume.is_exfat && volume.volume_sectors == 4096u &&
          volume.partition_sectors == 4096u,
          "exFAT volume and partition lengths are retained");
    CHECK(disk.root_reads == 1u,
          "exFAT root metadata is read only after bounds validation");

    disk.root_reads = 0u;
    CHECK(fat_mount_partition(&volume, fake_read, &disk,
                              disk.base_lba, 4095u) != 0,
          "exFAT volume length beyond partition is rejected");
    CHECK(disk.root_reads == 0u,
          "out-of-bound exFAT is rejected before root-cluster reads");

    make_exfat_vbr(&disk, 4096u);
    put32(disk.vbr + 0x58u, 4000u); /* Heap end exceeds VolumeLength. */
    CHECK(fat_mount_partition(&volume, fake_read, &disk,
                              disk.base_lba, 4096u) != 0,
          "exFAT cluster heap extending beyond volume is rejected");
    CHECK(disk.root_reads == 0u,
          "invalid exFAT heap is rejected before root access");
    return 1;
}

static int test_gpt_scan_uses_partition_extent(void)
{
    struct fake_disk disk;
    fat_vol volume;

    memset(&disk, 0, sizeof(disk));
    make_gpt(&disk);
    make_fat32_vbr(&disk, 70000u, 600u);

    CHECK(fat_mount(&volume, fake_read, &disk) == 0,
          "GPT p7 FAT32 volume mounts when BPB fits GPT extent");
    CHECK(volume.part_lba == 4u &&
          volume.partition_sectors == 70000u &&
          volume.volume_sectors == 70000u,
          "GPT partition extent is passed through to the mounted volume");

    make_fat32_vbr(&disk, 70001u, 600u);
    CHECK(fat_mount(&volume, fake_read, &disk) != 0,
          "GPT p7 BPB volume extending one sector beyond partition is rejected");
    return 1;
}

static int test_legacy_mbr_scan_uses_partition_extent(void)
{
    struct fake_disk disk;
    fat_vol volume;
    memset(&disk, 0, sizeof(disk));
    disk.base_lba = 100u;
    make_fat32_vbr(&disk, 70000u, 600u);

    disk.mbr[446u + 4u] = 0x0cu;
    put32(disk.mbr + 446u + 8u, disk.base_lba);
    put32(disk.mbr + 446u + 12u, 69999u); /* Too small for BPB. */
    disk.mbr[510] = 0x55u; disk.mbr[511] = 0xaau;

    CHECK(fat_mount(&volume, fake_read, &disk) != 0,
          "legacy MBR scan passes partition extent to BPB validation");
    return 1;
}

int main(void)
{
    if (!test_fat32_partition_bounds() ||
        !test_fat16_partition_bounds() ||
        !test_exfat_partition_bounds() ||
        !test_gpt_scan_uses_partition_extent() ||
        !test_legacy_mbr_scan_uses_partition_extent())
        return EXIT_FAILURE;

    puts("PASS: FAT16/FAT32/exFAT partition-bounds mount tests");
    return EXIT_SUCCESS;
}
