# armGBA committed-frame counter layout — official H700 v1.16.0-pvaibhav.2

Date: 2026-11-26

## Exact source identity

The official H700 release ZIP slot-v1.16.0-pvaibhav.2-h700.zip has SHA-256:

1f9bdaa1110eb4f6006cc9da6a8bb34bca694ea53555c3138dcf656f0dfd1fca

The packaged metadata file System/licenses/armgba-df6b2631291f249d54a00dd14f49bc893e438feb.meta identifies the core source:

- Repository: MagnominusGit/armGBA
- Commit: df6b2631291f249d54a00dd14f49bc893e438feb
- Target: armgba_libretro
- Device flags: -mcpu=cortex-a53
- Patch: link-stop-queued-once.patch
- Patch SHA-256: 6ffb6a9f83b6fdaf77350725e74f0bedbd530256bce3e5e4fb0cc4b9e5e794db

The device core SHA-256 is 4ebae098cc4848fb459fc18299c39acbdbbbeb732db5c3f66d1ff3f6e142e889, byte-identical to the official ZIP core. ELF Build ID: 08309a288967a62cf4155e27d1b10ce13a5e33ef.

## Frame counter source and layout

The pinned source stores the GBA committed-frame counter at struct GBA::video.frameCounter. src/video/video.c increments video->frameCounter at VBlank frame end.

Exact-source compile probe result:

| Field | Offset |
|---|---:|
| sizeof(struct GBA) | 788,712 bytes |
| offsetof(struct GBA, video) | 11,944 |
| offsetof(struct Video, frameCounter) | 4 |
| **Combined gba->video.frameCounter offset** | **11,948 bytes** |

CI: [run #45724910634](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45724910634), artifact ID 15072724723, file armgba-frame-layout/armgba-frame-layout.json. The run was superseded/cancelled after the dedicated layout step completed; that step concluded **success**. Static offset assertions passed for AArch32 ARM, AArch32 Thumb-2 and AArch64. A host executable produced the same values.

This exact-source result happens to match the earlier 11,948-byte value calculated from a different, newer upstream commit. The older result was not evidence for this release; this report is the source-matched validation.

## Important boundary: symbol resolution still required

The full ELF .symtab exists, but .dynsym has no gba export. The .symtab contains two same-named LOCAL OBJECT entries at 0x4c16a8 and 0x4259b8. Source contains both the primary GBA pointer in libretro/libretro.c and a separate file-local pointer in libretro/link.c; the two ELF addresses must be tied to code references before choosing the primary pointer.

Before any frame-counter sampling, collect the current process mapping for armgba_libretro.so, determine the ELF load bias from PT_LOAD segments, and confirm which gba symbol is referenced by retro_run / the core's active GBA path. This document verifies source layout only; it does **not** authorize reading /proc/PID/mem. The previous ADB capture explicitly did not access process memory. No hardware registers were written.

## Reproduction

The work branch contains:

- tools/armgba-frame-layout-probe.c
- scripts/test-armgba-frame-layout.py

The script checks exact commit df6b2631291f249d54a00dd14f49bc893e438feb, the upstream GBA size assertion, and cross-compiles compile-time offset assertions for host, AArch32 ARM, AArch32 Thumb-2 and AArch64.
