#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <blk.h>
#include "m1_firmware_readonly.h"

int h700_m1_spl_preflight(struct blk_desc *, uint64_t, uint64_t, uint32_t,
                          void *, size_t);

lbaint_t blk_dread(struct blk_desc *desc, lbaint_t start,
                   lbaint_t count, void *destination)
{
    desc->reads++;
    if (desc->fail_read || start < desc->first_lba ||
        start - desc->first_lba + count > desc->sectors ||
        count == 0 || count > H700_M1_STREAM_CHUNK_SECTORS)
        return 0;
    uint64_t offset = (start - desc->first_lba) * H700_M1_SECTOR_BYTES;
    memcpy(destination, desc->image + offset, (size_t)count * H700_M1_SECTOR_BYTES);
    return count;
}

static uint8_t *fixture(void)
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

int main(void)
{
    uint8_t *image = fixture();
    uint8_t *staging = malloc(H700_M1_STREAM_CHUNK_BYTES);
    assert(staging);
    const uint64_t p4_lba = 303104;
    const uint64_t p4_blocks = 131072;
    const uint64_t package_lba = p4_lba + H700_M1_PACKAGE_OFFSET_BYTES / 512u;
    struct blk_desc card = {image, package_lba, H700_M1_PACKAGE_SECTORS, 0, 0, 0};
    uint32_t digest = h700_m1_package_fnv1a(image, H700_M1_PACKAGE_BYTES);

    assert(h700_m1_spl_preflight(&card, p4_lba, p4_blocks, digest,
                                 staging, H700_M1_STREAM_CHUNK_BYTES));
    assert(card.reads == 71 && card.writes == 0);

    image[0x3d02e] = 1u;
    digest = h700_m1_package_fnv1a(image, H700_M1_PACKAGE_BYTES);
    unsigned reads_before_bad_padding = card.reads;
    assert(!h700_m1_spl_preflight(&card, p4_lba, p4_blocks, digest,
                                  staging, H700_M1_STREAM_CHUNK_BYTES));
    assert(card.reads > reads_before_bad_padding && card.writes == 0);
    image[0x3d02e] = 0u;
    digest = h700_m1_package_fnv1a(image, H700_M1_PACKAGE_BYTES);
    card.reads = 0;

    card.fail_read = 1;
    assert(!h700_m1_spl_preflight(&card, p4_lba, p4_blocks, digest,
                                  staging, H700_M1_STREAM_CHUNK_BYTES));
    assert(card.reads == 1 && card.writes == 0);
    card.fail_read = 0;

    assert(!h700_m1_spl_preflight(&card, p4_lba, 0x02800000u / 512u + 0x230u,
                                  digest, staging, H700_M1_STREAM_CHUNK_BYTES));
    assert(card.reads == 1 && card.writes == 0);

    assert(!h700_m1_spl_preflight(&card, p4_lba, p4_blocks, digest,
                                  staging, H700_M1_STREAM_CHUNK_BYTES - 1u));
    assert(card.reads == 1 && card.writes == 0);

    free(staging);
    free(image);
    puts("PASS SPL blk_dread adapter fake-card integration");
    puts("PASS bounded 8-sector reads; zero block writes");
    puts("LIMIT host shim only; not linked to or run by physical BaseOS SPL");
    return 0;
}
