# H700 newlib runtime / LK adapter

This document defines the current AArch32 mGBA runtime boundary. It is a host/CI integration candidate, not a complete H700 HAL and not an approved boot payload.

## Baseline

The pinned LK reference is:

- Repository: TheGammaSqueeze/lk-mt6785
- Commit: 9bf1419a49e0af6dba8b05916df382c9c9c6d0a8

Relevant source facts from that exact revision:

- include/lib/heap.h exports heap_alloc(size_t, unsigned int alignment), heap_realloc(void *, size_t) and heap_free(void *).
- kernel/main.c::kmain() calls heap_init() before thread_init(), then creates bootstrap and I/O threads.
- lib/heap/heap.c::heap_alloc() and heap_free() protect the global free-list with enter_critical_section()/exit_critical_section().
- include/kernel/thread.h implements those functions using one global critical_section_count and local arch_disable_ints()/arch_enable_ints(). This is a local interrupt critical section, not a cross-core lock.
- heap_realloc() reallocates using heap_alloc(size, 0). It therefore does not preserve a caller's 16-byte allocation alignment. It also forms an allocation-header pointer before checking a NULL input, so the adapter must special-case realloc(NULL, n).
- include/kernel/timer.h schedules timer callbacks in milliseconds; kernel/timer.c obtains current_time() for monotonic scheduling. This is not Unix wall-clock time.
- include/dev/uart.h declares UART hooks, but the pinned platform/mt6785/uart.c is MTK-specific and must not be used as an H700 UART driver. MTK _dputc/dgetc are implemented by platform/mt6785/debug.c; an H700 platform implementation is still required.

The LK source is a useful API/reference baseline, not proof that its MT6785 platform build boots on RG34XX H700.

## Allocator contract

platform/h700/runtime/newlib_syscalls.c overrides newlib's malloc/calloc/realloc/free and routes allocations through injected LK heap services.

The adapter does not call heap_realloc(). It places a 16-byte-aligned private header in each heap_alloc(size, 16) block, records the user allocation length, and implements realloc() as aligned allocate + bounded copy + free. The header size is rounded to a multiple of 16 so the returned user pointer remains 16-byte aligned on AArch32.

The adapter rejects allocations unless both callbacks report true:

1. heap_ready() — the platform has called heap_init() and the heap is initialized.
2. single_cpu_active() — all other CPUs are parked and cannot access the LK heap.

This is a hard requirement, not an optimization. LK's current global free-list and local IRQ masking are not SMP-safe. If the second condition is false, malloc/calloc/realloc fail with ENOMEM; freeing a live allocation while the condition is false enters the fatal/panic path.

The platform must install the service table exactly once during single-threaded bootstrap, before any Linkslot/mGBA code can allocate. It must keep the one-active-CPU invariant for the lifetime of the LK heap until a proven cross-core heap lock is implemented.

No heap base address, size or linker range is assumed in this adapter. The H700 RAM ownership and boot-handoff audit remains NO-GO; memory bounds must come from a separately verified linker/startup layout.

## Newlib syscall semantics

| Hook | Current adapter behavior | Remaining platform work |
|---|---|---|
| _sbrk | Always returns ENOMEM; no second contiguous heap arena is reserved. | None while all malloc-family calls use LK heap. Whole-archive link must verify no hidden allocator path requires sbrk. |
| _write | fd 1/2 only; forwards to injected console writer; other descriptors return EBADF. | Implement H700 debug/console output. |
| _read | fd 0 only; forwards to injected console reader; missing input service returns ENOSYS. | Implement H700 console input or deliberately leave unavailable. |
| _fstat | fd 0/1/2 reported as character devices. | No general file descriptors; file and directory access stays in Libretro VFS v3. |
| _isatty | fd 0/1/2 return true. | No additional service. |
| _close | fd 0/1/2 are persistent aliases and close is a no-op; others return EBADF. | Keep the standard streams persistent. |
| _lseek | Standard streams return ESPIPE; unknown descriptors return EBADF. | No seekable POSIX descriptors. |
| _gettimeofday | Calls injected RTC callback; without one returns ENOSYS. | Implement a real H700 RTC/epoch-time service and define UTC semantics. Do not substitute monotonic current_time(). |
| _getpid | Returns constant 1 as the single bare-metal runtime token. | No process isolation is provided. |
| _kill | Returns ENOSYS for pid 1 and ESRCH for other pids. | No Unix signal/process model. |
| _exit | Calls injected fatal callback and then stops forever if it returns. | Bind fatal callback to the LK panic/halt path. |

The console and RTC services are dependency-injected on purpose. This branch does not add speculative H700 UART/RTC register access and does not copy the MTK UART driver.

## Validation

The adapter test script is:

- scripts/test-h700-newlib-adapter.py
- Host harness: platform/h700/runtime/tests/newlib_syscalls_host.c

The test is intended to cover:
- allocator fail-closed behavior before heap init and while secondary CPUs are not parked;
- 16-byte alignment for malloc and after realloc;
- calloc zeroing and multiplication overflow;
- realloc(NULL, n) and allocation-failure preservation;
- heap fatal path if the single-core invariant is lost;
- console descriptor semantics, console I/O error mapping;
- character-device fstat, isatty, non-seekable standard streams;
- RTC success/failure, pid/signal and _sbrk behavior.

The same script compiles the production adapter as ELF32/ARM EABI5 against the actual arm-none-eabi/newlib toolchain. The CI whole-archive link then includes that adapter object and checks whether mGBA closes against newlib/libm/libgcc without libnosys.

A host test or successful link does not prove the real LK heap, CPU rendezvous, H700 console, RTC or boot handoff works.

## Still blocked

- No validated H700 secondary-core parking/PSCI startup implementation.
- No H700 LK heap_ready() / single_cpu_active() platform callbacks.
- No H700 UART/debug console provider.
- No H700 RTC wall-clock provider.
- No final LK firmware or H700 memory/linker layout.
- No SD/FAT service, ROM runtime, device test or payload execution.

These are separate bring-up tasks and must not be replaced by libnosys dummy syscalls.
