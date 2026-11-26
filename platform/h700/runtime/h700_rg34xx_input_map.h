#ifndef H700_RG34XX_INPUT_MAP_H
#define H700_RG34XX_INPUT_MAP_H
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include "input_service.h"
#include "input_queue.h"
#define H700_RG34XX_INPUT_MAP_COUNT 17u
#define H700_RG34XX_INPUT_DECLARED_POLL_INTERVAL_MS 20u
enum h700_rg34xx_gpio_port { H700_RG34XX_GPIO_PORT_A=0u, H700_RG34XX_GPIO_PORT_E=4u };
enum h700_rg34xx_input_event_type { H700_RG34XX_EV_KEY=1u, H700_RG34XX_EV_ABS=3u };
/* Data-only profile copied from the target's active live FDT; no GPIO hardware access. */
typedef struct h700_rg34xx_input_map_entry {
 uint8_t port,pin,mux_select,pull_config,drive_level,active_low,input_type,reserved;
 uint16_t linux_code;
 int16_t input_value;
 uint16_t logical_button;
} h700_rg34xx_input_map_entry_t;
extern const h700_rg34xx_input_map_entry_t h700_rg34xx_input_map[H700_RG34XX_INPUT_MAP_COUNT];
typedef struct h700_rg34xx_input_state { uint32_t previous_pressed_mask; bool initialized; } h700_rg34xx_input_state_t;
void h700_rg34xx_input_state_init(h700_rg34xx_input_state_t *state);
/*
 * Convert caller-supplied PIO bank A/E level snapshots into logical edges.
 * Bit 1 means high. All mapped pins are active-low. First sample reports
 * currently-held keys. No MMIO, pin configuration, GPIO request, debounce,
 * scheduling or IRQ work is performed here; the caller owns those duties.
 */
size_t h700_rg34xx_input_update(h700_rg34xx_input_state_t *state,
 h700_input_queue_t *queue,uint32_t port_a_levels,uint32_t port_e_levels);
#endif
