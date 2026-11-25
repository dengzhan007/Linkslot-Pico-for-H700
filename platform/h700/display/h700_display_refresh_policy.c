#include "h700_display_refresh_policy.h"

#include <stddef.h>

#define H700_DISPLAY_UI_DEFAULT_HZ H700_DISPLAY_REFRESH_120HZ
#define H700_DISPLAY_GAME_DEFAULT_HZ H700_DISPLAY_REFRESH_60HZ

/*
 * RG34XX timing values are based on TheGammaSqueeze/rgxx_boot_package_mods
 * branch rg34xx. Comparing its 60 Hz and 120 Hz DTBs showed lcd_ht is the
 * only property that differs in the sunxi.fex DTB:
 *   119.947 Hz: 54 MHz, lcd_ht=840,  lcd_vt=536
 *    60.006 Hz: 54 MHz, lcd_ht=1679, lcd_vt=536
 *
 * The external package also describes 720x480, hbp=46, hspw=2,
 * vbp=20 and vspw=5. Exact runtime programming is not implemented here.
 */
static const h700_display_refresh_timing_v1_t g_rg34xx_timings[] = {
    {
        H700_DISPLAY_REFRESH_60HZ,
        60006u,
        54u,
        1679u,
        536u,
        46u,
        2u,
        20u,
        5u,
        720u,
        480u
    },
    {
        H700_DISPLAY_REFRESH_120HZ,
        119947u,
        54u,
        840u,
        536u,
        46u,
        2u,
        20u,
        5u,
        720u,
        480u
    }
};

static int policy_is_ready(
    const h700_display_refresh_policy_v1_t *policy
)
{
    return policy != NULL &&
           policy->abi_version == H700_DISPLAY_REFRESH_POLICY_ABI_VERSION &&
           policy->struct_size == (uint32_t)sizeof(*policy) &&
           policy->magic == H700_DISPLAY_REFRESH_POLICY_MAGIC &&
           policy->initialized == 1u;
}

static int mode_is_valid(enum h700_display_refresh_mode mode)
{
    return mode == H700_DISPLAY_REFRESH_60HZ ||
           mode == H700_DISPLAY_REFRESH_120HZ;
}

static uint32_t resolve_mode(
    uint32_t ui_preference_hz,
    uint32_t game_120hz_enabled,
    uint32_t scene
)
{
    if (scene == H700_DISPLAY_SCENE_GAME) {
        return game_120hz_enabled != 0u
            ? H700_DISPLAY_REFRESH_120HZ
            : H700_DISPLAY_GAME_DEFAULT_HZ;
    }
    return ui_preference_hz;
}

static void increment_generation(h700_display_refresh_policy_v1_t *policy)
{
    policy->generation++;
    if (policy->generation == 0u) {
        policy->generation = 1u;
    }
}

int h700_display_refresh_policy_init_v1(
    h700_display_refresh_policy_v1_t *policy,
    uint32_t policy_size
)
{
    if (policy == NULL) {
        return H700_DISPLAY_REFRESH_ERR_INVALID_ARGUMENT;
    }
    if (policy_size < (uint32_t)sizeof(*policy)) {
        return H700_DISPLAY_REFRESH_ERR_ABI;
    }

    policy->abi_version = H700_DISPLAY_REFRESH_POLICY_ABI_VERSION;
    policy->struct_size = (uint32_t)sizeof(*policy);
    policy->magic = H700_DISPLAY_REFRESH_POLICY_MAGIC;
    policy->initialized = 1u;
    policy->ui_preference_hz = H700_DISPLAY_UI_DEFAULT_HZ;
    policy->scene = H700_DISPLAY_SCENE_UI;
    policy->requested_mode_hz = H700_DISPLAY_UI_DEFAULT_HZ;
    policy->generation = 1u;
    policy->hardware_applied = 0u;
    policy->game_120hz_enabled = 0u;
    policy->reserved[0] = 0u;
    policy->reserved[1] = 0u;

    return H700_DISPLAY_REFRESH_POLICY_ONLY;
}

int h700_display_refresh_policy_set_ui_preference_v1(
    h700_display_refresh_policy_v1_t *policy,
    enum h700_display_refresh_mode mode
)
{
    if (!policy_is_ready(policy)) {
        return H700_DISPLAY_REFRESH_ERR_NOT_INITIALIZED;
    }
    if (!mode_is_valid(mode)) {
        return H700_DISPLAY_REFRESH_ERR_INVALID_ARGUMENT;
    }

    if (policy->ui_preference_hz != (uint32_t)mode) {
        policy->ui_preference_hz = (uint32_t)mode;
        policy->requested_mode_hz = resolve_mode(
            policy->ui_preference_hz,
            policy->game_120hz_enabled,
            policy->scene
        );
        increment_generation(policy);
    }

    /* This state change only selects a target profile; it never applies it. */
    return H700_DISPLAY_REFRESH_POLICY_ONLY;
}

int h700_display_refresh_policy_set_game_120hz_enabled_v1(
    h700_display_refresh_policy_v1_t *policy,
    uint32_t enabled
)
{
    if (!policy_is_ready(policy)) {
        return H700_DISPLAY_REFRESH_ERR_NOT_INITIALIZED;
    }
    if (enabled > 1u) {
        return H700_DISPLAY_REFRESH_ERR_INVALID_ARGUMENT;
    }

    if (policy->game_120hz_enabled != enabled) {
        policy->game_120hz_enabled = enabled;
        policy->requested_mode_hz = resolve_mode(
            policy->ui_preference_hz,
            policy->game_120hz_enabled,
            policy->scene
        );
        increment_generation(policy);
    }

    return H700_DISPLAY_REFRESH_POLICY_ONLY;
}

int h700_display_refresh_policy_set_scene_v1(
    h700_display_refresh_policy_v1_t *policy,
    enum h700_display_scene scene
)
{
    if (!policy_is_ready(policy)) {
        return H700_DISPLAY_REFRESH_ERR_NOT_INITIALIZED;
    }
    if (scene != H700_DISPLAY_SCENE_UI &&
        scene != H700_DISPLAY_SCENE_GAME) {
        return H700_DISPLAY_REFRESH_ERR_INVALID_ARGUMENT;
    }

    if (policy->scene != (uint32_t)scene) {
        policy->scene = (uint32_t)scene;
        policy->requested_mode_hz = resolve_mode(
            policy->ui_preference_hz,
            policy->game_120hz_enabled,
            policy->scene
        );
        increment_generation(policy);
    }

    return H700_DISPLAY_REFRESH_POLICY_ONLY;
}

int h700_display_refresh_policy_get_state_v1(
    const h700_display_refresh_policy_v1_t *policy,
    h700_display_refresh_state_v1_t *state,
    uint32_t state_size
)
{
    if (!policy_is_ready(policy)) {
        return H700_DISPLAY_REFRESH_ERR_NOT_INITIALIZED;
    }
    if (state == NULL) {
        return H700_DISPLAY_REFRESH_ERR_INVALID_ARGUMENT;
    }
    if (state_size < (uint32_t)sizeof(*state)) {
        return H700_DISPLAY_REFRESH_ERR_ABI;
    }

    state->abi_version = H700_DISPLAY_REFRESH_POLICY_ABI_VERSION;
    state->struct_size = (uint32_t)sizeof(*state);
    state->generation = policy->generation;
    state->ui_preference_hz = policy->ui_preference_hz;
    state->scene = policy->scene;
    state->requested_mode_hz = policy->requested_mode_hz;
    state->hardware_applied = policy->hardware_applied;
    state->game_120hz_enabled = policy->game_120hz_enabled;
    return H700_DISPLAY_REFRESH_STATUS_OK;
}

const h700_display_refresh_timing_v1_t *
h700_display_refresh_get_rg34xx_timing_v1(
    enum h700_display_refresh_mode mode
)
{
    uint32_t i;

    if (!mode_is_valid(mode)) {
        return NULL;
    }

    for (i = 0u;
         i < (uint32_t)(sizeof(g_rg34xx_timings) / sizeof(g_rg34xx_timings[0]));
         ++i) {
        if (g_rg34xx_timings[i].mode_hz == (uint32_t)mode) {
            return &g_rg34xx_timings[i];
        }
    }

    return NULL;
}

int h700_display_refresh_get_period_ns_v1(
    enum h700_display_refresh_mode mode,
    uint64_t *period_ns
)
{
    const h700_display_refresh_timing_v1_t *timing;
    uint64_t total_pixels;

    if (period_ns == NULL) {
        return H700_DISPLAY_REFRESH_ERR_INVALID_ARGUMENT;
    }

    timing = h700_display_refresh_get_rg34xx_timing_v1(mode);
    if (timing == NULL ||
        timing->lcd_dclk_freq_mhz == 0u ||
        timing->lcd_ht == 0u ||
        timing->lcd_vt == 0u) {
        return H700_DISPLAY_REFRESH_ERR_INVALID_ARGUMENT;
    }

    /*
     * Period = htotal * vtotal / pixel_clock.
     * With the profile clock expressed in MHz:
     *   period_ns = htotal * vtotal * 1000 / clock_MHz.
     * Integer truncation matches the upstream Slot KMS mode_period helper.
     */
    total_pixels = (uint64_t)timing->lcd_ht * (uint64_t)timing->lcd_vt;
    *period_ns = (total_pixels * 1000ull) /
                 (uint64_t)timing->lcd_dclk_freq_mhz;
    return H700_DISPLAY_REFRESH_STATUS_OK;
}
