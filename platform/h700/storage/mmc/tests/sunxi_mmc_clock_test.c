#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "../sunxi_mmc_clock.h"
#include "../sunxi_mmc_regs.h"

#define REG_WORDS 129u
#define MAX_CLKCR_WRITES 8u

struct fake_mmc {
    uint32_t regs[REG_WORDS];
    uint32_t clkcr_writes[MAX_CLKCR_WRITES];
    size_t clkcr_write_count;
    uint32_t last_rint_write;
    unsigned rint_write_count;
    unsigned delay_count;
    uint64_t now_ms;
    int hold_command_start;
};

static uint32_t fake_read32(void *context, uint32_t offset)
{
    struct fake_mmc *fake = (struct fake_mmc *)context;

    if (offset >= REG_WORDS * 4u || (offset & 3u) != 0u)
        return 0u;

    return fake->regs[offset / 4u];
}

static void fake_write32(void *context, uint32_t offset, uint32_t value)
{
    struct fake_mmc *fake = (struct fake_mmc *)context;

    if (offset >= REG_WORDS * 4u || (offset & 3u) != 0u)
        return;

    if (offset == SUNXI_MMC_REG_CLKCR &&
        fake->clkcr_write_count < MAX_CLKCR_WRITES) {
        fake->clkcr_writes[fake->clkcr_write_count++] = value;
    }

    if (offset == SUNXI_MMC_REG_CMD) {
        fake->regs[offset / 4u] = fake->hold_command_start
                                    ? value
                                    : value & ~SUNXI_MMC_CMD_START;
        return;
    }

    if (offset == SUNXI_MMC_REG_RINT) {
        fake->last_rint_write = value;
        fake->rint_write_count++;
        /* RINT is write-one-to-clear. */
        fake->regs[offset / 4u] = 0u;
        return;
    }

    fake->regs[offset / 4u] = value;
}

static uint64_t fake_time_ms(void *context)
{
    struct fake_mmc *fake = (struct fake_mmc *)context;
    return fake->now_ms++;
}

static void fake_delay_us(void *context, uint32_t delay)
{
    struct fake_mmc *fake = (struct fake_mmc *)context;
    (void)delay;
    fake->delay_count++;
}

static struct sunxi_mmc_io make_io(struct fake_mmc *fake)
{
    struct sunxi_mmc_io io;

    io.context = fake;
    io.read32 = fake_read32;
    io.write32 = fake_write32;
    io.time_ms = fake_time_ms;
    io.delay_us = fake_delay_us;
    return io;
}

static int check(int condition, const char *message)
{
    if (!condition) {
        fprintf(stderr, "FAIL: %s\n", message);
        return 0;
    }
    return 1;
}

static void reset_fake(struct fake_mmc *fake)
{
    memset(fake, 0, sizeof(*fake));
}

int main(void)
{
    struct fake_mmc fake;
    struct sunxi_mmc_io io;
    uint32_t value;
    int result;

    /* Clock-off update: mask DAT0, clear low-power, preserve divider. */
    reset_fake(&fake);
    io = make_io(&fake);
    fake.regs[SUNXI_MMC_REG_CLKCR / 4u] =
        0x23u | SUNXI_MMC_CLKCR_CARD_CLOCK |
        SUNXI_MMC_CLKCR_LOW_POWER | SUNXI_MMC_CLKCR_MASK_DATA0;
    fake.regs[SUNXI_MMC_REG_RINT / 4u] = 0x00000124u;

    result = sunxi_mmc_set_output_clock(&io, false, true, 5u);
    if (!check(result == SUNXI_MMC_CLOCK_OK, "clock-off update completes"))
        return EXIT_FAILURE;
    value = fake.regs[SUNXI_MMC_REG_CLKCR / 4u];
    if (!check((value & SUNXI_MMC_CLKCR_CARD_CLOCK) == 0u,
               "clock-off leaves card clock disabled") ||
        !check((value & SUNXI_MMC_CLKCR_LOW_POWER) == 0u,
               "clock-off clears low-power state") ||
        !check((value & SUNXI_MMC_CLKCR_MASK_DATA0) == 0u,
               "clock-off clears DATA0 mask after update") ||
        !check((value & SUNXI_MMC_CLKCR_DIVIDER_MASK) == 0x23u,
               "clock-off preserves divider") ||
        !check(fake.clkcr_write_count == 2u,
               "clock-off writes masked then unmasked CLKCR") ||
        !check((fake.clkcr_writes[0] & SUNXI_MMC_CLKCR_MASK_DATA0) != 0u,
               "clock-off asserts DATA0 mask during update") ||
        !check((fake.clkcr_writes[1] & SUNXI_MMC_CLKCR_MASK_DATA0) == 0u,
               "clock-off deasserts DATA0 mask after update") ||
        !check(fake.rint_write_count == 1u &&
               fake.last_rint_write == 0x00000124u,
               "clock-off clears raw interrupt status"))
        return EXIT_FAILURE;

    /* Clock-on update: the same mask protocol applies when re-enabling. */
    reset_fake(&fake);
    io = make_io(&fake);
    fake.regs[SUNXI_MMC_REG_CLKCR / 4u] =
        0x05u | SUNXI_MMC_CLKCR_LOW_POWER;
    result = sunxi_mmc_set_output_clock(&io, true, true, 5u);
    if (!check(result == SUNXI_MMC_CLOCK_OK, "clock-on update completes"))
        return EXIT_FAILURE;
    value = fake.regs[SUNXI_MMC_REG_CLKCR / 4u];
    if (!check((value & SUNXI_MMC_CLKCR_CARD_CLOCK) != 0u,
               "clock-on enables card clock") ||
        !check((value & SUNXI_MMC_CLKCR_LOW_POWER) == 0u,
               "clock-on clears low-power state") ||
        !check((value & SUNXI_MMC_CLKCR_MASK_DATA0) == 0u,
               "clock-on clears DATA0 mask after update") ||
        !check((value & SUNXI_MMC_CLKCR_DIVIDER_MASK) == 0x05u,
               "clock-on preserves divider"))
        return EXIT_FAILURE;

    /* Non-H616 mode must clear any stale mask and not reassert it. */
    reset_fake(&fake);
    io = make_io(&fake);
    fake.regs[SUNXI_MMC_REG_CLKCR / 4u] =
        SUNXI_MMC_CLKCR_MASK_DATA0 | SUNXI_MMC_CLKCR_LOW_POWER;
    result = sunxi_mmc_set_output_clock(&io, true, false, 5u);
    if (!check(result == SUNXI_MMC_CLOCK_OK, "unmasked clock update completes"))
        return EXIT_FAILURE;
    value = fake.regs[SUNXI_MMC_REG_CLKCR / 4u];
    if (!check((value & SUNXI_MMC_CLKCR_MASK_DATA0) == 0u,
               "unmasked mode does not leave DATA0 masked") ||
        !check(fake.clkcr_write_count == 1u,
               "unmasked mode requires no follow-up CLKCR write"))
        return EXIT_FAILURE;

    /* A stalled UPCLK_ONLY command must time out and unmask DAT0. */
    reset_fake(&fake);
    io = make_io(&fake);
    fake.hold_command_start = 1;
    fake.regs[SUNXI_MMC_REG_CLKCR / 4u] =
        0x11u | SUNXI_MMC_CLKCR_LOW_POWER;
    result = sunxi_mmc_set_output_clock(&io, true, true, 3u);
    if (!check(result == SUNXI_MMC_CLOCK_TIMEOUT,
               "stalled clock update returns timeout"))
        return EXIT_FAILURE;
    value = fake.regs[SUNXI_MMC_REG_CLKCR / 4u];
    if (!check((value & SUNXI_MMC_CLKCR_CARD_CLOCK) != 0u,
               "timeout preserves requested card-clock state") ||
        !check((value & SUNXI_MMC_CLKCR_MASK_DATA0) == 0u,
               "timeout cleanup clears DATA0 mask") ||
        !check((value & SUNXI_MMC_CLKCR_LOW_POWER) == 0u,
               "timeout path leaves low-power cleared") ||
        !check(fake.rint_write_count == 0u,
               "timeout does not clear status as if command succeeded"))
        return EXIT_FAILURE;

    if (!check(sunxi_mmc_set_output_clock(NULL, true, true, 1u) ==
               SUNXI_MMC_CLOCK_INVALID, "null I/O rejected") ||
        !check(sunxi_mmc_set_output_clock(&io, true, true, 0u) ==
               SUNXI_MMC_CLOCK_INVALID, "zero timeout rejected"))
        return EXIT_FAILURE;

    puts("PASS: H616 MMC clock-update mask, cleanup, and timeout paths");
    return EXIT_SUCCESS;
}
