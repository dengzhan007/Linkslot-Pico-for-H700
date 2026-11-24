#include "input_queue.h"

#include <limits.h>
#include <string.h>

_Static_assert((H700_INPUT_QUEUE_CAPACITY & (H700_INPUT_QUEUE_CAPACITY - 1u)) == 0u,
               "input queue capacity must be a power of two");
_Static_assert(sizeof(h700_input_event_t) == 4u, "input queue event ABI must stay 4 bytes");

void h700_input_queue_init(h700_input_queue_t *queue) {
    if (queue == NULL) {
        return;
    }
    memset(queue, 0, sizeof(*queue));
}

bool h700_input_queue_push(h700_input_queue_t *queue, uint16_t button, uint8_t state) {
    if (queue == NULL ||
        button < H700_BTN_UP || button > H700_BTN_LID ||
        (state != H700_INPUT_PRESSED && state != H700_INPUT_RELEASED)) {
        return false;
    }

    /*
     * Prefer the newest edge on overflow. In particular, preserving a newer
     * release edge is safer than retaining an old press that could leave a
     * logical button stuck. Overflow remains observable via dropped_events.
     */
    if ((uint32_t)(queue->head - queue->tail) >= H700_INPUT_QUEUE_CAPACITY) {
        queue->tail++;
        if (queue->dropped_events != UINT32_MAX) {
            queue->dropped_events++;
        }
    }

    h700_input_event_t *event =
        &queue->events[queue->head & (H700_INPUT_QUEUE_CAPACITY - 1u)];
    event->button = button;
    event->state = state;
    event->reserved = 0u;
    queue->head++;
    return true;
}

size_t h700_input_queue_drain(void *context, h700_input_event_t *output, size_t capacity) {
    h700_input_queue_t *queue = (h700_input_queue_t *)context;
    if (queue == NULL || output == NULL || capacity == 0u) {
        return 0u;
    }

    size_t available = h700_input_queue_count(queue);
    size_t count = available < capacity ? available : capacity;
    for (size_t i = 0u; i < count; ++i) {
        output[i] = queue->events[queue->tail & (H700_INPUT_QUEUE_CAPACITY - 1u)];
        queue->tail++;
    }
    return count;
}

size_t h700_input_queue_count(const h700_input_queue_t *queue) {
    if (queue == NULL) {
        return 0u;
    }
    uint32_t count = queue->head - queue->tail;
    return count > H700_INPUT_QUEUE_CAPACITY ? H700_INPUT_QUEUE_CAPACITY : (size_t)count;
}

uint32_t h700_input_queue_dropped(const h700_input_queue_t *queue) {
    return queue == NULL ? 0u : queue->dropped_events;
}
