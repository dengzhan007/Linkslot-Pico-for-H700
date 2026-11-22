/*
 * Compile-only AArch64 assembly probe.
 *
 * Not a boot entry point. Do not jump here from U-Boot.
 */
.arch armv8-a
.text
.p2align 2
.global h700_aarch64_asm_probe
.type h700_aarch64_asm_probe, %function
h700_aarch64_asm_probe:
    mov     x0, xzr
    ret
.size h700_aarch64_asm_probe, .-h700_aarch64_asm_probe
