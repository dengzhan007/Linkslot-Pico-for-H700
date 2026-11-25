# RG34XX AArch64 probe: real-assembler build, SRAM audit, framebuffer observability

Host-only follow-up to `rg34xx-stage0-and-a64-probe.md`. Nothing was written to the
device, nothing was flashed, no SMC was issued, no payload was executed on hardware.
The probe binaries exist only under `build/` and are not committed, not uploaded and
not wrapped in any installable container. No serial number, MAC address or local
absolute path appears in this report.

Inputs: the stage-0 capture `build/rg34xx-inventory-20261122-214708/` (boot0 at LBA 16
and 512, TOC1 package, boot partition, live FDT, GPT text, mounts, cmdline) and the
BaseOS tree at `cbe253a43f46cf551cc9a6c79a84015bde3686cf`.

---

## 0. Headline

| Item | Result |
| --- | --- |
| SRAM map | A1 `0x20000`–`0x28000`, SRAM C `0x28000`–`0x58000`, confirmed against the TF-A H616 header. The deployed 128 KiB capsule *does not fit in A1*: 0x1C753 bytes of its non-zero content extends to `0x3C753`, i.e. 0x14753 bytes into SRAM C. |
| First SRAM C access | BROM itself, when it places the capsule; the first code that *chooses* to use SRAM C is the capsule dispatcher (`sp = 0x57000`, staging at `0x50000`/`0x52000`). |
| BROM-time ownership of SRAM C | **Accessible in practice, ownership UNKNOWN.** The vendor Boot0's own geometry and 40 of its literals require SRAM C to work, but the SYSCON bits that arbitrate SRAM C between CPU and display engine were not read on the device. |
| Full 128 KiB load | The dispatcher *requires* it (both FNV checks run before any copy, and one of the two extents lies entirely in SRAM C), and the unit demonstrably runs the native path. The agent that performs the load is still **UNKNOWN**. |
| Real assembler build | `zig cc` 0.17.1 (clang 22.1.8 + lld), target `aarch64-freestanding`. Baseline probe rebuilt: 12,288 bytes, `3cb5c440…`, 144 instructions, **118 byte-identical, 26 differences all classified**, zero relocations, reproducible. |
| Framebuffer probe | 18,432 bytes, `a6ec1ae3…`, gates on four independent FDT facts, 9/9 mutation tests behave correctly, 23,877 pixels written, **0 writes outside the reserved region**. |
| Hardware readiness | **Not yet.** The probe is qualified as an object; observability is qualified only in the "writes the right memory" sense. Whether the panel is still scanning at that moment is unproven, so the run would currently produce evidence only if the framebuffer is live. |

---

## 1. Method and read-only boundary

| Item | Rule applied |
| --- | --- |
| Device | Not touched this round at all. No adb command was issued. |
| Local analysis | `boot0-primary.bin`, `boot0-secondary.bin`, `live-fdt.dtb`, TOC1 package — byte analysis only. |
| Toolchain | `zig` 0.17.1 from the PyPI wheel `ziglang-0.17.1-py3-none-win_amd64` (SHA-256 `69c3dfc3…526fb`, matching the index), installed under `build/toolchain/` and never added to `PATH`. |
| Disassembly | capstone 5.0.9, AArch64. |
| Execution | The 12 KiB baseline probe runs under a host-side AArch64 subset interpreter. The framebuffer probe's gating logic runs as a **host build of the same C file**, not as a rewrite. No ARM64 code was executed. |
| External references | TF-A `plat/allwinner/sun50i_h616/include/sunxi_mmap.h` and Linux `arch/arm64/include/asm/image.h`, both fetched read-only. |

Two evidence levels are used throughout and are never mixed:

* **measured** — recomputed from captured bytes or produced by a build in this round;
* **source** — read from the BaseOS tree or from an upstream header;
* **UNKNOWN** — no host-side evidence exists, and none is invented.

---

## 2. SRAM usage audit

### 2.1 The map

From TF-A `sunxi_mmap.h` (`source`):

```
SUNXI_SRAM_A1_BASE 0x00020000   SUNXI_SRAM_A1_SIZE 0x00008000   -> 0x20000..0x28000
SUNXI_SRAM_C_BASE  0x00028000   SUNXI_SRAM_C_SIZE  0x00030000   -> 0x28000..0x58000
SUNXI_SRAM_BASE    0x00020000   SUNXI_SRAM_SIZE    0x00038000   -> 0x20000..0x58000
SUNXI_ROM_BASE     0x00000000   SUNXI_ROM_SIZE     0x00010000   -> BROM, not SRAM
```

This matches the ranges given in the task exactly, so the A1/SRAM C split used below is
the vendor-documented one, not an assumption.

### 2.2 Complete physical-address table

Offsets in the capsule column are `field offset + 0x20000`; the container is loaded so
that file offset 0 lands on `0x20000` (`source`: `native/capsule/contract.json`
`outer_layout`, cross-checked against the disassembled dispatcher).

| Region | Address range | Size | Who writes it | Evidence | Status |
| --- | --- | --- | --- | --- | --- |
| Capsule header + dispatcher | `0x20000`–`0x20883` | `0x884` | BROM | non-zero run measured in `boot0-primary.bin`; entry branch `b 0x20400` | measured |
| Capsule dispatcher literals | `0x20490`/`0x20494` hold `0x57000`/`0x50000` | — | — | disassembly of the dispatcher | measured |
| Vendor Boot0 copy (`STOCK`) | `0x21000`–`0x21AEF`, `0x22300`–`0x30FFF` | 64 KiB declared | BROM | `contract.json stock_pa = 135168 = 0x21000`; non-zero runs | measured |
| Native SPL copy (`NATIVE`) | `0x31000`–`0x3C753` | 48 KiB declared | BROM | `native_pa = 200704 = 0x31000`; single non-zero run | measured |
| Capsule padding | `0x3D000`–`0x40000` | `0x3000` | — | zeros in the capture | measured |
| Copy stub | `0x50000`–`0x500B0` | `0xB0` | dispatcher (`prepare_copy`) | `copy_stub_pa = 327680`; code copies `copy_start..copy_end` | measured |
| Saved 72-byte context | `0x52000`–`0x52048` | `0x48` | dispatcher | `context_pa = 335872`; 72 bytes copied | measured |
| Dispatcher stack | top `0x57000`, grows down | used depth well under 1 KiB | dispatcher | `private_stack_top = 356352`; `ldr sp, =0x57000` in `entry.S` | measured |
| SPL code after relocation | `0x20000`–`0x2C000` | `0xC000` | copy stub | relocator target `0x20000`, length `0xC000` | measured |
| SPL BSS | `0x30000`–`0x34000` | `0x4000` | SPL | `build-early-mmc.py` sets `SPL_BSS_START_ADDR 0x30000`, `SPL_BSS_MAX_SIZE 0x4000` | source |
| SPL stack | top `0x52A00`, grows down | — | SPL | `0x52a00` literal at native `+0x4B40`; `PROJECT-HANDOFF.md` | measured + source |
| SPL card staging | `0x38000` | 64 KiB read buffer | SPL (`early-mmc.c`) | `u32 *source = (void *)0x38000` with a `0xEA0004BE` guard | source |
| Loader (later stage) | DRAM from `0x4A000000` | — | vendor BL31 | previous reports | measured |

Which of these land in SRAM C, and how far:

```
0x20000  0x28000                 0x30000      0x3C753      0x50000   0x57000 0x58000
   |--------A1--------|------------- SRAM C ---------------------------------|
   [capsule hdr+disp ][stock tail  ][native SPL 48 KiB     ][stub][ctx][stack]
                      ^             ^        ^
                      |             |        └ SPL BSS 0x30000..0x34000
                      |             └ SPL code runs 0x20000..0x2C000 (0x28000.. in SRAM C)
                      └ vendor Boot0 content continues to 0x30FFF
```

### 2.3 Which code touches SRAM C first

Ordered by the moment of first effect, not by address:

1. **BROM, while loading.** The capsule's declared `image_size` is `0x20000` and its
   non-zero content reaches `0x3C753`, so the loader must have written SRAM C from
   `0x28000` upward. This is a write, not a deliberate use.
2. **Capsule dispatcher, first instruction block.** `entry.S` does `ldr sp, =0x57000`
   and calls `capsule_main`, which writes the 0xB0-byte copy stub to `0x50000` and the
   72-byte saved context to `0x52000` before it verifies anything. Those are the first
   *chosen* uses of SRAM C, and they sit in its top 8 KiB.
3. **Dispatcher FNV checks** read `0x21000` (partly in A1) and `0x31000`–`0x3C000`
   (entirely in SRAM C).
4. **Copy stub** reads `0x31000`–`0x3C000` and writes `0x20000`–`0x2C000`.
5. **SPL** clears BSS at `0x30000`–`0x34000` and sets its stack to `0x52A00`.

Note the phase reuse that this ordering implies, and that it is safe only because the
stages are sequential: the SPL's BSS clear overwrites `0x30000`–`0x30FFF`, which still
holds the tail of the vendor Boot0 image, and `0x31000`–`0x33FFF`, the head of the
native extent — both dead by then because the copy to `0x20000` has completed.

### 2.4 Is SRAM C CPU-accessible when the BROM starts?

Three separate claims, deliberately not merged:

* **Accessible in practice — strongly supported.** The vendor Boot0 declares
  `image_size = 0x10000` and runs at `0x20000`, so its own layout extends to
  `0x30000`; its measured non-zero content reaches `0x30FFF`. Its code also holds 40
  literals that point into SRAM C, for example `0x2B4F8`, `0x2B5C0`, `0x2AF10`,
  `0x2E1A8`, `0x2E810`, `0x2E1BC`. A vendor stage that stores its own pointers at
  `0x2Exxx` cannot be running with SRAM C inaccessible.
* **Ownership — UNKNOWN.** SRAM C is arbitrated by SYSCON: BaseOS borrows it with
  `system[0] |= 1; system[1] |= 1 << 24` before using it as SPL stack
  (`spl/probe-copy.c`) and gives it back with `system[0] &= 1u << 31;
  system[1] &= ~(1u << 24)` (`native/loader.c`, `release_display_sram`). The
  dispatcher performs **no SYSCON write at all**; its only use of `0x03000024` is in the
  SPL's AArch32 stub, where it reads the SoC revision byte to choose between
  `0x09010040` and `0x08100040` as the RVBAR address. Whether the BROM leaves SRAM C on
  the CPU side, or the capsule simply runs before anything contends for it, was not
  determined. The device's SYSCON registers were not read.
* **Conflict with VE/DE — UNKNOWN.** TF-A calls SRAM C "borrowed"; the BaseOS SPL
  README says "keep the entire warm probe and stack within A1; peripheral ownership of
  C may change", and yet the deployed capsule deliberately uses `0x50000`–`0x57000`.
  Both statements are consistent with the observed behaviour (the unit boots, so
  nothing clobbered those bytes) but neither is a proof of ownership.

### 2.5 Does the dispatcher depend on a full 128 KiB BROM load?

Structurally, yes, and this is not a matter of interpretation:

```
capsule_main()
  header_valid()                      reads 0x20010 and 0x20004..0x2000B
  fnv(STOCK,  0x10000) == STOCK_FNV   covers 0x21000..0x30FFF  (A1 and SRAM C)
  fnv(NATIVE, 0xC000)  == NATIVE_FNV  covers 0x31000..0x3CFFF  (SRAM C only)
  ... only then prepare_copy()
```

Both digests are computed **before** a single byte is copied, so the whole 128 KiB must
already be resident. Corroboration from behaviour: the unit runs the native SPL, which
means the `0x31000` digest matched, which means those 48 KiB were readable at that
moment.

What remains UNKNOWN is the mechanism. The eGON header asks for `0x20000` bytes, which
is 64 KiB beyond the 32 KiB A1 window, and no BROM-side trace exists in this capture.
"This works, so the loader must do it" is an inference from observed behaviour, not a
measurement, and it is labelled as such.

### 2.6 Do the copy stub and the dispatcher stack collide?

| Pair | Overlap | Verdict |
| --- | --- | --- |
| stub `0x50000`–`0x500B0` vs context `0x52000`–`0x52048` | none | disjoint |
| context `0x52000`–`0x52048` vs dispatcher stack (top `0x57000`) | none unless the dispatcher used > 0x4FB8 bytes of stack | the deepest observed path (RTC + FNV over 64 KiB) uses well under that |
| SPL stack (top `0x52A00`) vs context `0x52000` | possible if SPL nested deeper than 0xA00 bytes | harmless: the context is only read by the copy stub, which has already returned |
| SPL BSS `0x30000`–`0x34000` vs native extent `0x31000`–`0x3C753` | overlaps | harmless: the copy to `0x20000` completes before the SPL starts |
| anything above `0x57000` | SRAM C ends at `0x58000` | 4 KiB of headroom; nothing uses it |

No collision is reachable in the actual sequence. That is a statement about this
sequence, not about SRAM C ownership.

---

## 3. From hand-encoded words to a real assembler build

### 3.1 Toolchain

```
zig                 0.17.1
clang               22.1.8        (zig cc's driver)
linker              lld, bundled with zig
target              aarch64-freestanding
assembly            LLVM integrated assembler
```

No `aarch64-none-elf-*`, `clang` or `ld.lld` exists on this host. The wheel was fetched
from the Tsinghua PyPI mirror; its SHA-256 matches the index entry. It is used from
`build/toolchain/` and never installed into the user's `PATH`.

### 3.2 Source and linker script

`build/audit-work/probe64/probe64.S` is a faithful re-assembly of the instruction stream
that `build_probe.py` emitted by hand. Three syntactic changes only: the Image header is
produced by the assembler, the 16-entry vector table is emitted with `.rept`, and
operand syntax was normalised for the LLVM assembler (`lsl #16`, `ldrb w5, [x4, x3]`,
`strb w5, [x1], #1`).

`probe.ld` places one output section at `0x40080000` so that the header field

```
.quad __image_end - __image_start
```

is an ordinary same-section symbol difference, resolved by the assembler, leaving no
relocation. Two `ASSERT`s pin the image to `0x3000` bytes and `0x40083000`.

`build.py` assembles, links, reads the ELF back with pyelftools (section layout,
relocation count, entry), extracts `.image` as the flat Image, and repeats the whole
build to prove determinism.

### 3.3 Header, field by field

| Field | Offset | Value | Check |
| --- | --- | --- | --- |
| `code0` | `0x00` | `0x14000010` = `b #+0x40` | branch target `0x40080040`, verified by decoding imm26 |
| `code1` | `0x04` | 0 | reserved, zero |
| `text_offset` | `0x08` | 0 | image is position-fixed at `0x40080000` |
| `image_size` | `0x10` | 12,288 | equals the file length |
| `flags` | `0x18` | 2 | `bit0 = 0` little endian, `bits1-2 = 0b01` = 4 KiB pages, `bit3 = 0` (per `asm/image.h`) |
| `res2`–`res4` | `0x20`–`0x38` | 0 | zero |
| `magic` | `0x38` | `0x644D5241` | `ARM\x64`, the only field `loader.c` actually tests |
| `res5` | `0x3C` | 0 | no PE/COFF header |

On `flags`: the earlier report said "flags = 2 (4 KiB pages, little-endian)" and that
reading is correct, but for a reason worth stating precisely — 2 is *not* the page-size
code, it is `page-size code 1 << 1`. Linux `arch/arm64/include/asm/image.h` defines
`BE_SHIFT 0`, `PAGE_SIZE_SHIFT 1`, so `flags = 2` means "little endian, 4 KiB pages,
physical placement unspecified". The stage-0 probe happens to store the same value.

### 3.4 Product hashes

```
source          build/audit-work/probe64/probe64.S      (real assembly)
linker script   build/audit-work/probe64/probe.ld
image           probe64-image.bin    12,288 bytes
SHA-256         3cb5c4403b96d287a9aa6dc4629ea7703c7268667fdb9634bfbc7c83e077efa5
ELF             ET_EXEC, e_entry 0x40080040, 1 PT_LOAD, 0 relocations
reproducible    two independent builds, byte-identical
```

Hand-encoded baseline for comparison: `714d0ebf…b3c75c`, also 12,288 bytes.

### 3.5 Instruction-by-instruction comparison

144 instructions on both sides, indices aligned 1:1. **118 are byte-identical.** The 26
differences fall into exactly four classes, each verified against the architecture
rather than accepted by hash:

| Class | Count | Example | Why it is not a behaviour change |
| --- | --- | --- | --- |
| register copy: `add Rd, Rm, #0` (hand) vs `mov` alias (assembler) | 12 | `add x26, x1, #0` vs `mov x26, x1` | both leave `Rd = Rm`; `add #0` does not set flags |
| `movz`/`movk` width: sf=1 (hand) vs 32-bit (assembler) | 8 | `mov x22, #0xa` vs `mov w22, #0xa` | the immediate fits in 16 bits, and a 32-bit `movz` zero-extends into the full register, so `x22 = 0xa` either way |
| linker relaxation of `adrp`+`add` | 4 | `adrp x18, sym` → `nop`; `add x18, x18, #0` → `adr x18, sym` | same address, still PC-relative, still no relocation |
| unreachable padding | 1 | `udf #0` (hand left zeros) vs `nop` | follows `b park`; never fetched |

One of them is a **value**, not an encoding, difference and is called out as such:

> index 33: the hand encoder stored `0x40082F90` in record `+0x68` (its own `image_end`
> label, 0x70 before the real end); the assembler stores `0x40083000`, the true end of
> the image. The assembled value is the correct one, and the behavioural test asserts it.

Empty space inside the executable section differs cosmetically: the assembler fills
`.space` with `0xD503201F` (NOP) where the hand encoder emitted zeros. It affects the
stack, the log buffer and the guard page only, and none of it is read before being
written.

### 3.6 Behaviour of the assembled image

The emitted image was stepped through the host-side AArch64 subset interpreter that
already validated the hand-encoded version, with identical initial state
(`x0 = 0x44000000`, `x1 = 0x11111111`, `entry SP = 0x4008F000`, `CurrentEL = 4`,
`SCTLR = 0x30D00800`, `DAIF = 0x3C0`, `VBAR = 0x40000000`):

* record `0x40080290`: magic `0x50524231`, `x0 = 0x44000000`, `x1 = 0x11111111`,
  `x2 = 0x22222222`, `x3 = 0x33333333`, entry SP `0x4008F000`, `CurrentEL = 4`,
  `SCTLR_EL1 = 0x30D00800`, `DAIF = 0x3C0`, old VBAR `0x40000000`, new VBAR
  `0x40082800`, new SP `0x40082000`, image `0x40080000`–`0x40083000`, handler marker 0;
* log buffer `0x40080330`: `PROBE1` plus six labelled 16-digit hex lines, byte-exact;
* 123 stores and 96 loads, all inside `0x40080280`–`0x400803CD`;
* parked at `0x40080224` after 998 steps — the same count as the hand-encoded build.

The interpreter is a subset model, not hardware: it establishes that the emitted
instruction stream does what the source says, not that a Cortex-A53 will agree about
caches, MMU state or exception level.

---

## 4. A real observable output: the boot framebuffer

### 4.1 What the frame actually is

From the live tree (`measured`), three independent descriptions of the same region:

| Source | Content |
| --- | --- |
| `/chosen/bootargs` | `disp_reserve=1382400,0x7bf24800` → 1,382,400 bytes at `0x7BF24800` |
| `/soc@03000000/disp@01000000/boot_fb0` | `"7bf24800,2d0,1e0,20,b40,0,0,2d0,1e0"` → address, 720, 480, 32 bpp, stride 2880 |
| arithmetic | 720 × 480 × 4 = 1,382,400 = 2880 × 480 |

`/memory@40000000` declares `0x40000000` + `0x40000000`, so the region ends at
`0x7C076000`, comfortably inside DRAM.

`/chosen/baseos,boot-display` is the 80-byte record the loader wrote:

```
+0x00 magic 0x31505344 "DSP1"   +0x04 version 1      +0x08 bytes 80
+0x0C fnv 0xf733d668            +0x10 panel 0        +0x14 logo_bytes 0x000FD236
+0x18 logo_fnv 0xe2b42430       +0x1C pixel_fnv 0xd7a579f4
+0x20 generation[32] = 022f0d88 … 7b4c0fee   (identical to baseos,native-boot)
+0x40 shown_ticks 0x0000000001E0B30A         +0x48 reserved[2] = 0
```

It carries **no** framebuffer address and no geometry — its fields are the panel id,
the logo digest and the generation. The address can therefore only be corroborated
through `boot_fb0` and `disp_reserve`, which is exactly what the probe does.

Note the endianness trap: FDT structural cells are big-endian, but a property the
loader copied from a C struct is raw little-endian. Reading `DSP1` as `be32` yields
`0x44535031` and rejects a perfectly good tree; the probe reads it as `le32`.

### 4.2 The gate

The probe will paint only if **all eight** of these hold, each from a different part of
the tree (`measured`, exercised below):

| # | Gate | Failure reason |
| --- | --- | --- |
| 1 | FDT magic `0xd00dfeed`, `totalsize` in `64..2 MiB`, struct/strings offsets inside it | `R_FDT_HEADER` = 2 |
| 2 | a `chosen` node exists at depth 2, the walk terminated on `FDT_END` without an unknown token | `R_NO_CHOSEN` = 3 |
| 3 | `baseos,native-boot` present with length 32 | `R_NO_NATIVE_BOOT` = 4 |
| 4 | `baseos,boot-display` present, 80 bytes, magic `DSP1` | `R_NO_DISPLAY_RECORD` = 5 |
| 5 | its `generation[32]` equals `baseos,native-boot` byte for byte | `R_GENERATION` = 6 |
| 6 | `boot_fb0` present and parsable as five hex fields | `R_NO_BOOT_FB` = 7 |
| 7 | geometry is exactly 720 × 480 × 32 with stride 2880, and `bootargs` carries a `disp_reserve` whose size and address agree | `R_GEOMETRY` = 8, `R_NO_DISP_RESERVE` = 9, `R_RESERVE_MISMATCH` = 10 |
| 8 | the region lies entirely inside `/memory` | `R_NO_MEMORY` = 13, `R_OUTSIDE_DRAM` = 11 |

Gate 5 is the one that makes this more than a string match: it ties the tree the probe
is looking at to the generation that the loader recorded, the same 32 bytes that the
capsule cookie binds. Gate 7 is what stops a wrong-but-plausible address from being
painted: a change to *either* the descriptor or the command line makes them disagree.

### 4.3 What it draws

Nine 5×7 lines at 3× scale, in white and green, starting at (20, 24), each character
18 px wide and each line 26 px tall — 234 px of the 480 available:

```
A64 PROBE
EL   0000000000000004
FDT  0000000030000000
SP   000000004008F000
SCTL 0000000030D00800
DAIF 00000000000003C0
FB   000000007BF24800
BYTES0000000000151800
TEXT WRITTEN
```

`EL`, `SP`, `SCTL`, `DAIF` and `FB`/`BYTES` are the entry state captured by `head.S`
(or read from the tree), so the panel shows the same facts the memory record holds. The
`FDT` line is the pointer the probe was handed: `0x30000000` is the host harness's fixed
mapping (see 4.4), while on hardware it is whatever address the loader passes. The glyph
table is generated from ASCII art by `font.py` (36 glyphs, reviewable by eye) rather than
typed as hex; an unmapped character resolves to the blank cell rather than to another
glyph.

No display controller register is written, no panel or clock is re-initialised, no GPU
command is issued, and the only store into the framebuffer sits behind the x/y range
check in `fb_pixel`.

### 4.4 Verification: the same C file, built twice

`probe_fb.c` is compiled once for `aarch64-freestanding` (the Image under test) and once
for the host with `-DPROBE_HOST`, where only `fb_store()` is replaced by a recorder.
Every gate, parser and boundary check that would run on the device is therefore the code
that runs on the host, against the **real captured device tree**:

| Mutation | Expected | Observed reason | Pixels written |
| --- | --- | --- | --- |
| none | pass | `R_OK` = 1, `gate_mask` = 255 | 23,877 |
| flip FDT magic | 2 | 2 | 0 |
| `DSP1` → `XSP1` | 5 | 5 | 0 |
| flip `generation[0]` in the display record | 6 | 6 | 0 |
| rename `boot_fb0` | 7 | 7 | 0 |
| width `0x2d0` → `0x3d0` | 8 | 8 | 0 |
| address `7bf24800` → `6bf24800` | 10 | 10 | 0 |
| `disp_reserve` size `1382400` → `2382400` | 10 | 10 | 0 |
| rename `bootargs` | 9 | 9 | 0 |

Every refused run wrote **zero** pixels; the passing run wrote 23,877 pixels, all within
`0x7BF24800`–`0x7C076000`, with `writes_outside = 0`, over the range
`0x7BF3565C`–`0x7BFD5EA0`.

One reproducibility trap was found and closed here, and it is worth recording because it
looks like non-determinism in the probe. The probe paints the FDT pointer it was handed as
16 hex digits. The first version of the harness passed a `malloc()` address, so ASLR
changed those digits on every run and the pixel count moved between 23,409 and 23,679 for
identical input — while the *write range* stayed byte-identical, since only the glyphs of
the address line changed. The harness now maps the device tree at a fixed address
(`0x30000000`) with `VirtualAlloc`, and the count is stable at 23,877 across repeated runs
(`fdt_fixed = 1` in the JSON). This is a host-side artefact only: on hardware the loader
hands over one fixed pointer, so the target's output is deterministic for a given tree.

### 4.5 The honest limit: will anyone see it?

Writing the right bytes and *displaying* them are different claims, and only the first
one is established here.

What is known: the loader starts the DE/TCON stream and the panel in
`baseos_early_display_start`, and it explicitly calls `release_display_sram()` before
touching DE, i.e. it hands SRAM C back before the display program runs. What is not
known is whether the stream is still running at the moment the kernel is entered, and
whether the scan-out still reads `0x7BF24800` after the handoff. The previous reports
record display-related FDT differences but no proof of a live scan.

So the probe separates the two states in its own record: `reason = R_OK` and
`written = 1` mean *the framebuffer was painted as instructed*, not *the panel showed
it*. Until visibility is proven, a hardware run yields evidence only if someone can
observe the panel; if the panel is dark, the run proves nothing and must be reported as
inconclusive rather than failed.

If visibility turns out to be absent, the two candidate read-out channels are:

1. **UART0 at `0x05000000`** — its existence and use before Linux are attested by the
   live command line (`earlyprintk=sunxi-uart,0x05000000 console=ttyS0,115200`). This is
   the only MMIO address that the captured evidence actually justifies. It was still not
   used: writing it requires knowing that the divisor/clock set by the SPL is the one the
   probe assumes, which this round did not establish. A UART variant is the natural next
   step, gated on that check.
2. **Framebuffer at a known address, painted before the panel is started** — not
   possible from the kernel entry point, which runs after the display program.

---

## 5. Static safety verification

### 5.1 The 12 KiB baseline probe (`probe64`)

| Check | Result |
| --- | --- |
| header fields, entry branch target, `image_size == file size` | pass |
| 4 KiB page flag, little-endian, reserved fields zero | pass |
| stack 4 KiB, `stack_top` 16-byte aligned, inside the image | pass |
| vector table 2 KiB aligned, 16 of 16 slots branch | pass |
| `smc` / `hvc` / `eret` in the text | 0 / 0 / 0 |
| relocations in the ELF | 0 |
| `adrp`/`adr` targets outside the image | 0 |
| abstract interpretation of load/store addresses | 32 accesses, 31 resolved statically, 1 (the `strb` inside `hex64`) resolved dynamically |
| dynamic access trace | 123 stores, 96 loads, all in `0x40080280`–`0x400803CD` |
| overlaps with `0x42000000`, `0x44000000`, `0x45000000`, `0x48000000`–`0x49000000`, `0x7EC00000`–`0x7FC00000` | none (image ends `0x40083000`) |
| two builds byte-identical | pass |

### 5.2 The framebuffer probe (`probe-fb`)

| Check | Result |
| --- | --- |
| header, entry target `0x40080040`, `image_size` 18,432 = file length | pass |
| stack 8 KiB at `0x40082000`–`0x40084000`, 16-byte aligned | pass |
| vectors 2 KiB at `0x40084000`, 16 of 16 slots branch | pass |
| `smc` / `hvc` / `eret` | 0 / 0 / 0 |
| relocations | 0 |
| every `adrp`/`adr` target inside the image | pass |
| every `sp`-relative access inside the 8 KiB stack | pass (displacements `-0x10` … `+0x190`; the single negative one is the pre-index `str x30, [sp, #-0x10]!` at `0x4008110C`, all others non-negative and below 8 KiB) |
| **hard-coded physical addresses** in the instruction stream | **none** — no MMIO, no DRAM, no fixed framebuffer address; the address comes from the FDT at run time |
| `fb_store` (the only store into the framebuffer) | exactly 1 store instruction, at `0x40081158` |
| `fb_pixel` (its only caller) | 6 guard instructions (x<0, y<0, x≥w, y≥h) and 1 call to `fb_store`, spanning `0x400810DC`–`0x40081158` |
| store base registers across the image | `sp` 52, `x20` 18, `x8` 14, `x9` 8, `x6` 6, others ≤ 3 — all resolved to the stack or to loader-provided structures |
| framebuffer writes during the passing run | 23,877, range `0x7BF3565C`–`0x7BFD5EA0`, 0 outside |
| two builds byte-identical | pass (`a6ec1ae3…`) |

A note on what "none" means for the address check: the probe compares the FDT's
framebuffer descriptor against constants 720/480/32/2880, which are *geometry*, not
addresses. The address itself is only ever compared against another value read from the
same tree, so a wrong tree cannot make the probe write to a wrong place — it can only
make it refuse.

---

## 6. Delivered artefacts

Host only, under `build/`, nothing committed and nothing uploaded:

| Path | Contents |
| --- | --- |
| `build/audit-work/probe64/probe64.S`, `probe.ld`, `build.py`, `verify.py` | baseline probe, real assembly, its build and verification |
| `build/audit-work/probe64/out/` | `probe64-image.bin` (12,288 B), `.elf`, `.o`, map, disassembly, two JSON reports |
| `build/audit-work/probe-fb/head.S`, `probe_fb.c`, `probe.h`, `font.py`, `font.h`, `host_main.c`, `probe.ld`, `build.py`, `verify.py` | framebuffer probe and its host harness |
| `build/audit-work/probe-fb/out/` | `probe_fb-image.bin` (18,432 B), ELF, map, 1,096-instruction disassembly, `host_probe.exe`, two JSON reports |
| `build/audit-work/stage0/sram_audit.py`, `sram-audit.json` | the measurements behind section 2 |
| `build/toolchain/` | the zig wheel and its unpacked tree |

Binary hashes, for the record:

```
probe64-image.bin   12,288 B  3cb5c4403b96d287a9aa6dc4629ea7703c7268667fdb9634bfbc7c83e077efa5
probe_fb-image.bin  18,432 B  a6ec1ae37632bab1319cebc988a11902c85046fcd3f7be20932dead92f0e214a
```

Neither is wrapped in a BOL4/LZ4 frame, an Android v2 container or a capsule; neither
carries a generation cookie; neither was executed on hardware.

---

## 7. Does this reach the hardware-test threshold?

Three separate questions, answered separately.

**Is the object qualified?** Yes for both binaries: header, layout, relocation-freedom,
absence of privileged traps, memory-access whitelist, deterministic build, and — for the
baseline — behavioural equivalence with the hand-encoded version. The framebuffer
variant additionally survives nine adversarial trees without ever writing outside the
reserved region.

**Is the observation channel qualified?** Only half. The probe writes the right memory
with the right gating; whether the panel is scanning that memory at that instant is
UNKNOWN. A run that produces a dark screen must be recorded as inconclusive, not as a
failure, and the same run produces no other output — the probe parks in `wfi` and the
record dies with the next boot.

**Is the test safe to run?** Not yet, for one reason that is not about the probe: the
probe never returns. Booting it replaces the kernel for that attempt, so the only exit is
a watchdog reset. The BaseOS arm-watchdog protocol exists (`0x030090B0`/`B4`/`B8`, and
the capsule already arms it), but the probe deliberately does not touch it. A hardware
attempt therefore needs, first, a documented recovery path — watchdog arm-and-reload or a
known-good SD card swap — and second, a decision about which channel counts as evidence.

Recommended order for the next round, all still host-side until the last item:

1. add a UART0 variant gated on a verified divisor/clock assumption, and treat framebuffer
   visibility as the secondary channel rather than the primary one;
2. decide the exit path: `wfi`-park plus external power cycle, or arm the watchdog exactly
   as the SPL does and let it reload;
3. only then consider a single hardware attempt, with the SD card's original content
   preserved off-device.

---

## 8. Remaining unknowns

1. **SRAM C ownership at BROM time.** Not read from SYSCON; the borrowing protocol is
   documented in BaseOS sources but the register state at `0x20000` entry is not.
2. **The agent that loads 128 KiB.** The eGON header asks for `0x20000` bytes, 64 KiB
   beyond the A1 window; the fact that it works is inference from observed behaviour.
3. **Framebuffer visibility after handoff.** Whether DE/TCON still scan `0x7BF24800` when
   the kernel is entered.
4. **UART viability.** `0x05000000` is attested by the command line, but the divisor and
   clock set by the SPL were not measured, so a UART probe's baud rate is unproven.
5. **Byte-exact generation rebuild** — unchanged from the previous report: the pinned
   `boot-package.bin` and the build-time GPT prefix are still missing.
6. **CI artefact cross-check of the deployed SPL** — still blocked on an authenticated
   download of the 32 archived artefacts.

---

## Appendix — evidence index

| File | Used for |
| --- | --- |
| `build/audit-work/stage0/sram_audit.py` → `sram-audit.json` | occupied ranges, SRAM-like literals per stage |
| `build/audit-work/probe64/*` | assembled baseline, instruction diff, interpreter run |
| `build/audit-work/probe-fb/*` | framebuffer probe, host harness, mutation matrix |
| `build/toolchain/ziglang-0.17.1-py3-none-win_amd64.whl` | toolchain provenance (SHA-256 `69c3dfc3…`) |
| `build/rg34xx-inventory-20261122-214708/live-fdt.dtb` | all FDT facts in section 4 |
| BaseOS `native/capsule/{entry.S,capsule.c,build.py,contract.json}` | dispatcher placement, staging addresses, stack top |
| BaseOS `spl/{build-early-mmc.py,early-mmc.c,probe-copy.c,README.md}` | BSS/stack addresses, SRAM C borrowing, A1 guidance |
| BaseOS `native/{loader.c,early-display.c}`, `spl/native-display.h` | SRAM C release, display record ABI |
| TF-A `plat/allwinner/sun50i_h616/include/sunxi_mmap.h` | A1 / SRAM C ranges |
| Linux `arch/arm64/include/asm/image.h` | Image header flag encoding |

No binaries, serial numbers, MAC addresses or absolute host paths are included.
