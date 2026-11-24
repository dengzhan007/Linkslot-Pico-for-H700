# Linkslot 1.0.36 on H700 LK: runtime and renderer integration plan

**Status:** architecture plan plus host-only renderer/frontend/input integration proofs. The project has persistent `no_std + alloc` `slot-input`, `slot-model`, `slot-frontend-state`, `slot-core-picker`, `slot-link-state`, `slot-link-kind`, and `slot-video-mode` crates; a C input service/queue and Rust gesture bridge; generic frontend/core-picker state machines; paired-save, link-mode and source-crop policy modules injected into the real Frontend/App source tree. Focused CI compiles the complete patched Linkslot `slot` crate with all overlays. It still has no bootable Linkslot runtime, physical GPIO/input or display/audio driver, or on-device execution. The upstream snapshot on `upstream/linkslot` remains immutable.

This plan resolves a practical integration question: which parts of Linkslot can become LK-resident logic, and which Linux services must be replaced rather than emulated.

## 1. Decision

Do **not** port the current Linux `device` application as a whole and do not build a POSIX/Linux compatibility layer inside LK.

Use the confirmed three-language split:

```
AArch32 GammaOS LK / BL33 (C + ARM assembly)
  ├─ H700 C BSP/HAL and runtime services
  │    boot / IRQ / timer / memory / display / input / audio / SD / power
  ├─ C ABI service boundary
  │    framebuffer / VFS / monotonic time / input / PCM / power / allocator
  ├─ Rust no_std Linkslot runtime + frontend business
  │    event loop / game library / menu state / config / UI / save workflow
  │    no Linux device_app, std threads, process or dynamic .so loader
  ├─ Rust no_std software renderer
  │    Linkslot Draw contract → caller-owned framebuffer slice
  └─ C/C++ static emulator cores
       mGBA / Gambatte / gpSP → Libretro/Core ABI → VFS v3 + newlib services

Host-only Python/Shell tooling
  └─ source audit / patch application / build orchestration / test reporting
```

The first runtime remains single-active-core and fails closed until the BL31-to-BL33 secondary-core state is proven. LK's current scheduler is not assumed to be SMP-safe.

### Language ownership and interface rules

| Layer | Language | Owns | Must not own |
|---|---|---|---|
| Boot + H700 hardware | C / ARM assembly | entry/exception code, MMIO, clocks, IRQ, memory map, timer, display scanout, GPIO, PCM, MMC, PMIC, low-level diagnostics | Linkslot menu/game policy or emulator internals |
| Platform runtime | C services + narrow C ABI | allocator/syscalls, monotonic time, bounded input/audio queues, VFS and board service adapters | Linux process model or full POSIX compatibility |
| Linkslot frontend | Rust `no_std` | application state, game library, menu/UI transitions, config semantics, key mapping, user save/sync workflow, renderer orchestration | raw MMIO, guessed physical addresses, Linux `std` services |
| Emulator cores | C/C++ | emulation, core-owned state, video/audio generation and core options | frontend UI, board register access or private filesystem paths |
| Development tools | Python/Shell | host-only source transformation, test generation, analysis and packaging checks | any firmware/runtime dependency |

The C↔Rust boundary must be a small versioned ABI with explicit ownership/lifetime rules, fixed-width types and error returns. C owns device resources and DMA/scanout buffers; Rust borrows service handles and bounded buffers only for documented call lifetimes. A C++ core must export a C-compatible entry table. The frontend calls a core only through the stable Core ABI, not through C++ symbols.

The A32 LK image is the small resident boot/platform layer. The mGBA archive is approximately 1.87 MB in the measured diagnostic link, over twice the current 896 KiB host-side BL33 package ceiling; it remains a separate static-core/application payload. Its runtime load address is **not approved**. See [BL33 memory ownership](h700-bl33-memory-ownership.md).

## 2. What the actual Linkslot source permits

The `device` Cargo feature is a Linux handheld backend selection, not a freestanding runtime selection.

Direct source inspection of the 1.0.36 snapshot confirms:

| Component | Current source behavior | H700/LK treatment |
|---|---|---|
| `crates/slot/src/device_app.rs` | Uses `std::thread`, `Instant`, `sleep`, filesystem and environment variables; opens `FbdevSurface`; creates the GL compositor and Linux platform services | Replace the entry loop with an LK-owned event loop. Do not compile this file as-is. |
| `crates/slot/src/frontend.rs` | 4,000+ lines; concrete `&mut Compositor`; uses std paths, mpsc channels, threads, files and PNG decoding | Retain behavior as the reference. Extract state transitions and renderer contract incrementally; do not claim crate-level no_std portability. |
| `crates/slot/src/app.rs` | 18,000+ lines; uses std paths, filesystem, threads, channels, mutexes, atomics, clocks and persistent storage | Split pure application state/decisions from I/O, workers and persistence. The full file is not a first LK integration target. |
| `crates/slot-gfx/src/fbdev.rs` | Opens `/dev/fb0`, uses Linux ioctl/sysfs and dynamically loads EGL/GLES libraries | Linux-only. No equivalent EGL loader or userspace Mali stack is present in LK. |
| `crates/slot-gfx/src/fbo.rs`, `draw.rs`, `pipeline.rs` | Renderer implementation uses OpenGL textures, shaders, FBOs and GL state | Reuse the command/data contract where practical; replace the execution backend. |
| `crates/slot-ui/src/art.rs` | Uses std filesystem reads, PNG decode, SVG `resvg/usvg` rendering and CPU RGBA buffers | Start with embedded/predecoded assets. Add SD-backed PNG/SVG only after the VFS and memory budget are integrated. |
| `crates/slot-ui/src/text.rs` | Uses `fontdue`, `OnceLock`, dynamic font objects and heap-allocated coverage masks | First milestone should use a prebuilt glyph atlas or pre-rasterized labels. A runtime fontdue port is a later optional task. |
| `crates/slot/src/input/device.rs`, `input/evdev.rs` | Linux `/dev/input/event*`, sysfs capability probing, threads and mutex-protected queues | Reuse button mapping and `slot-input` state logic. Replace raw input acquisition with GPIO/ADC/key-scan HAL. |
| `crates/slot/src/audio/ring.rs`, `audio/alsa.rs` | std Mutex/Condvar/atomics/clock and ALSA device backend | Preserve the audio-sink contract and sample semantics; build an LK/H700 PCM DMA backend and bounded producer/consumer queue. |
| `crates/slot-power` | Platform implementation reads sysfs, invokes shell commands and Linux power utilities | Reimplement through explicit H700 PMIC/power HAL calls. |
| `crates/slot-store` and `linkslot` | SQLite, filesystem atomic writes, HTTP/WebDAV, TLS and optional Lua dependencies | Keep their data/protocol semantics as references. Bring up local ROM/core/save first; network and MOD execution are later milestones. |

The existing candidate results for `slot-input`, `slot-model`, `slot-i18n`, Libretro static API and VFS v3 remain valuable, but isolated host-only/no_std adapters are not evidence that `slot`, `slot-ui`, `slot-gfx` or the entire workspace can build for LK.

## 3. Renderer: keep the draw vocabulary, replace the backend

The current renderer already exposes a useful CPU-side command vocabulary:

- `Draw::Rect`: solid RGBA rectangle;
- `Draw::Tex`: RGBA texture;
- `Draw::Turned`: rotated texture;
- `Draw::Game`: insert the live emulator frame at this point in draw order;
- `Draw::Shot`: draw a captured still texture through the game-image path.

Texture creation and updates already receive CPU RGBA byte slices. This is a clean conceptual seam between UI face generation and the final pixel backend.

The renderer boundary has now been injected in the temporary Linkslot 1.0.36 worktree: the GL Compositor implements an object-safe `Renderer` trait, and the real `Frontend` plus `CartFaces` accept that trait instead of a concrete GL Compositor. The same trait is implemented by the H700 SoftwareCompositor candidate. The immutable `upstream/linkslot` snapshot is unchanged.

### LK renderer baseline

Implement a software renderer first, rather than assuming EGL/GLES or the Linux Panfrost/Mali userspace stack is available inside LK.

Initial features:

1. Fixed RGBA framebuffer and opaque clear.
2. Clipped `Rect` fill with alpha blend.
3. Nearest-neighbor and bilinear `Tex` blits with alpha.
4. `Turned` texture transform.
5. `Game` insertion with aspect-preserving scaling and black borders.
6. `Shot` insertion using the same texture path.
7. Explicit stride and pixel-format conversion at the final scanout boundary.

Add Grid/LCD/CRT/FourPass filters, frame split/lift, bezel and rotation only after the unfiltered baseline passes golden-image tests. The existing GL shaders are behavior references, not code that can be directly executed by LK.

### First host-only implementation and Frontend injection

A separate crate exists at `integration/linkslot/h700-render-core`. It is outside the immutable upstream snapshot and uses `#![no_std] + alloc` for non-test builds. It accepts a caller-supplied framebuffer slice and supports RGBA8888/BGRA8888, explicit width/height/stride validation, stable texture IDs, create/update/delete, clipped rectangles, nearest and bilinear texture sampling, source-over alpha, centre-based rotation, Game/Shot insertion, XRGB8888/GL_BGRA-compatible game-frame conversion, source-rectangle cropping and RGB gain. Its 13 host regression tests and non-test `cargo check` pass in GitHub Actions.

The integration patcher `scripts/apply-linkslot-render-core.py` applies the crate to a temporary checkout pinned to Linkslot 1.0.36 commit `8590d6ff1b2caa26118051069c975106fbb14bd3`. It makes `slot-gfx` re-export the exact shared `Draw/TexId` types, gives the existing GL `Compositor` an object-safe `Renderer` implementation, and changes `Frontend` plus `CartFaces` to consume `&mut dyn Renderer`. All 21 renderer operations used by Linkslot's real `Frontend` are covered by the trait. The original `upstream/linkslot` branch remains unchanged.

**Verified:** complete feasibility workflow [45346015251](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45346015251) passed. It compiled the H700 SoftwareCompositor, ran its framebuffer integration regression, and Cargo-checked the full patched Linkslot `slot` crate with `device,h700-software` features. The same workflow also passed A32 BL33/exception-vector linking, header packaging and memory fail-closed tests, H700 read-only storage/VFS host regressions, and the real mGBA A32/newlib static archive build and symbol binding.

This proves an interchangeable renderer seam and a host-tested software framebuffer backend, not full GL feature parity or device scanout. The compositor test covers the current integration contract and does not replace golden-image comparison against the upstream GL backend. Advanced shaders/filters, all frame effects, full-panel rotation, display ownership, double buffering and cache maintenance remain separate acceptance items. No physical framebuffer is mapped or written.

### Display memory facts and limits

The RG34XX live inventory reports a framebuffer at `0x7bf24800`, 720×480, 32 bpp, stride 2,880 bytes, size 1,382,400 bytes (`0x151800`), range `[0x7bf24800, 0x7c076000)`. That is a Linux live-FDT/bootargs reservation and must remain excluded from general payload allocation.

It is a **display-owned scanout candidate**, not approval to map/write it from LK. Before the renderer uses it, the H700 display path must prove that scanout remains active across the BL31-to-LK handoff, establish channel/format/order/stride/rotation and cache-maintenance requirements, and obtain explicit ownership of the buffer. Do not allocate a second 720×480×32bpp framebuffer at an unreviewed physical address.

The current Linkslot GL canvas is 720×480. Its 32-bit framebuffer alone is 1.38 MB; software texture caches and emulator frames are additional memory and must be bounded. Benchmark the 16.76 ms pacing target from the Linux device loop rather than assuming a CPU software renderer will meet it.

## 4. Minimal Rust runtime boundary

Create a narrow LK integration crate rather than adding the current complete Cargo workspace to the LK target.

The first no_std + alloc boundary should contain only:

- stable button/key enums, raw events, the H700 C Input Service v1 adapter, and the `slot-input` gesture/state machine;
- the generic `slot-frontend-state::FacesLang<L>` state machine, extracted from the real `Frontend` language-face build coordinator; resource rasterization and worker/thread orchestration remain outside this crate;
- the generic `slot-core-picker::CorePicker<C>` state machine, extracted from `App`; it produces selected-core outcomes and chip animation geometry without depending on `slot-store`, `slot-ui` or renderer resources;
- the `slot-link-state` `SLK1` paired-save codec and BIOS identity check, preserving the public `slot::link_state` API while validating the two bounded length-prefixed payloads;
- the `slot-link-kind` game-code/title classifier and GBA serial-option policy, preserving the `slot::link_kind` API and Pokémon retail/hack behaviour;
- the `slot-video-mode` option parser and normalized source crop calculator, while INI access remains in the host service wrapper;
- pure machine/core/layout identifiers from `slot-model`; filesystem path helpers remain behind the host-only `host-compat` feature;
- generated/static i18n lookup tables, not runtime JSON parsing through `OnceLock`;
- a renderer-independent `Draw` / texture-ID / geometry contract;
- the static Libretro core descriptors and VFS v3 ABI already under test.

Runtime services must be explicit interfaces, not hidden std assumptions:

| Service | Initial LK contract |
|---|---|
| Allocation | LK heap-backed Rust `GlobalAlloc`, with alignment, exhaustion, free/realloc and initialization tests. Do not enable alloc before the LK heap and physical arena are approved. |
| Time | Monotonic tick counter and bounded delay; no `Instant` or wall-clock dependency in the frame loop. |
| Work scheduling | Single-core cooperative jobs/steps initially; no `std::thread`, mpsc or condvar emulation. |
| Storage | H700 SD/FAT VFS for ROM/core reads. Add save/state writes only after flush, atomic replace, directory update and power-loss consistency are proven. |
| Input | C H700 Input Service v1 provides ordered logical edges from the bounded C queue; Rust `H700InputSource::drain_into()` maps them into Linkslot `RawEvent`, and `H700GestureInput` invokes the upstream `Gestures` state machine. The GPIO/ADC scanner and hardware producer are still pending. |
| Display | Renderer-independent framebuffer descriptor and cache/scanout handoff; no `/dev/fb0`, EGL or dynamic loading. |
| Audio | PCM producer interface with measured rate, format and bounded buffering; no ALSA/CPAL. |
| Power | Explicit battery/charger/backlight/LED/poweroff service; no sysfs or shell execution. |
| Network | Omit from the first bootable vertical slice. Add Wi-Fi, TCP/TLS, WebDAV and LAN server after local play/save works. |

Do not add Linux syscall shims for `open`, `read`, `pthread`, `dlopen`, `ioctl` or `syncfs` merely to make the old application compile. Implement the small services that the selected Linkslot modules genuinely need.

## 5. Vertical-slice sequence

| Stage | Deliverable | Acceptance evidence |
|---|---|---|
| R0 — runtime floor | LK console, approved heap, timer, fail-closed single-core runtime and stable VFS service | host tests + complete LK target link + reviewed memory/entry contract |
| R1 — display | static Linkslot-like boot/menu scene rendered to H700 scanout | color bars, alpha/rotation/texture golden images, cache/scanout and real-panel test |
| R2 — local frontend | static assets, game list, key mapping, menu/game transition | real SD directory enumeration, physical controls and stable frame loop |
| R3 — emulator | mGBA static A32 core, VFS v3, ROM launch and video frame upload | GB/GBC/GBA smoke tests on device; no dummy syscall stubs |
| R4 — persistence | SRAM/battery saves and save states | read/write/flush/atomic replacement and power-cut recovery tests |
| R5 — audio and power | PCM output, volume, battery, LEDs and orderly shutdown | real hardware loopback/latency, poweroff and repeated cold-start tests |
| R6 — advanced Linkslot | artwork/font/theme, Wi-Fi/WebDAV, LAN web UI, achievements and MODs | each subsystem gets separate memory, security, compatibility and device regression gates |

R1 does not require the entire 1.0.36 application to be ported first. R2/R3 establish the minimum playable vertical slice. Existing Linkslot behavior tests and golden UI assets should be reused to prevent the LK-specific implementation from drifting.

## 6. Immediate engineering order

1. Keep the original `upstream/linkslot` tree immutable. Record every extracted source file and upstream path in the integration manifest.
2. Golden-test the H700 SoftwareCompositor against the real GL backend for draw order, crop/rotation, brightness/power, bezel and texture sampling; treat unsupported shader effects as explicit capability gaps.
3. [Completed] Extract `slot-input` and pure `slot-model` into `integration/linkslot/h700-frontend-core`; preserve upstream tests, prove AArch32/AArch64 `no_std` compilation, and compile the complete Linkslot `slot` Frontend against the persistent overlay in a temporary upstream worktree.
4. [Completed] Extract the pure `FacesLang<L>` coordinator from `Frontend` into `slot-frontend-state`; six state-machine tests pass, the original Frontend imports the generic type through the temporary overlay, and the complete Frontend compiles with the overlay.
5. [Completed] Extract the core selection interaction/animation state machine from `App` into generic `slot-core-picker::CorePicker<C>`; six tests cover edge selection, single-core non-write behaviour, hop reversal continuity, easing completion, and write outcome. The temporary overlay replaces App's concrete core-picker module and full `slot` compilation passes.
6. [Completed] Extract `slot::link_state` into `slot-link-state`; retain the original public functions through a thin module re-export and verify paired-state parsing, player bounds, BIOS matching and malformed input handling in a no_std crate.
7. [Completed] Extract the pure GBA cable/wireless policy into `slot-link-kind`, keeping the 43-code wireless table, Pokémon title/clean-ROM exception, serial protocol selection and link-capability rules. The original module remains a public re-export.
8. [Completed] Extract `VideoMode` and normalized source-rectangle geometry into `slot-video-mode`; preserve INI read/write only in the existing host wrapper and re-export the original `slot::video_mode` API. Crop behavior passes regression tests for padded GB/GBC frames and full-frame systems.
9. [Input slice completed] Implement versioned C input service, bounded single-context queue, allocation-free Rust raw-event drain and adapter into upstream `Gestures`. Next extract pure menu/game-library state and build a bounded Rust LK event loop over C-provided time, input, VFS, display and core services. Keep GPIO/framebuffer hardware ownership in C.
10. Connect the H700 framebuffer/display handoff only after live scanout and BL33 resource ownership are proven; similarly defer audio/input/power MMIO until each board service has its own evidence.
11. Keep mGBA/Gambatte/gpSP as separate C/C++ static cores. Their linker map, BSS, stack, heap and ROM/save buffers must fit inside a subsequently approved application arena.

## 7. Explicit non-goals at this stage

- No attempt to run Linux AArch64 Linkslot ELF inside AArch32 LK.
- No reuse of Linux `/dev/fb0`, EGL/GLES, evdev, ALSA, SQLite or dynamic `.so` loaders.
- No arbitrary AArch64 SMC handoff.
- No guessed MEMBASE/MEMSIZE, heap, stack or core load address.
- No Boot0/TOC1/SD write, image installation or hardware payload execution.
- No claim of a bootable LK image or a working Linkslot port.

This is the integration boundary for the next implementation stage; the current project status remains host-only until the listed hardware gates are independently closed.
