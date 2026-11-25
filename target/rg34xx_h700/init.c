#include "h700_rg34xx_power_led.h"

/*
 * RG34XX H700 target lifecycle hooks.
 *
 * GammaOS LK kernel/main.c calls both hooks unconditionally:
 *   - target_early_init() before heap/timer initialization;
 *   - target_init() after platform_init() and before apps_init().
 *
 * The generic target/init.c is intentionally excluded by the H700 overlay
 * because it also provides MediaTek storage/fastboot defaults and includes
 * MTK-specific headers. Keep this replacement deliberately side-effect-free:
 * board services are not ready/approved yet, and this target does not enable
 * the MTK aboot/fastboot application.
 *
 * Future H700 board initialization belongs in target_init(), after platform
 * services are ready and before apps_init(), but only when each service has
 * its own reviewed hardware and memory contract.
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
