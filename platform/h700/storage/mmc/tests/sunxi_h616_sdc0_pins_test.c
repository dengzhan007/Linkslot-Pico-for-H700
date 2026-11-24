#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>

#include "../sunxi_h616_sdc0_pins.h"

#define REG_WORDS 64u
#define PORT_F_MUX0 0xb4u
#define PORT_F_DRV0 0xc8u
#define PORT_F_PULL0 0xd0u
#define MUX_MASK 0x00ffffffu
#define DRV_MASK 0x00000fffu
#define PULL_MASK 0x00000fffu
#define CHECK(condition, message) do { \
    if (!(condition)) { fprintf(stderr, "FAIL: %s (line %d)\n", message, __LINE__); return 0; } \
} while (0)

struct fake_pio {
    uint32_t regs[REG_WORDS];
    uint32_t write_offsets[4];
    uint32_t write_values[4];
    uint32_t write_count;
    int reject_write;
};

static uint32_t fake_read32(void *context, uint32_t offset)
{
    struct fake_pio *fake = (struct fake_pio *)context;
    if ((offset & 3u) != 0u || offset / 4u >= REG_WORDS)
        return 0u;
    return fake->regs[offset / 4u];
}

static void fake_write32(void *context, uint32_t offset, uint32_t value)
{
    struct fake_pio *fake = (struct fake_pio *)context;
    if ((offset & 3u) != 0u || offset / 4u >= REG_WORDS)
        return;
    if (fake->write_count < 4u) {
        fake->write_offsets[fake->write_count] = offset;
        fake->write_values[fake->write_count] = value;
    }
    ++fake->write_count;
    if (!fake->reject_write)
        fake->regs[offset / 4u] = value;
}

static int test_pinmux_drive_pull_and_preservation(void)
{
    struct fake_pio fake;
    struct sunxi_h616_pio_io pio;
    uint32_t mux_old = 0xa7654321u;
    uint32_t drv_old = 0x12345678u;
    uint32_t pull_old = 0x87654321u;

    memset(&fake, 0, sizeof(fake));
    fake.regs[PORT_F_MUX0 / 4u] = mux_old;
    fake.regs[PORT_F_DRV0 / 4u] = drv_old;
    fake.regs[PORT_F_PULL0 / 4u] = pull_old;

    pio.context = &fake;
    pio.read32 = fake_read32;
    pio.write32 = fake_write32;

    CHECK(sunxi_h616_sdc0_configure_pins(&pio) == 0,
          "H616 sdc0 pin configuration succeeds");

    CHECK((fake.regs[PORT_F_MUX0 / 4u] & MUX_MASK) == 0x00222222u,
          "PF0-PF5 select ALT2/MMC0");
    CHECK((fake.regs[PORT_F_DRV0 / 4u] & DRV_MASK) == 0x00000fffu,
          "PF0-PF5 use 40 mA drive setting");
    CHECK((fake.regs[PORT_F_PULL0 / 4u] & PULL_MASK) == 0x00000555u,
          "PF0-PF5 use pull-up setting");

    CHECK((fake.regs[PORT_F_MUX0 / 4u] & ~MUX_MASK) ==
              (mux_old & ~MUX_MASK),
          "PF6/PF7 mux fields are preserved");
    CHECK((fake.regs[PORT_F_DRV0 / 4u] & ~DRV_MASK) ==
              (drv_old & ~DRV_MASK),
          "unrelated drive fields are preserved");
    CHECK((fake.regs[PORT_F_PULL0 / 4u] & ~PULL_MASK) ==
              (pull_old & ~PULL_MASK),
          "unrelated pull fields are preserved");

    CHECK(fake.write_count == 3u,
          "only the three PF0-PF5 register words are written");
    CHECK(fake.write_offsets[0] == PORT_F_DRV0 &&
          fake.write_offsets[1] == PORT_F_PULL0 &&
          fake.write_offsets[2] == PORT_F_MUX0,
          "electrical setup precedes mux activation");
    return 1;
}

static int test_null_io_and_write_failure(void)
{
    struct fake_pio fake;
    struct sunxi_h616_pio_io pio;

    CHECK(sunxi_h616_sdc0_configure_pins(NULL) != 0,
          "null PIO descriptor rejected");

    memset(&fake, 0, sizeof(fake));
    pio.context = &fake;
    pio.read32 = fake_read32;
    pio.write32 = fake_write32;
    fake.reject_write = 1;

    CHECK(sunxi_h616_sdc0_configure_pins(&pio) != 0,
          "register readback mismatch is reported");
    return 1;
}

int main(void)
{
    if (!test_pinmux_drive_pull_and_preservation() ||
        !test_null_io_and_write_failure())
        return EXIT_FAILURE;

    puts("PASS: H616 sdc0 PF0-PF5 mux/drive/pull register tests");
    return EXIT_SUCCESS;
}
