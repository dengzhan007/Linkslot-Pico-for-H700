#include "m1_firmware_readonly.h"

#define M1_LBA_PER_MIB UINT64_C(2048)
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
    if (out == 0 || staging_capacity < H700_M1_PACKAGE_BYTES ||
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
    out->dtb     = (struct h700_m1_package_extent){0x1ae00, 0x23400, 0x23400, 0x4a200000};
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

int h700_m1_package_read_validate(
    const struct h700_m1_readonly_block_device *device,
    const struct h700_m1_package_plan *plan,
    void *staging, size_t staging_capacity)
{
    uint8_t *bytes = (uint8_t *)staging;
    if (device == 0 || device->read_blocks == 0 || plan == 0 ||
        bytes == 0 || staging_capacity < H700_M1_PACKAGE_BYTES ||
        plan->bytes != H700_M1_PACKAGE_BYTES ||
        plan->sectors != H700_M1_PACKAGE_SECTORS ||
        plan->monitor.offset != 0 || plan->monitor.payload_bytes != 0x1a2d0 ||
        plan->monitor.padded_bytes != 0x1a400 ||
        plan->dtbo.offset != 0x1a400 || plan->dtbo.payload_bytes != 0x920 ||
        plan->dtbo.padded_bytes != 0xa00 ||
        plan->dtb.offset != 0x1ae00 || plan->dtb.payload_bytes != 0x23400 ||
        plan->dtb.padded_bytes != 0x23400 ||
        plan->bl33.offset != 0x3e200 || plan->bl33.payload_bytes != 0x8000 ||
        plan->bl33.padded_bytes != 0x8000 ||
        plan->monitor.destination != 0x48000000 ||
        plan->dtbo.destination != 0x4a100000 ||
        plan->dtb.destination != 0x4a200000 ||
        plan->bl33.destination != 0x4a000000)
        return 0;

    /* Exactly one bounded block read; no write operation exists in this ABI. */
    if (!device->read_blocks(device->context, plan->first_lba,
                             plan->sectors, bytes))
        return 0;
    if (h700_m1_package_fnv1a(bytes, H700_M1_PACKAGE_BYTES) !=
        plan->expected_fnv1a)
        return 0;
    if (!zero_bytes(bytes + plan->monitor.offset + plan->monitor.payload_bytes,
                    plan->monitor.padded_bytes - plan->monitor.payload_bytes) ||
        !zero_bytes(bytes + plan->dtbo.offset + plan->dtbo.payload_bytes,
                    plan->dtbo.padded_bytes - plan->dtbo.payload_bytes))
        return 0;
    return 1;
}
