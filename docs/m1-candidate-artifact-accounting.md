# M1 AArch32 diagnostic artifact accounting

**Review date:** 2026-11-28  
**Source branch:** `porting/linkslot-1.0.45-feasibility`  
**Source commit:** `0469a6e17a3fc88b979ad82621122143f35630dc`  
**Evidence:** Linkslot/LK feasibility [#943](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46086258522), artifact [15239048815](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46086258522/artifacts/15239048815).  
**Artifact ZIP SHA-256:** `43bee56b2238fc484284eb9ee0677bfe7a9ec9e8f5279dbc07d5358c8a351f89`.

## Decision

The CI artifact contains three distinct AArch32 layout prototypes. They are useful source/ELF evidence, but **none is a deployable M1 payload**:

1. `h700-early-led-layout.bin` links real pinned LK early-MMU/CP15 code, H700 target early-init and PI12 service. It explicitly opts into the PI12 physical MMIO implementation for disposable layout validation; its Data Abort handler fail-stops on an unmatched PI12 access. This is not hardware authorization.
2. `lk-bl33-overlay-layout.bin` is a small CRT0/exception overlay linked against stubs. It does not include the H700 target service or full LK platform runtime.
3. `lk-a32-bl33-layout.bin` is a separate entry/vector/snapshot prototype with stubs. Its header-overlay companion is synthetic.

No real vendor header was inserted into any of these binaries. No matched firmware, SPL, Boot0-primary capsule, or TF1 image was produced.

## Byte and memory accounting

All three raw files are below the active loader's 32,768-byte file extent. This only establishes a file-size comparison.

| Artifact | Raw bytes | File headroom | Runtime end / span | Interpretation |
|---|---:|---:|---|---|
| `h700-early-led-layout.bin` | 7,940 | 24,828 | `_end=0x4a00c024`, span `0xC024` (49,188 bytes) | MMU + PI12 layout test; physical access explicitly enabled in disposable link |
| `lk-bl33-overlay-layout.bin` | 5,804 | 26,964 | `_end=0x4a0036d0`, span `0x36D0` (14,032 bytes) | CRT0/exception overlay + tiny stubs |
| `lk-a32-bl33-layout.bin` | 5,148 | 27,620 | `_end=0x4a00343c`, span `0x343C` (13,372 bytes) | separate A32 entry/vector/snapshot prototype |

The early-LED ELF program headers report:
- executable LOAD: file/memory `0x1EEC`;
- read-only LOAD: file/memory `0x14`;
- RW LOAD: file `0x4`, memory `0xA124` (the remaining bytes are zero-fill/BSS);
- ELF entry: `0x4A000000`.

Its linker map separately verifies:
- early/abort stack: `[0x4A004000,0x4A006000)`, 8 KiB;
- initial stack top: `0x4A006000`;
- section table: `[0x4A008000,0x4A00C000)`, 16 KiB;
- `_end=0x4A00C024`;
- stack/table gap: 8 KiB, no overlap.

The linked memory span is materially larger than the raw file because BSS, stack, and page table occupy runtime memory. None of these measurements grants physical ownership of the BL33 slot or proves that BL31, Boot0, secure firmware, or DMA cannot overlap it.

## Artifact hashes

| File | Bytes | SHA-256 |
|---|---:|---|
| `h700-early-led-layout.bin` | 7,940 | `98b4104a055efb84f2c6864e8ed309a487ecedefa8fd0198fd91f0fed9d21228` |
| `h700-early-led-layout.elf` | 79,008 | `d1dea3f3927235382b10f329497c23e62199e115679de2f887f6fd25ba40ef64` |
| `h700-early-led-layout.map` | 18,015 | `2ac684ac31db5fe9fb78b0f53d623d09d297f036017028be2b844f2e6cf43971` |
| `lk-bl33-overlay-layout.bin` | 5,804 | `3ac441bffb03b2da61bb4489b026c935fe631e2ddb3551d779312625db8e2213` |
| `lk-bl33-overlay-layout.elf` | 74,032 | `d087b198fa016d8f99153be6e16ae3e26f7ad595e7dfe2774f13439138227eca` |
| `lk-bl33-overlay-layout.map` | 7,630 | `cfd92e586d81cac6f9cd60e9760d6d40d6f0585502d26f18c53ac0514e5349e3` |
| `lk-a32-bl33-layout.bin` | 5,148 | `6a63857beaabf637bd090d6f8a209405fd8c6300f4954321b0ef8ba23a43a7c2` |
| `lk-a32-bl33-layout.elf` | 72,820 | `e70af125bb4cba786cf20c75d7c3638cda70e169341f54e03d633f7578c2906f` |
| `lk-a32-bl33-layout.map` | 5,860 | `d88ffa11af3cface0fedea9dfc393841cf54b041def181e430f3b6d85ea4a627` |

Companion files `lk-bl33-overlay-layout-synthetic-header.bin` and `lk-a32-bl33-layout-header-overlay.bin` are explicitly synthetic header tests. They must never be treated as the deployed vendor header or as package inputs.

## Header and entry checks

The three raw files begin with the same A32 branch bytes `07 04 00 EA` (branch target `0x4A001024`). The tested raw binaries have zero bytes in the reserved vendor-header input range `[+4,+0x640)`, as required by the BaseOS native builder contract. The vector bytes are placed at `+0x1000`, outside that header range.

This does **not** prove that the real vendor header is available in this artifact. The standalone wrapper still requires the exact deployed 32 KiB reference loader SHA-256 `48b2a837ac5d8daad8f524c976f2b3911d26a838f8b68fa695e3283412dd8590`; that reference loader was not part of the extracted artifact set used for this accounting.

## Required next packaging boundary

Before a candidate can be promoted beyond host-only audit:

1. Choose exactly one minimal diagnostic composition. Do not merge the three prototypes by assumption.
2. Remove PI12 physical access from any candidate unless M1-S1 permission/clock/abort gates are explicitly approved; a mock service or dormant default-off wrapper is not equivalent to the current disposable MMIO-enabled ELF.
3. Link the actual selected objects with real target symbols and no placeholder stubs; produce ELF, map, disassembly and raw binary from the same build invocation.
4. Verify entry, vectors, reserved header range, raw extent, every LOAD segment, BSS, stack, page table, and `_end`; emit machine-readable JSON with hashes and interval checks.
5. Obtain the exact pinned vendor header template and build matched firmware generation/FNV, AArch64 SPL, and Boot0-primary capsule from one source generation. The M1 capsule must not silently reuse the RTC/WDT v2 attempt protocol.
6. Keep all output labeled `host_only=true`, `not_installable=true`, `hardware_validated=false` until memory ownership, CPU1–CPU3 reset/RVBAR, PI12 or alternative proof-of-life, and capsule recovery gates close.

## Safety boundary

This accounting is host-side analysis of a downloaded CI artifact. It performed no device MMIO/SMC, TWI transaction, DRAM content access, suspend/resume, TF write, Boot0/TOC1 write, firmware flash, or payload execution. Keep `H700_MEMORY_LAYOUT_APPROVED=false` and `hardware_applied=0`.
