#include "h700_rg34xx_input_scan.h"

#include <stdint.h>

#define H700_RG34XX_PIO_BASE ((uintptr_t)0x0300b000u)

_Static_assert(H700_RG34XX_PIO_PA_DATA_OFFSET == 0x10u,
               "sunxi PA data-register offset changed");
_Static_assert(H700_RG34XX_PIO_PE_DATA_OFFSET == (4u * 0x24u + 0x10u),
               "sunxi PE data-register offset changed");

int h700_rg34xx_input_scan_once_v1(
    const h700_rg34xx_input_scan_io_v1_t *io,
    h700_rg34xx_input_state_t *state,
    h700_input_queue_t *queue,
    size_t *events_pushed
) {
    uint32_t port_a_levels;
    uint32_t port_e_levels;

    if (events_pushed != NULL) {
        *events_pushed = 0u;
    }
    if (io == NULL || state == NULL || queue == NULL || events_pushed == NULL) {
        return H700_RG34XX_INPUT_SCAN_ERR_ARGUMENT;
    }
    if (io->abi_version != H700_RG34XX_INPUT_SCAN_ABI_VERSION ||
        io->struct_size < sizeof(*io) || io->read32 == NULL) {
        return H700_RG34XX_INPUT_SCAN_ERR_ABI;
    }

    /* These are the only two register accesses in the scanner: read-only DAT. */
    port_a_levels = io->read32(io->context, H700_RG34XX_PIO_PA_DATA_OFFSET);
    port_e_levels = io->read32(io->context, H700_RG34XX_PIO_PE_DATA_OFFSET);
    *events_pushed = h700_rg34xx_input_update(
        state, queue, port_a_levels, port_e_levels);
    return H700_RG34XX_INPUT_SCAN_OK;
}

static uint32_t h700_rg34xx_pio_read32(void *context, uint32_t offset) {
    (void)context;
    return *(volatile uint32_t *)(H700_RG34XX_PIO_BASE + (uintptr_t)offset);
}

int h700_rg34xx_input_scan_once(
    h700_rg34xx_input_state_t *state,
    h700_input_queue_t *queue,
    size_t *events_pushed
) {
    const h700_rg34xx_input_scan_io_v1_t io = {
        H700_RG34XX_INPUT_SCAN_ABI_VERSION,
        (uint32_t)sizeof(h700_rg34xx_input_scan_io_v1_t),
        NULL,
        h700_rg34xx_pio_read32
    };
    return h700_rg34xx_input_scan_once_v1(&io, state, queue, events_pushed);
}
