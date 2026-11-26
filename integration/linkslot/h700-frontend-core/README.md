# H700 no_std Linkslot frontend business crates

This is the persistent Rust business/runtime extraction for the H700/LK port. The extraction history includes Linkslot 1.0.36 and 1.0.45 work; the **current overlay target and validation baseline is Linkslot main 1.0.49**. The immutable 1.0.49 main/test source snapshot is archived in `upstream/linkslot` at `241e7c1974926fa22552b200852914ceaccbfd42`; its original upstream main source commit is `b84f37f1a6259a625e9095279dd132f5ef2116af`. These identifiers differ: the former is this repository's archive commit, the latter is the original source commit recorded in `UPSTREAM-SNAPSHOT.md`. Extracted business crates retain per-crate source provenance; see [the 1.0.49/1.1.0 refresh audit](../../../docs/linkslot-1.0.49-1.1.0-refresh.md).

## Crates

| Crate | Extracted responsibility | Device-side boundary |
|---|---|---|
| `slot-input` | Linkslot button vocabulary, raw events, gesture state machine and action mapping | Includes H700 Input Service v1 adapter. C supplies ordered logical button edges; fixed raw-event storage and one preallocated Action buffer are reused per poll. |
| `slot-model` | Machine/core registry, platform metadata, battery/charge types and directory constants | Filesystem helpers are behind optional `host-compat`; LK uses `--no-default-features` and H700 VFS services. |
| `slot-frontend-state` | Generic `FacesLang<L>` resource-build state plus `GamePowerLedState` game-activity transition state | Worker threads and texture generation stay in the caller. LED state emits one C-service request per game start/exit transition; it does not perform MMIO. |
| `slot-core-picker` | Generic `CorePicker<C>` selection and animation state machine | Preserves single-core information-only behaviour, two-core hop and 1.0.45 three-position slot1/slot2/tray behaviour. |
| `slot-link-state` | `SLK1` paired-GBA save-state codec and BIOS identity comparison | Validates two length-prefixed states; BIOS inspection borrows its source slice. |
| `slot-link-kind` | GBA cable/wireless classification and Libretro serial-option mapping | Preserves the wireless table, clean retail Pokémon exception and ROM-hack cable fallback. |
| `slot-video-mode` | `VideoMode` values and normalized source-frame crop geometry | Allocation-free parse/crop policy; INI access stays in the host wrapper. |
| `slot-rom-id` | SHA-256 ROM identity, fixed-width hex conversion and incremental `RomHasher` | Pure hashing is `no_std`; file/stream acquisition stays in the host/VFS wrapper. |
| `slot-game-search` | Pinyin IME candidate index, ranking/pagination and cross-platform game-name hits | Injected `PinyinProvider` and generic platform identifier; scanning and dictionary storage stay outside. |
| `slot-hold-state` | Timed hold progress, completion and release-fade state | Generic `no_std` state machine; owner retains allow/deny policy and final action. |
| `slot-cpu-performance` | Rust client for the versioned H700 CPU Performance Service | Requests policy modes only; the C service owns evidence gates and hardware decisions. Runtime hardware writes remain disabled. |
| `slot-i18n` | Generated translation bindings from the pinned Linkslot translation sources | Generated during CI; locale loading and filesystem ownership stay in the platform layer. |
| `slot-quick-menu` | Quick Menu page/row catalogue, hierarchy, labels, key navigation and resume-action mapping | Pure presentation/navigation data; executing a row remains an application/service responsibility. |
| `slot-lk-runtime` | Cooperative single-core event-step loop joining monotonic time, H700 input and an application action handler | Samples the C-owned monotonic clock, drains ordered input/gesture actions, dispatches them, then ticks the application once. It does not schedule threads, own VFS/display/audio/core services, or perform MMIO. |
| `slot-app-router` | Pure Quick Menu, SearchResults and SearchKeyboard action routing from upstream `Action` values to typed app commands | Handles only an already-active matching phase; UI cursor, IME/query/candidates, SearchHit and all side effects remain App/UI-owned. |

The persistent workspace contains **15 packages**, including the LK event-step runtime and Quick Menu app router. They are built in host mode and checked with `--no-default-features` for both AArch32 and AArch64; packages that own bounded collections use `alloc` and must not be initialized before the LK heap and approved memory arena. The H700 target must not call allocation-bearing interfaces until the LK heap and approved memory arena are initialized. `slot-model` host path helpers require its default `host-compat` feature; freestanding builds use `--no-default-features`. The `slot-core-picker` crate uses the small `libm` dependency; `slot-rom-id` uses no_std `sha2`; `slot-app-router` depends only on in-tree `no_std` crates. No new third-party runtime dependency was added.

## C/Rust boundary and current limits

The H700 input service is a versioned C ABI with a 64-entry C FIFO. The queue currently supports one execution context only; it is not ISR-safe or multi-producer safe. Rust `H700GestureInput` allocates a 72-action buffer once at construction; the maximum 32-event batch plus timer actions fit without per-poll growth. The C GPIO/ADC input scanner and physical input producer are not yet implemented. Separately, an RG34XX-only PI12 work-LED service now exposes a narrow C ABI; Linkslot `Session` calls it only on game-phase transitions.

The Rust business crates do not perform MMIO, read physical clocks, mount storage, own framebuffer memory or drive display/audio hardware. PI12 writes remain in the target-owned C LED service; Rust carries only the transition state and calls the service ABI. `slot-lk-runtime` is now part of the persistent workspace and supplies a tested cooperative time/input/action/tick loop, but it is only the runtime floor: the full `slot::App` graph, Linkslot VFS/backend lifecycle, display/scanout, audio and static-core run loop are not yet connected into a bootable LK application. Filesystem workers, network/WebDAV, SQLite, dynamic loading and the full artwork/font stack remain later work.

The temporary integration overlay keeps `upstream/linkslot` immutable and injects the extracted crates back into the original Linkslot main 1.0.49 workspace. The focused CI workflow [H700 Linkslot runtime slices](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/workflows/h700-linkslot-runtime.yml) runs host tests and AArch32/AArch64 `no_std` checks for every persistent workspace member (now including `slot-lk-runtime` and `slot-app-router`), upstream input/model regression, and complete `slot` Frontend compilation with renderer and business overlays. The 1.0.49 focused runtime-slices run [#45742086671](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45742086671) **passed** with the 15-member workspace, including `slot-lk-runtime` and `slot-app-router`'s three phase routes plus the phase-aware dispatcher. It validates host tests (default and no-default features), `armv7a-none-eabi` and `aarch64-unknown-none` checks, upstream input/model regressions, complete Linkslot 1.0.49 Frontend overlay compilation, CPU Performance QuickRow fail-closed tests, and the C input ABI/queue host + AArch32/AArch64 checks. All 23 app-router tests and the three event-step tests run in both workspace host test modes. These are host/cross-build results, not a bootable LK image or proof that App::apply has been patched. The broader feasibility run remains a separate check.

## Upstream provenance

| Upstream file | Git blob |
|---|---|
| `crates/slot-input/src/lib.rs` | `3202a9544fa84f6f28305fd3244f74287528ae33` |
| `crates/slot-input/src/gesture.rs` | `bad8f8705a0e9ee72aec656257536bf482d81ca9` |
| `crates/slot-input/src/raw.rs` | `97d993096b6fa25aab158cb667f9e460a2738882` |
| `crates/slot-model/src/lib.rs` | `640c96fb48eaa4cd811a4439a51133816e77a9ac` |
| `crates/slot-model/src/core.rs` | `d7fe752e7968d00dafaa3dbadacf2a749580a64e` |
| `crates/slot-model/src/layout.rs` | `95e0e7c78b8be8be9aa4e1eef554a05bf91290a2` |
| `crates/slot-model/src/machine.rs` | `45d7e243d5631344613031c66fe9f2638959f85e` |
| `crates/slot-model/src/power.rs` | `45274f94c281815ef114676e7ddb299ab3b65d19` |
| `crates/slot/src/frontend.rs` (`FacesLang`) | `6b6a99fd33a433a0d2aeacbe5da4efdb282f586b` |
| `crates/slot/src/core_picker.rs` | `a46bfc6afa8757edd5a3254e74dad9ddcc0193c8` |
| `crates/slot/src/link_state.rs` | `4977eb3c563fb3d7b8fede8638b0b96c1fd36502` |
| `crates/slot/src/link_kind.rs` | `df8cccd1c78b7c88a53e11128c630ff0bfc05ecb` |
| `crates/slot/src/video_mode.rs` | `6cf21c2cae7fecfb2c090ab8c726fabf38442838` |
| `crates/slot/src/rom_id.rs` | `2479194beb9a055ad064fbab39009c4c4a0a4337` |
| `crates/slot/src/game_search.rs` | `167bc3d03ca02db7ec2cbe5f132b7a3ad5f3603b` |
| `crates/slot/src/app.rs` (`FolderHold`) | `74401b9599b24bdfacef4bd913f96e8de8d88bf1` |

The generic `FacesLang` state machine has six tests. The H700 `CorePicker<C>` has twelve tests covering open timing, navigation, edge refusal, reversal continuity, single-core behaviour, two-core write outcome, three-position layout validation, active-vs-highlight state, direct slot selection, tray/slot2 swap, initial tray normalization and invalid-layout fallback. The paired-state codec has seven tests; link-kind policy has eight; video-mode parsing/cropping has six. The input adapter and gesture bridge have host regressions covering event order, bounded draining, invalid service tables, menu long-press and power long-press behavior. Every package is checked for both `armv7a-none-eabi` and `aarch64-unknown-none`.
The ROM identity crate has four tests for the standard SHA-256 vector, upper/lowercase hex round trips, malformed hex rejection and incremental-versus-single-buffer equivalence. The host wrapper retains the original file/reader tests.
The game-search crate has four host tests for full pinyin and initials, cross-platform hits, candidate pagination, empty dictionaries and stable shelf indices; it is checked on both AArch32 and AArch64. Its runtime-facing API is generic over platform IDs and obtains pinyin through an injected provider.
The gesture adapter also compares `feed_into()`/`tick_into()` against the original allocating API across menu, power, volume/mute, fast-forward, rewind and Select/chord sequences, and checks that the reusable action buffer does not grow.
The timed-hold crate has seven tests for hidden tap feedback, progress timing, one-shot completion, release fade, disallowed cancellation, replacement/cancel, and backwards monotonic timestamps.

The `slot-model` core registry was re-synced to the 1.0.45 upstream blob to include armGBA metadata; the H700-specific `no_std`/`host-compat` changes in model/lib.rs, machine.rs and layout.rs remain intentional. The upstream workspace declares GPL-3.0-or-later. Preserve that license and attribution in redistributed builds.


## LK event-step runtime (new persistent workspace member)

`slot-lk-runtime` is now included in this workspace and aligned to the Linkslot 1.0.49 validation baseline. The crate provides a cooperative, single-active-thread step:

1. Read one monotonic timestamp from the versioned C `H700TimeServiceV1`.
2. Drain at most the bounded H700 input batch and translate it through Linkslot's existing `Gestures` state machine.
3. Dispatch ordered actions to an application-owned `ActionHandler`.
4. Call the application tick once using the same monotonic timestamp.

Host regression covers dispatch ordering, menu-hold timer behavior, power edge sequencing and rejection of invalid service tables. The workspace CI now includes those tests and both freestanding targets. The runtime does not own an LK scheduler or heap, and it does not yet call the VFS v3 bridge, display/scanout, PCM, or emulator core; those services must be integrated behind explicit C-owned interfaces after their individual lifecycle and memory gates are approved.

`slot-lk-runtime` consumes only the versioned monotonic-time and input-service tables. Its test doubles are host-only. No device MMIO, hardware timer setup, GPIO scanning, framebuffer access, storage write or core execution is performed by this crate.


## Quick Menu application routing (new persistent workspace member)

`slot-app-router` extracts the exact input-to-command transition from upstream Linkslot 1.0.49 `App::quick_menu_input()` without copying the monolithic App or taking ownership of its side effects:

- Up/down use the existing `QuickRow::up/down` navigation table and return `CursorMoved`.
- Left/right return `ChangeSetting { row, right }`; the application owner still calls its existing setting mutation/persistence logic.
- A returns `OpenRow { row }`; the owner decides whether this is a nested page, utility/modal, or leaf action.
- B/MENU return `Parent { row }` when a parent exists, otherwise `Close`.
- All other actions return `Ignore`.

The router is called only while the application is in its Quick Menu phase. It intentionally does not globally intercept MENU: upstream Shelf uses MENU to open the Quick Menu, Playing uses it for the in-game utility menu, and overlays/search have their own handlers. Search-results navigation is now routed through typed commands; search keyboard cursor/IME state, pinyin composition, candidate generation and actual library hit selection remain owned by the full app.

The Quick Menu route has six host tests for entry row, cursor movement, left/right delegation, A delegation, parent/root exit, ignored actions and page-local navigation. SearchResults adds six tests for vertical wrap, seven-row page movement, viewport visibility, A/Insert, B/X/MENU and empty-list boundaries. No file, configuration, input device, display, audio or hardware side effect is performed.


### Search-results routing

`SearchResultsRouter` now matches upstream Linkslot 1.0.49 `App::search_results_input()`:

- `SEARCH_ROWS = 7`, matching the upstream viewport.
- Up/down wrap through the complete hit list; left/right jump by seven entries and clamp at the first/last hit.
- The selected index keeps the viewport offset visible using the same `keep_search_result_visible` rule as upstream.
- A/Insert emit `SelectHit { index }`; B emits `ReturnToKeyboard` so the app can restore the prior query; X/MENU emit `Close`.
- The router stores only hit count, selected index and viewport offset. Query, pinyin composition/candidates, SearchHit records and shelf/folder mapping remain app-owned.

The app-router now has **19 host tests**: six Quick Menu tests, six SearchResults tests and seven SearchKeyboard tests. It remains a typed routing extraction, not a connected full App adapter.


### Search keyboard and Pinyin candidate routing

`route_search_keyboard(action, snapshot)` now mirrors the pure input transition in upstream Linkslot 1.0.49 `App::game_search_input()` while preserving the existing UI/App state owners:

- Select enters candidate focus only if composition is nonempty and candidates are available; when focused, Select exits candidate focus.
- Up/Down while focused move candidate pages with wrap; Left/Right move the selected candidate with wrap; A commits the selected candidate; B leaves candidate focus.
- Without candidate focus, Up/Down/Left/Right return cursor-movement intents for the existing `slot_ui::keyboard::Cursor`; Y cycles its layout; A activates the existing cursor key; B backs up composition/text; Start confirms search; X/MENU closes.
- The snapshot carries only candidate focus/count/page/pages/selected and whether composition exists. It carries no query, strings, dictionary, rendered key geometry or SearchHit. The caller refreshes candidates and applies the typed command using the existing injected PinyinProvider and App helpers.

The keyboard router contributes seven tests for focus entry/exit, candidate page wrap, candidate selection wrap, empty/invalid candidate boundaries, cursor/key intents, and Backspace/Confirm/Close dispatch.


### Phase-aware App input dispatcher

`route_app_input(owner, action)` composes the pure routes without changing the upstream input-priority tree:

- `Shelf + QuickMenu` => `OpenQuickMenu`; `Shelf + X` => `OpenGameSearch`.
- `Playing + QuickMenu` => `OpenInGameMenu`, preserving the different in-game meaning.
- `QuickMenu`, `SearchResults` and `SearchKeyboard` owners dispatch to their respective typed routers.
- `Other` delegates the action back to the existing App handler.

The caller must resolve App::apply overlay/modal priority first (power menu, upgrade, intro, sync, search/keyboard/popup/utility and other modal owners) and supply the already-active input owner. The dispatcher does not scan App state, reorder overlays, mutate the upstream App, or execute any returned side effect. It is the adapter seam for the next source-integration step, not yet a patched App::apply implementation.

Four tests cover Shelf-vs-Playing MENU, Shelf search open, active QuickMenu row dispatch, SearchResults/Keyboard ownership and delegation to other overlays. The app-router now has **23 tests** across its three pure routers and phase-aware dispatcher.
