/*
 * RG34XX H700 display refresh policy and candidate panel timing profiles.
 *
 * Policy only: this module does not touch TCON, DE, CCU, PLL, panel GPIO,
 * backlight, MMIO or any boot-package/SD-card area. A separate display HAL
 * must validate the panel revision and implement synchronized mode changes.
 */
#ifndef H700_DISPLAY_REFRESH_POLICY_H
#define H700_DISPLAY_REFRESH_POLICY_H

#include <stdint.h>

#define H700_DISPLAY_REFRESH_POLICY_ABI_VERSION 1u
#define H700_DISPLAY_REFRESH_POLICY_MAGIC 0x48445231u /* "HDR1" */

enum h700_display_scene {
    H700_DISPLAY_SCENE_UI = 0,
    H700_DISPLAY_SCENE_GAME = 1
};

enum h700_display_refresh_mode {
    H700_DISPLAY_REFRESH_60HZ = 60,
    H700_DISPLAY_REFRESH_120HZ = 120
};

enum h700_display_refresh_status {
    H700_DISPLAY_REFRESH_STATUS_OK = 0,
    H700_DISPLAY_REFRESH_POLICY_ONLY = 1,
    H700_DISPLAY_REFRESH_ERR_INVALID_ARGUMENT = -1,
    H700_DISPLAY_REFRESH_ERR_ABI = -2,
    H700_DISPLAY_REFRESH_ERR_NOT_INITIALIZED = -3
};

/*
 * Candidate timings transcribed from the RG34XX-specific 60/119.947 Hz
 * boot-package DTB profiles. lcd_ht is the Allwinner FEX property, not a
 * generic renderer frame-rate limit.
 */
typedef struct h700_display_refresh_timing_v1 {
    uint32_t mode_hz;
    uint32_t refresh_millihz;
    uint32_t lcd_dclk_freq_mhz;
    uint32_t lcd_ht;
    uint32_t lcd_vt;
    uint32_t lcd_hbp;
    uint32_t lcd_hspw;
    uint32_t lcd_vbp;
    uint32_t lcd_vspw;
    uint32_t width;
    uint32_t height;
} h700_display_refresh_timing_v1_t;

/*
 * Caller-owned policy state. requested_mode_hz is a desired mode only;
 * hardware_applied remains zero until a separately validated LCD HAL exists.
 */
typedef struct h700_display_refresh_policy_v1 {
    uint32_t abi_version;
    uint32_t struct_size;
    uint32_t magic;
    uint32_t initialized;
    uint32_t ui_preference_hz;
    uint32_t scene;
    uint32_t requested_mode_hz;
    uint32_t generation;
    uint32_t hardware_applied;
    uint32_t reserved[3];
} h700_display_refresh_policy_v1_t;

typedef struct h700_display_refresh_state_v1 {
    uint32_t abi_version;
    uint32_t struct_size;
    uint32_t generation;
    uint32_t ui_preference_hz;
    uint32_t scene;
    uint32_t requested_mode_hz;
    uint32_t hardware_applied;
    uint32_t reserved;
} h700_display_refresh_state_v1_t;

/* Default policy: UI 120 Hz, game 60 Hz. No hardware mode is applied. */
int h700_display_refresh_policy_init_v1(
    h700_display_refresh_policy_v1_t *policy,
    uint32_t policy_size
);

/*
 * Change the UI refresh preference (the future user-visible 60/120 Hz toggle).
 * In GAME scene the requested mode remains 60 Hz; the preference is restored
 * automatically when the scene returns to UI.
 */
int h700_display_refresh_policy_set_ui_preference_v1(
    h700_display_refresh_policy_v1_t *policy,
    enum h700_display_refresh_mode mode
);

/* Switch the logical scene. GAME always resolves to 60 Hz. */
int h700_display_refresh_policy_set_scene_v1(
    h700_display_refresh_policy_v1_t *policy,
    enum h700_display_scene scene
);

int h700_display_refresh_policy_get_state_v1(
    const h700_display_refresh_policy_v1_t *policy,
    h700_display_refresh_state_v1_t *state,
    uint32_t state_size
);

/* Returns NULL for an unsupported mode. Profiles do not program hardware. */
const h700_display_refresh_timing_v1_t *
h700_display_refresh_get_rg34xx_timing_v1(
    enum h700_display_refresh_mode mode
);

#endif /* H700_DISPLAY_REFRESH_POLICY_H */
