#ifndef H700_INPUT_QUEUE_H
#define H700_INPUT_QUEUE_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "input_service.h"

#define H700_INPUT_QUEUE_CAPACITY 64u

typedef struct h700_input_queue {
    h700_input_event_t events[H700_INPUT_QUEUE_CAPACITY];
    uint32_t head;
    uint32_t tail;
    uint32_t dropped_events;
} h700_input_queue_t;

/*
 * This is a bounded, single-context queue. The board input scanner pushes
 * events and the LK event loop drains them in the same execution context.
 * It is deliberately not ISR-safe or multi-producer safe; an interrupt-driven
 * producer must add the LK critical-section wrapper before using this queue.
 */
void h700_input_queue_init(h700_input_queue_t *queue);
bool h700_input_queue_push(h700_input_queue_t *queue, uint16_t button, uint8_t state);
size_t h700_input_queue_drain(void *context, h700_input_event_t *output, size_t capacity);
size_t h700_input_queue_count(const h700_input_queue_t *queue);
uint32_t h700_input_queue_dropped(const h700_input_queue_t *queue);

#endif /* H700_INPUT_QUEUE_H */
