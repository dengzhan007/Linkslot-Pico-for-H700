#include "h700_gpt_readonly.h"

#include <string.h>

#define GPT_HEADER_LBA 1u
#define GPT_HEADER_MIN_SIZE 92u
#define GPT_HEADER_REVISION_1_0 0x00010000u
#define GPT_MAX_TABLE_BYTES (H700_GPT_MAX_ENTRIES * H700_GPT_SECTOR_SIZE)
#define GPT_MBR_PARTITION_OFFSET 446u
#define GPT_MBR_PARTITION_SIZE 16u
#define GPT_MBR_PROTECTIVE_TYPE 0xEEu

static uint16_t read_le16(const uint8_t *p)
{
    return (uint16_t)((uint16_t)p[0] | ((uint16_t)p[1] << 8));
}

static uint32_t read_le32(const uint8_t *p)
{
    return (uint32_t)p[0] |
           ((uint32_t)p[1] << 8) |
           ((uint32_t)p[2] << 16) |
           ((uint32_t)p[3] << 24);
}

static uint64_t read_le64(const uint8_t *p)
{
    return (uint64_t)read_le32(p) | ((uint64_t)read_le32(p + 4) << 32);
}

static uint32_t crc32_update(uint32_t crc, const uint8_t *data, size_t length)
{
    size_t i;
    unsigned int bit;

    for (i = 0; i < length; ++i) {
        crc ^= data[i];
        for (bit = 0; bit < 8u; ++bit)
            crc = (crc >> 1) ^ (0xEDB88320u & (uint32_t)-(int32_t)(crc & 1u));
    }
    return crc;
}

static uint32_t crc32_finish(uint32_t crc)
{
    return ~crc;
}

static int bytes_are_zero(const uint8_t *p, size_t length)
{
    size_t i;
    for (i = 0; i < length; ++i) {
        if (p[i] != 0u)
            return 0;
    }
    return 1;
}

static int read_one(const struct h700_block_reader *reader,
                    uint64_t lba, uint8_t sector[H700_GPT_SECTOR_SIZE])
{
    if (lba >= reader->sector_count)
        return -1;
    return reader->read_sector(reader->context, lba, sector);
}

static enum h700_gpt_result validate_protective_mbr(
    const struct h700_block_reader *reader,
    uint8_t sector[H700_GPT_SECTOR_SIZE])
{
    size_t i;
    int protective_found = 0;
    uint32_t expected_size;

    if (read_one(reader, 0u, sector) != 0)
        return H700_GPT_ERR_IO;
    if (sector[510] != 0x55u || sector[511] != 0xAAu)
        return H700_GPT_ERR_PROTECTIVE_MBR;

    expected_size = reader->sector_count - 1u > 0xFFFFFFFFu
                        ? 0xFFFFFFFFu
                        : (uint32_t)(reader->sector_count - 1u);

    for (i = 0; i < 4u; ++i) {
        const uint8_t *entry = sector + GPT_MBR_PARTITION_OFFSET + i * GPT_MBR_PARTITION_SIZE;
        if (entry[4] == GPT_MBR_PROTECTIVE_TYPE &&
            read_le32(entry + 8) == 1u &&
            read_le32(entry + 12) == expected_size) {
            protective_found = 1;
            break;
        }
    }

    return protective_found ? H700_GPT_OK : H700_GPT_ERR_PROTECTIVE_MBR;
}

static void decode_partition_name(const uint8_t *utf16le, char out[H700_GPT_NAME_CAPACITY])
{
    size_t i;

    for (i = 0; i < H700_GPT_NAME_CHARS; ++i) {
        uint16_t code = read_le16(utf16le + i * 2u);
        if (code == 0u)
            break;
        if (code >= 0x20u && code <= 0x7Eu)
            out[i] = (char)code;
        else
            out[i] = '?';
    }
    out[i] = '\0';
}

static enum h700_gpt_result parse_partition_entry(
    const uint8_t *entry,
    uint32_t entry_size,
    uint32_t slot,
    const struct h700_gpt_table *table,
    struct h700_gpt_partition *out,
    int *is_populated)
{
    uint64_t first_lba;
    uint64_t last_lba;

    *is_populated = 0;
    if (bytes_are_zero(entry, 16u))
        return H700_GPT_OK;

    if (entry_size < 128u)
        return H700_GPT_ERR_ENTRY_LAYOUT;

    first_lba = read_le64(entry + 32);
    last_lba = read_le64(entry + 40);
    if (bytes_are_zero(entry + 16, 16u) ||
        first_lba > last_lba ||
        first_lba < table->first_usable_lba ||
        last_lba > table->last_usable_lba)
        return H700_GPT_ERR_PARTITION_BOUNDS;

    memcpy(out->type_guid, entry, 16u);
    memcpy(out->unique_guid, entry + 16, 16u);
    out->first_lba = first_lba;
    out->last_lba = last_lba;
    out->attributes = read_le64(entry + 48);
    out->number = slot + 1u;
    decode_partition_name(entry + 56, out->name);
    *is_populated = 1;
    return H700_GPT_OK;
}

static enum h700_gpt_result validate_partition_set(
    const struct h700_gpt_partition *partitions,
    size_t count)
{
    size_t i;
    size_t j;

    for (i = 0; i < count; ++i) {
        for (j = i + 1u; j < count; ++j) {
            if (memcmp(partitions[i].unique_guid, partitions[j].unique_guid, 16u) == 0)
                return H700_GPT_ERR_DUPLICATE_GUID;
            if (partitions[i].first_lba <= partitions[j].last_lba &&
                partitions[j].first_lba <= partitions[i].last_lba)
                return H700_GPT_ERR_OVERLAP;
        }
    }
    return H700_GPT_OK;
}

enum h700_gpt_result h700_gpt_read_primary(
    const struct h700_block_reader *reader,
    struct h700_gpt_partition *partitions,
    size_t partition_capacity,
    struct h700_gpt_table *table)
{
    uint8_t sector[H700_GPT_SECTOR_SIZE];
    uint8_t header[H700_GPT_SECTOR_SIZE];
    uint64_t entries_lba;
    uint64_t table_bytes;
    uint64_t table_sectors;
    uint64_t table_end_lba;
    uint64_t offset;
    uint32_t header_size;
    uint32_t expected_header_crc;
    uint32_t expected_entries_crc;
    uint32_t entries_crc = 0xFFFFFFFFu;
    uint32_t entry_count;
    uint32_t entry_size;
    uint64_t first_usable;
    uint64_t last_usable;
    uint64_t backup_lba;
    size_t populated = 0;
    enum h700_gpt_result result;

    if (table != NULL)
        memset(table, 0, sizeof(*table));
    if (partitions != NULL && partition_capacity != 0u) {
        size_t clear_count = partition_capacity;
        if (clear_count > H700_GPT_MAX_ENTRIES)
            clear_count = H700_GPT_MAX_ENTRIES;
        memset(partitions, 0, clear_count * sizeof(*partitions));
    }
    if (reader == NULL || partitions == NULL || table == NULL ||
        reader->read_sector == NULL || reader->sector_size != H700_GPT_SECTOR_SIZE ||
        reader->sector_count < 8u)
        return H700_GPT_ERR_ARGUMENT;

    result = validate_protective_mbr(reader, sector);
    if (result != H700_GPT_OK)
        return result;

    if (read_one(reader, GPT_HEADER_LBA, header) != 0)
        return H700_GPT_ERR_IO;
    if (memcmp(header, "EFI PART", 8u) != 0)
        return H700_GPT_ERR_SIGNATURE;
    if (read_le32(header + 8) != GPT_HEADER_REVISION_1_0)
        return H700_GPT_ERR_UNSUPPORTED_REVISION;

    header_size = read_le32(header + 12);
    if (header_size < GPT_HEADER_MIN_SIZE || header_size > H700_GPT_SECTOR_SIZE ||
        read_le32(header + 20) != 0u || read_le64(header + 24) != GPT_HEADER_LBA)
        return H700_GPT_ERR_HEADER;

    backup_lba = read_le64(header + 32);
    first_usable = read_le64(header + 40);
    last_usable = read_le64(header + 48);
    entries_lba = read_le64(header + 72);
    entry_count = read_le32(header + 80);
    entry_size = read_le32(header + 84);
    expected_header_crc = read_le32(header + 16);
    expected_entries_crc = read_le32(header + 88);

    memset(header + 16, 0, 4u);
    if (crc32_finish(crc32_update(0xFFFFFFFFu, header, header_size)) != expected_header_crc)
        return H700_GPT_ERR_HEADER_CRC;

    if (backup_lba != reader->sector_count - 1u || backup_lba == GPT_HEADER_LBA ||
        first_usable < 2u || first_usable > last_usable ||
        last_usable >= backup_lba || entry_count == 0u ||
        entry_count > H700_GPT_MAX_ENTRIES || entry_size < 128u ||
        entry_size > H700_GPT_SECTOR_SIZE || (entry_size % 8u) != 0u)
        return H700_GPT_ERR_HEADER;

    if ((H700_GPT_SECTOR_SIZE % entry_size) != 0u)
        return H700_GPT_ERR_ENTRY_LAYOUT;
    if (partition_capacity < entry_count)
        return H700_GPT_ERR_CAPACITY;

    table_bytes = (uint64_t)entry_count * (uint64_t)entry_size;
    if (table_bytes == 0u || table_bytes > GPT_MAX_TABLE_BYTES)
        return H700_GPT_ERR_ENTRY_LAYOUT;
    table_sectors = (table_bytes + H700_GPT_SECTOR_SIZE - 1u) / H700_GPT_SECTOR_SIZE;
    if (entries_lba < 2u || entries_lba >= reader->sector_count ||
        table_sectors > reader->sector_count - entries_lba)
        return H700_GPT_ERR_ENTRY_LAYOUT;
    table_end_lba = entries_lba + table_sectors;
    if (table_end_lba > first_usable ||
        (entries_lba <= backup_lba && backup_lba < table_end_lba))
        return H700_GPT_ERR_ENTRY_LAYOUT;

    /* First pass: verify the entire partition-array CRC before parsing entries. */
    for (offset = 0; offset < table_bytes; offset += H700_GPT_SECTOR_SIZE) {
        size_t bytes_this_sector = (size_t)(table_bytes - offset);
        if (bytes_this_sector > H700_GPT_SECTOR_SIZE)
            bytes_this_sector = H700_GPT_SECTOR_SIZE;
        if (read_one(reader, entries_lba + offset / H700_GPT_SECTOR_SIZE, sector) != 0)
            return H700_GPT_ERR_IO;
        entries_crc = crc32_update(entries_crc, sector, bytes_this_sector);
    }
    if (crc32_finish(entries_crc) != expected_entries_crc)
        return H700_GPT_ERR_ENTRY_CRC;

    table->first_usable_lba = first_usable;
    table->last_usable_lba = last_usable;
    table->backup_header_lba = backup_lba;
    table->entry_count = entry_count;
    table->entry_size = entry_size;

    /* Second pass: decode entries. Supported entry sizes divide 512. */
    for (offset = 0; offset < table_bytes; offset += H700_GPT_SECTOR_SIZE) {
        size_t bytes_this_sector = (size_t)(table_bytes - offset);
        uint32_t entries_in_sector;
        uint32_t j;
        if (bytes_this_sector > H700_GPT_SECTOR_SIZE)
            bytes_this_sector = H700_GPT_SECTOR_SIZE;
        if (read_one(reader, entries_lba + offset / H700_GPT_SECTOR_SIZE, sector) != 0) {
            table->partition_count = 0;
            return H700_GPT_ERR_IO;
        }
        entries_in_sector = (uint32_t)(bytes_this_sector / entry_size);
        for (j = 0; j < entries_in_sector; ++j) {
            int used = 0;
            uint32_t entry_slot = (uint32_t)(offset / entry_size) + j;
            result = parse_partition_entry(sector + (size_t)j * entry_size,
                                           entry_size, entry_slot, table,
                                           &partitions[populated], &used);
            if (result != H700_GPT_OK) {
                table->partition_count = 0;
                return result;
            }
            if (used)
                ++populated;
        }
    }

    result = validate_partition_set(partitions, populated);
    if (result != H700_GPT_OK) {
        table->partition_count = 0;
        return result;
    }
    table->partition_count = populated;
    return H700_GPT_OK;
}

enum h700_gpt_result h700_gpt_find_unique_name(
    const struct h700_gpt_partition *partitions,
    size_t partition_count,
    const char *name,
    const struct h700_gpt_partition **match)
{
    size_t i;
    const struct h700_gpt_partition *found = NULL;

    if (match != NULL)
        *match = NULL;
    if (partitions == NULL || name == NULL || match == NULL || name[0] == '\0')
        return H700_GPT_ERR_ARGUMENT;

    for (i = 0; i < partition_count; ++i) {
        if (strcmp(partitions[i].name, name) == 0) {
            if (found != NULL)
                return H700_GPT_ERR_AMBIGUOUS_NAME;
            found = &partitions[i];
        }
    }
    if (found == NULL)
        return H700_GPT_ERR_NOT_FOUND;
    *match = found;
    return H700_GPT_OK;
}

const char *h700_gpt_result_string(enum h700_gpt_result result)
{
    switch (result) {
    case H700_GPT_OK: return "ok";
    case H700_GPT_ERR_ARGUMENT: return "invalid argument";
    case H700_GPT_ERR_IO: return "sector read failed";
    case H700_GPT_ERR_PROTECTIVE_MBR: return "protective MBR missing or invalid";
    case H700_GPT_ERR_SIGNATURE: return "GPT signature mismatch";
    case H700_GPT_ERR_UNSUPPORTED_REVISION: return "unsupported GPT revision";
    case H700_GPT_ERR_HEADER: return "invalid GPT header";
    case H700_GPT_ERR_HEADER_CRC: return "GPT header CRC mismatch";
    case H700_GPT_ERR_ENTRY_LAYOUT: return "invalid GPT entry-array layout";
    case H700_GPT_ERR_ENTRY_CRC: return "GPT entry-array CRC mismatch";
    case H700_GPT_ERR_CAPACITY: return "partition output capacity too small";
    case H700_GPT_ERR_PARTITION_BOUNDS: return "partition outside usable LBA range";
    case H700_GPT_ERR_DUPLICATE_GUID: return "duplicate partition unique GUID";
    case H700_GPT_ERR_OVERLAP: return "overlapping partitions";
    case H700_GPT_ERR_NOT_FOUND: return "partition name not found";
    case H700_GPT_ERR_AMBIGUOUS_NAME: return "partition name is ambiguous";
    default: return "unknown GPT result";
    }
}
