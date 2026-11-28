#ifndef H700_M1_GPT_SPL_ADAPTER_H
#define H700_M1_GPT_SPL_ADAPTER_H

#include <stdint.h>

struct blk_desc;

/*
 * Resolve the uniquely named "boot" GPT partition using the project's
 * read-only primary-GPT parser and U-Boot blk_dread(). The RG34XX SPL profile
 * accepts at most eight GPT entry slots; larger tables fail closed.
 */
int h700_m1_spl_find_boot_partition(struct blk_desc *card,
                                    uint64_t device_sector_count,
                                    uint32_t sector_size,
                                    uint64_t *partition_first_lba,
                                    uint64_t *partition_block_count);

/* Resolve p4 and preflight its p4+40MiB firmware stream using the adapter's
 * named 4KiB SRAM/BSS scratch reservation. */
int h700_m1_spl_preflight_gpt(struct blk_desc *card,
                              uint64_t device_sector_count,
                              uint32_t sector_size,
                              uint32_t expected_firmware_fnv1a);

#endif
