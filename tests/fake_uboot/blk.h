#ifndef H700_TEST_FAKE_UBOOT_BLK_H
#define H700_TEST_FAKE_UBOOT_BLK_H
#include <stdint.h>
typedef uint64_t lbaint_t;
struct blk_desc {
    const uint8_t *image;
    uint64_t first_lba;
    uint32_t sectors;
    unsigned reads;
    unsigned writes;
    int fail_read;
    const uint8_t *gpt_image;
    uint32_t gpt_sector_count;
};
lbaint_t blk_dread(struct blk_desc *desc, lbaint_t start,
                   lbaint_t count, void *destination);
#endif
