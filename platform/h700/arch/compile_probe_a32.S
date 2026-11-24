/*
 * Compile-only AArch32 assembly probe.
 *
 * Not a BL33 entry point. Do not jump here from the vendor monitor.
 */
.syntax unified
.cpu cortex-a53
.arm
.text
.align 2
.global h700_aarch32_asm_probe
.type h700_aarch32_asm_probe, %function
h700_aarch32_asm_probe:
    mov     r0, #0
    bx      lr
.size h700_aarch32_asm_probe, .-h700_aarch32_asm_probe
