/*
 * Compile-only AArch32 C++ probe.
 *
 * This checks freestanding ARM-state C++ code generation only. It does not
 * provide libc++, exception unwinding, RTTI, or C++ ABI runtime support.
 */
#if !defined(__arm__) || defined(__aarch64__)
#error "This source must be compiled for AArch32"
#endif

extern "C" __attribute__((used, noinline))
unsigned long h700_aarch32_cpp_compile_probe(unsigned long value) noexcept
{
    return value + 1UL;
}
