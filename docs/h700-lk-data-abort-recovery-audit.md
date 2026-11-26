# H700 LK AArch32 Data Abort recovery audit

Date: 2026-11-26

Status: **PROBE-SCOPED SOURCE RECOVERY + HOST/A32 ELF LINK VALIDATED; NOT DEVICE-EXECUTED.**

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

## 3. Implemented probe-scoped source path (2026-11-26)

The H700 overlay now modifies only the pinned LK Data Abort vector and its
handler. The default handler remains fatal; the one explicit SYS_CFG
diagnostic can recover only when every match condition below holds.

### 3.1 Exception-frame and call-stack contract

The LK frame remains the pinned 68-byte layout:

| Offset | Field |
|---:|---|
| 0 | SPSR_abt |
| 4 | User/System SP |
| 8 | User/System LR |
| 12–60 | r0–r12 |
| 64 | LR_abt |

The patched vector keeps that frame untouched, records its base in
callee-saved r4, rounds the Abort-mode C-call stack down to an 8-byte
boundary, then passes the original frame pointer in r0. The modified C
handler preserves r4 by the AAPCS callee-saved rule. On a positive match,
the assembly trampoline restores the original frame SP from r4, restores
SPSR_abt, skips the three-word prefix, and performs
LDMIA sp!, {r0-r12, pc}^ . Because PC is in the list and the S bit is set,
this is the AArch32 exception return that restores the saved CPSR from SPSR.

This avoids assuming that the original abort stack top was 8-byte aligned:
the vector aligns only the temporary C-call SP, while r4 retains the exact
frame address to be restored.

### 3.2 Exact recovery predicate

The helper returns recoverable only when all fields match:

1. Probe-active cookie is exactly 0x48375031.
2. Saved origin is SVC mode and ARM state (SPSR.T=0).
3. Saved LR_abt equals the address of the single probe LDR plus 8.
4. DFAR equals exactly 0x03000024.
5. DFSR is a whitelisted short-descriptor synchronous fault with WnR=0
   (read access) and LPAE=0.
6. The recovery label is nonzero.

The short-descriptor fault-status whitelist contains alignment, access-flag,
translation, synchronous external abort, domain, permission, and synchronous
parity-error classes for which DFAR is defined. It deliberately excludes
debug events, asynchronous external aborts, asynchronous parity errors,
write-side faults, unknown/reserved fault-status values, and long-descriptor
syndrome format. The field extraction is
FS[4:0] = {DFSR[10], DFSR[3:0]}.

The exact diagnostic is a separate ARM-state assembly function. It writes
the active cookie, executes one aligned 32-bit LDR from 0x03000024,
masks the result to VER_REG[2:0], writes only that value to the caller's
ordinary RAM result pointer, clears the cookie, and returns. On a matched
Data Abort, the handler rewrites only the saved PC to a recovery label that
returns status H700_DATA_ABORT_RECOVERY_ABORTED. It does not retry the
faulting load.

### 3.3 Fatal fallback and call-site policy

The modified arm_data_abort_handler() calls the H700 matcher first. Only
a positive result returns 1 to the vector trampoline. Every non-match calls
the original exception_die(frame, -8, "data abort, halting") and halts.
The vector has no generic "skip abort" behavior.

h700_syscfg_ver_reg_low3() is a dormant diagnostic API. No call site was
added to target_early_init(), target_init(), input polling, timer
initialization, scheduler setup, or default runtime startup. The ordinary
result channel is the caller-provided RAM word plus the function's return
status; no UART, panel, heap, Linux, or filesystem is assumed.

### 3.4 Tests and current limits

- Host matcher tests cover an accepted synchronous external read and reject
  wrong cookie, DFAR, saved PC, origin mode, Thumb state, write-side fault,
  asynchronous external/parity fault, debug event, long-descriptor format,
  null frame, and zero recovery PC.
- H700 AArch32 ARM-state and Thumb-2 builds compile the C recovery code;
  the probe assembly is forced to ARM state and its object format is checked.
- The pinned-LK overlay test compiles/links the patched exception vector,
  verifies the fatal fallback remains present, and checks ARM-to-Thumb
  interworking for the C handler.

These are host/source/assembler/link checks only. They do not emulate the
Cortex-A53 exception hardware, prove BL31 security attribution or peripheral
firewall access, establish the target's selector value, or authorize executing
the diagnostic on the RG34XX. The probe has not been called.

A Linux-runtime VER_REG sample can settle the selector's later runtime value;
it cannot establish CPU reset/RVBAR state at the earlier BL31→BL33 handoff.
The pre-BL33 snapshot remains a separate requirement.

## 4. Current disposition

- Source-level probe-scoped AArch32 recovery is implemented in the H700
  overlay generator; all non-matching Data Aborts retain the fatal path.
- The diagnostic remains dormant and is not wired into startup/input/timer.
- The latest device-side /dev/mem attempt remains EFAULT; userspace
  retries are stopped.
- CI for the current recovery implementation passed: Linkslot feasibility #45834208280 (81/81, no skipped steps), H700 compile #45834208274, and boot audit #45834060935.
- hardware_applied=0: no MMIO read/write, module load, boot-argument change,
  SD write, firmware flash, or diagnostic payload execution.


### 3.5 Abort-mode asynchronous exception isolation

The H700 overlay now emits `cpsid if` as the first instruction in the
`arm_data_abort` vector, before it saves the 68-byte fault frame. This is
deliberate for the pinned GammaOS early-start path: CRT0 initializes the
SVC, Abort and FIQ stacks from the same early stack region, and the generic
IRQ code explicitly assumes supervisor-mode interruption. An IRQ/FIQ nested
after the abort frame is allocated could therefore interfere with the early
stack or replace DFSR/DFAR before the exact matcher evaluates them.

The saved origin CPSR is already captured in the frame by the vector, so a
successful `LDM^` return restores the original IRQ/FIQ mask state together
with the origin mode. A non-matching abort retains the fatal path and remains
halted with interrupts masked. The overlay test asserts `cpsid if` precedes
the first stack-save instruction in the Data Abort vector.


### 3.6 Final CI and linked ELF disassembly (2026-11-26)

The final code commit is
`625e9181d493445fcbff77a140830cad428aa44f`.

- Full Linkslot/LK feasibility: [#45834208280](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45834208280),
  **81/81 steps successful, zero failures, zero skipped**.
- H700 AArch64 / AArch32 ARM / AArch32 Thumb-2 compile and host tests:
  [#45834208274](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45834208274), success.
- H700 boot audit tools (Linux and Windows):
  [#45834060935](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45834060935), success.
- Feasibility artifact:
  [15123653120](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45834208280/artifacts/15123653120),
  387,940,045 bytes.

The linked A32 layout-test ELF disassembly confirms the following actual
instruction sequence in this build (addresses are for the disposable test
ELF, not a deployed image):

| Address | Instruction / symbol | Verified meaning |
|---|---|---|
| `0x4a0011d0` | `cpsid if` | Mask IRQ/FIQ before constructing the abort frame |
| `0x4a0011d4` | `stmfd` | Save r0-r12 and LR_abt |
| `0x4a0011f4` | `blx 0x4a00142c` | A32 vector calls the Thumb C handler with interworking |
| `0x4a001204` | `h700_arm_data_abort_exception_return` | Restore original frame SP/SPSR, then `LDM^` only after a positive match |
| `0x4a001640` | `blx 0x4a001650` | Thumb API calls the ARM-state probe with interworking |
| `0x4a00166c` | `h700_syscfg_probe_fault_load` | The one candidate 32-bit LDR instruction |
| `0x4a001688` | `h700_syscfg_probe_recovery` | Abort recovery label; returns the aborted status without retry |
| `0x4a0036cc` | probe active cookie | Test ELF BSS location; `_end=0x4a0036d0` |

The A32 probe's literal pool resolves the read address to
`0x03000024`; the matcher and the probe symbols are both resolved in the
same ELF. The overlay test also confirms the original fatal C fallback
remains for all non-matching Data Aborts.

**Scope limitation:** this is a 5,804-byte minimal A32 overlay layout/link
test with stubs, not a full LK target build, not a packaged 32 KiB BaseOS
loader replacement, not a bootable firmware image, and not a Cortex-A53
hardware exception test. No diagnostic API call or register read was made
on the RG34XX. The SYS_CFG security/firewall/bus permission and actual
exception recovery behavior therefore remain unproven on hardware.

`hardware_applied=0`; no boot-media change, module load, MMIO access, flash,
or diagnostic payload execution occurred.


### 3.7 Active 32 KiB native loader extent regression

The final host regression now checks the two independent limits separately:

- The linked minimal A32 overlay test rejects a flat raw image larger than
  `0x8000` bytes, the active native loader's 32 KiB file extent. The current
  stub-linked raw image is 5,804 bytes (26,964 bytes of file headroom).
- The BaseOS native override validator accepts an exactly-32,768-byte raw
  input and rejects 32,769 bytes; the native builder pads an accepted image
  to exactly 32 KiB before regenerating the paired firmware/SPL metadata.
- The in-memory linker bound `[0x4a000000, 0x4a0e0000)` remains a separate
  conservative runtime address-space bound. It is not the loader file size
  and it does not approve stack/heap/BSS ownership.

These new tests passed in the latest full feasibility run
[#45834208280](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45834208280),
81/81, artifact [15123653120](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45834208280/artifacts/15123653120).
This remains host evidence only; no native firmware package was built from
the test ELF and no device payload was executed.


### 3.8 Peripheral-fault scope and PI12 default-off policy

The matcher is intentionally limited to the one armed SYS_CFG VER_REG load at
`0x03000024`. It does not recover PIO faults at `0x0300b124` /
`0x0300b130`, nor arbitrary external aborts. A PIO external Data Abort still
takes the original fatal LK exception path; the PI12 service's ordinary
readback-error handling cannot catch a bus fault.

Therefore the physical PI12 MMIO wrapper is compile-time disabled by default.
Only a disposable host layout build defines
`H700_RG34XX_PI12_EARLY_ACCESS_ENABLE=1` to prove the real object/link path.
The production platform make rules leave it undefined. BaseOS native SPL
uses PI15/PI14/PE6 before BL31, but this pre-BL31 fact does not prove the
post-BL31 APB1/PIO state at A32 BL33 entry. Do not enable the wrapper until
that handoff state, RAM ownership and secondary-core/Single-CPU contract are
independently closed.

Latest full Linkslot/LK feasibility after the default-off change:
[#45847816950](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45847816950), 81/81 steps, zero failures and zero skipped;
artifact [15133935777](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45847816950/artifacts/15133935777).
H700 compile/host [#45847406072](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45847406072) and boot-audit
[#45847165507](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45847165507) passed.
