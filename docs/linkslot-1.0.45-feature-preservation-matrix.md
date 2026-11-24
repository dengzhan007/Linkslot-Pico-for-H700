# Linkslot 1.0.45 feature-preservation matrix for the H700 bare-metal port

**Baseline:** archived Linkslot main 1.0.45; original upstream commit 25664ecd2a712279dfa840fbc1ea896ebc0ef0ba  
**Target:** RG34XX / H700, single frontend, first playable slice GB/GBC/GBA

This document records upstream semantics so features can be deferred without silently changing user data, wire compatibility or input behaviour.

## 1. Implementation boundaries

| Area | Linkslot 1.0.45 behaviour | H700 status | Priority |
|---|---|---|---|
| Frontend event/input/model | menus, state transitions, game shelf and shortcut dispatch | selected state logic extracted into no_std crates; C input service and cooperative event-loop slices compile | P0 |
| Software rendering | compositor and draw pipeline | H700 software framebuffer compositor and full Frontend renderer overlay compile | P0 |
| GBA core picker | mGBA fixed in slot 1; gpSP/armGBA hot slot plus tray; tray exchanges with slot 2 | generic CorePicker<C> + ThreeLayout<C> integrated into 1.0.45 App overlay | P0 |
| Game execution | Libretro lifecycle, video/input/audio, save RAM and state | mGBA AArch32 static archive and Libretro VFS v3 adapter compile/link-probe; real LK service runtime and playable loop not connected | P0 |
| ROM storage | Games/<platform>/, SD/GPT/FAT | read-only CMD17 → GPT → FAT32 → VFS host path tested; real controller/card execution unverified | P0 |
| Cartridge/save transfer | TCP/link transport plus filesystem assets | not ported; preserve contract below | P2 |
| Controls and SFC/MD input | controls.txt, custom X/Y mapping, MD 6-button controller | source audited; not a first GB/GBC/GBA blocker | P1 |
| Link sessions / cable | game-specific network/session/serialization | Linux std/thread/socket path is not portable; needs a bare-metal transport and cooperative session model | P2 |

P0 is required for the first playable GB/GBC/GBA vertical slice. P1 must be retained for later console support. P2 is deferred, not discarded.

## 2. 1.0.43 cartridge/save transfer contract

### 2.1 Persisted transfer modes

TransferMode has stable numeric values persisted in Linkslot app state:

| Code | Mode | Asset groups |
|---:|---|---|
| 0 | CartOnly (default) | ROM, cover label, intro text/video, shell choice |
| 1 | SaveOnly | battery save, resume state, resume thumbnail |
| 2 | All | both groups |

An unset config entry reads as CartOnly. An older offer that ends after shell metadata has no mode/core tail; 1.0.45 interprets that as the old cart-only behaviour. Keep this backward-compatible decode rule.

### 2.2 Assets and paths

| Asset | Source | Receive target |
|---|---|---|
| ROM | current cart path | Games/<platform>/<stem>.<ext> |
| cover | cart label or Labels/<platform>/<stem>.png | Labels/<platform>/<stem>.png |
| intro | Info/<platform>/<stem>.txt | same |
| video | Info/<platform>/<stem>.mp4 | same |
| battery save | Saves/<platform>/<stem>.sav, or legacy .srm when that is the existing file | canonical .sav target |
| resume state | States/<platform>/<core>/<stem>/resume.state | same |
| thumbnail | States/<platform>/<core>/<stem>/resume.png | same |

The offer carries the Core that produced its resume state. Save states are core-specific; place them under the same core directory. The wire core ID follows Core::EVERY, not arbitrary strings or local enum discriminants. Define a stable mapping and handle unknown codes safely.

CartOnly does not move battery saves, resume states, rewind data or playtime. SaveOnly never sends the ROM. A partial offer can send only missing side material plus selected save assets; the receiver must not infer that a missing ROM is an error for a partial offer.

### 2.3 Integrity and conflict policy

- Protocol framing uses magic SCX4, typed OFFER/ACCEPT/REJECT/DATA/DONE/ACK/CANCEL/DUPLICATE messages, and 32 KiB data chunks.
- Each DATA frame carries an asset kind, 64-bit little-endian file offset and payload. Receiver requires the offset to equal bytes already received; out-of-order data aborts the receive.
- The offer carries full SHA-256 of the ROM. Keep integrity checking, duplicate-cart detection and exact asset-size checks.
- Incoming files are written to temporary .part files. Verify full byte count, flush, then rename into place.
- Before replacing an existing .sav, resume.state or resume.png, 1.0.45 copies the old file to a timestamped .conflict.<stamp>.<ext> backup. Preserve the recovery copy if replacement fails.
- Cart stems must be unique across every supported ROM extension because a stem is also the key for save, state and cover paths.
- A full transfer can update shell metadata. A parts-only transfer must not change the cart shell choice.
- Legacy offers without transfer-mode/core tail remain cart-only. Do not reinterpret them as All.

### 2.4 Why the code cannot be copied directly into LK

cart_transfer.rs uses std::fs, File, OpenOptions, Read/Seek/Write, PathBuf, SystemTime, TCP/link radio abstractions, and Linkslot's existing LinkChannel. Those are Linux-host conveniences, not an LK runtime contract.

Preserve the wire and file semantics behind explicit interfaces:
1. A versioned C transport service (send/receive, peer state, cancellation, monotonic timeout), without Linux sockets.
2. A versioned C VFS/file service (open/read/write/seek/flush/rename/stat/mkdir/remove) with explicit ownership and errors.
3. A Rust no_std cooperative transfer state machine using bounded buffers, offsets and protocol parsing.
4. Power-loss-safe .part/commit/backup behaviour over the actual FAT backend.

This is deferred until the single-player path, storage and runtime services are stable. Do not link Linux std, threads, sockets, dynamic loader or POSIX shims into LK to shortcut it.

## 3. 1.0.44 controls/remapping semantics

controls.txt custom mappings apply before legacy layout transforms. The remapper remembers the logical button selected at physical key-down so the matching key-up releases the same logical button even if settings/layout change while held. Multiple physical keys may map to one logical button; emit logical Down on the first physical press and logical Up after the final physical release.

Custom bindings take precedence over default bindings. This matters for older configs where TurboA/TurboB aliases use physical X/Y keys now exposed as real SFC/MD X/Y defaults. A port must not let a default row mask an explicit user mapping.

1.0.44 added real SFC X/Y controls and controls.txt X/Y swap, and fixed SELECT+shoulder combination release leaving fast-forward/rewind latched. The first GB/GBC/GBA slice can defer SFC-specific actions, but keep the generic physical-down/logical-down pairing model so the fix is not lost.

## 4. 1.0.45 Mega Drive six-button behaviour

1.0.45 asks the Libretro core for DEVICE_MDPAD_6B on ports 0 and 1 when enabled, before ROM load so the first frame receives the right device. When disabled it uses the standard joypad. This is MD-specific and not required for GB/GBC/GBA, but preserve it when MD returns.

## 5. Static core / ISA policy

- mGBA and gpSP are the first target static cores for the AArch32 LK/BL33 path.
- 1.0.45 armGBA documentation describes an AArch64 recompiler. Do not assume it links into the current AArch32 BL33. Validate ISA, calling convention, toolchain, allocator and memory model separately.
- Only expose a core when the target build actually contains a compatible implementation.

## 6. Acceptance gates before transfer/session port

1. Validate physical RG34XX SD-controller read path with a read-only probe; host synthetic GPT/FAT tests are not enough.
2. Review write/flush/rename and power-loss recovery over the actual FAT backend before file-replacement features.
3. Specify transport ABI independently of Linux socket types and test bounded queues/cooperative deadlines.
4. Wire compatibility tests must cover 1.0.42 legacy CartOnly offers, all three transfer modes, .sav/.srm, core-specific resume state paths, partial transfer, duplicates, out-of-order chunks, cancellation and conflict backups.
5. Only then port the transfer state machine to no_std; do not block the first GB/GBC/GBA playable slice on this subsystem.

## 7. Source anchors

- crates/slot/src/cart_transfer.rs
- crates/slot/src/input/remap.rs
- crates/slot/src/session.rs
- crates/slot-retro/src/libretro.rs
- crates/slot/src/app.rs
- CHANGELOG.md (1.0.43–1.0.45)

## 8. Current validation evidence

- The complete 1.0.45 workflow passed 69/69 steps: [run 45373379245](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45373379245).
- Uploaded build/maps/ELF/log/report bundle: [artifact 14909943545](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45373379245/artifacts/14909943545), 381,283,589 bytes, not expired at the time of this audit.
- The same workflow validates the 1.0.45 App/Renderer overlay, three-core picker integration, C input ABI, no_std frontend crates, VFS v3, SD/GPT/FAT host path and AArch32 mGBA static build/link diagnostics.
- These are host/cross-build results. No full H700 LK image, physical SD-controller transaction, ROM-to-frame runtime, or successful RG34XX boot is claimed.
