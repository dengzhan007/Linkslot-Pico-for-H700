#include "input_queue.h"

#include <stdio.h>
#include <stdlib.h>

#define CHECK(condition) do { \
    if (!(condition)) { \
        fprintf(stderr, "CHECK failed at %s:%d: %s\n", __FILE__, __LINE__, #condition); \
        return 1; \
    } \
} while (0)

int main(void) {
    h700_input_queue_t queue;
    h700_input_event_t output[H700_INPUT_QUEUE_CAPACITY];
    h700_input_queue_init(&queue);

    CHECK(h700_input_queue_count(&queue) == 0u);
    CHECK(h700_input_queue_dropped(&queue) == 0u);
    CHECK(h700_input_queue_drain(&queue, output, H700_INPUT_QUEUE_CAPACITY) == 0u);
    CHECK(h700_input_queue_drain(&queue, NULL, H700_INPUT_QUEUE_CAPACITY) == 0u);
    CHECK(h700_input_queue_drain(&queue, output, 0u) == 0u);
    CHECK(!h700_input_queue_push(&queue, 0u, H700_INPUT_PRESSED));
    CHECK(!h700_input_queue_push(&queue, H700_BTN_A, 2u));

    /* FIFO order and partial drains must preserve the unread tail. */
    CHECK(h700_input_queue_push(&queue, H700_BTN_A, H700_INPUT_PRESSED));
    CHECK(h700_input_queue_push(&queue, H700_BTN_A, H700_INPUT_RELEASED));
    CHECK(h700_input_queue_push(&queue, H700_BTN_MENU, H700_INPUT_PRESSED));
    CHECK(h700_input_queue_count(&queue) == 3u);
    CHECK(h700_input_queue_drain(&queue, output, 1u) == 1u);
    CHECK(output[0].button == H700_BTN_A && output[0].state == H700_INPUT_PRESSED);
    CHECK(h700_input_queue_count(&queue) == 2u);
    CHECK(h700_input_queue_drain(&queue, output, 2u) == 2u);
    CHECK(output[0].button == H700_BTN_A && output[0].state == H700_INPUT_RELEASED);
    CHECK(output[1].button == H700_BTN_MENU && output[1].state == H700_INPUT_PRESSED);
    CHECK(h700_input_queue_count(&queue) == 0u);

    /*
     * Fill past capacity. The policy drops oldest events and retains the
     * newest 64 logical edges so a later release is not hidden behind stale
     * button-down events.
     */
    for (uint32_t i = 0u; i < H700_INPUT_QUEUE_CAPACITY + 6u; ++i) {
        uint16_t button = (uint16_t)(H700_BTN_UP + (i % 19u));
        uint8_t state = (uint8_t)(i & 1u);
        CHECK(h700_input_queue_push(&queue, button, state));
    }
    CHECK(h700_input_queue_count(&queue) == H700_INPUT_QUEUE_CAPACITY);
    CHECK(h700_input_queue_dropped(&queue) == 6u);
    CHECK(h700_input_queue_drain(&queue, output, H700_INPUT_QUEUE_CAPACITY) ==
          H700_INPUT_QUEUE_CAPACITY);
    for (uint32_t i = 0u; i < H700_INPUT_QUEUE_CAPACITY; ++i) {
        uint32_t source = i + 6u;
        CHECK(output[i].button == (uint16_t)(H700_BTN_UP + (source % 19u)));
        CHECK(output[i].state == (uint8_t)(source & 1u));
        CHECK(output[i].reserved == 0u);
    }
    CHECK(h700_input_queue_count(&queue) == 0u);

    /* Reinitialization must clear both data and overflow diagnostics. */
    h700_input_queue_init(&queue);
    CHECK(h700_input_queue_count(&queue) == 0u);
    CHECK(h700_input_queue_dropped(&queue) == 0u);

    puts("H700 input queue host regression: PASS");
    return 0;
}
