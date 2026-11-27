#include "m1_oneshot_selection_model.h"

/*
 * Dummy capsule-facing ABI fixture. It intentionally selects only; it does
 * not copy to fixed SRAM addresses, jump to an image, or access peripherals.
 */
enum h700_m1_capsule_choice h700_m1_capsule_dispatch_fixture(
    const struct h700_m1_capsule_image *stock,
    const struct h700_m1_capsule_image *m1_spl)
{
    return h700_m1_capsule_select(stock, m1_spl);
}
