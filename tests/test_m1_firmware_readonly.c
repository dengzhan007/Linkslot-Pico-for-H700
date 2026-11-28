#include "m1_firmware_readonly.h"

#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

struct fake_card {
    const uint8_t *image;
    uint64_t first_lba;
    uint32_t sectors;
    unsigned reads;
    unsigned writes;
    int fail_read;
};

static int fake_read(void *context, uint64_t lba, uint32_t count, void *out)
{
    struct fake_card *card = (struct fake_card *)context;
    card->reads++;
    if (card->fail_read || lba != card->first_lba || count != card->sectors)
        return 0;
    memcpy(out, card->image, H700_M1_PACKAGE_BYTES);
    return 1;
}

static uint8_t *make_image(void)
{
    uint8_t *image = (uint8_t *)calloc(1, H700_M1_PACKAGE_BYTES);
    assert(image != 0);
    /* Deterministic nonzero payload bytes; both alignment tails stay zero. */
    for (uint32_t i = 0; i < 0x1a2d0; ++i)
        image[i] = (uint8_t)(i * 13u + 7u);
    for (uint32_t i = 0x1a400; i < 0x1a400 + 0x920; ++i)
        image[i] = (uint8_t)(i * 3u + 11u);
    for (uint32_t i = 0x1ae00; i < 0x3e200; ++i)
        image[i] = (uint8_t)(i * 5u + 19u);
    for (uint32_t i = 0x3e200; i < H700_M1_PACKAGE_BYTES; ++i)
        image[i] = (uint8_t)(i * 7u + 23u);
    return image;
}

int main(void)
{
    uint8_t *image = make_image();
    uint8_t *staging = (uint8_t *)malloc(H700_M1_PACKAGE_BYTES);
    assert(staging != 0);
    const uint64_t p4_lba = 32768;
    const uint64_t p4_blocks = 200000;
    const uint64_t expected_lba = p4_lba + 0x02800000u / 512u;
    struct h700_m1_package_plan plan;
    assert(h700_m1_package_make_plan(p4_lba, p4_blocks,
        H700_M1_PACKAGE_BYTES, h700_m1_package_fnv1a(image, H700_M1_PACKAGE_BYTES),
        &plan));
    assert(plan.first_lba == expected_lba);
    assert(plan.sectors == 0x231u);

    struct fake_card card = {image, expected_lba, 0x231u, 0, 0, 0};
    const struct h700_m1_readonly_block_device dev = {&card, fake_read};
    assert(h700_m1_package_read_validate(&dev, &plan, staging, H700_M1_PACKAGE_BYTES));
    assert(card.reads == 1 && card.writes == 0);
    assert(memcmp(image, staging, H700_M1_PACKAGE_BYTES) == 0);

    assert(!h700_m1_package_make_plan(p4_lba, 0x02800000u / 512u + 0x230u,
        H700_M1_PACKAGE_BYTES, plan.expected_fnv1a, &plan));
    assert(!h700_m1_package_make_plan(UINT64_MAX - 10u, UINT64_MAX,
        H700_M1_PACKAGE_BYTES, plan.expected_fnv1a, &plan));
    assert(!h700_m1_package_make_plan(p4_lba, p4_blocks,
        H700_M1_PACKAGE_BYTES - 1u, plan.expected_fnv1a, &plan));

    assert(h700_m1_package_make_plan(p4_lba, p4_blocks,
        H700_M1_PACKAGE_BYTES, plan.expected_fnv1a, &plan));
    card.fail_read = 1;
    assert(!h700_m1_package_read_validate(&dev, &plan, staging, H700_M1_PACKAGE_BYTES));
    card.fail_read = 0;

    image[0] ^= 1u;
    assert(!h700_m1_package_read_validate(&dev, &plan, staging, H700_M1_PACKAGE_BYTES));
    image[0] ^= 1u;

    image[0x1a2d0] = 1u; /* monitor padding must be zero */
    plan.expected_fnv1a = h700_m1_package_fnv1a(image, H700_M1_PACKAGE_BYTES);
    card.image = image;
    assert(!h700_m1_package_read_validate(&dev, &plan, staging, H700_M1_PACKAGE_BYTES));
    image[0x1a2d0] = 0u;

    card.image = image;
    plan.expected_fnv1a = h700_m1_package_fnv1a(image, H700_M1_PACKAGE_BYTES);
    plan.bl33.destination ^= 4u;
    assert(!h700_m1_package_read_validate(&dev, &plan, staging, H700_M1_PACKAGE_BYTES));

    free(staging);
    free(image);
    puts("PASS M1 firmware read-only package preflight");
    puts("LIMIT: host fake-card only; not wired to BaseOS SPL or physical addresses");
    return 0;
}
