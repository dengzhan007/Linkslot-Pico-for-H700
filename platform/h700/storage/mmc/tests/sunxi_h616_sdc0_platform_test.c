#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>

#include "../sunxi_h616_sdc0_platform.h"
#include "../sunxi_mmc_regs.h"

#define CHECK(condition, message) do { \
    if (!(condition)) { fprintf(stderr, "FAIL: %s (line %d)\n", message, __LINE__); return 0; } \
} while (0)

struct fake_platform {
    uint32_t mmc_regs[129];
    uint32_t ccu_regs[0x850u / 4u];
    uint32_t clock_writes;
    uint32_t update_commands;
    uint64_t now_ms;
    int hold_update;
};

static uint32_t read_mmc(void *context, uint32_t offset)
{
    struct fake_platform *fake = (struct fake_platform *)context;
    if ((offset & 3u) != 0u || offset >= sizeof(fake->mmc_regs))
        return 0u;
    return fake->mmc_regs[offset / 4u];
}

static void write_mmc(void *context, uint32_t offset, uint32_t value)
{
    struct fake_platform *fake = (struct fake_platform *)context;
    uint32_t index;
    if ((offset & 3u) != 0u || offset >= sizeof(fake->mmc_regs))
        return;
    index = offset / 4u;

    if (offset == SUNXI_MMC_REG_CMD &&
        (value & SUNXI_MMC_CMD_UPCLK_ONLY) != 0u) {
        ++fake->update_commands;
        fake->mmc_regs[index] = fake->hold_update
                                    ? value
                                    : value & ~SUNXI_MMC_CMD_START;
        fake->mmc_regs[SUNXI_MMC_REG_RINT / 4u] |=
            SUNXI_MMC_RINT_VOLTAGE_CHANGE_DONE;
        return;
    }
    if (offset == SUNXI_MMC_REG_RINT) {
        fake->mmc_regs[index] &= ~value;
        return;
    }
    fake->mmc_regs[index] = value;
}

static uint64_t fake_time_ms(void *context)
{
    struct fake_platform *fake = (struct fake_platform *)context;
    return fake->now_ms++;
}

static void fake_delay_us(void *context, uint32_t delay)
{
    struct fake_platform *fake = (struct fake_platform *)context;
    fake->now_ms += delay / 1000u;
}

static uint32_t read_ccu(void *context, uint32_t offset)
{
    struct fake_platform *fake = (struct fake_platform *)context;
    if ((offset & 3u) != 0u || offset >= sizeof(fake->ccu_regs))
        return 0u;
    return fake->ccu_regs[offset / 4u];
}

static void write_ccu(void *context, uint32_t offset, uint32_t value)
{
    struct fake_platform *fake = (struct fake_platform *)context;
    if ((offset & 3u) != 0u || offset >= sizeof(fake->ccu_regs))
        return;
    fake->ccu_regs[offset / 4u] = value;
    if (offset == SUNXI_CCU_MMC0_CLK_CFG)
        ++fake->clock_writes;
}

static void fake_init(struct fake_platform *fake,
                      struct sunxi_mmc_io *mmc,
                      struct sunxi_h616_sdc0_platform *platform)
{
    memset(fake, 0, sizeof(*fake));
    mmc->context = fake;
    mmc->read32 = read_mmc;
    mmc->write32 = write_mmc;
    mmc->time_ms = fake_time_ms;
    mmc->delay_us = fake_delay_us;
    mmc->read_barrier = NULL;

    platform->mmc_io = mmc;
    platform->ccu.context = fake;
    platform->ccu.read32 = read_ccu;
    platform->ccu.write32 = write_ccu;
    platform->osc24m_hz = 24000000u;
    platform->pll6_effective_hz = 600000000u;
}

static int test_pio_apb1_clock_selects_osc24m(void)
{
    struct fake_platform fake;
    struct sunxi_mmc_io mmc;
    struct sunxi_h616_sdc0_platform platform;
    const uint32_t initial = 0x8000040cu;
    const uint32_t apb1_mask = 0x03000303u;

    fake_init(&fake, &mmc, &platform);
    platform.pll6_effective_hz = 0u; /* PIO/APB1 path does not use PLL. */
    fake.ccu_regs[0x520u / 4u] = initial;

    CHECK(sunxi_h616_pio_enable_apb1_24m(&platform) == 0,
          "H616 APB1 clock selects OSC24M for PIO");
    CHECK(fake.ccu_regs[0x520u / 4u] == (initial & ~apb1_mask),
          "APB1 mux/divider fields are programmed and unrelated bits preserved");

    platform.osc24m_hz = 25000000u;
    CHECK(sunxi_h616_pio_enable_apb1_24m(&platform) != 0,
          "unverified oscillator rate is rejected rather than guessed");
    CHECK(fake.ccu_regs[0x520u / 4u] == (initial & ~apb1_mask),
          "invalid oscillator rate does not modify APB1");
    return 1;
}

static int test_bus_gate_and_reset(void)
{
    struct fake_platform fake;
    struct sunxi_mmc_io mmc;
    struct sunxi_h616_sdc0_platform platform;
    uint32_t initial = 0x80000000u;

    fake_init(&fake, &mmc, &platform);
    fake.ccu_regs[SUNXI_CCU_MMC_GATE_RESET / 4u] = initial;

    CHECK(sunxi_h616_sdc0_enable_bus(&platform) == 0,
          "sdc0 bus gate and reset deassert succeed");
    CHECK(fake.ccu_regs[SUNXI_CCU_MMC_GATE_RESET / 4u] ==
              (initial | SUNXI_CCU_MMC_GATE_BIT(0) |
               SUNXI_CCU_MMC_RESET_BIT(0)),
          "only sdc0 gate/reset bits are added");
    return 1;
}

static int test_400khz_clock_programming(void)
{
    struct fake_platform fake;
    struct sunxi_mmc_io mmc;
    struct sunxi_h616_sdc0_platform platform;
    uint32_t clkcr = 0x0000045au;

    fake_init(&fake, &mmc, &platform);
    platform.pll6_effective_hz = 0u; /* 400 kHz uses HOSC only. */
    fake.mmc_regs[SUNXI_MMC_REG_CLKCR / 4u] = clkcr;

    CHECK(sunxi_h616_sdc0_set_clock_hz(&platform, 400000u, 20u) == 0,
          "400 kHz sdc0 clock config succeeds");
    CHECK(fake.clock_writes == 1u, "module-clock CCU register written once");
    CHECK(fake.ccu_regs[SUNXI_CCU_MMC0_CLK_CFG / 4u] == 0x8000020eu,
          "400 kHz module-clock register value is exact");
    CHECK((fake.mmc_regs[SUNXI_MMC_REG_CLKCR / 4u] &
           SUNXI_MMC_CLKCR_CARD_CLOCK) != 0u,
          "card output clock enabled");
    CHECK((fake.mmc_regs[SUNXI_MMC_REG_CLKCR / 4u] &
           SUNXI_MMC_CLKCR_MASK_DATA0) == 0u,
          "MASK_DATA0 cleared after update");
    CHECK((fake.mmc_regs[SUNXI_MMC_REG_CLKCR / 4u] &
           SUNXI_MMC_CLKCR_DIVIDER_MASK) == 0u,
          "internal divider is zero");
    CHECK(fake.update_commands == 2u,
          "clock-off and clock-on each use UPCLK_ONLY");
    CHECK((fake.mmc_regs[SUNXI_MMC_REG_CLKCR / 4u] & 0x00000400u) != 0u,
          "unrelated CLKCR bits are preserved");
    return 1;
}

static int test_rejects_unvalidated_transfer_rates(void)
{
    struct fake_platform fake;
    struct sunxi_mmc_io mmc;
    struct sunxi_h616_sdc0_platform platform;

    fake_init(&fake, &mmc, &platform);
    CHECK(sunxi_h616_sdc0_set_clock_hz(&platform, 1000000u, 20u) != 0,
          "non-identification rate below 24 MHz is not yet enabled");
    CHECK(sunxi_h616_sdc0_set_clock_hz(&platform, 24000000u, 20u) != 0,
          "24 MHz card clock is not yet enabled");
    CHECK(sunxi_h616_sdc0_set_clock_hz(&platform, 25000000u, 20u) != 0,
          "clock above 24 MHz is rejected pending parent/timing validation");
    CHECK(fake.clock_writes == 0u && fake.update_commands == 0u,
          "all unvalidated transfer-rate requests fail before hardware access");
    return 1;
}

static int test_clock_update_timeout(void)
{
    struct fake_platform fake;
    struct sunxi_mmc_io mmc;
    struct sunxi_h616_sdc0_platform platform;

    fake_init(&fake, &mmc, &platform);
    fake.hold_update = 1;
    CHECK(sunxi_h616_sdc0_set_clock_hz(&platform, 400000u, 2u) != 0,
          "stuck UPCLK_ONLY update returns failure");
    CHECK((fake.mmc_regs[SUNXI_MMC_REG_CLKCR / 4u] &
           SUNXI_MMC_CLKCR_MASK_DATA0) == 0u,
          "MASK_DATA0 is cleared after timeout");
    CHECK(fake.clock_writes == 0u,
          "module clock is not programmed after failed clock-off update");
    return 1;
}

int main(void)
{
    if (!test_pio_apb1_clock_selects_osc24m() ||
        !test_bus_gate_and_reset() ||
        !test_400khz_clock_programming() ||
        !test_rejects_unvalidated_transfer_rates() ||
        !test_clock_update_timeout())
        return EXIT_FAILURE;

    puts("PASS: H616 sdc0 gate/reset and 400 kHz clock integration tests");
    return EXIT_SUCCESS;
}
