#include "../../platform/h700/power/h700_rg34xx_power_led.h"
#include "../../platform/h700/runtime/lk_heap_service.h"

/*
 * RG34XX H700 target lifecycle hooks.
 *
 * GammaOS LK kernel/main.c calls both hooks unconditionally:
 *   - target_early_init() before heap/timer initialization;
 *   - target_init() after platform_init() and before apps_init().
 *
 * The generic MTK target/init.c is excluded. The H700 arch MMU hook maps
 * the PIO peripheral window as device memory before target_early_init().
 * Physical PI12 MMIO stays disabled by default until BL33 APB1/PIO access is
 * qualified; a separate build may explicitly opt in after that review.
 * No MTK fastboot/eMMC services are imported or enabled.
 */
void target_early_init(void)
{
#if defined(H700_RG34XX_PI12_EARLY_ACCESS_ENABLE) && H700_RG34XX_PI12_EARLY_ACCESS_ENABLE
    (void)h700_rg34xx_power_led_early_init();
#endif
}

void target_init(void)
{
#if defined(H700_RG34XX_PI12_EARLY_ACCESS_ENABLE) && H700_RG34XX_PI12_EARLY_ACCESS_ENABLE
    if (!h700_rg34xx_power_led_is_ready())
        (void)h700_rg34xx_power_led_early_init();
#endif

    /* The NAND-write bootstrap bypasses heap_init(); never qualify that path. */
#if !defined(ENABLE_NANDWRITE) || !(ENABLE_NANDWRITE)
    /* Normal pinned-LK path reaches target_init() after heap_init() and timer_init(). */
    h700_lk_heap_service_mark_target_init_complete();
#endif
}
