#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>

#include "../h700_sunxi_sdc0_readonly_scan.h"
#include "../sunxi_mmc_regs.h"

#define DISK_SECTORS 1024u
#define FIFO_WORDS 128u
#define PIO_WORDS 64u
#define CCU_WORDS 0x860u
#define CHECK(c, m) do { \
    if (!(c)) { fprintf(stderr, "FAIL: %s (line %d)\n", m, __LINE__); return 0; } \
} while (0)

struct fake_h700 {
    uint32_t mmc[129];
    uint32_t pio[PIO_WORDS];
    uint32_t ccu[CCU_WORDS / 4u];
    uint8_t media[DISK_SECTORS * 512u];
    uint32_t fifo_index;
    uint64_t selected_lba;
    uint64_t now_ms;
    uint32_t card_command_count;
    uint32_t read_command_count;
    uint32_t clock_update_count;
    uint32_t write_command_count;
    uint32_t acmd41_count;
    uint32_t csd[4];
    int fifo_active;
};

static void set_csd_bits(uint32_t response[4],
                         uint32_t start,
                         uint32_t count,
                         uint32_t value)
{
    uint32_t i;
    for (i = 0; i < count; ++i) {
        uint32_t bit = start + i;
        uint32_t word = 3u - bit / 32u;
        uint32_t shift = bit % 32u;
        uint32_t mask = 1u << shift;
        if ((value & (1u << i)) != 0u)
            response[word] |= mask;
        else
            response[word] &= ~mask;
    }
}

static void make_sdhc_csd(uint32_t response[4], uint32_t c_size)
{
    memset(response, 0, 4u * sizeof(response[0]));
    set_csd_bits(response, 126u, 2u, 1u);
    set_csd_bits(response, 48u, 22u, c_size);
}

static void fake_set_r2(struct fake_h700 *f, const uint32_t response[4])
{
    f->mmc[SUNXI_MMC_REG_RESP3 / 4u] = response[0];
    f->mmc[SUNXI_MMC_REG_RESP2 / 4u] = response[1];
    f->mmc[SUNXI_MMC_REG_RESP1 / 4u] = response[2];
    f->mmc[SUNXI_MMC_REG_RESP0 / 4u] = response[3];
}

static uint32_t mmc_read32(void *context, uint32_t offset)
{
    struct fake_h700 *f = context;
    if ((offset & 3u) != 0u || offset >= sizeof(f->mmc))
        return 0u;

    if (offset == SUNXI_MMC_REG_STATUS) {
        uint32_t left;
        if (!f->fifo_active || f->selected_lba >= DISK_SECTORS ||
            f->fifo_index >= FIFO_WORDS)
            return 0u;
        left = FIFO_WORDS - f->fifo_index;
        if (left > 32u)
            left = 32u;
        return left << 17;
    }

    if (offset == SUNXI_MMC_REG_FIFO) {
        const uint8_t *p;
        uint32_t i;
        if (!f->fifo_active || f->selected_lba >= DISK_SECTORS ||
            f->fifo_index >= FIFO_WORDS)
            return 0u;
        p = f->media + (size_t)f->selected_lba * 512u;
        i = f->fifo_index++ * 4u;
        if (f->fifo_index >= FIFO_WORDS)
            f->fifo_active = 0;
        return (uint32_t)p[i] |
               ((uint32_t)p[i + 1u] << 8) |
               ((uint32_t)p[i + 2u] << 16) |
               ((uint32_t)p[i + 3u] << 24);
    }

    return f->mmc[offset / 4u];
}

static void mmc_write32(void *context, uint32_t offset, uint32_t value)
{
    struct fake_h700 *f = context;
    uint32_t index;

    if ((offset & 3u) != 0u || offset >= sizeof(f->mmc))
        return;
    index = offset / 4u;

    if (offset == SUNXI_MMC_REG_RINT) {
        f->mmc[index] &= ~value;
        return;
    }

    if (offset == SUNXI_MMC_REG_GCTRL) {
        /* Model self-clearing controller/FIFO/DMA reset bits. */
        f->mmc[index] = value & ~SUNXI_MMC_GCTRL_RESET_MASK;
        return;
    }

    if (offset == SUNXI_MMC_REG_CMD) {
        uint32_t command_index = value & 0x3fu;
        uint32_t response = 0u;

        f->mmc[index] = value & ~SUNXI_MMC_CMD_START;

        if ((value & SUNXI_MMC_CMD_UPCLK_ONLY) != 0u) {
            ++f->clock_update_count;
            f->mmc[SUNXI_MMC_REG_RINT / 4u] |=
                SUNXI_MMC_RINT_VOLTAGE_CHANGE_DONE;
            return;
        }

        ++f->card_command_count;
        if ((value & SUNXI_MMC_CMD_WRITE) != 0u)
            ++f->write_command_count;

        switch (command_index) {
        case 0u: /* CMD0 */
            break;
        case 8u: /* CMD8 / R7 */
            response = 0x000001aau;
            f->mmc[SUNXI_MMC_REG_RESP0 / 4u] = response;
            break;
        case 55u: /* CMD55 / R1 */
            response = 1u << 5;
            f->mmc[SUNXI_MMC_REG_RESP0 / 4u] = response;
            break;
        case 41u: /* ACMD41 / R3 */
            ++f->acmd41_count;
            response = (1u << 31) | (1u << 30) | 0x00ff8000u;
            f->mmc[SUNXI_MMC_REG_RESP0 / 4u] = response;
            break;
        case 2u: /* CMD2 / R2 */
            fake_set_r2(f, (uint32_t[4]){
                0x11223344u, 0x55667788u, 0x99aabbccu, 0xddeeff00u
            });
            break;
        case 3u: /* CMD3 / R6 */
            f->mmc[SUNXI_MMC_REG_RESP0 / 4u] = 0x12340000u;
            break;
        case 9u: /* CMD9 / R2 */
            fake_set_r2(f, f->csd);
            break;
        case 7u: /* CMD7 / R1b */
            f->mmc[SUNXI_MMC_REG_RESP0 / 4u] = 0x00000900u;
            break;
        case 17u: /* CMD17 / R1 + 512-byte PIO read */
            ++f->read_command_count;
            f->selected_lba = f->mmc[SUNXI_MMC_REG_ARG / 4u];
            f->fifo_index = 0u;
            f->fifo_active = 1;
            f->mmc[SUNXI_MMC_REG_RESP0 / 4u] = 0x00000900u;
            break;
        default:
            f->mmc[SUNXI_MMC_REG_RESP0 / 4u] = 0u;
            break;
        }

        f->mmc[SUNXI_MMC_REG_RINT / 4u] |=
            SUNXI_MMC_RINT_COMMAND_DONE;
        if (command_index == 17u)
            f->mmc[SUNXI_MMC_REG_RINT / 4u] |= SUNXI_MMC_RINT_DATA_OVER;
        return;
    }

    f->mmc[index] = value;
}

static uint64_t fake_time_ms(void *context)
{
    return ((struct fake_h700 *)context)->now_ms++;
}

static void fake_delay_us(void *context, uint32_t delay)
{
    struct fake_h700 *f = context;
    f->now_ms += delay / 1000u;
}

static void mmc_read_barrier(void *context)
{
    (void)context;
}

static uint32_t pio_read32(void *context, uint32_t offset)
{
    struct fake_h700 *f = context;
    if ((offset & 3u) != 0u || offset / 4u >= PIO_WORDS)
        return 0u;
    return f->pio[offset / 4u];
}

static void pio_write32(void *context, uint32_t offset, uint32_t value)
{
    struct fake_h700 *f = context;
    if ((offset & 3u) != 0u || offset / 4u >= PIO_WORDS)
        return;
    f->pio[offset / 4u] = value;
}

static uint32_t ccu_read32(void *context, uint32_t offset)
{
    struct fake_h700 *f = context;
    if ((offset & 3u) != 0u || offset / 4u >= CCU_WORDS / 4u)
        return 0u;
    return f->ccu[offset / 4u];
}

static void ccu_write32(void *context, uint32_t offset, uint32_t value)
{
    struct fake_h700 *f = context;
    if ((offset & 3u) != 0u || offset / 4u >= CCU_WORDS / 4u)
        return;
    f->ccu[offset / 4u] = value;
}

static uint32_t crc32_bytes(const uint8_t *data, size_t length)
{
    uint32_t crc = 0xffffffffu;
    size_t i;
    unsigned int bit;
    for (i = 0; i < length; ++i) {
        crc ^= data[i];
        for (bit = 0; bit < 8u; ++bit)
            crc = (crc >> 1) ^
                (0xedb88320u & (uint32_t)-(int32_t)(crc & 1u));
    }
    return ~crc;
}

static void put_le16(uint8_t *p, uint16_t value)
{
    p[0] = (uint8_t)value;
    p[1] = (uint8_t)(value >> 8);
}

static void put_le32(uint8_t *p, uint32_t value)
{
    p[0] = (uint8_t)value;
    p[1] = (uint8_t)(value >> 8);
    p[2] = (uint8_t)(value >> 16);
    p[3] = (uint8_t)(value >> 24);
}

static void put_le64(uint8_t *p, uint64_t value)
{
    put_le32(p, (uint32_t)value);
    put_le32(p + 4u, (uint32_t)(value >> 32));
}

static void set_entry_name(uint8_t *entry, const char *name)
{
    size_t i;
    for (i = 0; name[i] != '\0' && i < 36u; ++i)
        put_le16(entry + 56u + i * 2u, (uint8_t)name[i]);
}

static void make_entry(struct fake_h700 *f,
                       unsigned int slot,
                       uint64_t first_lba,
                       uint64_t last_lba,
                       const char *name)
{
    uint8_t *entry = f->media + 2u * 512u + slot * 128u;
    unsigned int i;

    memset(entry, 0, 128u);
    entry[0] = 0x28u;
    for (i = 0; i < 16u; ++i)
        entry[16u + i] = (uint8_t)(slot + i + 1u);
    put_le64(entry + 32u, first_lba);
    put_le64(entry + 40u, last_lba);
    set_entry_name(entry, name);
}

static void make_synthetic_gpt(struct fake_h700 *f)
{
    uint8_t *mbr = f->media;
    uint8_t *header = f->media + 512u;
    uint8_t *entries = f->media + 2u * 512u;

    memset(mbr, 0, 512u);
    memset(header, 0, 512u);
    memset(entries, 0, 1024u);

    mbr[450u] = 0xeeu;
    put_le32(mbr + 454u, 1u);
    put_le32(mbr + 458u, DISK_SECTORS - 1u);
    mbr[510u] = 0x55u;
    mbr[511u] = 0xaau;

    make_entry(f, 5u, 4u, 10u, "UDISK");
    make_entry(f, 6u, 11u, 1022u, "primary");

    memcpy(header, "EFI PART", 8u);
    put_le32(header + 8u, 0x00010000u);
    put_le32(header + 12u, 92u);
    put_le64(header + 24u, 1u);
    put_le64(header + 32u, DISK_SECTORS - 1u);
    put_le64(header + 40u, 4u);
    put_le64(header + 48u, DISK_SECTORS - 2u);
    header[56u] = 0x11u;
    put_le64(header + 72u, 2u);
    put_le32(header + 80u, 8u);
    put_le32(header + 84u, 128u);
    put_le32(header + 88u, crc32_bytes(entries, 1024u));
    put_le32(header + 16u, 0u);
    put_le32(header + 16u, crc32_bytes(header, 92u));
}

static void fake_init(struct fake_h700 *f)
{
    memset(f, 0, sizeof(*f));
    make_sdhc_csd(f->csd, 0u); /* 1024-sector SDHC media. */
    make_synthetic_gpt(f);
}

static int test_complete_readonly_sdc0_gpt_scan(void)
{
    struct fake_h700 fake;
    struct sunxi_mmc_io mmc_io;
    struct sunxi_h616_pio_io pio_io;
    struct sunxi_h616_sdc0_platform platform;
    struct h700_sunxi_sdc0_scan_config config;
    struct h700_gpt_partition partitions[H700_GPT_MAX_ENTRIES];
    struct h700_sunxi_sdc0_scan_result result;
    enum h700_sunxi_sdc0_scan_status status;

    fake_init(&fake);

    mmc_io.context = &fake;
    mmc_io.read32 = mmc_read32;
    mmc_io.write32 = mmc_write32;
    mmc_io.time_ms = fake_time_ms;
    mmc_io.delay_us = fake_delay_us;
    mmc_io.read_barrier = mmc_read_barrier;

    pio_io.context = &fake;
    pio_io.read32 = pio_read32;
    pio_io.write32 = pio_write32;

    platform.mmc_io = &mmc_io;
    platform.ccu.context = &fake;
    platform.ccu.read32 = ccu_read32;
    platform.ccu.write32 = ccu_write32;
    platform.osc24m_hz = 24000000u;
    platform.pll6_effective_hz = 600000000u;

    config.pio = &pio_io;
    config.platform = &platform;
    config.controller_reset_timeout_ms = 20u;
    config.command_timeout_ms = 20u;
    config.ocr_timeout_ms = 50u;
    config.card_busy_timeout_ms = 20u;
    config.clock_update_timeout_ms = 20u;
    config.fifo_timeout_ms = 20u;
    config.data_timeout_ms = 20u;
    config.recovery_timeout_ms = 20u;

    status = h700_sunxi_sdc0_scan_primary_gpt(
        &config, partitions, H700_GPT_MAX_ENTRIES, &result);

    CHECK(status == H700_SDC0_SCAN_OK,
          "complete read-only sdc0 bring-up and GPT scan succeeds");
    CHECK(result.card.capacity.block_addressed &&
          result.card.capacity.sector_count == DISK_SECTORS,
          "SD CSD capacity feeds block-reader capacity");
    CHECK(result.gpt.partition_count == 2u,
          "primary GPT returns two populated partitions");
    CHECK(partitions[0].number == 6u &&
          strcmp(partitions[0].name, "UDISK") == 0,
          "p6 UDISK is resolved from GPT");
    CHECK(partitions[1].number == 7u &&
          strcmp(partitions[1].name, "primary") == 0,
          "p7 primary is resolved from GPT");

    CHECK((fake.ccu[0x520u / 4u] & 0x03000303u) == 0u,
          "APB1 is configured to OSC24M before PIO use");
    CHECK((fake.ccu[SUNXI_CCU_MMC_GATE_RESET / 4u] &
           (SUNXI_CCU_MMC_GATE_BIT(0) | SUNXI_CCU_MMC_RESET_BIT(0))) ==
          (SUNXI_CCU_MMC_GATE_BIT(0) | SUNXI_CCU_MMC_RESET_BIT(0)),
          "MMC0 bus gate and reset deassert are enabled");
    CHECK(fake.ccu[SUNXI_CCU_MMC0_CLK_CFG / 4u] == 0x8000020eu,
          "sdc0 module clock is set to the 400 kHz identification rate");
    CHECK((fake.pio[0xb4u / 4u] & 0x00ffffffu) == 0x00222222u,
          "PF0-PF5 are configured for MMC0 ALT2");
    CHECK((fake.pio[0xc8u / 4u] & 0x00000fffu) == 0x00000fffu,
          "PF0-PF5 drive fields use decoded 40 mA board setting");
    CHECK((fake.pio[0xd0u / 4u] & 0x00000fffu) == 0x00000555u,
          "PF0-PF5 pull fields use pull-up reference setting");

    CHECK(fake.acmd41_count == 1u,
          "SDHC OCR negotiation completes");
    CHECK(fake.read_command_count == 6u,
          "GPT MBR/header/entry array are read with six CMD17 operations");
    CHECK(fake.write_command_count == 0u,
          "no SD write command is issued");
    CHECK((fake.mmc[SUNXI_MMC_REG_CLKCR / 4u] &
           SUNXI_MMC_CLKCR_MASK_DATA0) == 0u,
          "MASK_DATA0 is cleared after clock updates");
    return 1;
}

int main(void)
{
    if (!test_complete_readonly_sdc0_gpt_scan())
        return EXIT_FAILURE;

    puts("PASS: H700 sdc0 bring-up -> SD identification -> CMD17 -> GPT end-to-end test");
    return EXIT_SUCCESS;
}
