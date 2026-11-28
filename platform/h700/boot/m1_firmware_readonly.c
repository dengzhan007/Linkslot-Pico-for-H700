#include "m1_firmware_readonly.h"

#define M1_PACKAGE_OFFSET_SECTORS (H700_M1_PACKAGE_OFFSET_BYTES / H700_M1_SECTOR_BYTES)

static int add_u64(uint64_t a, uint64_t b, uint64_t *out)
{
    if (UINT64_MAX - a < b)
        return 0;
    *out = a + b;
    return 1;
}

uint32_t h700_m1_package_fnv1a(const uint8_t *bytes, size_t length)
{
    uint32_t hash = UINT32_C(2166136261);
    if (bytes == 0)
        return 0;
    for (size_t i = 0; i < length; ++i) {
        hash ^= bytes[i];
        hash *= UINT32_C(16777619);
    }
    return hash;
}

int h700_m1_package_make_plan(uint64_t p4_first_lba,
                              uint64_t p4_block_count,
                              size_t staging_capacity,
                              uint32_t expected_fnv1a,
                              struct h700_m1_package_plan *out)
{
    uint64_t package_lba, p4_end, package_end;
    if (out == 0 || staging_capacity < H700_M1_STREAM_CHUNK_BYTES ||
        expected_fnv1a == 0)
        return 0;
    if (!add_u64(p4_first_lba, p4_block_count, &p4_end) ||
        !add_u64(p4_first_lba, M1_PACKAGE_OFFSET_SECTORS, &package_lba) ||
        !add_u64(package_lba, H700_M1_PACKAGE_SECTORS, &package_end))
        return 0;
    if (package_lba < p4_first_lba || package_end > p4_end)
        return 0;

    out->first_lba = package_lba;
    out->sectors = H700_M1_PACKAGE_SECTORS;
    out->bytes = H700_M1_PACKAGE_BYTES;
    out->expected_fnv1a = expected_fnv1a;
    out->monitor = (struct h700_m1_package_extent){0x00000, 0x1a2d0, 0x1a400, 0x48000000};
    out->dtbo    = (struct h700_m1_package_extent){0x1a400, 0x00920, 0x00a00, 0x4a100000};
    out->dtb     = (struct h700_m1_package_extent){0x1ae00, 0x2222e, 0x23400, 0x4a200000};
    out->bl33    = (struct h700_m1_package_extent){0x3e200, 0x08000, 0x08000, 0x4a000000};
    return 1;
}

static int zero_bytes(const uint8_t *bytes, uint32_t length)
{
    for (uint32_t i = 0; i < length; ++i) {
        if (bytes[i] != 0)
            return 0;
    }
    return 1;
}

static int plan_valid(const struct h700_m1_package_plan *plan)
{
    return plan != 0 && plan->expected_fnv1a != 0 &&
        UINT64_MAX - plan->first_lba >= plan->sectors &&
        plan->bytes == H700_M1_PACKAGE_BYTES &&
        plan->sectors == H700_M1_PACKAGE_SECTORS &&
        plan->monitor.offset == 0 && plan->monitor.payload_bytes == 0x1a2d0 &&
        plan->monitor.padded_bytes == 0x1a400 &&
        plan->dtbo.offset == 0x1a400 && plan->dtbo.payload_bytes == 0x920 &&
        plan->dtbo.padded_bytes == 0xa00 &&
        plan->dtb.offset == 0x1ae00 && plan->dtb.payload_bytes == 0x2222e &&
        plan->dtb.padded_bytes == 0x23400 &&
        plan->bl33.offset == 0x3e200 && plan->bl33.payload_bytes == 0x8000 &&
        plan->bl33.padded_bytes == 0x8000 &&
        plan->monitor.destination == 0x48000000 &&
        plan->dtbo.destination == 0x4a100000 &&
        plan->dtb.destination == 0x4a200000 &&
        plan->bl33.destination == 0x4a000000;
}

static uint32_t fnv1a_update(uint32_t hash, const uint8_t *bytes, size_t length)
{
    for (size_t i = 0; i < length; ++i) {
        hash ^= bytes[i];
        hash *= UINT32_C(16777619);
    }
    return hash;
}

static int zero_intersection(const uint8_t *bytes, uint32_t chunk_offset,
                             uint32_t chunk_bytes, uint32_t range_start,
                             uint32_t range_end)
{
    uint32_t start = chunk_offset > range_start ? chunk_offset : range_start;
    uint32_t chunk_end = chunk_offset + chunk_bytes;
    uint32_t end = chunk_end < range_end ? chunk_end : range_end;
    if (start >= end)
        return 1;
    return zero_bytes(bytes + (start - chunk_offset), end - start);
}

int h700_m1_package_read_validate_streaming(
    const struct h700_m1_readonly_block_device *device,
    const struct h700_m1_package_plan *plan,
    void *scratch, size_t scratch_capacity)
{
    uint8_t *buffer = (uint8_t *)scratch;
    uint32_t hash = UINT32_C(2166136261);
    uint32_t offset = 0;
    if (device == 0 || device->read_blocks == 0 || !plan_valid(plan) ||
        buffer == 0 || scratch_capacity < H700_M1_STREAM_CHUNK_BYTES)
        return 0;

    /* At most 4 KiB of SRAM is needed; 561 sectors are streamed in 8-sector reads. */
    while (offset < plan->bytes) {
        uint32_t remaining = plan->bytes - offset;
        uint32_t count = remaining / H700_M1_SECTOR_BYTES;
        if (remaining % H700_M1_SECTOR_BYTES)
            return 0;
        if (count > H700_M1_STREAM_CHUNK_SECTORS)
            count = H700_M1_STREAM_CHUNK_SECTORS;
        uint32_t bytes = count * H700_M1_SECTOR_BYTES;
        uint64_t lba = plan->first_lba + offset / H700_M1_SECTOR_BYTES;
        if (!device->read_blocks(device->context, lba, count, buffer))
            return 0;
        hash = fnv1a_update(hash, buffer, bytes);
        if (!zero_intersection(buffer, offset, bytes, 0x1a2d0, 0x1a400) ||
            !zero_intersection(buffer, offset, bytes, 0x1ad20, 0x1ae00) ||
            !zero_intersection(buffer, offset, bytes, 0x3d02e, 0x3e200))
            return 0;
        offset += bytes;
    }
    return offset == plan->bytes && hash == plan->expected_fnv1a;
}

int h700_m1_package_read_validate(
    const struct h700_m1_readonly_block_device *device,
    const struct h700_m1_package_plan *plan,
    void *staging, size_t staging_capacity)
{
    uint8_t *bytes = (uint8_t *)staging;
    if (device == 0 || device->read_blocks == 0 || !plan_valid(plan) ||
        bytes == 0 || staging_capacity < H700_M1_PACKAGE_BYTES)
        return 0;
    if (!device->read_blocks(device->context, plan->first_lba,
                             plan->sectors, bytes))
        return 0;
    if (h700_m1_package_fnv1a(bytes, H700_M1_PACKAGE_BYTES) !=
        plan->expected_fnv1a)
        return 0;
    if (!zero_bytes(bytes + plan->monitor.offset + plan->monitor.payload_bytes,
                    plan->monitor.padded_bytes - plan->monitor.payload_bytes) ||
        !zero_bytes(bytes + plan->dtbo.offset + plan->dtbo.payload_bytes,
                    plan->dtbo.padded_bytes - plan->dtbo.payload_bytes) ||
        !zero_bytes(bytes + plan->dtb.offset + plan->dtb.payload_bytes,
                    plan->dtb.padded_bytes - plan->dtb.payload_bytes))
        return 0;
    return 1;
}
