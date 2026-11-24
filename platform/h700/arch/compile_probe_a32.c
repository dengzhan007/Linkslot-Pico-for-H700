/*
 * Compile-only AArch32 probe for the H700 target.
 *
 * This checks the 32-bit ARM data model only. It is not a BL33 entry point and
 * does not assume a stack, load address, exception level, or register ABI.
 */
#if !defined(__arm__) || defined(__aarch64__)
#error "This source must be compiled for AArch32"
#endif

typedef unsigned long h700_u32;

_Static_assert(sizeof(h700_u32) == 4, "H700 AArch32 target must use ILP32");

__attribute__((used, noinline))
h700_u32 h700_aarch32_compile_probe(h700_u32 value)
{
    return value ^ 0x3566UL;
}
