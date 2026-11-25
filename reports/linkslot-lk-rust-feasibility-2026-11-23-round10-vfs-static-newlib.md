# Linkslot 1.0.36 H700 feasibility — round 10

Date: 2026-11-23  
Feature branch: `porting/linkslot-1.0.36-feasibility`  
Feature commit: `2ea0b1f8f58b04b1004ea0fabc8f7ee39d9bd9b5`  
Immutable Linkslot source snapshot: `upstream/linkslot` @ `8590d6ff1b2caa26118051069c975106fbb14bd3`  
mGBA source: `libretro/mgba` @ `e31759b24e7a4e3899285ff720d7b573ac328ae7`  
LK reference: `TheGammaSqueeze/lk-mt6785` @ `9bf1419a49e0af6dba8b05916df382c9c9c6d0a8`

## Result

**PASS — complete configured host/CI feasibility workflow.**

- Workflow: [45125097115](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45125097115)
- Conclusion: success
- Artifact: [linkslot-lk-rust-feasibility-45125097115](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45125097115)
- Artifact ID: `14794884889`
- Size: 91,104,214 bytes
- SHA-256: `f50a8902147b43e82208a4a0a57a76f666686218bd7fade94370090e9e77608c`
- Artifact retention expiry: 2026-12-23

The workflow was run from PR #16 at the exact feature commit above. The main feasibility workflow no longer has a branch-push trigger; it runs on PR synchronization or manual dispatch, avoiding the previous duplicate push/PR runs that could cancel a long mGBA build while its artifact was being packed.

## New integration coverage

### 1. Actual Linkslot slot-retro VFS v3 callback

The workflow now applies a narrow patch to a temporary checkout of the immutable Linkslot 1.0.36 source snapshot:

- Adds `platform/h700/runtime/linkslot_vfs.rs` as a stable VFS v3 callback bridge.
- Adds the C ABI contract in `platform/h700/runtime/include/linkslot_vfs.h`.
- Routes `RETRO_ENVIRONMENT_GET_VFS_INTERFACE` through the real `slot-retro::environment()` callback.
- Installs a process-lifetime backend table before `retro_init()`; the bridge rejects null/incomplete tables, repeated installation, null request data and requested VFS versions above v3.
- Returns a stable frontend-owned table and forwards file/directory operations to the platform provider.

The patched `slot-retro` host test suite completed **60 passed, 0 failed**. The VFS regression calls the real `crate::libretro::environment()` path; it validates no-backend behavior, one-shot backend installation, v3 negotiation, v4 rejection, file open/path/size/read/close/stat and directory open/enumerate/name/type/EOF/close.

The patch is applied only to the temporary CI checkout. The `upstream/linkslot` snapshot branch is unchanged.

### 2. Actual mGBA VFS v3 directory adapter

The workflow compiles the pinned mGBA `libretro-vfs-dir.c` and calls its public VDir callbacks against a fake VFS v3 provider.

**PASS** for:
- dot-entry filtering;
- file/directory names and types;
- end-of-directory behavior;
- rewind by close/reopen;
- nested directory open;
- VFileOpen path joining;
- traversal, absolute-path and backslash rejection in that test adapter;
- VFS remove, mkdir and existing-directory stat fallback.

This is an actual mGBA adapter host regression, not only a reimplementation of the expected behavior.

### 3. mGBA VFS v3 AArch32 static archives

Pinned mGBA produces two relevant static archives:

| Build | Archive | Size | Objects | Result |
|---|---|---:|---:|---|
| CMake Generic + VFS v3 | `mgba_libretro-vfs3-aarch32-static.a` | 2,870,400 B | 114 | PASS |
| arm-none-eabi GCC/newlib + VFS v3 | `mgba_libretro-vfs3-arm-none-eabi.a` | 2,856,556 B | 94 | PASS |

For both archives:
- target is Cortex-A53 AArch32 ARM state, NEON/VFPv4, softfp;
- all 25 Libretro exported entry points are namespaced as `mgba_retro_*`;
- no unprefixed global `retro_*` exports remain;
- no guarded POSIX/stdio filesystem fallback symbols remain;
- archive-internal references are subtracted before reporting true external dependencies.

The Generic archive reports 51 true external undefined symbols; the arm-none-eabi/newlib archive reports 52. These are still dependencies to provide or resolve in the eventual target runtime; an archive build is not itself a runnable core.

### 4. H700 newlib adapter and whole-archive link diagnostic

The H700 adapter validation passed:
- host allocator/syscall regression: `PASS: H700 newlib allocator gate and 11 syscall hooks`;
- actual arm-none-eabi/newlib headers and AArch32 ELF32/ARM/EABI5 target compile;
- `lk_bindings.c` compiles with `-Werror` against the pinned LK `include/lib/heap.h` declarations;
- all 22 public/reentrant allocator, syscall, lock and install symbols are present.

The adapter keeps its fail-closed gates: allocation is denied until both `heap_ready()` and `single_cpu_active()` report true. _sbrk returns ENOMEM because allocation is routed through the LK heap rather than creating another arena. Console, RTC and fatal-exit providers remain injected platform services.

The whole mGBA archive was then linked with the adapter object and newlib:
- `newlib-only`: PASS, final test ELF32/ARM has no undefined symbols;
- `newlib-nosys`: PASS, also closes the test ELF.

This proves the archive plus adapter object can close the tested newlib symbol set. It does **not** mean `libnosys` is an acceptable runtime; its system-call routines are dummy stubs. The `_start` entry is only a link-test stub, not LK startup code. The adapter still requires real LK heap, UART/console, RTC and fatal services.

### 5. LK AArch32 image-layout probe

The existing host-only BL33 wrapper/layout probe passed again. It confirms only that the synthetic AArch32 wrapper layout and opaque-header-overwrite test are internally consistent. It does not build the full LK kernel, identify safe free RAM, validate the real vendor header, or execute the image.

## Corrections captured during this round

### Libretro path handling

The VFS design note was corrected after inspecting Linkslot and mGBA call paths. A blanket ban on all absolute paths would break valid frontend-supplied game/system paths.

The eventual H700 backend must instead:
- resolve requests only against explicit configured roots (game content, system/BIOS, saves, states and cache);
- accept absolute paths only when component-wise resolution proves they are inside an allowed root, then translate them to card-relative paths;
- resolve relative paths against an explicitly configured base, never an implicit process working directory;
- reject paths outside allowed roots, traversal, invalid separators and unrecognized URI/drive schemes;
- preserve the exact opened path string for the lifetime required by Libretro `get_path`;
- require same-volume rename/replace and documented durable flush/barrier ordering.

This policy is documented but not yet implemented by a real SD/FAT provider.

### Workflow cancellation

The previous full run could be duplicated by both `push` and `pull_request` triggers. A cancelled CMake build removed a transient progress file while `upload-artifact` was zipping the work tree. The main workflow's duplicate branch-push trigger was removed; the current PR-triggered run completed and uploaded its 91 MB artifact successfully.

## Still not complete — hard gates

1. **No H700 SD/block/FAT32/exFAT provider.** The VFS backend in this round is a bridge plus test fake. No physical card ROM, BIOS, save or state I/O is implemented or tested.
2. **No durable save-state semantics.** Flush/barrier ordering, same-volume rename/replace, interruption behavior and power-loss recovery remain unimplemented.
3. **No static-core integration inside Linkslot.** The separate static API registry is still a probe using mock cores. `slot-retro` still uses `libloading::Library` in the supplied source; this VFS bridge does not convert it into a static mGBA loader.
4. **No complete no_std Linkslot frontend.** Only selected logic/data candidates have been isolated; `slot`, `slot-ui`, `slot-store`, `slot-retro` and the remaining workspace have not been ported as a runnable LK application.
5. **No real LK runtime.** The pinned LK heap header/API test uses a test stub, not the real heap implementation. The LK adapter is not approved for SMP until secondary cores are proven parked for the LK lifetime or a cross-core-safe allocator/scheduler is implemented.
6. **No H700 hardware boot/handoff proof.** The existing BL33 entry and vector work is synthetic host-only. Safe memory ownership, secondary CPU state, EL/MMU/cache/GIC setup and the real handoff contract remain unresolved. Boot/handoff remains **NO-GO**.
7. No firmware image, Boot0/TOC1/U-Boot/monitor write, SD write, payload execution, or device ROM test was performed.
8. armGBA is not present in the supplied Linkslot 1.0.36 snapshot; it remains a future core candidate and is not covered by this round.

## Next engineering stage

Proceed in this order:

1. Add a statically supplied Libretro function-table entry to Linkslot's `slot-retro`, keeping host/Linux dynamic loading intact behind a feature. Test the same `LibretroCore` lifecycle against a mock static core.
2. Bind the namespaced real mGBA symbols to that static table and compile/link a minimal Linkslot-core harness against the real archive. Do not yet call it an LK application.
3. Implement and host-test a bounded H700 VFS path resolver and a multi-handle storage backend contract, including flush/rename failure injection. Keep card I/O disconnected until the H700 memory/runtime gates are resolved.
4. Resolve the AArch32-vs-AArch64 LK/runtime decision and the H700 secondary-core ownership/SMP issue before any boot experiment.

## Source and PR

- Main repository: https://github.com/dengzhan007/Linkslot-Pico-for-H700
- Draft PR #16: https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16
- Feature branch: https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/porting/linkslot-1.0.36-feasibility
- Immutable source snapshot: https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/upstream/linkslot

The PR remains **draft**. This round closes a static-link dependency diagnostic and a frontend↔core VFS callback gap; it does not close the hardware/runtime/boot gates.
