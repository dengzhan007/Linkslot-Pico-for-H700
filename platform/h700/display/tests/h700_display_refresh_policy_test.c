#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "../h700_display_refresh_policy.h"

#define CHECK(condition, message) do { \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        return EXIT_FAILURE; \
    } \
} while (0)

int main(void)
{
    h700_display_refresh_policy_v1_t policy;
    h700_display_refresh_state_v1_t state;
    const h700_display_refresh_timing_v1_t *timing;
    uint64_t period_ns;
    int rc;

    memset(&policy, 0, sizeof(policy));
    memset(&state, 0, sizeof(state));

    rc = h700_display_refresh_policy_init_v1(&policy, sizeof(policy));
    CHECK(rc == H700_DISPLAY_REFRESH_POLICY_ONLY,
          "initialization establishes policy without touching hardware");

    CHECK(sizeof(policy) == 48u && sizeof(state) == 32u,
          "GAME toggle consumes reserved ABI words without changing v1 sizes");

    rc = h700_display_refresh_policy_get_state_v1(
        &policy, &state, sizeof(state));
    CHECK(rc == H700_DISPLAY_REFRESH_STATUS_OK &&
          state.ui_preference_hz == H700_DISPLAY_REFRESH_120HZ &&
          state.scene == H700_DISPLAY_SCENE_UI &&
          state.requested_mode_hz == H700_DISPLAY_REFRESH_120HZ &&
          state.game_120hz_enabled == 0u &&
          state.hardware_applied == 0u,
          "default UI policy requests 120 Hz, GAME toggle is off, and no hardware application is claimed");

    timing = h700_display_refresh_get_rg34xx_timing_v1(
        H700_DISPLAY_REFRESH_120HZ);
    CHECK(timing != NULL &&
          timing->width == 720u && timing->height == 480u &&
          timing->lcd_dclk_freq_mhz == 54u &&
          timing->lcd_ht == 840u && timing->lcd_vt == 536u &&
          timing->refresh_millihz == 119947u,
          "120 Hz profile matches the RG34XX timing source");

    timing = h700_display_refresh_get_rg34xx_timing_v1(
        H700_DISPLAY_REFRESH_60HZ);
    CHECK(timing != NULL &&
          timing->lcd_dclk_freq_mhz == 54u &&
          timing->lcd_ht == 1679u && timing->lcd_vt == 536u &&
          timing->refresh_millihz == 60006u,
          "60 Hz profile matches the RG34XX timing source");

    period_ns = 0u;
    rc = h700_display_refresh_get_period_ns_v1(
        H700_DISPLAY_REFRESH_120HZ, &period_ns);
    CHECK(rc == H700_DISPLAY_REFRESH_STATUS_OK &&
          period_ns == 8337777ull,
          "120 Hz period is derived from pixel clock and totals");

    period_ns = 0u;
    rc = h700_display_refresh_get_period_ns_v1(
        H700_DISPLAY_REFRESH_60HZ, &period_ns);
    CHECK(rc == H700_DISPLAY_REFRESH_STATUS_OK &&
          period_ns == 16665629ull,
          "60 Hz period is derived from pixel clock and totals");

    CHECK(h700_display_refresh_get_period_ns_v1(
              H700_DISPLAY_REFRESH_120HZ, NULL) ==
              H700_DISPLAY_REFRESH_ERR_INVALID_ARGUMENT,
          "null period output is rejected");

    CHECK(h700_display_refresh_get_period_ns_v1(
              (enum h700_display_refresh_mode)90, &period_ns) ==
              H700_DISPLAY_REFRESH_ERR_INVALID_ARGUMENT,
          "period calculation rejects unsupported modes");

    rc = h700_display_refresh_policy_set_scene_v1(
        &policy, H700_DISPLAY_SCENE_GAME);
    CHECK(rc == H700_DISPLAY_REFRESH_POLICY_ONLY,
          "entering game changes the policy target only");
    rc = h700_display_refresh_policy_get_state_v1(
        &policy, &state, sizeof(state));
    CHECK(rc == H700_DISPLAY_REFRESH_STATUS_OK &&
          state.requested_mode_hz == H700_DISPLAY_REFRESH_60HZ &&
          state.game_120hz_enabled == 0u,
          "GAME defaults to 60 Hz while its optional 120 Hz toggle is off");

    rc = h700_display_refresh_policy_set_game_120hz_enabled_v1(
        &policy, 1u);
    CHECK(rc == H700_DISPLAY_REFRESH_POLICY_ONLY,
          "GAME 120 Hz can be opted in without touching hardware");
    rc = h700_display_refresh_policy_get_state_v1(
        &policy, &state, sizeof(state));
    CHECK(rc == H700_DISPLAY_REFRESH_STATUS_OK &&
          state.game_120hz_enabled == 1u &&
          state.requested_mode_hz == H700_DISPLAY_REFRESH_120HZ,
          "enabling GAME 120 Hz while in GAME requests 120 Hz");

    {
        uint32_t generation = state.generation;
        rc = h700_display_refresh_policy_set_game_120hz_enabled_v1(
            &policy, 1u);
        CHECK(rc == H700_DISPLAY_REFRESH_POLICY_ONLY,
              "setting the same GAME toggle value is accepted");
        rc = h700_display_refresh_policy_get_state_v1(
            &policy, &state, sizeof(state));
        CHECK(state.generation == generation,
              "setting an unchanged GAME toggle does not advance generation");
    }

    rc = h700_display_refresh_policy_set_ui_preference_v1(
        &policy, H700_DISPLAY_REFRESH_60HZ);
    CHECK(rc == H700_DISPLAY_REFRESH_POLICY_ONLY,
          "UI refresh preference can be changed while in game");
    rc = h700_display_refresh_policy_get_state_v1(
        &policy, &state, sizeof(state));
    CHECK(rc == H700_DISPLAY_REFRESH_STATUS_OK &&
          state.ui_preference_hz == H700_DISPLAY_REFRESH_60HZ &&
          state.game_120hz_enabled == 1u &&
          state.requested_mode_hz == H700_DISPLAY_REFRESH_120HZ,
          "changing UI preference during a game does not override GAME 120 Hz");

    rc = h700_display_refresh_policy_set_scene_v1(
        &policy, H700_DISPLAY_SCENE_UI);
    CHECK(rc == H700_DISPLAY_REFRESH_POLICY_ONLY,
          "returning to UI restores configured UI preference");
    rc = h700_display_refresh_policy_get_state_v1(
        &policy, &state, sizeof(state));
    CHECK(rc == H700_DISPLAY_REFRESH_STATUS_OK &&
          state.requested_mode_hz == H700_DISPLAY_REFRESH_60HZ &&
          state.game_120hz_enabled == 1u,
          "UI 60 Hz preference is restored while the GAME 120 Hz toggle remains independent");

    rc = h700_display_refresh_policy_set_ui_preference_v1(
        &policy, H700_DISPLAY_REFRESH_120HZ);
    CHECK(rc == H700_DISPLAY_REFRESH_POLICY_ONLY,
          "UI preference can be switched back to 120 Hz");
    rc = h700_display_refresh_policy_get_state_v1(
        &policy, &state, sizeof(state));
    CHECK(state.requested_mode_hz == H700_DISPLAY_REFRESH_120HZ,
          "UI 120 Hz preference takes effect in UI scene");

    rc = h700_display_refresh_policy_set_scene_v1(
        &policy, H700_DISPLAY_SCENE_GAME);
    CHECK(rc == H700_DISPLAY_REFRESH_POLICY_ONLY,
          "re-entering GAME restores the independent GAME preference");
    rc = h700_display_refresh_policy_get_state_v1(
        &policy, &state, sizeof(state));
    CHECK(state.requested_mode_hz == H700_DISPLAY_REFRESH_120HZ &&
          state.game_120hz_enabled == 1u,
          "GAME 120 Hz remains selected across UI/GAME scene transitions");

    rc = h700_display_refresh_policy_set_game_120hz_enabled_v1(
        &policy, 0u);
    CHECK(rc == H700_DISPLAY_REFRESH_POLICY_ONLY,
          "GAME 120 Hz can be switched off again");
    rc = h700_display_refresh_policy_get_state_v1(
        &policy, &state, sizeof(state));
    CHECK(state.requested_mode_hz == H700_DISPLAY_REFRESH_60HZ &&
          state.game_120hz_enabled == 0u,
          "disabling GAME 120 Hz returns the active game request to 60 Hz");

    rc = h700_display_refresh_policy_set_game_120hz_enabled_v1(
        &policy, 2u);
    CHECK(rc == H700_DISPLAY_REFRESH_ERR_INVALID_ARGUMENT,
          "GAME 120 Hz toggle rejects values other than 0 or 1");

    rc = h700_display_refresh_policy_set_scene_v1(
        &policy, H700_DISPLAY_SCENE_UI);
    CHECK(rc == H700_DISPLAY_REFRESH_POLICY_ONLY,
          "test returns to UI after exercising the GAME toggle");

    rc = h700_display_refresh_policy_set_ui_preference_v1(
        &policy, (enum h700_display_refresh_mode)90);
    CHECK(rc == H700_DISPLAY_REFRESH_ERR_INVALID_ARGUMENT,
          "unsupported refresh values are rejected");

    rc = h700_display_refresh_policy_set_scene_v1(
        &policy, (enum h700_display_scene)9);
    CHECK(rc == H700_DISPLAY_REFRESH_ERR_INVALID_ARGUMENT,
          "unsupported scene values are rejected");

    CHECK(h700_display_refresh_get_rg34xx_timing_v1(
              (enum h700_display_refresh_mode)90) == NULL,
          "no timing profile is returned for unsupported modes");

    CHECK(h700_display_refresh_policy_init_v1(
              &policy, (uint32_t)sizeof(policy) - 1u) ==
              H700_DISPLAY_REFRESH_ERR_ABI,
          "undersized policy storage is rejected");

    puts("PASS: RG34XX display policy has independent UI preference and opt-in GAME 120 Hz toggle (default off)");
    return EXIT_SUCCESS;
}
