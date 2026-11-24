#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "../h700_mmc_board_profile.h"

#define CHECK(condition, message) do { \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        return EXIT_FAILURE; \
    } \
} while (0)

int main(void)
{
    const struct h700_mmc_controller_profile *sdc0;
    const struct h700_mmc_controller_profile *sdc1;
    const struct h700_mmc_controller_profile *sdc2;
    const struct h700_mmc_controller_profile *primary;
    size_t i;
    size_t j;

    CHECK(h700_mmc_profile_count() == 3u, "exactly three observed controllers");

    sdc0 = h700_mmc_profile_by_base(0x04020000u);
    sdc1 = h700_mmc_profile_by_base(0x04021000u);
    sdc2 = h700_mmc_profile_by_base(0x04022000u);
    CHECK(sdc0 != NULL && sdc1 != NULL && sdc2 != NULL,
          "lookup by MMIO base finds all controllers");
    CHECK(h700_mmc_profile_by_base(0x04023000u) == NULL,
          "unknown controller base is rejected");

    primary = h700_mmc_primary_storage_profile();
    CHECK(primary == sdc0, "same-card primary storage is sdc0");
    CHECK(primary->role == H700_MMC_ROLE_PRIMARY_BOOT_SD,
          "sdc0 is marked as current boot SD");
    CHECK(strcmp(primary->compatible, "allwinner,sunxi-mmc-v4p1x") == 0,
          "sdc0 compatible matches capture");
    CHECK(primary->card_observed == H700_MMC_CARD_SDHC_SEEN,
          "sdc0 is classified as observed SDHC, not eMMC");
    CHECK(primary->bus_width == 4u, "sdc0 live bus width is four");
    CHECK(primary->linux_nonremovable_observed,
          "Linux non-removable observation remains explicit");
    CHECK(primary->cd_gpios == H700_MMC_DT_PROPERTY_ABSENT,
          "sdc0 does not claim a card-detect GPIO");

    CHECK(sdc1->role == H700_MMC_ROLE_WIFI_SDIO,
          "sdc1 is excluded from primary storage");
    CHECK(sdc1->observed_linux_host_index == 2,
          "sdc1 observed host index is diagnostic only");
    CHECK(sdc1->card_observed == H700_MMC_CARD_SDIO_SEEN,
          "sdc1 captured SDIO enumeration");

    CHECK(sdc2->role == H700_MMC_ROLE_SECONDARY_SD_SLOT_CANDIDATE,
          "sdc2 remains a second-slot candidate");
    CHECK(sdc2->observed_linux_host_index == 1,
          "sdc2 host index differs from controller suffix");
    CHECK(sdc2->card_observed == H700_MMC_CARD_NOT_PRESENT_AT_CAPTURE,
          "no card is assumed present in sdc2");
    CHECK(sdc2->cd_gpios == H700_MMC_DT_PROPERTY_PRESENT_UNRESOLVED,
          "sdc2 CD GPIO is not decoded into unsafe pin config");
    CHECK(sdc2->vqmmc_supply == H700_MMC_DT_PROPERTY_PRESENT_UNRESOLVED,
          "sdc2 vqmmc provider is not hard-coded");
    CHECK(sdc2->vmmc_supply == H700_MMC_DT_PROPERTY_ABSENT,
          "sdc2 has no vmmc-supply DT property");

    for (i = 0; i < h700_mmc_profile_count(); ++i) {
        const struct h700_mmc_controller_profile *a =
            h700_mmc_profile_at(i);
        CHECK(a != NULL, "profile index is valid");
        for (j = i + 1u; j < h700_mmc_profile_count(); ++j) {
            const struct h700_mmc_controller_profile *b =
                h700_mmc_profile_at(j);
            CHECK(a->mmio_base != b->mmio_base,
                  "controller MMIO bases are unique");
        }
    }
    CHECK(h700_mmc_profile_at(h700_mmc_profile_count()) == NULL,
          "out-of-range profile index is rejected");

    puts("PASS: RG34XX MMC board profile selects sdc0 and keeps sdc2 GPIO/supply unresolved");
    return EXIT_SUCCESS;
}
