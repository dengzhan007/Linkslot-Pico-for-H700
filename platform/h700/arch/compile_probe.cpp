/*
 * Compile-only C++ probe for the H700 target.
 *
 * This checks freestanding C++ code generation only. It does not provide
 * libc++, exception unwinding, RTTI, or C++ ABI runtime support.
 */
#if !defined(__aarch64__)
#error "This source must be compiled for AArch64"
#endif

extern "C" __attribute__((used, noinline))
unsigned long h700_cpp_compile_probe(unsigned long value) noexcept
{
    return value + 1UL;
}
