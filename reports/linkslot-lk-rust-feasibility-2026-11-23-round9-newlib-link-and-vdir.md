# Linkslot 1.0.36 — newlib whole-archive link and mGBA VDir regression (round 9)

- Date: 2026-11-23
- Feature branch commit: fc817b3fa84b7a0b57a693ae2785090cf9e86763.
- PR: https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16 (draft; not merged).
- GitHub Actions: https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45115290950 — conclusion SUCCESS.
- Artifact: linkslot-lk-rust-feasibility-45115290950, ID 14790883150, about 83.5 MB.
- mGBA source: libretro/mgba @ e31759b24e7a4e3899285ff720d7b573ac328ae7.
- The upstream Linkslot snapshot and upstream mGBA repository were not modified; the VFS patches are applied only to the CI checkout.

## 1. mGBA VDir Libretro VFS v3 host regression

The test compiles the actual temporary src/platform/libretro/libretro-vfs-dir.c and calls its public mGBA VDir API against a fake Libretro VFS v3 backend. Result: PASS, host process exit 0.

Coverage:
- directory open, dot-entry filtering, name/type and EOF;
- rewind implemented as close + reopen;
- nested VDir open;
- relative VFile open and joined path;
- rejects .., absolute child paths and backslash paths before invoking the backend;
- delete via VFS remove;
- create via VFS mkdir and existing-directory VFS stat fallback;
- close of nested and root handles.

This proves the mGBA VDir adapter's actual C implementation behaves against the Libretro interface, not just that a similarly shaped Rust table can compile. It remains a fake in-memory backend: no SD card, FAT/exFAT, media removal or persistent writes.

## 2. Whole-archive newlib link diagnostic

Input archive:
- mgba_libretro-vfs3-arm-none-eabi.a;
- AArch32 Cortex-A53 / ARM / NEON-VFPv4 / softfp;
- mGBA pinned source and VFS v3 file+directory adapter from round 8.

A small _start function containing only a WFI loop was compiled as the link entry. The mGBA archive was forced in with --whole-archive; the diagnostic is not a device startup image and must never be executed.

### newlib + libm + libgcc, without libnosys

The whole-archive link did not close. Exactly 11 system-call entry points remained undefined:

~~~text
_close
_exit
_fstat
_getpid
_gettimeofday
_isatty
_kill
_lseek
_read
_sbrk
_write
~~~

All 52 archive-external symbols from the previous round were otherwise satisfied by newlib/libm/libgcc or resolved as internal archive references. This narrows the bare-metal ABI boundary substantially.

### newlib + libm + libgcc + libnosys

The whole-archive link completed successfully:
- output: newlib-nosys.elf;
- ELF32 / ARM / EABI5;
- soft-float ABI flag;
- nm -u returned no undefined symbols;
- size: text 1,061,848 bytes; data 801,140 bytes; bss 5,540 bytes; total 1,868,528 bytes.

**Critical caveat:** libnosys supplies dummy/no-system implementations. It makes the linker close, but does not provide a functioning heap, RTC, console, block device, filesystem, process model or actual H700 system calls. In particular _sbrk must be backed by a real, bounded allocator; _gettimeofday by a real time source; _read/_write by a defined console/debug service. This ELF uses the test WFI _start and the generic bare-metal linker defaults, not the H700 RAM map or approved boot entry. It is a symbol-closure diagnostic only.

## 3. H700/LK adapter work now narrowed to 11 calls

| Newlib hook | Required H700/LK semantic |
|---|---|
| _sbrk | Provide a bounded newlib heap arena or replace newlib allocation with LK heap hooks. Must respect heap initialization, alignment, free semantics and SMP safety; do not assume an address or size is available. |
| _gettimeofday | Read a real H700 RTC/time source and define UTC/timezone semantics. No valid H700 RTC service is wired yet. |
| _write | Route diagnostics to LK console/serial/debug output without recursively calling newlib printf. |
| _read | Define whether console input exists; return a proper EOF/ENOSYS/error for absent input. |
| _fstat, _isatty | Describe only explicitly supported console descriptors; don't claim general POSIX file descriptors. |
| _close, _lseek | Define behavior for supported descriptors only; VFS file operations are routed through Libretro VFS instead. |
| _exit, _kill, _getpid | Map fatal/termination semantics to LK panic/thread identity policy; these are not Unix processes on the target. |

The next implementation should be a small, separate adapter with tests for error codes and non-supported descriptors, not a claim that libnosys itself is usable.

## 4. Validation boundary

- The Libretro VFS v3 fake service and mGBA VDir fake backend are host tests only.
- The mGBA archive compiles using arm-none-eabi/newlib and links only with libnosys stubs; no real LK service implementations have been added.
- No final H700 LK image, boot package, RAM linker layout or handoff was produced.
- No ROM was run; no audio/video, save-state or battery RAM operation was tested on RG34XX.
- No SD/boot media was written, no boot areas were changed and no payload was executed.
