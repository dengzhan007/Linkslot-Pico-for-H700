#ifndef H700_INPUT_SERVICE_H
#define H700_INPUT_SERVICE_H

#include <stddef.h>
#include <stdint.h>

#define H700_INPUT_ABI_VERSION 1u
#define H700_INPUT_MAX_EVENTS 32u

enum h700_input_button {
    H700_BTN_UP = 1,
    H700_BTN_DOWN = 2,
    H700_BTN_LEFT = 3,
    H700_BTN_RIGHT = 4,
    H700_BTN_A = 5,
    H700_BTN_B = 6,
    H700_BTN_X = 7,
    H700_BTN_Y = 8,
    H700_BTN_L1 = 9,
    H700_BTN_R1 = 10,
    H700_BTN_L2 = 11,
    H700_BTN_R2 = 12,
    H700_BTN_START = 13,
    H700_BTN_SELECT = 14,
    H700_BTN_MENU = 15,
    H700_BTN_VOL_UP = 16,
    H700_BTN_VOL_DOWN = 17,
    H700_BTN_POWER = 18,
    H700_BTN_LID = 19
};

enum h700_input_state {
    H700_INPUT_RELEASED = 0,
    H700_INPUT_PRESSED = 1
};

typedef struct h700_input_event {
    uint16_t button;
    uint8_t state;
    uint8_t reserved;
} h700_input_event_t;

typedef size_t (*h700_input_drain_fn)(
    void *context,
    h700_input_event_t *output,
    size_t capacity
);

typedef struct h700_input_service_v1 {
    uint32_t abi_version;
    uint32_t struct_size;
    void *context;
    h700_input_drain_fn drain;
} h700_input_service_v1_t;

#endif /* H700_INPUT_SERVICE_H */
