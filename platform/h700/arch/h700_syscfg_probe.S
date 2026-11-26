.syntax unified
.arch armv7-a
.arm
.text
.align 2

/* Explicit dormant probe: one aligned LDR, masked result only. */
.global h700_syscfg_ver_reg_low3_asm
.type h700_syscfg_ver_reg_low3_asm, %function
h700_syscfg_ver_reg_low3_asm:
    push    {r4, lr}
    mov     r4, r0
    ldr     r1, =0x03000024
    ldr     r3, =h700_syscfg_probe_active
    ldr     r2, =0x48375031
    str     r2, [r3]
    dmb     sy

.global h700_syscfg_probe_fault_load
.type h700_syscfg_probe_fault_load, %notype
h700_syscfg_probe_fault_load:
    ldr     r0, [r1]
    and     r0, r0, #7
    str     r0, [r4]
    mov     r2, #0
    str     r2, [r3]
    mov     r0, #0
    pop     {r4, pc}

.global h700_syscfg_probe_recovery
.type h700_syscfg_probe_recovery, %notype
h700_syscfg_probe_recovery:
    mov     r2, #0
    str     r2, [r3]
    mov     r0, #1
    pop     {r4, pc}

.size h700_syscfg_ver_reg_low3_asm, . - h700_syscfg_ver_reg_low3_asm
.ltorg
