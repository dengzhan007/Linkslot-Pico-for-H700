#include "h700_mmc_board_profile.h"

static const struct h700_mmc_controller_profile profiles[] = {
    {
        .controller_name = "sdc0",
        .compatible = "allwinner,sunxi-mmc-v4p1x",
        .mmio_base = 0x04020000u,
        .mmio_size = 0x1000u,
        .role = H700_MMC_ROLE_PRIMARY_BOOT_SD,
        .card_observed = H700_MMC_CARD_SDHC_SEEN,
        .bus_width = 4,
        .observed_linux_host_index = 0,
        .cd_gpios = H700_MMC_DT_PROPERTY_ABSENT,
        .vmmc_supply = H700_MMC_DT_PROPERTY_ABSENT,
        .vqmmc_supply = H700_MMC_DT_PROPERTY_ABSENT,
        .linux_nonremovable_observed = true,
    },
    {
        .controller_name = "sdc1",
        .compatible = "allwinner,sunxi-mmc-v4p1x",
        .mmio_base = 0x04021000u,
        .mmio_size = 0x1000u,
        .role = H700_MMC_ROLE_WIFI_SDIO,
        .card_observed = H700_MMC_CARD_SDIO_SEEN,
        .bus_width = 4,
        .observed_linux_host_index = 2,
        .cd_gpios = H700_MMC_DT_PROPERTY_ABSENT,
        .vmmc_supply = H700_MMC_DT_PROPERTY_ABSENT,
        .vqmmc_supply = H700_MMC_DT_PROPERTY_ABSENT,
        .linux_nonremovable_observed = false,
    },
    {
        .controller_name = "sdc2",
        .compatible = "allwinner,sunxi-mmc-v4p6x",
        .mmio_base = 0x04022000u,
        .mmio_size = 0x1000u,
        .role = H700_MMC_ROLE_SECONDARY_SD_SLOT_CANDIDATE,
        .card_observed = H700_MMC_CARD_NOT_PRESENT_AT_CAPTURE,
        .bus_width = 4,
        .observed_linux_host_index = 1,
        .cd_gpios = H700_MMC_DT_PROPERTY_PRESENT_UNRESOLVED,
        .vmmc_supply = H700_MMC_DT_PROPERTY_ABSENT,
        .vqmmc_supply = H700_MMC_DT_PROPERTY_PRESENT_UNRESOLVED,
        .linux_nonremovable_observed = false,
    },
};

size_t h700_mmc_profile_count(void)
{
    return sizeof(profiles) / sizeof(profiles[0]);
}

const struct h700_mmc_controller_profile *
h700_mmc_profile_at(size_t index)
{
    if (index >= h700_mmc_profile_count())
        return NULL;

    return &profiles[index];
}

const struct h700_mmc_controller_profile *
h700_mmc_profile_by_base(uint32_t mmio_base)
{
    size_t i;

    for (i = 0; i < h700_mmc_profile_count(); ++i) {
        if (profiles[i].mmio_base == mmio_base)
            return &profiles[i];
    }

    return NULL;
}

const struct h700_mmc_controller_profile *
h700_mmc_primary_storage_profile(void)
{
    size_t i;

    for (i = 0; i < h700_mmc_profile_count(); ++i) {
        if (profiles[i].role == H700_MMC_ROLE_PRIMARY_BOOT_SD)
            return &profiles[i];
    }

    return NULL;
}
