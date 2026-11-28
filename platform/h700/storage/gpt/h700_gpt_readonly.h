/*
 * Read-only GPT parser for H700 storage discovery.
 *
 * This API intentionally exposes no block-write operation. It reads one
 * 512-byte logical sector per callback and validates both primary GPT CRCs
 * before returning any partition metadata.
 */
#ifndef H700_GPT_READONLY_H
#define H700_GPT_READONLY_H

#include <stddef.h>
#if defined(CONFIG_XPL_BUILD) || defined(CONFIG_SPL_BUILD)
#include <linux/types.h>
#else
#include <stdint.h>
#endif

#define H700_GPT_SECTOR_SIZE 512u
#define H700_GPT_MAX_ENTRIES 128u
#define H700_GPT_NAME_CHARS 36u
#define H700_GPT_NAME_CAPACITY (H700_GPT_NAME_CHARS + 1u)

struct h700_block_reader {
    void *context;
    uint64_t sector_count;
    uint32_t sector_size;
    /* Return 0 only when exactly one sector was read successfully. */
    int (*read_sector)(void *context, uint64_t lba, void *buffer);
};

struct h700_gpt_partition {
    uint8_t type_guid[16];
    uint8_t unique_guid[16];
    uint64_t first_lba;
    uint64_t last_lba;
    uint64_t attributes;
    uint32_t number; /* 1-based entry slot in the GPT array. */
    /* ASCII names are preserved; non-ASCII/control UTF-16 code units map to '?'. */
    char name[H700_GPT_NAME_CAPACITY];
};

struct h700_gpt_table {
    uint64_t first_usable_lba;
    uint64_t last_usable_lba;
    uint64_t backup_header_lba;
    uint32_t entry_count;      /* Number of GPT entry slots, including unused. */
    uint32_t entry_size;
    size_t partition_count;    /* Number of populated entries returned. */
};

enum h700_gpt_result {
    H700_GPT_OK = 0,
    H700_GPT_ERR_ARGUMENT,
    H700_GPT_ERR_IO,
    H700_GPT_ERR_PROTECTIVE_MBR,
    H700_GPT_ERR_SIGNATURE,
    H700_GPT_ERR_UNSUPPORTED_REVISION,
    H700_GPT_ERR_HEADER,
    H700_GPT_ERR_HEADER_CRC,
    H700_GPT_ERR_ENTRY_LAYOUT,
    H700_GPT_ERR_ENTRY_CRC,
    H700_GPT_ERR_CAPACITY,
    H700_GPT_ERR_PARTITION_BOUNDS,
    H700_GPT_ERR_DUPLICATE_GUID,
    H700_GPT_ERR_OVERLAP,
    H700_GPT_ERR_NOT_FOUND,
    H700_GPT_ERR_AMBIGUOUS_NAME
};

/*
 * Parse and validate the primary GPT. The partitions array must have room for
 * at least the header's entry_count records (at most H700_GPT_MAX_ENTRIES);
 * table->partition_count is zero on failure. No secondary GPT is read and
 * no block writes are possible.
 */
enum h700_gpt_result h700_gpt_read_primary(
    const struct h700_block_reader *reader,
    struct h700_gpt_partition *partitions,
    size_t partition_capacity,
    struct h700_gpt_table *table);

/* Exact, case-sensitive ASCII name lookup. Duplicate names fail closed. */
enum h700_gpt_result h700_gpt_find_unique_name(
    const struct h700_gpt_partition *partitions,
    size_t partition_count,
    const char *name,
    const struct h700_gpt_partition **match);

const char *h700_gpt_result_string(enum h700_gpt_result result);

#endif /* H700_GPT_READONLY_H */
