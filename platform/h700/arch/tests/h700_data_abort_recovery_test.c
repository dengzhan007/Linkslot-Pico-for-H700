#include "h700_data_abort_recovery.h"

#include <assert.h>
#include <stdio.h>

#define DFSR_SYNC_EXTERNAL_READ UINT32_C(0x08)
#define DFSR_ASYNC_EXTERNAL_READ UINT32_C(0x406) /* FS[4:0]=0x16 */
#define DFSR_ASYNC_PARITY_READ UINT32_C(0x408) /* FS[4:0]=0x18 */
#define DFSR_SYNC_EXTERNAL_WRITE (DFSR_SYNC_EXTERNAL_READ | H700_DFSR_WNR)

static h700_arm_fault_frame_v1_t valid_frame(void)
{
    h700_arm_fault_frame_v1_t frame = {0};
    frame.spsr = H700_A32_MODE_SVC;
    frame.pc = 0x1008u; /* A32 synchronous Data Abort LR_abt = instruction + 8. */
    return frame;
}

static int match(h700_arm_fault_frame_v1_t *frame, uint32_t cookie,
                 uint32_t dfsr, uint32_t dfar, uint32_t saved_lr,
                 uint32_t recovery_pc)
{
    return h700_data_abort_frame_match(
        frame, cookie, dfsr, dfar, saved_lr,
        H700_SYS_CFG_VER_REG_ADDRESS, recovery_pc);
}

int main(void)
{
    h700_arm_fault_frame_v1_t frame;

    assert(h700_data_abort_dfsr_is_recoverable_read(DFSR_SYNC_EXTERNAL_READ) == 1);
    assert(h700_data_abort_dfsr_is_recoverable_read(0x0fu) == 1); /* short-descriptor permission page */
    assert(h700_data_abort_dfsr_is_recoverable_read(DFSR_ASYNC_EXTERNAL_READ) == 0);
    assert(h700_data_abort_dfsr_is_recoverable_read(DFSR_ASYNC_PARITY_READ) == 0);
    assert(h700_data_abort_dfsr_is_recoverable_read(DFSR_SYNC_EXTERNAL_WRITE) == 0);
    assert(h700_data_abort_dfsr_is_recoverable_read(0x02u) == 0); /* debug event */
    assert(h700_data_abort_dfsr_is_recoverable_read(DFSR_SYNC_EXTERNAL_READ | H700_DFSR_LPAE) == 0);

    frame = valid_frame();
    assert(match(&frame, H700_SYS_CFG_PROBE_ACTIVE_MAGIC, DFSR_SYNC_EXTERNAL_READ,
                 H700_SYS_CFG_VER_REG_ADDRESS, 0x1008u, 0x2000u) == 1);
    assert(frame.pc == 0x2000u);

    frame = valid_frame();
    assert(match(&frame, 0u, DFSR_SYNC_EXTERNAL_READ,
                 H700_SYS_CFG_VER_REG_ADDRESS, 0x1008u, 0x2000u) == 0);
    assert(frame.pc == 0x1008u);

    frame = valid_frame();
    assert(match(&frame, H700_SYS_CFG_PROBE_ACTIVE_MAGIC, DFSR_SYNC_EXTERNAL_READ,
                 0x03000028u, 0x1008u, 0x2000u) == 0);
    assert(frame.pc == 0x1008u);

    frame = valid_frame();
    assert(match(&frame, H700_SYS_CFG_PROBE_ACTIVE_MAGIC, DFSR_SYNC_EXTERNAL_READ,
                 H700_SYS_CFG_VER_REG_ADDRESS, 0x100cu, 0x2000u) == 0);
    assert(frame.pc == 0x1008u);

    frame = valid_frame();
    frame.spsr = 0x12u;
    assert(match(&frame, H700_SYS_CFG_PROBE_ACTIVE_MAGIC, DFSR_SYNC_EXTERNAL_READ,
                 H700_SYS_CFG_VER_REG_ADDRESS, 0x1008u, 0x2000u) == 0);

    frame = valid_frame();
    frame.spsr |= H700_A32_CPSR_T;
    assert(match(&frame, H700_SYS_CFG_PROBE_ACTIVE_MAGIC, DFSR_SYNC_EXTERNAL_READ,
                 H700_SYS_CFG_VER_REG_ADDRESS, 0x1008u, 0x2000u) == 0);

    frame = valid_frame();
    assert(match(&frame, H700_SYS_CFG_PROBE_ACTIVE_MAGIC, DFSR_ASYNC_EXTERNAL_READ,
                 H700_SYS_CFG_VER_REG_ADDRESS, 0x1008u, 0x2000u) == 0);
    assert(frame.pc == 0x1008u);

    frame = valid_frame();
    assert(match(&frame, H700_SYS_CFG_PROBE_ACTIVE_MAGIC, DFSR_SYNC_EXTERNAL_WRITE,
                 H700_SYS_CFG_VER_REG_ADDRESS, 0x1008u, 0x2000u) == 0);
    assert(frame.pc == 0x1008u);

    frame = valid_frame();
    assert(match(&frame, H700_SYS_CFG_PROBE_ACTIVE_MAGIC, DFSR_SYNC_EXTERNAL_READ,
                 H700_SYS_CFG_VER_REG_ADDRESS, 0x1008u, 0u) == 0);
    assert(frame.pc == 0x1008u);

    assert(match(NULL, H700_SYS_CFG_PROBE_ACTIVE_MAGIC, DFSR_SYNC_EXTERNAL_READ,
                 H700_SYS_CFG_VER_REG_ADDRESS, 0x1008u, 0x2000u) == 0);

    puts("H700 Data Abort matcher: exact PC + DFAR + synchronous read DFSR + cookie + SVC/ARM: PASS");
    return 0;
}
