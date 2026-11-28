/*
 * Read-only GPT-to-U-Boot SPL adapter for the M1 preflight path.
 * No U-Boot partition subsystem or write-capable block API is used.
 */
#include <blk.h>

#include "h700_gpt_readonly.h"
#include "m1_gpt_spl_adapter.h"

#define H700_M1_GPT_ENTRY_SLOTS 8u

static struct h700_gpt_partition m1_boot_partitions[H700_M1_GPT_ENTRY_SLOTS]
    __attribute__((aligned(8)));

static int m1_gpt_read_sector(void *context, uint64_t lba, void *buffer)
{
    struct blk_desc *card = (struct blk_desc *)context;
    return blk_dread(card, (lbaint_t)lba, 1, buffer) == 1 ? 0 : -1;
}

int h700_m1_spl_find_boot_partition(struct blk_desc *card,
                                    uint64_t device_sector_count,
                                    uint32_t sector_size,
                                    uint64_t *partition_first_lba,
                                    uint64_t *partition_block_count)
{
    struct h700_block_reader reader;
    struct h700_gpt_table table;
    const struct h700_gpt_partition *boot = 0;

    if (card == 0 || partition_first_lba == 0 || partition_block_count == 0 ||
        sector_size != H700_GPT_SECTOR_SIZE || device_sector_count < 8u)
        return 0;

    reader.context = card;
    reader.sector_count = device_sector_count;
    reader.sector_size = sector_size;
    reader.read_sector = m1_gpt_read_sector;

    if (h700_gpt_read_primary(&reader, m1_boot_partitions,
                              H700_M1_GPT_ENTRY_SLOTS, &table) != H700_GPT_OK)
        return 0;
    if (h700_gpt_find_unique_name(m1_boot_partitions, table.partition_count,
                                  "boot", &boot) != H700_GPT_OK || boot == 0)
        return 0;
    if (boot->first_lba > boot->last_lba ||
        boot->last_lba >= device_sector_count)
        return 0;

    *partition_first_lba = boot->first_lba;
    *partition_block_count = boot->last_lba - boot->first_lba + 1u;
    return *partition_block_count != 0;
}
