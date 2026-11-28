/*
 * U-Boot SPL adapter for the M1 read-only package preflight.
 * It intentionally exposes only blk_dread; no blk_dwrite symbol is referenced.
 * The caller must supply a validated p4 extent and a bounded SRAM scratch buffer (4 KiB minimum).
 */
#include <blk.h>
#include "m1_firmware_readonly.h"

static int spl_read_blocks(void *context, uint64_t first_lba,
                           uint32_t block_count, void *destination)
{
    struct blk_desc *card = (struct blk_desc *)context;
    return blk_dread(card, (lbaint_t)first_lba, (lbaint_t)block_count,
                     destination) == (lbaint_t)block_count;
}

int h700_m1_spl_preflight(struct blk_desc *card,
                          uint64_t p4_first_lba,
                          uint64_t p4_block_count,
                          uint32_t expected_fnv1a,
                          void *sram_staging,
                          size_t staging_capacity)
{
    struct h700_m1_package_plan plan;
    const struct h700_m1_readonly_block_device device = {
        card, spl_read_blocks
    };
    if (card == 0 ||
        !h700_m1_package_make_plan(p4_first_lba, p4_block_count,
                                   staging_capacity, expected_fnv1a, &plan))
        return 0;
    return h700_m1_package_read_validate_streaming(&device, &plan, sram_staging,
                                                   staging_capacity);
}
