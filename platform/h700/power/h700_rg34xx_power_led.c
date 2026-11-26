#include "h700_rg34xx_power_led.h"
#include <stddef.h>
#include <stdint.h>

/* Physical PIO access is opt-in until the BL33 bus/handoff gates are qualified. */
#ifndef H700_RG34XX_PI12_EARLY_ACCESS_ENABLE
#define H700_RG34XX_PI12_EARLY_ACCESS_ENABLE 0
#endif

/*
 * RG34XX (non-SP): PIO@0x0300b000, Bank I index 8, stride 0x24.
 * PI12: MUX1 +0x124, nibble bits 19:16, mux 1 = GPIO output.
 * PI12: DATA +0x130, bit 12. PI11 (low-power LED) is never modified.
 * Preload the active-high "on" output latch before switching pinmux.
 * This driver does not touch CCU/APB1, PMIC, CPU clocks or UART clocks.
 */
#define PIO_BASE ((uintptr_t)0x0300b000u)
#define BANK_STRIDE 0x24u
#define PORT_I 8u
#define PI12_MUX1 (PORT_I * BANK_STRIDE + 0x04u)
#define PI12_DATA (PORT_I * BANK_STRIDE + 0x10u)
#define SERVICE_MAGIC 0x504C4544u

static int configure_output(const h700_rg34xx_power_led_io_v1_t *io)
{
    uint32_t value = io->read32(io->context, PI12_DATA);
    if ((value & H700_RG34XX_POWER_LED_PIN_MASK) == 0u) {
        io->write32(io->context, PI12_DATA, value | H700_RG34XX_POWER_LED_PIN_MASK);
        value = io->read32(io->context, PI12_DATA);
        if ((value & H700_RG34XX_POWER_LED_PIN_MASK) == 0u)
            return H700_RG34XX_POWER_LED_ERR_READBACK;
    }

    value = io->read32(io->context, PI12_MUX1);
    if ((value & H700_RG34XX_POWER_LED_PINMUX_MASK) != H700_RG34XX_POWER_LED_PINMUX_GPIO_OUT) {
        value = (value & ~H700_RG34XX_POWER_LED_PINMUX_MASK) | H700_RG34XX_POWER_LED_PINMUX_GPIO_OUT;
        io->write32(io->context, PI12_MUX1, value);
        value = io->read32(io->context, PI12_MUX1);
        if ((value & H700_RG34XX_POWER_LED_PINMUX_MASK) != H700_RG34XX_POWER_LED_PINMUX_GPIO_OUT)
            return H700_RG34XX_POWER_LED_ERR_READBACK;
    }
    return H700_RG34XX_POWER_LED_OK;
}

int h700_rg34xx_power_led_init_v1(h700_rg34xx_power_led_service_v1_t *service,
                                   uint32_t service_size,
                                   const h700_rg34xx_power_led_io_v1_t *io)
{
    int result;
    if (service == NULL || io == NULL || service_size < sizeof(*service))
        return H700_RG34XX_POWER_LED_ERR_ARGUMENT;
    if (io->abi_version != H700_RG34XX_POWER_LED_ABI_VERSION ||
        io->struct_size < sizeof(*io) || io->read32 == NULL || io->write32 == NULL)
        return H700_RG34XX_POWER_LED_ERR_ABI;

    service->abi_version = H700_RG34XX_POWER_LED_ABI_VERSION;
    service->struct_size = (uint32_t)sizeof(*service);
    service->initialized = 0u;
    service->game_active = 0u;
    service->io = *io;

    result = configure_output(&service->io);
    if (result != H700_RG34XX_POWER_LED_OK)
        return result;
    service->initialized = SERVICE_MAGIC;
    return H700_RG34XX_POWER_LED_OK;
}

int h700_rg34xx_power_led_set_game_active_v1(h700_rg34xx_power_led_service_v1_t *service,
                                             uint32_t game_active)
{
    uint32_t mux, data, desired, updated;
    if (service == NULL || game_active > 1u)
        return H700_RG34XX_POWER_LED_ERR_ARGUMENT;
    if (service->abi_version != H700_RG34XX_POWER_LED_ABI_VERSION ||
        service->struct_size < sizeof(*service) || service->initialized != SERVICE_MAGIC ||
        service->io.read32 == NULL || service->io.write32 == NULL)
        return H700_RG34XX_POWER_LED_ERR_NOT_READY;

    mux = service->io.read32(service->io.context, PI12_MUX1);
    if ((mux & H700_RG34XX_POWER_LED_PINMUX_MASK) != H700_RG34XX_POWER_LED_PINMUX_GPIO_OUT)
        return H700_RG34XX_POWER_LED_ERR_PINMUX;

    data = service->io.read32(service->io.context, PI12_DATA);
    desired = game_active != 0u ? 0u : H700_RG34XX_POWER_LED_PIN_MASK;
    updated = (data & ~H700_RG34XX_POWER_LED_PIN_MASK) | desired;
    if (updated != data) {
        service->io.write32(service->io.context, PI12_DATA, updated);
        data = service->io.read32(service->io.context, PI12_DATA);
        if ((data & H700_RG34XX_POWER_LED_PIN_MASK) != desired)
            return H700_RG34XX_POWER_LED_ERR_READBACK;
    }
    service->game_active = game_active;
    return H700_RG34XX_POWER_LED_OK;
}

#if H700_RG34XX_PI12_EARLY_ACCESS_ENABLE
static uint32_t mmio_read32(void *context, uint32_t offset)
{
    (void)context;
    return *(volatile uint32_t *)(PIO_BASE + (uintptr_t)offset);
}
static void mmio_write32(void *context, uint32_t offset, uint32_t value)
{
    (void)context;
    *(volatile uint32_t *)(PIO_BASE + (uintptr_t)offset) = value;
}

static h700_rg34xx_power_led_service_v1_t g_power_led_service;

#endif

int h700_rg34xx_power_led_early_init(void)
{
#if H700_RG34XX_PI12_EARLY_ACCESS_ENABLE
    const h700_rg34xx_power_led_io_v1_t io = {
        H700_RG34XX_POWER_LED_ABI_VERSION,
        (uint32_t)sizeof(h700_rg34xx_power_led_io_v1_t),
        NULL, mmio_read32, mmio_write32
    };
    if (g_power_led_service.initialized == SERVICE_MAGIC)
        return h700_rg34xx_power_led_set_game_active_v1(&g_power_led_service, 0u);
    return h700_rg34xx_power_led_init_v1(&g_power_led_service,
                                          (uint32_t)sizeof(g_power_led_service), &io);
#else
    return H700_RG34XX_POWER_LED_ERR_NOT_READY;
#endif
}
int h700_rg34xx_power_led_is_ready(void)
{
#if H700_RG34XX_PI12_EARLY_ACCESS_ENABLE
    return g_power_led_service.initialized == SERVICE_MAGIC;
#else
    return 0;
#endif
}
int h700_rg34xx_power_led_set_game_active(uint32_t game_active)
{
#if H700_RG34XX_PI12_EARLY_ACCESS_ENABLE
    int result;
    if (!h700_rg34xx_power_led_is_ready()) {
        result = h700_rg34xx_power_led_early_init();
        if (result != H700_RG34XX_POWER_LED_OK)
            return result;
    }
    return h700_rg34xx_power_led_set_game_active_v1(&g_power_led_service, game_active);
#else
    (void)game_active;
    return H700_RG34XX_POWER_LED_ERR_NOT_READY;
#endif
}
