#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "../h700_sunxi_mmc_block_device.h"
#include "../sunxi_mmc_regs.h"
#include "../../gpt/h700_gpt_readonly.h"

#define SECTORS 40u
#define ENTRY_COUNT 8u
#define ENTRY_SIZE 128u
#define FIFO_WORDS 128u
#define CHECK(c, m) do { if (!(c)) { fprintf(stderr, "FAIL: %s (line %d)\n", m, __LINE__); return 0; } } while (0)

struct fake_sd {
    uint32_t regs[129];
    uint8_t media[SECTORS * 512u];
    uint32_t fifo_index;
    uint64_t selected_lba;
    uint32_t command_count;
    uint32_t now;
    int write_seen;
};

static uint32_t read32(void *context, uint32_t offset)
{
    struct fake_sd *f = context;
    if ((offset & 3u) || offset >= sizeof(f->regs)) return 0;
    if (offset == SUNXI_MMC_REG_STATUS) {
        uint32_t left;
        if (f->selected_lba >= SECTORS || f->fifo_index >= FIFO_WORDS)
            return SUNXI_MMC_STATUS_FIFO_EMPTY;
        left = FIFO_WORDS - f->fifo_index;
        return (left > 32u ? 32u : left) << 17;
    }
    if (offset == SUNXI_MMC_REG_FIFO) {
        const uint8_t *p;
        uint32_t i;
        if (f->selected_lba >= SECTORS || f->fifo_index >= FIFO_WORDS) return 0;
        p = f->media[f->selected_lba];
        i = f->fifo_index++ * 4u;
        return (uint32_t)p[i] | ((uint32_t)p[i+1] << 8) |
               ((uint32_t)p[i+2] << 16) | ((uint32_t)p[i+3] << 24);
    }
    return f->regs[offset / 4u];
}

static void write32(void *context, uint32_t offset, uint32_t value)
{
    struct fake_sd *f = context;
    uint32_t n;
    if ((offset & 3u) || offset >= sizeof(f->regs)) return;
    n = offset / 4u;
    if (offset == SUNXI_MMC_REG_RINT) {
        f->regs[n] &= ~value;
        return;
    }
    if (offset == SUNXI_MMC_REG_CMD) {
        ++f->command_count;
        if (value & SUNXI_MMC_CMD_WRITE) f->write_seen = 1;
        f->selected_lba = f->regs[SUNXI_MMC_REG_ARG / 4u];
        f->fifo_index = 0u;
        f->regs[n] = value & ~SUNXI_MMC_CMD_START;
        f->regs[SUNXI_MMC_REG_RINT / 4u] |=
            SUNXI_MMC_RINT_COMMAND_DONE | SUNXI_MMC_RINT_DATA_OVER;
        f->regs[SUNXI_MMC_REG_RESP0 / 4u] = 0x900u;
        return;
    }
    f->regs[n] = value;
}

static uint64_t time_ms(void *context) { return ((struct fake_sd *)context)->now++; }
static void delay_us(void *context, uint32_t us) { ((struct fake_sd *)context)->now += us / 1000u; }
static void barrier(void *context) { (void)context; }

static void put16(uint8_t *p, uint16_t v) { p[0]=(uint8_t)v; p[1]=(uint8_t)(v>>8); }
static void put32(uint8_t *p, uint32_t v)
{
    p[0]=(uint8_t)v; p[1]=(uint8_t)(v>>8); p[2]=(uint8_t)(v>>16); p[3]=(uint8_t)(v>>24);
}
static void put64(uint8_t *p, uint64_t v) { put32(p,(uint32_t)v); put32(p+4,(uint32_t)(v>>32)); }

static uint32_t crc32(const uint8_t *p, size_t n)
{
    uint32_t c=0xffffffffu;
    size_t i; unsigned b;
    for(i=0;i<n;i++){ c^=p[i]; for(b=0;b<8;b++) c=(c>>1)^(0xedb88320u & (uint32_t)-(int32_t)(c&1u)); }
    return ~c;
}

static void entry_name(uint8_t *e, const char *s)
{
    size_t i;
    for(i=0;s[i] && i<36;i++) put16(e+56+i*2,(uint8_t)s[i]);
}

static void make_entry(struct fake_sd *f, unsigned slot,
                       uint64_t first, uint64_t last, const char *name)
{
    uint8_t *e=f->media + 2u*512u + slot*ENTRY_SIZE;
    unsigned i;
    memset(e,0,ENTRY_SIZE);
    e[0]=0x28;
    for(i=0;i<16;i++) e[16+i]=(uint8_t)(slot+i+1);
    put64(e+32,first); put64(e+40,last); entry_name(e,name);
}

static void make_gpt(struct fake_sd *f)
{
    uint8_t *m=f->media;
    uint8_t *h=f->media + 512u;
    uint8_t *a=f->media + 2u*512u;
    memset(m,0,512); memset(h,0,512); memset(a,0,1024);
    m[450]=0xee; put32(m+454,1); put32(m+458,SECTORS-1); m[510]=0x55; m[511]=0xaa;
    make_entry(f,5,4,10,"UDISK");
    make_entry(f,6,11,38,"primary");
    memcpy(h,"EFI PART",8); put32(h+8,0x00010000); put32(h+12,92);
    put64(h+24,1); put64(h+32,SECTORS-1); put64(h+40,4); put64(h+48,38);
    h[56]=0x11; put64(h+72,2); put32(h+80,ENTRY_COUNT); put32(h+84,ENTRY_SIZE);
    put32(h+88,crc32(a,ENTRY_COUNT*ENTRY_SIZE));
    put32(h+16,0); put32(h+16,crc32(h,92));
}

int main(void)
{
    struct fake_sd fake;
    struct sunxi_mmc_io io;
    struct sunxi_sd_card_info card;
    struct h700_sunxi_mmc_block_device device;
    struct h700_block_reader reader;
    struct h700_sunxi_mmc_block_timeouts to;
    struct h700_gpt_partition parts[ENTRY_COUNT];
    struct h700_gpt_table table;

    memset(&fake,0,sizeof(fake));
    make_gpt(&fake);
    io.context=&fake; io.read32=read32; io.write32=write32;
    io.time_ms=time_ms; io.delay_us=delay_us; io.read_barrier=barrier;
    memset(&card,0,sizeof(card));
    card.capacity.sector_count=SECTORS;
    card.capacity.block_addressed=true;
    to.command_timeout_ms=20; to.fifo_timeout_ms=20; to.data_timeout_ms=20; to.recovery_timeout_ms=20;

    if (h700_sunxi_mmc_block_device_init(&device,&reader,&io,&card,&to)!=0) {
        puts("FAIL: block reader init"); return EXIT_FAILURE;
    }
    if (h700_gpt_read_primary(&reader,parts,ENTRY_COUNT,&table)!=H700_GPT_OK) {
        puts("FAIL: GPT parser through CMD17"); return EXIT_FAILURE;
    }
    CHECK(table.partition_count==2,"two partitions discovered");
    CHECK(parts[0].number==6 && strcmp(parts[0].name,"UDISK")==0,"p6 UDISK resolved");
    CHECK(parts[1].number==7 && strcmp(parts[1].name,"primary")==0,"p7 primary resolved");
    CHECK(fake.command_count==6,"MBR/header/entry array read via six CMD17 operations");
    CHECK(!fake.write_seen,"no write command used");
    puts("PASS: SD CMD17 -> GPT parser end-to-end synthetic disk test");
    return EXIT_SUCCESS;
}
