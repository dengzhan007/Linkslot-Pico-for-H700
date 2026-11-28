/*
 * Host-build-only M1 SPL preflight harness.
 *
 * This path intentionally has no continuation into sunxi_board_init(), DRAM
 * training, PMIC/TWI, display GPIO, or the normal vendor handoff. The test
 * fingerprint is zero, which the read-only planner rejects before firmware
 * payload reads. This is not an installable or bootable Linkslot image.
 */
#include <blk.h>
#include <mmc.h>

#include "m1_gpt_spl_adapter.h"

#define BASEOS_M1_PREFLIGHT_TEST_FNV1A 0u

static void m1_preflight_halt(void) __attribute__((noreturn));

static void m1_preflight_halt(void)
{
    for (;;)
        asm volatile("wfi" ::: "memory");
}

void baseos_early_mmc(void) __attribute__((noreturn));

void baseos_early_mmc(void)
{
    struct mmc *mmc;
    struct blk_desc *card;

    if (mmc_initialize(NULL))
        m1_preflight_halt();

    mmc = find_mmc_device(0);
    if (mmc == NULL || mmc_init(mmc))
        m1_preflight_halt();

    card = mmc_get_blk_desc(mmc);
    if (card == NULL || card->blksz > 0xffffffffUL)
        m1_preflight_halt();

    /*
     * FNV=0 is an explicit fail-closed build sentinel. The path can validate
     * the GPT and link the complete package preflight call graph, but cannot
     * accept a package generation or proceed to any post-preflight handoff.
     */
    (void)h700_m1_spl_preflight_gpt(card, card->lba, (uint32_t)card->blksz,
                                   BASEOS_M1_PREFLIGHT_TEST_FNV1A);
    m1_preflight_halt();
}
