#ifndef H700_DATA_ABORT_RECOVERY_H
#define H700_DATA_ABORT_RECOVERY_H

#include <stddef.h>
#include <stdint.h>

#define H700_SYS_CFG_VER_REG_ADDRESS UINT32_C(0x03000024)
#define H700_SYS_CFG_PROBE_ACTIVE_MAGIC UINT32_C(0x48375031)
#define H700_A32_DATA_ABORT_LR_OFFSET 8u
#define H700_A32_MODE_MASK UINT32_C(0x1f)
#define H700_A32_MODE_SVC UINT32_C(0x13)
#define H700_A32_CPSR_T UINT32_C(0x20)

#define H700_DATA_ABORT_RECOVERY_OK 0
#define H700_DATA_ABORT_RECOVERY_ABORTED 1
#define H700_DATA_ABORT_RECOVERY_EINVAL (-1)
#define H700_DATA_ABORT_RECOVERY_ENOTSUP (-2)

/* Layout-compatible mirror of the pinned GammaOS LK arm_fault_frame. */
typedef struct h700_arm_fault_frame_v1 {
    uint32_t spsr;
    uint32_t user_sp;
    uint32_t user_lr;
    uint32_t r[13];
    uint32_t pc; /* saved LR_abt, not the faulting instruction address */
} h700_arm_fault_frame_v1_t;

_Static_assert(offsetof(h700_arm_fault_frame_v1_t, spsr) == 0u, "LK SPSR offset changed");
_Static_assert(offsetof(h700_arm_fault_frame_v1_t, user_sp) == 4u, "LK user SP offset changed");
_Static_assert(offsetof(h700_arm_fault_frame_v1_t, user_lr) == 8u, "LK user LR offset changed");
_Static_assert(offsetof(h700_arm_fault_frame_v1_t, r) == 12u, "LK GPR offset changed");
_Static_assert(offsetof(h700_arm_fault_frame_v1_t, pc) == 64u, "LK LR_abt offset changed");
_Static_assert(sizeof(h700_arm_fault_frame_v1_t) == 68u, "LK fault-frame size changed");

extern volatile uint32_t h700_syscfg_probe_active;

int h700_data_abort_frame_match(
    h700_arm_fault_frame_v1_t *frame,
    uint32_t active_magic,
    uint32_t observed_dfar,
    uint32_t expected_fault_lr,
    uint32_t expected_dfar,
    uint32_t recovery_pc
);
int h700_data_abort_try_recover(void *frame);

/* Explicit dormant one-register diagnostic. No default startup path calls it. */
int h700_syscfg_ver_reg_low3(uint32_t *value);

#endif
