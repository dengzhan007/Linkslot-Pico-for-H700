#ifndef H700_RG34XX_INPUT_SCAN_H
#define H700_RG34XX_INPUT_SCAN_H

#include <stddef.h>
#include <stdint.h>

#include "h700_rg34xx_input_map.h"
#include "input_queue.h"

#define H700_RG34XX_INPUT_SCAN_ABI_VERSION 1u
#define H700_RG34XX_INPUT_SCAN_OK 0
#define H700_RG34XX_INPUT_SCAN_ERR_ARGUMENT (-1)
#define H700_RG34XX_INPUT_SCAN_ERR_ABI (-2)

/* Derived from sunxi_data_reg(): bank stride 0x24, DATA offset 0x10. */
#define H700_RG34XX_PIO_PA_DATA_OFFSET 0x010u
#define H700_RG34XX_PIO_PE_DATA_OFFSET 0x0A0u
#define H700_RG34XX_PIO_DECLARED_SCAN_INTERVAL_MS 20u

/*
 * The interface deliberately exposes read32 only: the input sampler has
 * no write callback and cannot configure pinmux, pull, drive or EINT state.
 * Offsets are relative to PIO base 0x0300b000.
 */
typedef uint32_t (*h700_rg34xx_input_read32_fn)(void *context, uint32_t offset);

typedef struct h700_rg34xx_input_scan_io_v1 {
    uint32_t abi_version;
    uint32_t struct_size;
    void *context;
    h700_rg34xx_input_read32_fn read32;
} h700_rg34xx_input_scan_io_v1_t;

/*
 * Read PA_DAT and PE_DAT exactly once each, then convert the two snapshots
 * into logical press/release edges through h700_rg34xx_input_update().
 * This function performs no writes, waits, debounce, timer setup or IRQ work.
 * The caller owns cadence; the live FDT declares a 20 ms polling interval.
 */
int h700_rg34xx_input_scan_once_v1(
    const h700_rg34xx_input_scan_io_v1_t *io,
    h700_rg34xx_input_state_t *state,
    h700_input_queue_t *queue,
    size_t *events_pushed
);

/* Direct H700 PIO implementation. Not called from target_init or LK main. */
int h700_rg34xx_input_scan_once(
    h700_rg34xx_input_state_t *state,
    h700_input_queue_t *queue,
    size_t *events_pushed
);

#endif /* H700_RG34XX_INPUT_SCAN_H */
