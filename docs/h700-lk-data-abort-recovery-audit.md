# H700 LK AArch32 Data Abort recovery audit

Date: 2026-11-26

Status: **RECOVERY NOT IMPLEMENTED — MMIO PROBES MUST REMAIN DISABLED.**

This review is pinned to GammaOS LK
`TheGammaSqueeze/lk-mt6785@9bf1419a49e0af6dba8b05916df382c9c9c6d0a8`.
It is a source audit only; no H700 register read or write was attempted.

## 1. Existing exception path

The pinned source has three relevant pieces:

- `arch/arm/exceptions.S::arm_data_abort` saves `r0-r12` and the abort-mode
  link register, reserves three words, stores SPSR plus user/system SP and LR,
  then branches to `arm_data_abort_handler`. It has no local exception-return
  sequence after the C handler.
- `arch/arm/faults.c::arm_data_abort_handler` calls
  `exception_die(frame, -8, "data abort, halting\\n")`.
- `exception_die()` marks the LK crash path, increments the critical-section
  count, adjusts the recorded frame PC for diagnostics, dumps fault state,
  calls `halt()`, and loops forever.

The frame contract in `arch/arm/include/arch/arm.h` is:

```c
struct arm_fault_frame {
    uint32_t spsr;
    uint32_t usp;
    uint32_t ulr;
    uint32_t r[13];
    uint32_t pc;
};
```

The `-8` passed by the fatal path is diagnostic PC adjustment; it is not a
recoverable skip/retry contract. A normal C wrapper around a volatile load
cannot catch this exception.

## 2. Consequence for SYS_CFG / PIO probes

Even though the H700 LK MMU table maps
`[0x03000000, 0x03100000)` as Device/Shared, a read at
`0x03000024` or `0x0300b010` can still fault because page-table coverage
does not establish BL31 security attribution, peripheral firewall permission,
bus reachability, or register implementation.

The current generic handler would halt LK on such a fault. Therefore:

- Do not add a bare `volatile` SYS_CFG or PIO read to
  `target_early_init()`, `target_init()`, the input scanner, or the timer path.
- Do not treat a C `setjmp`/`longjmp`-style wrapper as sufficient; it does
  not alter the exception vector's fatal control flow.
- Do not weaken exception handling globally or silently swallow unrelated
  Data Aborts.

## 3. Requirements before a contained one-word probe

A future diagnostic may proceed only after all of the following are designed
and independently reviewed:

1. **Narrow match:** only the exact probe load PC is recoverable; every other
   Data Abort retains the existing fatal path.
2. **Correct exception return:** provide an AArch32 assembly return path that
   restores the saved general registers and SPSR, and resumes at a known
   recovery label. Account explicitly for the abort LR offset and the
   execution state/instruction width; do not blindly add 4 or 8 to the PC.
3. **No hidden side effects:** the probe performs one aligned 32-bit read,
   has no retry loop, and emits only a status plus `VER_REG & 7`; it does not
   dump the full register or touch adjacent addresses.
4. **Safe result channel:** select a result channel that does not assume UART,
   framebuffer, heap, Linux, or a working filesystem before those services
   have been independently initialized.
5. **Regression coverage:** host tests exercise success/fault status semantics,
   while disassembly/source guards verify the exact load address, handler
   match window, exception-return instructions, and unchanged fatal fallback.
6. **Execution gate:** only after the source contract passes CI should a
   separate diagnostic BL33 execution be reviewed against the existing
   recovery/rollback plan.

A Linux-runtime VER_REG sample can settle the selector's later runtime value;
it cannot establish CPU reset/RVBAR state at the earlier BL31→BL33 handoff.
The pre-BL33 snapshot remains a separate requirement.

## 4. Current disposition

- No recoverable Data Abort path exists in the pinned generic LK handler.
- No probe is wired to startup or input scanning.
- The latest device-side `/dev/mem` attempt remains `EFAULT`; userspace
  retries are stopped.
- `hardware_applied=0`: no MMIO read/write, module load, boot-argument change,
  SD write, firmware flash, or diagnostic payload execution.

**Next source task:** implement and test a *probe-scoped* AArch32 Data Abort
recovery path without changing the fatal handling of unrelated exceptions.
Do not implement the register read until that recovery primitive and result
channel have been reviewed.
