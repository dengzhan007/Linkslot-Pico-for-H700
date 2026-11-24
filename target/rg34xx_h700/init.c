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
    /* No H700 target-specific work is safe at this early stage yet. */
}

void target_init(void)
{
    /* Intentionally empty until a validated H700 target init sequence exists. */
}
