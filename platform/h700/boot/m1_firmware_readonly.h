#ifndef H700_M1_FIRMWARE_READONLY_H
#define H700_M1_FIRMWARE_READONLY_H

#include <stddef.h>
#if defined(CONFIG_XPL_BUILD) || defined(CONFIG_SPL_BUILD)
#include <linux/types.h>
#ifndef UINT32_C
#define UINT32_C(value) value##U
#endif
#ifndef UINT64_C
#define UINT64_C(value) value##ULL
#endif
#ifndef UINT64_MAX
#define UINT64_MAX 0xffffffffffffffffULL
#endif
#else
#include <stdint.h>
#endif

/*
 * Host-tested M1 firmware-package preflight contract.
 * This is not yet wired into BaseOS SPL and performs no MMIO or copying to
 * physical destination addresses. The reader ABI exposes read only.
 */
#define H700_M1_PACKAGE_OFFSET_BYTES UINT64_C(0x02800000)
#define H700_M1_PACKAGE_BYTES        UINT32_C(0x00046200)
#define H700_M1_PACKAGE_SECTORS      UINT32_C(0x231)
#define H700_M1_SECTOR_BYTES         UINT32_C(512)
#define H700_M1_STREAM_CHUNK_SECTORS UINT32_C(8)
#define H700_M1_STREAM_CHUNK_BYTES   (H700_M1_STREAM_CHUNK_SECTORS * H700_M1_SECTOR_BYTES)

struct h700_m1_package_extent {
    uint32_t offset;
    uint32_t payload_bytes;
    uint32_t padded_bytes;
    uint32_t destination;
};

struct h700_m1_package_plan {
    uint64_t first_lba;
    uint32_t sectors;
    uint32_t bytes;
    struct h700_m1_package_extent monitor;
    struct h700_m1_package_extent dtbo;
    struct h700_m1_package_extent dtb;
    struct h700_m1_package_extent bl33;
    uint32_t expected_fnv1a;
};

struct h700_m1_readonly_block_device {
    void *context;
    int (*read_blocks)(void *context, uint64_t first_lba,
                       uint32_t block_count, void *destination);
};

int h700_m1_package_make_plan(uint64_t p4_first_lba,
                              uint64_t p4_block_count,
                              size_t staging_capacity,
                              uint32_t expected_fnv1a,
                              struct h700_m1_package_plan *out);
uint32_t h700_m1_package_fnv1a(const uint8_t *bytes, size_t length);
int h700_m1_package_read_validate_streaming(
    const struct h700_m1_readonly_block_device *device,
    const struct h700_m1_package_plan *plan,
    void *scratch, size_t scratch_capacity);
int h700_m1_package_read_validate(
    const struct h700_m1_readonly_block_device *device,
    const struct h700_m1_package_plan *plan,
    void *staging, size_t staging_capacity);

#endif
