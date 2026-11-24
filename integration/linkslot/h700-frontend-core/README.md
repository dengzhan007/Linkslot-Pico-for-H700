# H700 no_std Linkslot frontend business crates

This is the persistent Rust business-layer extraction for the H700/LK port. Sources are derived from the immutable user-provided Linkslot 1.0.36 snapshot on `upstream/linkslot` at archive commit `8590d6ff1b2caa26118051069c975106fbb14bd3`. That SHA identifies the archived Git snapshot in this repository; it is not claimed to be an original upstream Git commit.

## Crates

| Crate | Extracted responsibility | Device-side boundary |
|---|---|---|
| `slot-input` | Linkslot button vocabulary, raw events, gesture state machine and action mapping | Includes H700 Input Service v1 adapter. C supplies ordered logical button edges; fixed raw-event storage and one preallocated Action buffer are reused per poll. `feed_into()`/`tick_into()` avoid temporary action vectors while original `feed()`/`tick()` APIs remain compatible. |
| `slot-model` | Machine/core registry, platform metadata, battery/charge types and directory constants | Filesystem helpers are behind optional `host-compat`; LK uses `--no-default-features` and H700 VFS services. |
| `slot-frontend-state` | Generic `FacesLang<L>` resource-build generation state machine | Owns only baked/building/generation state. Worker threads, texture generation and result delivery stay in the caller. |
| `slot-core-picker` | Generic `CorePicker<C>` selection and animation state machine | Preserves single-core information-only behavior, edge refusal, hop reversal, easing and selected-core write result. Uses `libm` for no_std trigonometry. |
| `slot-link-state` | `SLK1` paired-GBA save-state codec and BIOS identity comparison | Validates exactly two length-prefixed states; BIOS inspection borrows the source slice rather than cloning the whole save state. |
| `slot-link-kind` | GBA cable/wireless classification and Libretro serial-option mapping | Keeps the 43-code wireless table, clean retail Pokémon exception and ROM-hack cable fallback. |
| `slot-video-mode` | `VideoMode` values and normalized source-frame crop geometry | Allocation-free parse/crop policy. INI file access remains in the host-side wrapper. |
| `slot-rom-id` | SHA-256 ROM identity, fixed-width hex encoding/decoding and incremental `RomHasher` | Pure hashing is `no_std`; file/stream acquisition stays in the host/VFS wrapper. The hasher supports bounded chunk-by-chunk hashing. |
| `slot-game-search` | Pinyin IME candidate index, candidate ranking/pagination and cross-platform game-name hits | Accepts an injected `PinyinProvider` and generic platform identifier; game-directory scanning and the large pinyin table stay outside the crate. |
| `slot-hold-state` | Timed hold progress, completion and release-fade state | Generic `no_std` state machine. The owner still decides whether the hold is allowed and what action completion triggers. |

All ten packages are `no_std`; APIs that own collections use `alloc`. The H700 target must not call allocation-bearing interfaces until the LK heap and approved memory arena are initialized. `slot-model` host path helpers require its default `host-compat` feature; freestanding builds use `--no-default-features`. The `slot-core-picker` crate uses the small `libm` dependency; `slot-rom-id` uses no_std `sha2`; the other eight crates have no external runtime dependencies.

## C/Rust boundary and current limits

The H700 input service is a versioned C ABI with a 64-entry C FIFO. The queue currently supports one execution context only; it is not ISR-safe or multi-producer safe. Rust `H700GestureInput` allocates a 72-action buffer once at construction; the maximum 32-event batch plus timer actions fit without per-poll growth. The C GPIO/ADC scanner and physical input producer are not yet implemented.

These crates do not perform MMIO, read physical clocks, mount storage, own framebuffer memory or drive display/audio hardware. They are selected business-logic slices, not a complete Linkslot frontend. The Linux `device_app`, the full `slot::App` state graph, filesystem workers, network/WebDAV, SQLite, dynamic loading, full artwork/font stack and the LK main event loop are not yet a bootable device application.

The temporary integration overlay keeps `upstream/linkslot` immutable and injects the extracted crates back into the original Linkslot workspace. The focused CI workflow [H700 Linkslot runtime slices](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/workflows/h700-linkslot-runtime.yml) verifies host tests, AArch32/AArch64 `no_std` compilation, upstream input/model regression, and complete `slot` Frontend compilation with the renderer and business overlays. Its latest verified run is [45362859979](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45362859979), which verifies all ten packages, reusable gesture output, timed folder-hold integration and the full Frontend.

## Upstream provenance

| Upstream file | Git blob |
|---|---|
| `crates/slot-input/src/lib.rs` | `3202a9544fa84f6f28305fd3244f74287528ae33` |
| `crates/slot-input/src/gesture.rs` | `bad8f8705a0e9ee72aec656257536bf482d81ca9` |
| `crates/slot-input/src/raw.rs` | `97d993096b6fa25aab158cb667f9e460a2738882` |
| `crates/slot-model/src/lib.rs` | `640c96fb48eaa4cd811a4439a51133816e77a9ac` |
| `crates/slot-model/src/core.rs` | `d12cba8b59455a5219c05e277c106901d22c939f` |
| `crates/slot-model/src/layout.rs` | `95e0e7c78b8be8be9aa4e1eef554a05bf91290a2` |
| `crates/slot-model/src/machine.rs` | `45d7e243d5631344613031c66fe9f2638959f85e` |
| `crates/slot-model/src/power.rs` | `45274f94c281815ef114676e7ddb299ab3b65d19` |
| `crates/slot/src/frontend.rs` (`FacesLang`) | `6b6a99fd33a433a0d2aeacbe5da4efdb282f586b` |
| `crates/slot/src/core_picker.rs` | `1e564d676e0842c59f34c1874e33fc8dcf7daf70` |
| `crates/slot/src/link_state.rs` | `4977eb3c563fb3d7b8fede8638b0b96c1fd36502` |
| `crates/slot/src/link_kind.rs` | `df8cccd1c78b7c88a53e11128c630ff0bfc05ecb` |
| `crates/slot/src/video_mode.rs` | `6cf21c2cae7fecfb2c090ab8c726fabf38442838` |
| `crates/slot/src/rom_id.rs` | `2479194beb9a055ad064fbab39009c4c4a0a4337` |
| `crates/slot/src/game_search.rs` | `167bc3d03ca02db7ec2cbe5f132b7a3ad5f3603b` |
| `crates/slot/src/app.rs` (`FolderHold`) | `74401b9599b24bdfacef4bd913f96e8de8d88bf1` |

The generic `FacesLang` state machine has six tests. `CorePicker<C>` has six tests for open timing, navigation, edge refusal, reversal continuity, single-core behavior and write outcome. The paired-state codec has seven tests; link-kind policy has eight; video-mode parsing/cropping has six. The input adapter and gesture bridge have host regressions covering event order, bounded draining, invalid service tables, menu long-press and power long-press behavior. Every package is checked for both `armv7a-none-eabi` and `aarch64-unknown-none`.
The ROM identity crate has four tests for the standard SHA-256 vector, upper/lowercase hex round trips, malformed hex rejection and incremental-versus-single-buffer equivalence. The host wrapper retains the original file/reader tests.
The game-search crate has four host tests for full pinyin and initials, cross-platform hits, candidate pagination, empty dictionaries and stable shelf indices; it is checked on both AArch32 and AArch64. Its runtime-facing API is generic over platform IDs and obtains pinyin through an injected provider.
The gesture adapter also compares `feed_into()`/`tick_into()` against the original allocating API across menu, power, volume/mute, fast-forward, rewind and Select/chord sequences, and checks that the reusable action buffer does not grow.
The timed-hold crate has seven tests for hidden tap feedback, progress timing, one-shot completion, release fade, disallowed cancellation, replacement/cancel, and backwards monotonic timestamps.

The upstream workspace declares GPL-3.0-or-later. Preserve that license and attribution in redistributed builds.
