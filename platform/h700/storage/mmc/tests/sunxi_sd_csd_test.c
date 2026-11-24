#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>

#include "../sunxi_sd_csd.h"

#define CHECK(condition, message) do { \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s (line %d)\n", message, __LINE__); \
        return 0; \
    } \
} while (0)

static void set_csd_bits(uint32_t response[4],
                         uint32_t start_bit,
                         uint32_t bit_count,
                         uint32_t value)
{
    uint32_t i;
    for (i = 0u; i < bit_count; ++i) {
        uint32_t bit = start_bit + i;
        uint32_t word_index = 3u - (bit / 32u);
        uint32_t shift = bit % 32u;
        uint32_t mask = 1u << shift;
        if ((value & (1u << i)) != 0u)
            response[word_index] |= mask;
        else
            response[word_index] &= ~mask;
    }
}

static void make_sdsc_csd(uint32_t response[4],
                          uint32_t c_size,
                          uint32_t c_size_mult,
                          uint32_t read_block_length)
{
    memset(response, 0, 4u * sizeof(response[0]));
    set_csd_bits(response, 126u, 2u, 0u);
    set_csd_bits(response, 62u, 12u, c_size);
    set_csd_bits(response, 47u, 3u, c_size_mult);
    set_csd_bits(response, 80u, 4u, read_block_length);
}

static void make_sdhc_csd(uint32_t response[4], uint32_t c_size)
{
    memset(response, 0, 4u * sizeof(response[0]));
    set_csd_bits(response, 126u, 2u, 1u);
    set_csd_bits(response, 48u, 22u, c_size);
}

static int test_rg34xx_observed_capacity(void)
{
    uint32_t response[4];
    struct sunxi_sd_csd_info info;

    /*
     * The observed Linux block device reports 30,375,936 sectors.
     * The corresponding synthetic SDHC C_SIZE is 29,663.
     */
    make_sdhc_csd(response, 29663u);
    CHECK(sunxi_sd_csd_parse(response, true, &info) == SUNXI_SD_CSD_OK,
          "synthetic SDHC CSD parses");
    CHECK(info.structure == 1u && info.block_addressed,
          "SDHC block-addressed identity preserved");
    CHECK(info.sector_count == 30375936ull,
          "parsed capacity matches RG34XX live inventory");
    CHECK(info.capacity_bytes == 15552479232ull,
          "parsed byte capacity matches sector count");
    CHECK(info.read_block_length == 9u,
          "SDHC fixed read block length is 512 bytes");
    return 1;
}

static int test_sdhc_capacity_boundaries(void)
{
    uint32_t response[4];
    struct sunxi_sd_csd_info info;

    make_sdhc_csd(response, 0u);
    CHECK(sunxi_sd_csd_parse(response, true, &info) == SUNXI_SD_CSD_OK,
          "minimum SDHC C_SIZE parses");
    CHECK(info.sector_count == 1024ull,
          "minimum SDHC capacity is 1024 sectors");

    make_sdhc_csd(response, 0x3fffffu);
    CHECK(sunxi_sd_csd_parse(response, true, &info) == SUNXI_SD_CSD_OK,
          "maximum SDHC C_SIZE parses");
    CHECK(info.sector_count == 4294967296ull,
          "maximum SDHC capacity uses full 32-bit LBA range");
    return 1;
}

static int test_sdsc_capacity_formula(void)
{
    uint32_t response[4];
    struct sunxi_sd_csd_info info;

    make_sdsc_csd(response, 1023u, 3u, 9u);
    CHECK(sunxi_sd_csd_parse(response, false, &info) == SUNXI_SD_CSD_OK,
          "SDSC CSD parses");
    CHECK(info.sector_count == 32768ull,
          "SDSC C_SIZE/C_SIZE_MULT formula is correct");
    CHECK(info.capacity_bytes == 16777216ull,
          "SDSC capacity is reported in bytes");

    make_sdsc_csd(response, 1023u, 3u, 10u);
    CHECK(sunxi_sd_csd_parse(response, false, &info) == SUNXI_SD_CSD_OK,
          "SDSC with 1024-byte read block parses");
    CHECK(info.sector_count == 65536ull,
          "SDSC READ_BL_LEN is normalized to 512-byte sectors");
    return 1;
}

static int test_addressing_mismatch_and_unsupported_structure(void)
{
    uint32_t response[4];
    struct sunxi_sd_csd_info info;

    make_sdhc_csd(response, 29663u);
    CHECK(sunxi_sd_csd_parse(response, false, &info) ==
              SUNXI_SD_CSD_ADDRESSING_MISMATCH,
          "SDHC CSD cannot be marked byte-addressed");

    make_sdsc_csd(response, 1023u, 3u, 9u);
    CHECK(sunxi_sd_csd_parse(response, true, &info) ==
              SUNXI_SD_CSD_ADDRESSING_MISMATCH,
          "SDSC CSD cannot be marked block-addressed");

    memset(response, 0, sizeof(response));
    set_csd_bits(response, 126u, 2u, 2u);
    CHECK(sunxi_sd_csd_parse(response, true, &info) ==
              SUNXI_SD_CSD_UNSUPPORTED,
          "SDUC CSD structure is rejected by the H700 SDHC profile");
    return 1;
}

static int test_invalid_read_block_length_and_arguments(void)
{
    uint32_t response[4];
    struct sunxi_sd_csd_info info;

    make_sdsc_csd(response, 0u, 0u, 8u);
    CHECK(sunxi_sd_csd_parse(response, false, &info) ==
              SUNXI_SD_CSD_INVALID_BLOCK_LENGTH,
          "read block length below 512 bytes rejected");

    make_sdsc_csd(response, 0u, 0u, 12u);
    CHECK(sunxi_sd_csd_parse(response, false, &info) ==
              SUNXI_SD_CSD_INVALID_BLOCK_LENGTH,
          "read block length above supported range rejected");

    CHECK(sunxi_sd_csd_parse(NULL, false, &info) == SUNXI_SD_CSD_INVALID,
          "null CSD response rejected");
    CHECK(sunxi_sd_csd_parse(response, false, NULL) == SUNXI_SD_CSD_INVALID,
          "null output structure rejected");
    CHECK(info.sector_count == 0u && info.capacity_bytes == 0u,
          "failed parse leaves output zeroed");
    return 1;
}

int main(void)
{
    if (!test_rg34xx_observed_capacity() ||
        !test_sdhc_capacity_boundaries() ||
        !test_sdsc_capacity_formula() ||
        !test_addressing_mismatch_and_unsupported_structure() ||
        !test_invalid_read_block_length_and_arguments())
        return EXIT_FAILURE;

    puts("PASS: H700 SD CSD capacity and addressing tests");
    return EXIT_SUCCESS;
}
