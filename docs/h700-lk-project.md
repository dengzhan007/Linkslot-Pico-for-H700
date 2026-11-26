[object Object]

### Probe-scoped AArch32 Data Abort handling

The H700 LK overlay now includes a dormant one-word SYS_CFG diagnostic. Its
handler accepts only the exact A32 SVC load instruction at
0x03000024, with matching saved LR_abt, DFAR, a short-descriptor
synchronous/read DFSR status, active cookie and ARM state. Asynchronous
aborts, write-side faults, debug events and all unrelated aborts remain fatal.

Before calling the Thumb-2 C handler, the A32 vector retains the original frame
address in callee-saved r4 and aligns the Abort stack to an 8-byte boundary.
A positive match alone reaches the LDM^ exception-return trampoline. The
probe API is not called by any default startup or runtime hook, and it has not
been executed on hardware. Source/host/link checks are not a substitute for a
real Cortex-A53 exception test or bus/firewall permission proof. See
docs/h700-lk-data-abort-recovery-audit.md.


The Data Abort vector masks IRQ/FIQ before saving its exception frame. This
avoids nested interrupts interfering with the pinned LK early shared stack or
the DFSR/DFAR snapshot. The exception-return trampoline restores the original
SPSR, including the caller's interrupt mask state.
