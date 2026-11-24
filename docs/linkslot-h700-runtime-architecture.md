# Linkslot 1.0.36 on H700 LK: runtime and renderer integration plan

**Status:** source-audited architecture proposal. No Linkslot frontend binary, software renderer, H700 display/input/audio driver, or on-device runtime is implemented by this document. The upstream snapshot on `upstream/linkslot` remains immutable.

This plan resolves a practical integration question: which parts of Linkslot can become LK-resident logic, and which Linux services must be replaced rather than emulated.

## 1. Decision

Do **not** port the current Linux `device` application as a whole and do not build a POSIX/Linux compatibility layer inside LK.

Use a split runtime:

```
GammaOS LK (AArch32 BL33, Cortex-A53)
  ├─ H700 board services
  │    timer / console / allocator / display / input / audio / SD
  ├─ Linkslot LK runtime
  │    bounded single-core event loop, path/VFS service, save manager
  ├─ Linkslot reusable logic
  │    selected domain models, input state machine, UI state transitions
  ├─ software 2D renderer
  │    Draw list + RGBA texture store + H700 framebuffer output
  └─ static Libretro host
       mGBA A32 archive + VFS v3 + newlib/LK service bindings
```

The first runtime remains single-active-core and fails closed until the BL31-to-BL33 secondary-core state is proven. LK's current scheduler is not assumed to be SMP-safe.

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

The current seam is incomplete for LK: `Frontend` and many helper methods take the concrete GL `Compositor` type, not a renderer trait. Refactor the renderer boundary before the full UI can select a backend. The common contract needs texture create/update/delete, frame begin/end, draw-list submission, emulator-frame upload, screen geometry/effect controls and an optional readback operation for diagnostics.

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

### First host-only implementation slice

A separate crate now exists at `integration/linkslot/h700-render-core`. It is intentionally outside the upstream snapshot and uses `#![no_std] + alloc` for non-test builds. The first implementation accepts a caller-supplied framebuffer slice and supports:

- RGBA8888 or BGRA8888 framebuffer byte order, with explicit width/height/stride validation;
- stable texture IDs, create/update/delete, and tightly packed RGBA texture data;
- clipped solid rectangles, nearest-neighbour and bilinear texture sampling, and source-over alpha blending;
- centre-based texture rotation;
- game-frame insertion and screenshot (`Shot`) insertion in draw-list order;
- XRGB8888/GL_BGRA-compatible game-frame byte conversion;
- fallible texture-buffer reservations reported as `RenderError::TextureTooLarge`.

Texture filtering matches Linkslot's current API convention: `create_texture()` is linear, `create_texture_nearest()` is nearest, and `update_texture()` preserves the handle's current filter. Eleven host regression tests cover framebuffer bounds, clipping, both sampling modes, filter persistence, alpha, rotation, Game/Shot ordering, byte order and texture-handle lifetime.

The crate's `cargo check` and `cargo test` commands are included in the Linkslot/LK feasibility workflow. The new `scripts/apply-linkslot-render-core.py` copies this crate into an isolated checkout of the pinned Linkslot snapshot, makes `slot-gfx` re-export the same `Draw/TexId` types, and updates the GL sprite backend to use the shared handle index. The workflow now Cargo-checks that patched `slot-gfx` crate. The immutable `upstream/linkslot` branch is not modified. The current GitHub connector session has not returned an Actions run for the updated PR head, and this environment has no Rust compiler installed, so the implementation is **not yet marked build- or test-passed**. It has not been linked into `Frontend` with a software Compositor, connected to the H700 scanout buffer, or benchmarked on the device.

The first renderer deliberately omits the GL shader effects, bezel, full-display rotation, double buffering and physical cache maintenance. It is a host-testable raster core, not a display driver. Its `Draw` and `TexId` data shapes mirror upstream, but they remain separate Rust types until an integration patch makes both sides share the same crate types. and deliberately omits the GL shader effects, bezel, rotation of the full display, double buffering and physical cache maintenance. It is a host-testable raster core, not a display driver.

### Display memory facts and limits

The RG34XX live inventory reports a framebuffer at `0x7bf24800`, 720×480, 32 bpp, stride 2,880 bytes, size 1,382,400 bytes (`0x151800`), range `[0x7bf24800, 0x7c076000)`. That is a Linux live-FDT/bootargs reservation and must remain excluded from general payload allocation.

It is a **display-owned scanout candidate**, not approval to map/write it from LK. Before the renderer uses it, the H700 display path must prove that scanout remains active across the BL31-to-LK handoff, establish channel/format/order/stride/rotation and cache-maintenance requirements, and obtain explicit ownership of the buffer. Do not allocate a second 720×480×32bpp framebuffer at an unreviewed physical address.

The current Linkslot GL canvas is 720×480. Its 32-bit framebuffer alone is 1.38 MB; software texture caches and emulator frames are additional memory and must be bounded. Benchmark the 16.76 ms pacing target from the Linux device loop rather than assuming a CPU software renderer will meet it.

## 4. Minimal Rust runtime boundary

Create a narrow LK integration crate rather than adding the current complete Cargo workspace to the LK target.

The first no_std + alloc boundary should contain only:

- stable button/key enums, raw events and the `slot-input` gesture/state machine;
- pure machine/core/layout identifiers from `slot-model`, after separating path and filesystem helpers;
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
| Input | Poll a bounded event queue from the GPIO/ADC input driver; reuse Linkslot's logical mapping and gesture behavior. |
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
2. Define a renderer-independent `Draw` crate/trait contract and write host golden tests for `Rect`, `Tex`, `Turned`, `Game` and `Shot`.
3. Implement the software raster backend against a host buffer first; do not connect it to the physical framebuffer yet.
4. Split `slot-input` and the pure portion of `slot-model` into isolated no_std + alloc-compatible modules without changing their public behavior.
5. Add the H700 framebuffer/display handoff only after the live scanout and BL33 resource-ownership proof; similarly defer audio/input/power MMIO until each board service has its own evidence.
6. Keep the mGBA core as a separate static payload. Its linker map, BSS, stack, heap and ROM/save buffers must fit inside a subsequently approved application arena.

## 7. Explicit non-goals at this stage

- No attempt to run Linux AArch64 Linkslot ELF inside AArch32 LK.
- No reuse of Linux `/dev/fb0`, EGL/GLES, evdev, ALSA, SQLite or dynamic `.so` loaders.
- No arbitrary AArch64 SMC handoff.
- No guessed MEMBASE/MEMSIZE, heap, stack or core load address.
- No Boot0/TOC1/SD write, image installation or hardware payload execution.
- No claim of a bootable LK image or a working Linkslot port.

This is the integration boundary for the next implementation stage; the current project status remains host-only until the listed hardware gates are independently closed.
