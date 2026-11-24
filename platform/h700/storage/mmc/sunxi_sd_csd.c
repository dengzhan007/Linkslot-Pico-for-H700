#include "sunxi_sd_csd.h"

#define SD_BLOCK_SIZE 512u
#define SDSC_MAX_SECTORS 8388608ull

static uint32_t csd_extract_bits(const uint32_t response[4],
                                 uint32_t start_bit,
                                 uint32_t bit_count)
{
    uint32_t result = 0u;
    uint32_t i;

    for (i = 0u; i < bit_count; ++i) {
        uint32_t bit = start_bit + i;
        uint32_t word_index = 3u - (bit / 32u);
        uint32_t shift = bit % 32u;
        uint32_t value = (response[word_index] >> shift) & 1u;
        result |= value << i;
    }
    return result;
}

enum sunxi_sd_csd_result sunxi_sd_csd_parse(
    const uint32_t response[4],
    bool block_addressed,
    struct sunxi_sd_csd_info *info)
{
    uint32_t structure;
    uint64_t sectors;
    uint32_t read_block_length;

    if (info != 0) {
        info->sector_count = 0u;
        info->capacity_bytes = 0u;
        info->structure = 0u;
        info->read_block_length = 0u;
        info->block_addressed = false;
    }

    if (response == 0 || info == 0)
        return SUNXI_SD_CSD_INVALID;

    structure = csd_extract_bits(response, 126u, 2u);

    if (structure == 1u) {
        uint32_t c_size;

        if (!block_addressed)
            return SUNXI_SD_CSD_ADDRESSING_MISMATCH;

        c_size = csd_extract_bits(response, 48u, 22u);
        sectors = ((uint64_t)c_size + 1u) << 10;
        read_block_length = 9u;
    } else if (structure == 0u) {
        uint32_t c_size;
        uint32_t c_size_mult;
        uint32_t shift;

        if (block_addressed)
            return SUNXI_SD_CSD_ADDRESSING_MISMATCH;

        read_block_length = csd_extract_bits(response, 80u, 4u);
        if (read_block_length < 9u || read_block_length > 11u)
            return SUNXI_SD_CSD_INVALID_BLOCK_LENGTH;

        c_size = csd_extract_bits(response, 62u, 12u);
        c_size_mult = csd_extract_bits(response, 47u, 3u);
        shift = c_size_mult + 2u + (read_block_length - 9u);
        sectors = ((uint64_t)c_size + 1u) << shift;

        /* SDSC uses a 32-bit byte address in CMD17. */
        if (sectors == 0u || sectors > SDSC_MAX_SECTORS)
            return SUNXI_SD_CSD_CAPACITY_OVERFLOW;
    } else {
        return SUNXI_SD_CSD_UNSUPPORTED;
    }

    if (sectors == 0u || sectors > (UINT64_MAX / SD_BLOCK_SIZE))
        return SUNXI_SD_CSD_CAPACITY_OVERFLOW;

    info->sector_count = sectors;
    info->capacity_bytes = sectors * SD_BLOCK_SIZE;
    info->structure = (uint8_t)structure;
    info->read_block_length = (uint8_t)read_block_length;
    info->block_addressed = block_addressed;
    return SUNXI_SD_CSD_OK;
}

