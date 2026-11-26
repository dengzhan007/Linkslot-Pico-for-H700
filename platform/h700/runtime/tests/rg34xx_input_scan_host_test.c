#include <assert.h>
#include <stdio.h>
#include <string.h>

#include "h700_rg34xx_input_scan.h"

typedef struct mock_pio {
    uint32_t port_a;
    uint32_t port_e;
    uint32_t offsets[4];
    size_t read_count;
} mock_pio_t;

static uint32_t mock_read32(void *context, uint32_t offset) {
    mock_pio_t *mock = (mock_pio_t *)context;
    assert(mock != NULL);
    assert(mock->read_count < 4u);
    mock->offsets[mock->read_count++] = offset;
    if (offset == H700_RG34XX_PIO_PA_DATA_OFFSET) {
        return mock->port_a;
    }
    if (offset == H700_RG34XX_PIO_PE_DATA_OFFSET) {
        return mock->port_e;
    }
    assert(0 && "scanner attempted to read a non-DATA PIO register");
    return 0u;
}

static h700_rg34xx_input_scan_io_v1_t make_io(mock_pio_t *mock) {
    h700_rg34xx_input_scan_io_v1_t io = {
        H700_RG34XX_INPUT_SCAN_ABI_VERSION,
        (uint32_t)sizeof(h700_rg34xx_input_scan_io_v1_t),
        mock,
        mock_read32
    };
    return io;
}

static size_t drain(h700_input_queue_t *queue, h700_input_event_t *events,
                    size_t capacity) {
    return h700_input_queue_drain(queue, events, capacity);
}

static void test_offsets_and_idle_snapshot(void) {
    mock_pio_t mock;
    h700_rg34xx_input_scan_io_v1_t io;
    h700_rg34xx_input_state_t state;
    h700_input_queue_t queue;
    size_t pushed = 99u;

    memset(&mock, 0, sizeof(mock));
    mock.port_a = UINT32_MAX;
    mock.port_e = UINT32_MAX;
    io = make_io(&mock);
    h700_rg34xx_input_state_init(&state);
    h700_input_queue_init(&queue);

    assert(H700_RG34XX_PIO_DECLARED_SCAN_INTERVAL_MS == 20u);
    assert(H700_RG34XX_PIO_PA_DATA_OFFSET == 0x10u);
    assert(H700_RG34XX_PIO_PE_DATA_OFFSET == 0xA0u);
    assert(h700_rg34xx_input_scan_once_v1(&io, &state, &queue, &pushed) ==
           H700_RG34XX_INPUT_SCAN_OK);
    assert(pushed == 0u);
    assert(mock.read_count == 2u);
    assert(mock.offsets[0] == H700_RG34XX_PIO_PA_DATA_OFFSET);
    assert(mock.offsets[1] == H700_RG34XX_PIO_PE_DATA_OFFSET);
    assert(h700_input_queue_count(&queue) == 0u);
}

static void test_physical_xy_edge_mapping(void) {
    mock_pio_t mock;
    h700_rg34xx_input_scan_io_v1_t io;
    h700_rg34xx_input_state_t state;
    h700_input_queue_t queue;
    h700_input_event_t events[4];
    size_t pushed = 0u;

    memset(&mock, 0, sizeof(mock));
    mock.port_a = UINT32_MAX & ~((uint32_t)1u << 2); /* Physical X: PA2 / code 307. */
    mock.port_e = UINT32_MAX;
    io = make_io(&mock);
    h700_rg34xx_input_state_init(&state);
    h700_input_queue_init(&queue);

    assert(h700_rg34xx_input_scan_once_v1(&io, &state, &queue, &pushed) ==
           H700_RG34XX_INPUT_SCAN_OK);
    assert(pushed == 1u);
    assert(drain(&queue, events, 4u) == 1u);
    assert(events[0].button == H700_BTN_X);
    assert(events[0].state == H700_INPUT_PRESSED);

    /* Release X and press physical Y: PA3 / code 306. */
    mock.port_a = (UINT32_MAX | ((uint32_t)1u << 2)) &
                  ~((uint32_t)1u << 3);
    mock.read_count = 0u;
    assert(h700_rg34xx_input_scan_once_v1(&io, &state, &queue, &pushed) ==
           H700_RG34XX_INPUT_SCAN_OK);
    assert(pushed == 2u);
    assert(drain(&queue, events, 4u) == 2u);
    assert(events[0].button == H700_BTN_X);
    assert(events[0].state == H700_INPUT_RELEASED);
    assert(events[1].button == H700_BTN_Y);
    assert(events[1].state == H700_INPUT_PRESSED);
}

static void test_bad_descriptor_is_rejected_without_reads(void) {
    mock_pio_t mock;
    h700_rg34xx_input_scan_io_v1_t io;
    h700_rg34xx_input_state_t state;
    h700_input_queue_t queue;
    size_t pushed = 0u;

    memset(&mock, 0, sizeof(mock));
    io = make_io(&mock);
    h700_rg34xx_input_state_init(&state);
    h700_input_queue_init(&queue);

    io.abi_version++;
    assert(h700_rg34xx_input_scan_once_v1(&io, &state, &queue, &pushed) ==
           H700_RG34XX_INPUT_SCAN_ERR_ABI);
    assert(mock.read_count == 0u);

    io = make_io(&mock);
    io.read32 = NULL;
    assert(h700_rg34xx_input_scan_once_v1(&io, &state, &queue, &pushed) ==
           H700_RG34XX_INPUT_SCAN_ERR_ABI);
    assert(mock.read_count == 0u);

    assert(h700_rg34xx_input_scan_once_v1(NULL, &state, &queue, &pushed) ==
           H700_RG34XX_INPUT_SCAN_ERR_ARGUMENT);
    assert(h700_rg34xx_input_scan_once_v1(&io, NULL, &queue, &pushed) ==
           H700_RG34XX_INPUT_SCAN_ERR_ARGUMENT);
    assert(h700_rg34xx_input_scan_once_v1(&io, &state, NULL, &pushed) ==
           H700_RG34XX_INPUT_SCAN_ERR_ARGUMENT);
    assert(h700_rg34xx_input_scan_once_v1(&io, &state, &queue, NULL) ==
           H700_RG34XX_INPUT_SCAN_ERR_ARGUMENT);
    assert(mock.read_count == 0u);
}

int main(void) {
    test_offsets_and_idle_snapshot();
    test_physical_xy_edge_mapping();
    test_bad_descriptor_is_rejected_without_reads();
    puts("H700 RG34XX PIO input scan host tests passed.");
    return 0;
}
