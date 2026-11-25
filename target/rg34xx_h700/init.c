#include "../../platform/h700/power/h700_rg34xx_power_led.h"

/*
 * RG34XX H700 target lifecycle hooks.
 *
 * GammaOS LK kernel/main.c calls both hooks unconditionally:
 *   - target_early_init() before heap/timer initialization;
 *   - target_init() after platform_init() and before apps_init().
 *
 * The generic MTK target/init.c is excluded. The H700 arch MMU hook maps
 * the PIO peripheral window as device memory before target_early_init().
 * target_early_init() then turns on PI12; target_init() retries if needed.
 * No MTK fastboot/eMMC services are imported or enabled.
 */
void target_early_init(void)
{
    /* Turn the RG34XX work LED on before LK heap/thread startup. */
    (void)h700_rg34xx_power_led_early_init();
}

void target_init(void)
{
    /* Retry if PIO was not accessible during the early hook. */
    if (!h700_rg34xx_power_led_is_ready())
        (void)h700_rg34xx_power_led_early_init();
}
