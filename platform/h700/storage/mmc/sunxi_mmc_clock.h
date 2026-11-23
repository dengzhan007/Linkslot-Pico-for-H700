#ifndef H700_SUNXI_MMC_CLOCK_H
#define H700_SUNXI_MMC_CLOCK_H

#include <stdbool.h>
#include <stdint.h>

struct sunxi_mmc_io {
    void *context;
    uint32_t (*read32)(void *context, uint32_t offset);
    void (*write32)(void *context, uint32_t offset, uint32_t value);
    uint64_t (*time_ms)(void *context);
    void (*delay_us)(void *context, uint32_t delay);
};

enum sunxi_mmc_clock_result {
    SUNXI_MMC_CLOCK_OK = 0,
    SUNXI_MMC_CLOCK_INVALID = -1,
    SUNXI_MMC_CLOCK_TIMEOUT = -2,
};

/*
 * Toggle the card clock and execute the controller's UPCLK_ONLY update.
 *
 * For H616/H700, mask_data0 must be true: CLKCR.MASK_DATA0 is asserted for
 * the update command and cleared after completion. The callback interface
 * keeps register access injectable for host tests. No other controller
 * configuration, card command, or block I/O is performed here.
 *
 * timeout_ms must be non-zero. On timeout the helper clears MASK_DATA0 so
 * the host is not left with DAT0 masked. Other CLKCR fields (including the
 * requested card-clock state and divider) are preserved.
 */
int sunxi_mmc_set_output_clock(const struct sunxi_mmc_io *io,
                               bool enable,
                               bool mask_data0,
                               uint32_t timeout_ms);

#endif /* H700_SUNXI_MMC_CLOCK_H */
