[object Object]

### Probe-scoped Data Abort recovery — hardened AArch32 path

- [x] Mirror the pinned GammaOS LK 68-byte fault frame with compile-time offset checks.
- [x] Implement the exact matcher: active cookie, A32 SVC origin, saved LR_abt = probe LDR + 8, DFAR = 0x03000024, short-descriptor synchronous/read DFSR whitelist, and nonzero recovery label.
- [x] Reject asynchronous external/parity faults, debug events, write-side aborts, LPAE syndrome format, unknown fault codes, and unrelated PCs/addresses.
- [x] Align the Abort-mode stack to AAPCS 8-byte boundary for the C handler while retaining the exact frame pointer in callee-saved r4; restore frame/SPSR and use LDM^ only on a positive matcher result.
- [x] Keep every non-matching Data Abort on the original fatal exception_die() path.
- [x] Add a dormant, explicit single-register SYS_CFG VER_REG[2:0] API; no target startup, timer, input, scheduler, or runtime call site invokes it.
- [x] Add host matcher matrix, AArch32 ARM-state assembly object check, and A32-vector-to-Thumb-C interworking link check.
- [ ] Pass current H700 compile/host CI and Linkslot LK overlay generator/link regression.
- [ ] Review emitted exception-vector and probe disassembly before considering device execution. No device probe or firmware execution is authorized yet.
