#include "h700_data_abort_recovery.h"

volatile uint32_t h700_syscfg_probe_active;

#if defined(__arm__) && !defined(__aarch64__)
extern const unsigned char h700_syscfg_probe_fault_load[];
extern const unsigned char h700_syscfg_probe_recovery[];
extern int h700_syscfg_ver_reg_low3_asm(uint32_t *value);
#endif

int h700_data_abort_dfsr_is_recoverable_read(uint32_t dfsr)
{
    uint32_t fault_status;

    /* This diagnostic is for LK's known short-descriptor AArch32 MMU path. */
    if ((dfsr & H700_DFSR_LPAE) != 0u)
        return 0;
    if ((dfsr & H700_DFSR_WNR) != 0u)
        return 0;

    /* DFSR.FS is split across bit 10 and bits [3:0]. */
    fault_status = ((dfsr >> 6) & 0x10u) | (dfsr & 0x0fu);

    switch (fault_status) {
    case 0x01u: /* alignment fault */
    case 0x03u: /* access flag, section */
    case 0x05u: /* translation, section */
    case 0x06u: /* access flag, page */
    case 0x07u: /* translation, page */
    case 0x08u: /* synchronous external abort, not translation walk */
    case 0x09u: /* domain, section */
    case 0x0bu: /* domain, page */
    case 0x0cu: /* synchronous external abort, translation walk L1 */
    case 0x0du: /* permission, section */
    case 0x0eu: /* synchronous external abort, translation walk L2 */
    case 0x0fu: /* permission, page */
    case 0x19u: /* synchronous parity error, memory access */
    case 0x1cu: /* synchronous parity error, translation walk L1 */
    case 0x1eu: /* synchronous parity error, translation walk L2 */
        return 1;
    default:
        /* In particular reject debug events and asynchronous aborts/parity. */
        return 0;
    }
}

int h700_data_abort_frame_match(
    h700_arm_fault_frame_v1_t *frame,
    uint32_t active_magic,
    uint32_t observed_dfsr,
    uint32_t observed_dfar,
    uint32_t expected_fault_lr,
    uint32_t expected_dfar,
    uint32_t recovery_pc
) {
    if (frame == (h700_arm_fault_frame_v1_t *)0)
        return 0;
    if (active_magic != H700_SYS_CFG_PROBE_ACTIVE_MAGIC)
        return 0;
    if (!h700_data_abort_dfsr_is_recoverable_read(observed_dfsr))
        return 0;
    if ((frame->spsr & H700_A32_MODE_MASK) != H700_A32_MODE_SVC)
        return 0;
    if ((frame->spsr & H700_A32_CPSR_T) != 0u)
        return 0;
    if (frame->pc != expected_fault_lr)
        return 0;
    if (observed_dfar != expected_dfar)
        return 0;
    if (recovery_pc == 0u)
        return 0;

    frame->pc = recovery_pc;
    return 1;
}

int h700_data_abort_try_recover(void *opaque_frame)
{
#if defined(__arm__) && !defined(__aarch64__)
    h700_arm_fault_frame_v1_t *frame = (h700_arm_fault_frame_v1_t *)opaque_frame;
    uint32_t dfsr;
    uint32_t dfar;
    uint32_t expected_fault_lr;
    uint32_t recovery_pc;
    int matched;

    if (frame == (h700_arm_fault_frame_v1_t *)0)
        return 0;
    if (h700_syscfg_probe_active != H700_SYS_CFG_PROBE_ACTIVE_MAGIC)
        return 0;

    __asm__ volatile(
        "mrc p15, 0, %0, c5, c0, 0\n"
        "mrc p15, 0, %1, c6, c0, 0"
        : "=r"(dfsr), "=r"(dfar) : : "memory");
    expected_fault_lr =
        (uint32_t)(uintptr_t)h700_syscfg_probe_fault_load +
        H700_A32_DATA_ABORT_LR_OFFSET;
    recovery_pc = (uint32_t)(uintptr_t)h700_syscfg_probe_recovery;

    matched = h700_data_abort_frame_match(
        frame,
        h700_syscfg_probe_active,
        dfsr,
        dfar,
        expected_fault_lr,
        H700_SYS_CFG_VER_REG_ADDRESS,
        recovery_pc
    );
    if (matched)
        h700_syscfg_probe_active = 0u;
    return matched;
#else
    (void)opaque_frame;
    return 0;
#endif
}

int h700_syscfg_ver_reg_low3(uint32_t *value)
{
    if (value == (uint32_t *)0)
        return H700_DATA_ABORT_RECOVERY_EINVAL;
    *value = 0u;
#if defined(__arm__) && !defined(__aarch64__)
    return h700_syscfg_ver_reg_low3_asm(value);
#else
    return H700_DATA_ABORT_RECOVERY_ENOTSUP;
#endif
}
