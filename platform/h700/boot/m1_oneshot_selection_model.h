#ifndef H700_M1_ONESHOT_SELECTION_MODEL_H
#define H700_M1_ONESHOT_SELECTION_MODEL_H

#include <stddef.h>
#include <stdint.h>

/*
 * Host-only policy model for the pre-entry image selection of a future M1
 * Boot0 capsule. This file is NOT included by LK, SPL, or capsule build rules.
 * It performs no MMIO, RTC/watchdog access, block I/O, or image copying.
 */
#define H700_M1_STOCK_BOOT0_BYTES 65536u
#define H700_M1_SPL_IMAGE_BYTES   49152u

struct h700_m1_capsule_image {
    const uint8_t *bytes;
    size_t size;
    uint32_t expected_fnv1a;
};

enum h700_m1_capsule_choice {
    H700_M1_CAPSULE_INVALID = 0,
    H700_M1_CAPSULE_STOCK_BOOT0 = 1,
    H700_M1_CAPSULE_SPL = 2
};

/* FNV-1a is used here only to model the current BaseOS capsule contract. */
uint32_t h700_m1_capsule_fnv1a(const uint8_t *bytes, size_t size);

/*
 * The stock Boot0 must always validate before either choice is allowed.
 * A valid M1 SPL is selected; an invalid M1 SPL falls back to the valid
 * embedded stock Boot0. Invalid stock Boot0 means fail-stop.
 */
enum h700_m1_capsule_choice h700_m1_capsule_select(
    const struct h700_m1_capsule_image *stock,
    const struct h700_m1_capsule_image *m1_spl);

#endif
