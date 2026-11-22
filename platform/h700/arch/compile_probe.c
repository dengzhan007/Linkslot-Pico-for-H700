/*
 * Compile-only AArch64 probe for the H700 target.
 *
 * This file deliberately performs no MMIO and makes no assumptions about
 * U-Boot's entry ABI, stack pointer, load address, or board peripherals.
 * It exists to verify the freestanding compiler target before board bring-up.
 */
#if !defined(__aarch64__)
#error "This source must be compiled for AArch64"
#endif

typedef unsigned long h700_u64;

_Static_assert(sizeof(h700_u64) == 8, "H700 target must use the AArch64 LP64 data model");

__attribute__((used, noinline))
h700_u64 h700_arch_compile_probe(h700_u64 value)
{
    return value ^ 0x3566UL;
}
