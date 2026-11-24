# H700 no_std Linkslot frontend business crates

This directory is the first persistent device-side Rust business slice. It is derived from the immutable user-provided Linkslot 1.0.36 source snapshot on `upstream/linkslot` (snapshot commit `8590d6ff1b2caa26118051069c975106fbb14bd3`; this is an archive identity, not an upstream Git commit).

## Crate contents

- `slot-input`: Linkslot raw button events, button enum, gesture state machine and action vocabulary. The only runtime substitutions are `std::mem::take` → `core::mem::take` and explicit `alloc` imports.
- `slot-model`: Linkslot static machine/core tables, platform metadata, battery/charge types and card directory constants. Filesystem-specific `resolve_root(PathBuf)` and `Platform::accepts(&Path)` helpers are intentionally excluded; device path access belongs to the H700 VFS/runtime service, not the pure business model.
- `slot-frontend-state`: the pure `FacesLang` build coordinator extracted from Linkslot `Frontend`; it is generic over the language ID and owns only baked/building/generation state. Worker threads, texture construction, and result delivery remain the caller's responsibility.

The upstream source files are recorded by their Git blob IDs in the source manifest below. `slot-frontend-state` is extracted from the `FacesLang` state block in `crates/slot/src/frontend.rs` (upstream blob `6b6a99fd33a433a0d2aeacbe5da4efdb282f586b`); its worker/GL coupling was deliberately left in the host Frontend. The packages have no third-party crate dependencies and compile as `no_std`; allocation-bearing APIs use `alloc`. `slot-model` enables its `host-compat` default feature for the original Linux path helper API. Freestanding checks and the LK candidate must use `--no-default-features`, which removes those `std::path` helpers while preserving the domain data API. `slot-input` contains the versioned H700 C Input Service v1 adapter: C supplies ordered logical button edges, Rust `H700InputSource::drain_into()` maps them to the upstream `RawEvent` type without allocating a per-poll vector, and `H700GestureInput` forwards them into the original Linkslot gesture state machine. The 64-event C queue is a single-context primitive; board GPIO/ADC scanning is not yet implemented. The caller retains ownership of the C service table and queue.

## Target boundary

These crates do not read GPIO, MMIO, clocks, files, or framebuffer memory. The H700 C layer must provide the allocator and input/time services before device runtime code calls allocation-bearing APIs. Do not enable the allocator or call these crates from a booting target until the LK heap, memory ownership, and single-active-CPU gate are approved.

They are a selected business-logic slice, not a complete Linkslot frontend. The Linux `device_app`, `slot::App`, filesystem workers, network/WebDAV, SQLite, dynamic loading, full artwork/font stack and UI event loop have not been made bare-metal.

## Upstream provenance

| Upstream path | Git blob |
|---|---|
| `crates/slot-input/src/lib.rs` | `3202a9544fa84f6f28305fd3244f74287528ae33` |
| `crates/slot-input/src/gesture.rs` | `bad8f8705a0e9ee72aec656257536bf482d81ca9` |
| `crates/slot-input/src/raw.rs` | `97d993096b6fa25aab158cb667f9e460a2738882` |
| `crates/slot-model/src/lib.rs` | `640c96fb48eaa4cd811a4439a51133816e77a9ac` |
| `crates/slot-model/src/core.rs` | `d12cba8b59455a5219c05e277c106901d22c939f` |
| `crates/slot-model/src/layout.rs` | `95e0e7c78b8be8be9aa4e1eef554a05bf91290a2` |
| `crates/slot-model/src/machine.rs` | `45d7e243d5631344613031c66fe9f2638959f85e` |
| `crates/slot-model/src/power.rs` | `45274f94c281815ef114676e7ddb299ab3b65d19` |

The generic `FacesLang` state machine has six host tests for first build, matching language, rapid-change coalescing, stale-generation rejection, abandoned worker retry and synchronous completion. It is compiled as `no_std` for AArch32/AArch64 and is injected back into the real Frontend by the temporary source overlay.

Upstream workspace declares GPL-3.0-or-later. Preserve that licensing and attribution in all redistributed builds.
