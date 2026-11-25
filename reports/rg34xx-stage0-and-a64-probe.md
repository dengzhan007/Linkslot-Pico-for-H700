# RG34XX stage-0 identity closure and a minimal AArch64 entry probe (host only)

Read-only follow-up to `rg34xx-active-loader-provenance.md`. Nothing on the device was
written, flashed, reset or executed; no Boot0, TOC1, U-Boot, monitor or GPT byte was
modified. The probe in section 5 was built, verified and hashed on the host only. No
binary, serial number, MAC address or local absolute path appears in this report.

Inputs: `build/rg34xx-inventory-20261122-214708/` (boot0 at LBA 16 and 512, TOC1
package, boot partition, live FDT, env, GPT text, mounts, cmdline) and the BaseOS tree
at `cbe253a43f46cf551cc9a6c79a84015bde3686cf`.

---

## 0. Headline

**The identity question is closed, and the answer corrects both earlier reports in one
detail each.**

1. `boot0-primary.bin` (LBA 16, 128 KiB, SHA-256 `bf886995…19289f`) is **not** the
   vendor Boot0 and is **not** a plain SPL file. It is a BaseOS **raw capsule**
   (`native/capsule/build.py`): a 0x1000 dispatcher, then a **byte-exact copy of the
   vendor Boot0 pinned by the repository contract** at `+0x1000`, then the **48 KiB
   BaseOS SPL** at `+0x11000`, zero-padded to 128 KiB.
2. `boot0-secondary.bin` (LBA 512, 64 KiB valid, SHA-256 `4701a85e…fa9f5d`) is a
   **second, different vendor Boot0**. It is not the image embedded in the capsule and
   matches no pinned contract hash.
3. The link between the running system and that capsule is cryptographic, not
   circumstantial: `FNV-1a32(generation)` of the 32-byte generation read from the live
   FDT equals the capsule cookie compiled into the dispatcher, `0xf7bed1c8`.
4. The deployed SPL is no longer "missing": it is recoverable byte-exactly from the
   capsule, with SHA-256 `cdfc6d48…64a8570` and FNV-1a32 `0xa53ccffb`. Both embedded
   extents are covered by **constants compiled into the dispatcher** that match the
   recomputed values exactly.
5. Stage-0 hand-off is therefore: **BROM → capsule dispatcher (AArch32) → copy 0xC000 to
   0x20000 → SPL AArch32 stub → RVBAR = 0x20060 + RMR reset → AArch64 SPL at 0x20060**.
6. A 12,288-byte freestanding AArch64 probe was built and verified on the host
   (header, static safety, behaviour, reproducible build). It is **not** installed and
   is deliberately not wrapped into an installable frame.

---

## 1. Scope, method and read-only boundary

| Item | Rule applied |
| --- | --- |
| Device | Read-only adb reads only (`/init`, `/etc/init.d/rcS`, `/etc/fstab`). No write, no flash, no reset, no ioctl. |
| Local analysis | `boot0-primary.bin`, `boot0-secondary.bin`, `boot-partition.bin`, `live-fdt.dtb`, TOC1 package — read-only byte analysis in `build/audit-work/stage0/`. |
| Disassembly | capstone 5.0.9, AArch32 (ARM) and AArch64 decoding of the recovered extents. |
| BaseOS reference | single-commit clone at `cbe253a4`; contract hashes also read at `05c39a7f` over the GitHub raw API (read-only HTTP GET). |
| Probe | host-only build; hand-encoded instruction stream re-decoded with capstone; no SMC, no MMIO, no payload execution. |

All hashes below are SHA-256 unless marked FNV-1a32 or eGON-additive.

---

## 2. Stage 0 — identity verification

### 2.1 The three layers inside the 128 KiB image at LBA 16

`boot0-primary.bin` is a container. Parsing `eGON.BT0` headers at every 4-byte offset
finds exactly three, and each one is corroborated by its own checksum and by the
dispatcher that consumes it:

| Layer | Offset | Bytes | eGON branch | eGON size | eGON checksum | SHA-256 |
| --- | --- | --- | --- | --- | --- | --- |
| Capsule header + dispatcher | `0x000000` | `0x21000` used (header block `0x0`–`0x1000`) | `0xea0000fe` → `0x20400` | `0x20000` | `0x9f548d9c` | `bf886995…19289f` (whole 128 KiB) |
| Vendor Boot0 copy | `0x001000` | `0x10000` | `0xea0004be` → `0x1000+0x1300` | `0x10000` | `0x099a22e7` | `b6edbf39…5464049b` |
| BaseOS SPL | `0x011000` | `0xc000` | `0xea000016` → `0x20000+0x60` | `0xc000` | `0x244655c0` | `cdfc6d48…964a8570` |
| Padding | `0x01d000` | `0x3000` | — | — | — | zeros |

The vendor copy is **byte-identical to the pinned rg34xx contract**:
`b6edbf39af0bf1f35a7e2fd991c8966b04d061e247c81985d2516f695464049b` equals
`targets.rg34xx.boot0_sha256` in `evidence/compatibility.json` at `05c39a7f`. The
capsule therefore carries the *contract* vendor Boot0, not a snapshot of whatever the
unit happened to hold.

The SPL's own header carries the mainline U-Boot shape:

```
+0x00 16 00 00 ea            b  +0x60
+0x04 "eGON.BT0"
+0x0c c0 55 46 24            checksum
+0x10 00 c0 00 00            size = 0xc000
+0x14 "SPL\x02"              sunxi SPL magic + header version 2
+0x1c 2c 00 00 00            name offset = 0x2c
+0x2c "allwinner/sun50i-h700-anbernic-rg35xx-2024"
```

and its strings identify the builder unambiguously:
`"U-Boot SPL 2026.10 (Oct 02 2026 - 00:00:00 +0000)"` at file `0x1bffb`
(`SOURCE_DATE_EPOCH=1790899200`, i.e. 2026-10-02, the value hard-coded in
`spl/build-early-mmc.py`), plus BaseOS-only literals `"sunxi SPL version mismatch"`,
`"all DRAM candidates failed"`, `"unsupported candidate type"`, `"Timeout initialising
DRAM"` — none of which exist in the vendor Boot0 beside it.

### 2.2 Full hash, first 512 bytes, body hash

| Object | Bytes | Full SHA-256 | First 512 B | Body (after first sector) |
| --- | --- | --- | --- | --- |
| LBA 16 capsule | 131072 | `bf886995745a0eb9b5030296f9f6163035e164b3a82183716e67ff876319289f` | `15ee3cd9…0f8c3` | `ff847db9…e4b50` |
| Vendor Boot0 inside capsule | 65536 | `b6edbf39af0bf1f35a7e2fd991c8966b04d061e247c81985d2516f695464049b` | `8e1cd271…6691a` | FNV-1a32 `0x8fb6d562` |
| BaseOS SPL | 49152 | `cdfc6d480a257c5ccc3f5826c4e6cbcf499163c57119390e8bebeba6964a8570` | `cdfc6d48…` (no mutable sector) | FNV-1a32 `0xa53ccffb` |
| LBA 512 vendor Boot0 | 65536 (valid) | `4701a85e4db7e8339b91b92f691a83cc954ec63a09369087c2e41d80eafa9f5d` | `35946610…726cd4e` | `1bdc26d9…46a5608` |

The 512-byte boundary matters because the first sector is the mutable eGON header
(checksum, debug byte, trained DRAM words). Hashing the body separately is the only
honest way to compare two copies that have both been booted.

### 2.3 LBA 16 versus LBA 512 — what the two candidates actually are

The embedded vendor copy and the LBA 512 copy differ in **five bytes**:

| Offset | Capsule copy | LBA 512 | Meaning |
| --- | --- | --- | --- |
| `0x0c`–`0x0f` | `e7 22 9a 09` | `e5 22 9a 0b` | eGON additive checksum (derived, not independent evidence) |
| `0x34` | `0x03` | `0x00` | vendor debug/boot-mode byte |
| `0xa3` | `0x3e` | `0x40` | byte 3 of DRAM word `0xa0` (`dx_odt`) |
| `0xac` | `0x06` | `0x07` | byte 3 of DRAM word `0xac` |

Because the capsule copy reproduces the *pinned contract hash exactly*, the difference
cannot be explained away as "training noise between two copies of the same image". The
two are **different vendor Boot0 revisions**: the capsule one is the contract's rg34xx
image, the LBA 512 one is whatever the unit's boot area already held. Its provenance is
**unresolved** — it matches none of the eleven pinned `boot0_sha256` values in the
contract.

What can and cannot be concluded:

* Confirmed: two structurally valid eGON images exist at LBA 16 and LBA 512; the
  LBA 16 one is the active one (it contains the SPL that the running system's capsule
  cookie ties to).
* **Not** confirmed: any BROM fallback behaviour at sector 512. The BaseOS tree says
  the same thing in `spl/README.md` ("does not establish BROM fallback behavior at
  sector 512"), and this round produced no new evidence either way.

### 2.4 eGON header, entry instructions, RVBAR, RMR and 0x20060

Capsule dispatcher, disassembled from `0x400` (the outer branch target):

* `0x4e0`: loads `0x02000004`, tests the word at `+0x0c`… i.e. it re-checks the outer
  header (`image_size == 0x20000`) and the eight magic bytes at `0x20004`.
* `0x504`–`0x528`: compares eight bytes against the reference string at `0x838`
  (`"eGON.BT0"`).
* `0x530`: reads `0x070001f8` bit 16 (RTC general-purpose attempt flag) and branches to
  the alternate path at `0x67c` when set.
* `0x550`–`0x58c`: `bl 0x738` twice — the FNV-1a32 routine at `0x738` (offset basis
  `0x811c9dc5` and prime `0x01000193` are visible as literals at `0x760`/`0x764`) — and
  compares against `0x8fb6d562` (stock extent) and `0xa53ccffb` (native extent).
* `0x5b8`–`0x5f8`: the `0x030090b4`/`0x030090b8`/`0x030090b0` writes
  (`1`, `0xb1`, `0x14af`) — the same handshake string that `spl/build-early-mmc.py`
  injects into `board_init_f`.
* `0x6e0`–`0x71c`: writes the attempt cookie to the RTC, reads it back, then
  `mov r4, #0x31000` and calls the relocator.

Relocator at `0x840`, entered with `r0 = 0x31000`, `r1 = 0xc000`, `r2 = 0x20000`:

```
0x840 mov r2, #0x20000
0x844 ldr r3, [r0], #4
0x848 str r3, [r2], #4
0x84c subs r1, r1, #4
0x850 bne 0x844
0x854 mov r0, #0x52000        @ relocated state block
0x860 strb r1, [r2, #0x28]    @ version byte into the new header
0x874 ldmib r0, {…}           @ restore caller state
0x87c ldr pc, [pc, #-4]       @ literal 0x00020000
```

So the SPL is executed **from 0x20000**, which is why the SPL's own AArch32 stub can
program an absolute resume address.

Inside the SPL (`+0x60`, the eGON entry):

```
+0x060 1f 00 00 ea   AArch32: b +0xE4      |  AArch64: ands xzr, x0, x0
+0x064 49 00 00 14   AArch32: (strne)      |  AArch64: b +0x188
```

This is the sunxi64 **polyglot entry**: AArch32 follows `b 0x200E4` into the armv7
stub; after `RMR` the core restarts in AArch64 at the same address, where the two words
decode as a harmless flags-only `ands` followed by a real AArch64 branch to `0x20188`.
At `0x20188` the code is unambiguous AArch64 (`b 0x278`, `mrs x1, currentel`,
`cmp x1,#8`, `b.eq/b.gt/b.lt` to the EL3/EL2/EL1 paths, `msr scr_el3`,
`msr cpt_el2`, `msr cpacr_el1`).

RVBAR and RMR, in the AArch32 stub:

| Location | Encoding | Effect |
| --- | --- | --- |
| SPL `+0x154` | literal `0x09010040` | RVBAR_EL3 address (SoC revision 0) |
| SPL `+0x15c` | literal `0x08100040` | RVBAR_EL3 address (SoC revision 1) |
| SPL `+0x160` | literal `0x00020060` | value written to RVBAR |
| SPL A32 stub | `mrc/mcr p15, c12` | RMR read/reset (`RMR_EL3`), matching `spl/verify-brom-entry.py`, which models exactly this pair and asserts `rmr_reset_value == 3` |

The vendor Boot0 contains the same two RVBAR address literals (file `0x21d0`,
`0x21d4`) but **no `0x00020060` literal**, so the vendor resume address is computed
rather than fixed. That is a real structural difference between the two stage-0
implementations, not a cosmetic one.

### 2.5 Verdict: is the current Boot0 already the BaseOS SPL?

Both halves of the question have to be answered separately:

* **Is it the vendor Boot0?** No. The 128 KiB block at LBA 16 is a BaseOS capsule; its
  outer header, dispatcher, FNV coverage and embedded SPL are all BaseOS artefacts.
* **Is it "the BaseOS SPL"?** It *contains* the BaseOS SPL (48 KiB, `cdfc6d48…`) and
  executes it, and it *also* contains the contract vendor Boot0 as the integrity-checked
  restore source. In capsule terms this is `raw-capsule-outer-commit-inner-adopt`,
  protocol version 2.

The rest of the running chain is unchanged from the provenance report: vendor BL31 at
`0x48000000`, BaseOS AArch32 loader at `0x4a000000`, BOL4/LZ4 image at p4 + 48 MiB,
AArch64 kernel at `0x40080000`.

### 2.6 The actual BROM → AArch32 → AArch64 path

```
BROM (eGON/FEL reader, AArch32)
  |  reads eGON header at LBA 16, checksum + size 0x20000
  v
capsule dispatcher  [AArch32, 0x400]
  |  re-checks header + magic
  |  FNV-1a32(0x31000, 0xC000) == 0xa53ccffb   (SPL extent)
  |  FNV-1a32(0x21000, 0x10000) == 0x8fb6d562  (vendor Boot0 extent)
  |  RTC cookie 0xf7bed1c8 = FNV-1a32(generation)
  |  watchdog/PMIC handshake 0x030090b0/b4/b8
  |  copies 0xC000 bytes 0x31000 -> 0x20000, then jumps to 0x20000
  v
SPL AArch32 stub  [0x20000 + 0x60]
  |  saves state, picks RVBAR address by SoC revision
  |  RVBAR_EL3 = 0x20060;  RMR_EL3 reset
  v
AArch64 reset vector 0x20060
  |  polyglot: ands xzr,x0,x0 ; b 0x20188
  v
mainline U-Boot SPL 2026.10 + BaseOS dram-select  [AArch64]
  |  CurrentEL dispatch (EL3/EL2/EL1 paths visible at 0x1a0–0x1d4)
  |  DRAM candidate selection (BaseOS additions), BL31 copy, A32 loader copy
  v
vendor BL31 0x48000000 -> A32 BL33 loader 0x4a000000 -> kernel
```

One open point, stated with its exact numbers rather than waved away: the container
assumes `0x20000`–`0x40000` is fully resident before the dispatcher runs its FNV checks,
which is **64 KiB beyond** the 32 KiB SRAM A1 window documented in the BaseOS SPL
README. Host-side evidence cannot decide whether the BROM loads the whole 128 KiB or
whether the extra extent is staged by something else. Confirming it needs either a
BROM-side trace or a documented H700 SRAM map covering `0x20000`–`0x40000`; it is listed
as unresolved in section 7.

### 2.7 "If the final BaseOS spl.bin is missing"

It is not missing any more, but the provenance level has to be stated honestly:

* **Obtained:** the exact 49,152 bytes the device executes, extracted from the capsule's
  `+0x11000` extent, `SHA-256 cdfc6d480a257c5ccc3f5826c4e6cbcf499163c57119390e8bebeba6964a8570`,
  `FNV-1a32 0xa53ccffb`, eGON checksum `0x244655c0`, entry `0x20060`.
* **Not obtained:** a CI-produced `spl.bin` with a published `manifest.json` that could
  be compared hash-to-hash against this one. The GitHub Actions artefacts for this
  repository (32 entries, newest `2026-11-20T07:29Z`, names carrying commit SHAs) are
  only downloadable with an authenticated token, so no cross-check was performed.
* Therefore the SPL fingerprint here is an **on-device binary fingerprint** recovered
  from the capsule and corroborated by the dispatcher's compiled-in FNV constant — not a
  simulator result and not a CI hash match. `spl/verify-brom-entry.py` was read as
  source and its modelled facts (entry `0x20060`, RMR reset value 3, RVBAR writes at
  `0x08100040`/`0x09010040` with `0x20060`) are **independently confirmed** by the real
  bytes, but the emulator itself is not used as evidence of identity.

---

## 3. generation and GPT

### 3.1 The generation formula, item by item

From `native/build.py` (line 316) and `native/capsule/build.py`:

```
generation = SHA-256(
      json.dumps(policy, sort_keys=True)
    + SHA-256(image)
    + bytes.fromhex(PIN)                  # f943a883…5b60c739 (u-boot-v2026.10.tar.gz)
    + source_boot0                        # 65,536 bytes
    + concat(input source files)          # entry32.S, loader.c, runtime.c, … in listed order
)
cookie = FNV-1a32(generation)             # must be non-zero; compiled into the capsule
```

`policy` is not free-form: `model_policy()` derives it from `evidence/compatibility.json`
(`package_sha256` per target), `work/<target>/boot-gzip/boot-package.bin`,
`inventory/env.bin`, `inventory/prefix.bin` (the GPT prefix, up to the first partition),
`work/<target>/boot-prefix.img`, `tools/device_profile.py` and
`native/vendor-handoff.json`.

### 3.2 What is available and what is not

| Input | Status | Evidence |
| --- | --- | --- |
| `PIN` (U-Boot tarball) | **available** | literal in `spl/build-early-mmc.py` and `native/build.py` |
| `source_boot0` (64 KiB) | **available** | capsule `+0x1000`, SHA-256 `b6edbf39…`, equals `targets.rg34xx.boot0_sha256` at `05c39a7f` |
| `env.bin` | **available, and consistent** | device `env-partition.bin` SHA-256 `8dd964ef…` equals `backup-manifest.json` `env.sha256` and `vendor-handoff.json` `environment_sha256` |
| `image` | **available as deployed** | p4 + 48 MiB extent, SHA-256 `d71e5bd8…5f76fc3`; whether the installer rewrote the frame header is unproven |
| source files | **obtainable** | commit `05c39a7f` is identified by the artefact name `RG34XX-MultiFrontend-Upgrade-05c39a7f…`; the local clone is a single-commit checkout of `cbe253a4` |
| `boot-package.bin` (pinned rg34xx) | **missing** | contract pins `56371d984e29e74db63009ce27f829eb0a55ce9007ee5e40c52479b22418af3d`; the unit's TOC1 is `e6c20e3a…`, i.e. **not** the contract image, and 1.25 MiB of bytes cannot be reconstructed from a hash |
| `prefix.bin` (build-time GPT prefix) | **missing** | not captured by the inventory script (it starts at p4); `gpt-fdisk.txt` is text, not bytes |
| build-time `manifest.json` | **missing** | the installer's native manifest was never published with the release |

### 3.3 What *can* be verified without rebuilding

Three independent bindings, all recomputed from the captured bytes:

| Claim | Check | Result |
| --- | --- | --- |
| The capsule's stock extent is the contract's Boot0 | SHA-256 of `+0x1000` vs `targets.rg34xx.boot0_sha256` | **match** (`b6edbf39…`) |
| The capsule's native extent is the running SPL | FNV-1a32 of `+0x11000` vs constant at `0x728` | **match** (`0xa53ccffb`) |
| The capsule belongs to the running generation | FNV-1a32(live `baseos,native-boot`) vs constant at `0x730` | **match** (`0xf7bed1c8`) |

The third one is the important one: the 32 bytes
`022f0d8816c861ebbec05f0d0d7f513d6b974d9930661d20fa8ad51b7b4c0fee` that the loader
writes into `/chosen` are the generation whose cookie selected this capsule. The boot
area and the running kernel are therefore bound to each other by content, not by
timestamp.

### 3.4 Conclusion: reconstruction is impossible, and here is exactly why

The 32-byte generation is a SHA-256 digest. Reproducing it requires **every** input
byte-for-byte; two of them are unavailable and one of those is a 1.25 MiB binary known
only by digest. Trying candidate substitutions would be a search over a 2^256 preimage
with a handful of guesses — that is guessing, not verification, and it is explicitly not
done here.

Verifiable hashes that stand in place of a rebuilt generation:

```
generation (live FDT)   022f0d8816c861ebbec05f0d0d7f513d6b974d9930661d20fa8ad51b7b4c0fee
generation SHA-256      516e5da8d9fe5d2dc2e7a49b90cd419c7cb12589bf3bea5cbeb6d5609de59674
generation FNV-1a32     0xf7bed1c8        == capsule cookie
capsule SHA-256         bf886995745a0eb9b5030296f9f6163035e164b3a82183716e67ff876319289f
capsule eGON checksum   0x9f548d9c
stock extent SHA-256    b6edbf39af0bf1f35a7e2fd991c8966b04d061e247c81985d2516f695464049b
stock extent FNV-1a32   0x8fb6d562
SPL SHA-256             cdfc6d480a257c5ccc3f5826c4e6cbcf499163c57119390e8bebeba6964a8570
SPL FNV-1a32            0xa53ccffb
SPL eGON checksum       0x244655c0
contract package        (missing)  56371d984e29e74db63009ce27f829eb0a55ce9007ee5e40c52479b22418af3d
unit TOC1 package       e6c20e3a… (40 MiB–40.28 MiB extent, SHA-256 dc9dc3ae…)
```

The last two lines are the real finding of this section: **the unit's TOC1 is not the
contract's rg34xx package**, while its Boot0 *is* the contract's rg34xx Boot0. The media
and the capsule come from different builds, which is consistent with a locally built
tree pushed to CI and re-flashed later — and it is the main reason a byte-exact
generation rebuild is out of reach.

### 3.5 GPT labels, `bootargs`, mounted filesystems

| Partition | GPT name (on device) | `bootargs partitions=` | Mounted as | Mount source |
| --- | --- | --- | --- | --- |
| p1 | `special` | `special` | — | — |
| p2 | `boot-resource` | `boot-resource` | — (logo source) | read by loader via sector offset |
| p3 | `env` | `env` | — | — |
| p4 | `boot` | `boot` | — | — |
| p5 | `rootfs` | `rootfs` | `/` (ext4) | `/dev/root` |
| p6 | `UDISK` | `appfs` | `/data` (ext4) | `/dev/mmcblk0p6` |
| p7 | `primary` | `UDISK` | `/mnt/sdcard` (vfat) | `/dev/mmcblk0p7` |

The label drift is real and documented on the device itself: `/etc/fstab` states
`/data … (mmcblk0p6 "UDISK", persistent state)` and `/init` explains that the
"sacrificial appfs stub … no longer exists". `rcS` mounts by **device node**, not by
label: `mount -t ext4 … /dev/mmcblk0p6 /data` (line 120) and
`for dev in /dev/mmcblk1p1 /dev/mmcblk0p7 …` → `/mnt/sdcard` (lines 336–345).

**Conclusion:** the `p6/p7` label difference does **not** affect data mounting. Only a
consumer that trusts the `partitions=` label list (vendor userspace, some Android init
paths) would be misled, and the BaseOS init does not. It should still be fixed in the
generated `bootargs` template, because the string is what an outside tool reads.

---

## 4. Stage-1 target: what a replacement entry must satisfy

Recorded here because it constrains the probe in section 5.

* The vendor BL31 enters BL33 with `SMC #0`, `r0 = 0x8000ff04`, `r1 = entry`,
  `r2 = FDT`, `r3 = 1`, and `SPSR = 0x3c5` (EL1h, AArch64). The loader uses
  `r1 = 0x40080000`, `r2 = 0x44000000`.
* `native/loader.c` accepts an Image only if, after decoding, at least 64 bytes exist
  and `little32(0x40080038) == 0x644d5241` — i.e. the ARM64 `Image` magic at `+0x38`.
* Arena map from `native/build.py`: kernel `0x40080000`–`0x42000000`, initrd
  `0x42000000`–`0x42100000`, display-staging `0x42e00000`, FDT `0x44000000`–`0x44100000`,
  image `0x45000000`–`0x46000000`, BL31 `0x48000000`–`0x49000000`.
* SPL BSS starts at `0x30000` with a 16 KiB cap (`SPL_BSS_START_ADDR`,
  `SPL_BSS_MAX_SIZE` in `spl/build-early-mmc.py`).

---

## 5. The host-only AArch64 probe

### 5.1 Constraints honoured

| Requirement | How it is met |
| --- | --- |
| Cortex-A53 / `aarch64-none-elf` | flat AArch64 image, no relocations, no external symbols |
| Matches the loader's Image test | `+0x38` = `41 52 4d 64` (`0x644d5241`), `image_size` ≥ 64, `code0` = `b` |
| Expected entry `0x40080000` | `code0` at `0x40080000` branches to `0x40080040`; the record stores the base |
| Own stack | 4 KiB inside the image, `SP = 0x40082000`, 16-byte aligned, never outside |
| Own exception vectors | 2 KiB-aligned table at `0x40082800`, 16 entries, `VBAR_EL1` installed |
| Minimal log interface | in-image ring-free text buffer, `PROBE1\n` + six labelled hex lines |
| Records CurrentEL, x0–x3, SCTLR, DAIF, SP, FDT | see 5.3 |
| No unconfirmed physical registers | only `CurrentEL`, `SCTLR_EL1`, `DAIF`, `VBAR_EL1`, `ESR_EL1`, `FAR_EL1`, `MPIDR_EL1` are named; no MMIO is touched |
| No SMC | statically verified: zero `smc`/`hvc`/`eret` instructions in the text |
| No unknown memory read/write | every `adrp`+`add` pair is checked to land inside the image |
| No libc / Linux / Rust std | 158 hand-encoded instructions, no calls, no data dependencies |

`SCTLR_EL1` is read through the EL1 register because the entry EL is unknown to a
boot-time payload; reading it from EL2/EL3 is architecturally permitted. `DAIF` is the
full mask register (`0xD53B4220`), not `NZCV` — an earlier draft used the neighbouring
`0xD53B4200` and the round-trip check caught it.

### 5.2 Layout

| Section | Range | Size |
| --- | --- | --- |
| header | `0x40080000`–`0x40080040` | 64 B |
| text | `0x40080040`–`0x40080280` | 576 B |
| rodata (`0123456789abcdef`) | `0x40080280`–`0x40080290` | 16 B |
| data (record 160 B, log 512 B) | `0x40080290`–`0x40081000` | 0xD70 |
| stack | `0x40081000`–`0x40082000` | 4 KiB |
| guard | `0x40082000`–`0x40082800` | 2 KiB (keeps `stack_top` off the vector base) |
| vectors | `0x40082800`–`0x40083000` | 2 KiB |
| **image** | `0x40080000`–`0x40083000` | **12,288 B** |

### 5.3 What the probe records and logs

Record at `0x40080290` (little-endian u64 each):

| Offset | Field |
| --- | --- |
| `+0x00` | magic `0x50524231` (`"PRB1"`), low 32 bits used |
| `+0x08` | x0 (FDT pointer under the ARM64 boot contract) |
| `+0x10`/`+0x18`/`+0x20` | x1, x2, x3 |
| `+0x28` | entry SP (before the probe took its own) |
| `+0x30` | CurrentEL |
| `+0x38` | SCTLR_EL1 |
| `+0x40` | DAIF |
| `+0x48` | old VBAR_EL1 |
| `+0x50` | new VBAR_EL1 |
| `+0x58` | new SP |
| `+0x60`/`+0x68` | image start / image end (`0x40080000` / `0x40082f90`) |
| `+0x70`/`+0x78` | ESR_EL1, FAR_EL1 (written by the fatal handler) |
| `+0x80` | handler marker `0xdead` (set only if a vector fired) |

Log text at `0x40080330` (host-interpreted example, real values are the entry state):

```
PROBE1
FDT  x0 0000000044000000
X1      0000000011111111
CUR_EL  0000000000000004
SCTLR_E10000000030d00800
DAIF    00000000000003c0
SP_ENTRY000000004008f000
```

The probe then parks in a `wfi` loop; it never returns, never calls SMC and never leaves
the image.

### 5.4 Build and verification results

No cross toolchain (`aarch64-none-elf-*`, `clang`, `ld.lld`) exists on this host, so the
probe is **hand-encoded** by `build/audit-work/probe/build_probe.py`, which emits the
`.S` listing, the flat binary, the map and the disassembly from one instruction list.
Every encoded word is re-decoded with capstone and compared to the intended mnemonic;
`verify_probe.py` adds independent checks.

| Check | Result |
| --- | --- |
| `code0` is `b`, target `0x40080040` | pass |
| `image_size` field = file length = 12,288 | pass |
| `+0x38` = `ARM\x64` little-endian `0x644d5241` | pass |
| flags = `2` (4 KiB pages, little-endian) | pass |
| no `smc` / `hvc` / `eret` in text | pass (0 hits) |
| every `adrp`-derived address inside the image | pass (0 violations) |
| vector table 2 KiB-aligned, 16 branch entries | pass |
| stack 4 KiB, 16-byte aligned, inside the image | pass |
| behavioural model: record fields + log text exact | pass (`parked` after 998 steps) |
| reproducible build (two runs, same bytes) | pass |
| overlap with `0x42000000` / `0x44000000` / `0x45000000` | none (image ends `0x40083000`) |

```
probe-image.bin  12,288 bytes
SHA-256          714d0ebfd3b8a42ae8fed46c8ba8a2e24187e7fa687452391d0fbfe75fb3c75c
```

Artefacts (host only, `build/audit-work/probe/out/`): `probe.S`, `probe-image.bin`,
`probe.map.json`, `probe.disasm.txt`, `probe-build.json`, `probe-verify.json`. None of
them is committed and none is packaged as firmware.

### 5.5 What the probe deliberately is not

* Not wrapped in a BOL4/LZ4 frame and not placed in an Android v2 container, so it
  cannot be dropped into p4 and booted by accident.
* Not registered in any attempt journal, not signed, not given a generation cookie.
* Not executed anywhere — the "behaviour" result above is a host-side interpreter of
  the emitted instructions, not hardware execution.

---

## 6. Suggested precise edits to `main` documentation

Text-only suggestions; nothing under `main` was modified in this round.

1. `docs/00-boot-chain-and-partitions.md` — where the boot chain starts with "vendor
   Boot0", add: *"On units installed by the raw-capsule path, LBA 16 holds a 128 KiB
   BaseOS capsule (`native/capsule/build.py`): dispatcher at `0x0`, the contract vendor
   Boot0 at `+0x1000`, the 48 KiB SPL at `+0x11000`. The dispatcher verifies both extents
   by FNV-1a32 (`0x8fb6d562` / `0xa53ccffb`) and copies the SPL to `0x20000`."*
2. `experiments/h700-boot-speed/spl/README.md` — the sentence "The fresh snapshots
   confirm valid eGON images at sectors 16 and 512" can be sharpened to note that the
   two are **different vendor revisions** (five differing bytes, `0x0c`/`0x34`/`0xa3`/
   `0xac`), and that only the LBA 16 image is the capsule.
3. `docs/07-partition-layout-and-updates.md` — record the current label drift: GPT says
   `p6 = UDISK`, `p7 = primary`, while the generated `bootargs` still says
   `appfs@mmcblk0p6:UDISK@mmcblk0p7`; mounts are device-node based, so the drift is
   cosmetic but visible to external tools. The fix belongs in the `bootargs` template,
   not in the GPT.
4. `native/build.py` — consider emitting `generation_inputs_sha256` (a digest over the
   same inputs, in the same order) into `manifest.json`. It would let a field unit prove
   a generation without shipping the inputs. (Design suggestion only.)

---

## 7. Remaining unknowns

1. **BROM residency of `0x20000`–`0x40000`.** The capsule verifies extents at `0x21000`
   and `0x31000`, 64 KiB beyond the documented 32 KiB SRAM A1 window. Provenance of that
   mapping is not established on the host.
2. **LBA 512 provenance.** A structurally valid vendor Boot0 (`4701a85e…`) that matches
   no contract hash. Whether the BROM ever falls back to it is still unproven.
3. **Byte-exact generation rebuild.** Blocked by the pinned `boot-package.bin`
   (`56371d98…`) and the build-time GPT prefix; both are absent, and the unit's TOC1
   (`e6c20e3a…`) is not the contract image.
4. **CI artefact cross-check of the SPL.** 32 artefacts exist with commit-SHA names; the
   download requires an authenticated token, so no manifest-to-binary comparison was
   made.
5. **Whether the deployed p4 + 48 MiB image (`d71e5bd8…`) is byte-identical to the
   builder's `image` input.** The installer may rewrite the frame header.

---

## Appendix — evidence index (local, read-only)

| File | Used for |
| --- | --- |
| `build/audit-work/stage0/id_probe.py`, `id_probe2.py`, `id_probe3.py` | header scan, string map, segment alignment |
| `build/audit-work/stage0/disasm_boot0.py`, `disasm2/3/4/5.py`, `a64entry.py` | dispatcher, relocator, SPL entry, A64 entry |
| `build/audit-work/stage0/fnvcheck.py`, `cookie.py`, `collect.py` | FNV coincidence checks, cookie binding, `stage0.json` |
| `build/audit-work/stage0/spl_extract.py` → `spl-from-capsule.bin` | 48 KiB SPL extraction (local only) |
| `build/audit-work/probe/build_probe.py`, `verify_probe.py` | probe build and verification |
| `build/rg34xx-inventory-20261122-214708/{boot0-primary,boot0-secondary,boot-partition}.bin` | captured stage-0 media |
| `build/rg34xx-inventory-20261122-214708/{live-fdt.dtb,cmdline.txt,gpt-fdisk.txt,mounts.txt}` | generation, bootargs, GPT, mounts |
| Device reads: `/init`, `/etc/init.d/rcS`, `/etc/fstab` | mount semantics |

No binaries, serial numbers, MAC addresses or absolute host paths are included.
