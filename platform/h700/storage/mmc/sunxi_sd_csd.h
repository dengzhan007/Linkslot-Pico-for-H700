#ifndef H700_SUNXI_SD_CSD_H
#define H700_SUNXI_SD_CSD_H

#include <stdbool.h>
#include <stdint.h>

struct sunxi_sd_csd_info {
    uint64_t sector_count;
    uint64_t capacity_bytes;
    uint8_t structure;
    uint8_t read_block_length;
    bool block_addressed;
};

enum sunxi_sd_csd_result {
    SUNXI_SD_CSD_OK = 0,
    SUNXI_SD_CSD_INVALID = -1,
    SUNXI_SD_CSD_UNSUPPORTED = -2,
    SUNXI_SD_CSD_ADDRESSING_MISMATCH = -3,
    SUNXI_SD_CSD_INVALID_BLOCK_LENGTH = -4,
    SUNXI_SD_CSD_CAPACITY_OVERFLOW = -5
};

/*
 * Decode the four 32-bit CSD response words in controller order:
 * response[0] contains bits 127..96 and response[3] contains bits 31..0.
 * The caller must have obtained a CRC-checked R2 response from the host.
 *
 * Only standard-capacity CSD structure 0 and high-capacity structure 1 are
 * supported. SDUC structure 2 and reserved structure 3 fail closed.
 */
enum sunxi_sd_csd_result sunxi_sd_csd_parse(
    const uint32_t response[4],
    bool block_addressed,
    struct sunxi_sd_csd_info *info);

#endif /* H700_SUNXI_SD_CSD_H */
