/*
 * Read-only board profile derived from the RG34XX live MMC capture.
 *
 * This file describes observed controller identity and storage policy only.
 * It does not initialize clocks, pins, regulators, or MMIO registers.
 */
#ifndef H700_MMC_BOARD_PROFILE_H
#define H700_MMC_BOARD_PROFILE_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

enum h700_mmc_role {
    H700_MMC_ROLE_PRIMARY_BOOT_SD = 0,
    H700_MMC_ROLE_WIFI_SDIO,
    H700_MMC_ROLE_SECONDARY_SD_SLOT_CANDIDATE,
};

enum h700_mmc_card_observation {
    H700_MMC_CARD_SDHC_SEEN,
    H700_MMC_CARD_SDIO_SEEN,
    H700_MMC_CARD_NOT_PRESENT_AT_CAPTURE,
};

enum h700_mmc_dt_property_state {
    H700_MMC_DT_PROPERTY_ABSENT,
    H700_MMC_DT_PROPERTY_PRESENT_UNRESOLVED,
};

struct h700_mmc_controller_profile {
    const char *controller_name;
    const char *compatible;
    uint32_t mmio_base;
    uint32_t mmio_size;
    enum h700_mmc_role role;
    enum h700_mmc_card_observation card_observed;
    uint8_t bus_width;

    /*
     * These host numbers are captured Linux observations only. Driver code
     * must select by MMIO base/compatible, never by this field or DT aliases.
     */
    int8_t observed_linux_host_index;

    enum h700_mmc_dt_property_state cd_gpios;
    enum h700_mmc_dt_property_state vmmc_supply;
    enum h700_mmc_dt_property_state vqmmc_supply;

    /* True means Linux's live DT/driver treated the current card as fixed. */
    bool linux_nonremovable_observed;
};

size_t h700_mmc_profile_count(void);
const struct h700_mmc_controller_profile *
h700_mmc_profile_at(size_t index);
const struct h700_mmc_controller_profile *
h700_mmc_profile_by_base(uint32_t mmio_base);

/*
 * Primary same-card storage path: sdc0 owns the running rootfs and the
 * bootargs-designated UDISK partition. This is a policy selector, not I/O.
 */
const struct h700_mmc_controller_profile *
h700_mmc_primary_storage_profile(void);

#endif /* H700_MMC_BOARD_PROFILE_H */
