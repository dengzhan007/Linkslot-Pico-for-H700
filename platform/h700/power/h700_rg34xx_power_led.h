#ifndef H700_RG34XX_POWER_LED_H
#define H700_RG34XX_POWER_LED_H
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
#define H700_RG34XX_POWER_LED_ABI_VERSION 1u
#define H700_RG34XX_POWER_LED_OK 0
#define H700_RG34XX_POWER_LED_ERR_ARGUMENT (-1)
#define H700_RG34XX_POWER_LED_ERR_ABI (-2)
#define H700_RG34XX_POWER_LED_ERR_NOT_READY (-3)
#define H700_RG34XX_POWER_LED_ERR_PINMUX (-4)
#define H700_RG34XX_POWER_LED_ERR_READBACK (-5)
#define H700_RG34XX_POWER_LED_PIO_MUX1_OFFSET 0x124u
#define H700_RG34XX_POWER_LED_PIO_DATA_OFFSET 0x130u
#define H700_RG34XX_POWER_LED_PIN_MASK (1u << 12)
#define H700_RG34XX_POWER_LED_PINMUX_MASK (0xFu << 16)
#define H700_RG34XX_POWER_LED_PINMUX_GPIO_OUT (0x1u << 16)
typedef uint32_t (*h700_rg34xx_power_led_read32_fn)(void *context, uint32_t offset);
typedef void (*h700_rg34xx_power_led_write32_fn)(void *context, uint32_t offset, uint32_t value);
typedef struct h700_rg34xx_power_led_io_v1 {
    uint32_t abi_version;
    uint32_t struct_size;
    void *context;
    h700_rg34xx_power_led_read32_fn read32;
    h700_rg34xx_power_led_write32_fn write32;
} h700_rg34xx_power_led_io_v1_t;
typedef struct h700_rg34xx_power_led_service_v1 {
    uint32_t abi_version;
    uint32_t struct_size;
    uint32_t initialized;
    uint32_t game_active;
    h700_rg34xx_power_led_io_v1_t io;
} h700_rg34xx_power_led_service_v1_t;
int h700_rg34xx_power_led_init_v1(h700_rg34xx_power_led_service_v1_t *service, uint32_t service_size, const h700_rg34xx_power_led_io_v1_t *io);
/* game_active=1 turns the active-high PI12 LED off; 0 turns it on. */
int h700_rg34xx_power_led_set_game_active_v1(h700_rg34xx_power_led_service_v1_t *service, uint32_t game_active);
int h700_rg34xx_power_led_early_init(void);
int h700_rg34xx_power_led_is_ready(void);
int h700_rg34xx_power_led_set_game_active(uint32_t game_active);
#ifdef __cplusplus
}
#endif
#endif
