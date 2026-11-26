#include "h700_data_abort_recovery.h"

#include <assert.h>
#include <stdio.h>

static h700_arm_fault_frame_v1_t valid_frame(void)
{
    h700_arm_fault_frame_v1_t frame = {0};
    frame.spsr = H700_A32_MODE_SVC;
    frame.pc = 0x1008u; /* A32 Data Abort LR_abt = fault instruction + 8. */
    return frame;
}

int main(void)
{
    h700_arm_fault_frame_v1_t frame;

    frame = valid_frame();
    assert(h700_data_abort_frame_match(
        &frame, H700_SYS_CFG_PROBE_ACTIVE_MAGIC,
        H700_SYS_CFG_VER_REG_ADDRESS, 0x1008u,
        H700_SYS_CFG_VER_REG_ADDRESS, 0x2000u) == 1);
    assert(frame.pc == 0x2000u);

    frame = valid_frame();
    assert(h700_data_abort_frame_match(
        &frame, 0u, H700_SYS_CFG_VER_REG_ADDRESS, 0x1008u,
        H700_SYS_CFG_VER_REG_ADDRESS, 0x2000u) == 0);
    assert(frame.pc == 0x1008u);

    frame = valid_frame();
    assert(h700_data_abort_frame_match(
        &frame, H700_SYS_CFG_PROBE_ACTIVE_MAGIC, 0x03000028u, 0x1008u,
        H700_SYS_CFG_VER_REG_ADDRESS, 0x2000u) == 0);
    assert(frame.pc == 0x1008u);

    frame = valid_frame();
    assert(h700_data_abort_frame_match(
        &frame, H700_SYS_CFG_PROBE_ACTIVE_MAGIC, H700_SYS_CFG_VER_REG_ADDRESS,
        0x100cu, H700_SYS_CFG_VER_REG_ADDRESS, 0x2000u) == 0);
    assert(frame.pc == 0x1008u);

    frame = valid_frame();
    frame.spsr = 0x12u;
    assert(h700_data_abort_frame_match(
        &frame, H700_SYS_CFG_PROBE_ACTIVE_MAGIC, H700_SYS_CFG_VER_REG_ADDRESS,
        0x1008u, H700_SYS_CFG_VER_REG_ADDRESS, 0x2000u) == 0);

    frame = valid_frame();
    frame.spsr |= H700_A32_CPSR_T;
    assert(h700_data_abort_frame_match(
        &frame, H700_SYS_CFG_PROBE_ACTIVE_MAGIC, H700_SYS_CFG_VER_REG_ADDRESS,
        0x1008u, H700_SYS_CFG_VER_REG_ADDRESS, 0x2000u) == 0);

    frame = valid_frame();
    assert(h700_data_abort_frame_match(
        &frame, H700_SYS_CFG_PROBE_ACTIVE_MAGIC, H700_SYS_CFG_VER_REG_ADDRESS,
        0x1008u, H700_SYS_CFG_VER_REG_ADDRESS, 0u) == 0);

    assert(h700_data_abort_frame_match(
        NULL, H700_SYS_CFG_PROBE_ACTIVE_MAGIC, H700_SYS_CFG_VER_REG_ADDRESS,
        0x1008u, H700_SYS_CFG_VER_REG_ADDRESS, 0x2000u) == 0);

    puts("H700 Data Abort matcher: exact PC + DFAR + cookie + SVC/ARM only: PASS");
    return 0;
}
