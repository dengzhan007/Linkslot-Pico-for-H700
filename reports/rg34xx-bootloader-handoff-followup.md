# RG34XX / H700 stock U-Boot, monitor and DTBO hand-off — deep read-only follow-up audit

This is a follow-up to `reports/rg34xx-boot-partition-deep-audit.md`. It answers four questions
that the earlier report left open: how the stock chain selects a boot image, how the DTB is
modified before Linux, how a 32-bit U-Boot reaches a 64-bit kernel through the vendor monitor,
and whether a maintainable LK take-over path exists. It also closes the previously unresolved
`BOL4` image format.

Evidence level notation (same as the earlier report):
- **A** = byte-exact match against a primary artifact (local binary, on-device readback, or cited source line).
- **B** = strong structural/named-symbol correlation, not whole-file identity.
- **C** = inference from surrounding evidence; explicitly not proof.

All analysis was performed **read-only** on local copies of binaries captured earlier, plus
read-only ADB queries on the device. No writes to the device, SD card, Boot0, TOC1, U-Boot,
monitor, GPT or any device node were made. No SMC was issued and no ION ioctl was executed by
this audit. No captured code was executed. All original binaries remain local-only and are
**not** uploaded to this branch. Device serial numbers, MAC addresses and local paths are
redacted throughout.

---

## 0. Input validation (mandatory gate)

All four TOC1 items were re-hashed and checked against `toc1-extracted/manifest.json`
before any analysis. All matched exactly (**A**):

| Item | Size (B) | SHA-256 (full) | Verdict |
| --- | --- | --- | --- |
| `items/u-boot.bin` | 1,048,576 | `4d60a22ff13eac00dba89e0ee7df52a49852c0d9d856bec545ce40126191d127` | match |
| `items/monitor.bin` | 107,216 | `676bc0206e3c9a7f229d93ea7f3286095f669940ad91f3c4f3d37bdb18696c31` | match |
| `items/dtbo.bin` | 2,336 | `4bee281701610f96dcbdd1d38cb8df28386224d8c68b668b9a7a7a9df8e73ec2` | match |
| `items/dtb.bin` | 144,384 | `c28c87380ffab00148bf4dc457e70b8c6c576470dcfbc53df64addddf89603a1` | match |

TOC1 package SHA-256 `e6c20e3a54c51b9336c798e3a4fa1ca6c45b28c6f70456e2db9c252aa4ba4604`
also matches. The `monitor.bin` hash is identical across all eight local inventory batches
(`…181713` through `…214708`) and equals the BaseOS RG34XX prepared monitor fingerprint cited
in `docs/rg34xx-boot-handoff-audit.md` (**A**). No analysis below mixes versions.

---

## 1. U-Boot boot flow (question 1 & part of 4)

### 1.1 Execution state confirmed: AArch32 ARM

`u-boot.bin` is a **32-bit ARM (AArch32) image**, not AArch64 and not Thumb (**A**):
- Reset word @0x0 = `0xea00018e` = ARM `B 0x638` (`EA` = unconditional ARM branch).
- At 0x638 the standard ARM exception vector table follows (`b 0x4a000900`, then
  `ldr pc,[pc,#0x14]` entries), with `smc #0` / `hvc #0` thunks at 0xb24/0xb44/0xb5c.
- Header: magic `uboot\0`, version `4.0.0\0\0\002.0.0`, size 0x100000, and `0x4a000000`
  at header+0x2c — the sunxi U-Boot load/relocation base (consistent with the BaseOS
  A32 BL33 address `0x4a000000`).

This is the structural reason a monitor is required at all: an AArch32 BL33 cannot directly
enter an AArch64 `ARMd` kernel; an EL3 firmware must perform the width/exception-level change.

### 1.2 Image selection chain

From the U-Boot default-environment pointer tables in rodata (**A**, literal words
decoded to their target C strings):

```
bootcmd  -> table@0x5978 -> "bootcmd"
             candidate values in the same table:
             0x597c -> "run setargs_mmc boot_normal"
             0x5988 -> "run setargs_nand boot_normal"
boot_normal      -> 0x9bb75 -> "boot_normal"
                     0x36bc points at it from the command/env table
setargs_mmc      -> 0x9a81c ; setargs_nand -> 0x9a828 ; setargs_nor -> 0x9a835
update_bootcmd   -> 0x9a841 ("update bootcmd\n")
boot_type        -> 0x9bcb2 -> " boot_type=%d"
androidboot.boot_type -> 0x9bcc0
```

The captured env partition (`env-report.json`, CRC32 valid at offset 4) contains the
**NAND** variant verbatim:
`bootcmd=run setargs_nand boot_normal`, `boot_normal=sunxi_flash read 45000000 boot;bootm 45000000`,
`setargs_nand ... root=${nand_root}` with `nand_root=/dev/nand0p4`. The **live** cmdline
(from `/proc/cmdline`, re-read this session) is the **MMC** variant:
`root=/dev/mmcblk0p5 rootwait quiet splash ... boot_type=1 androidboot.boot_type=1
uboot_message=2018.05(05/26/2026-13:55:09) ...` (**A**).

Thus the bootcmd stored in the CRC-valid env is **not** the bootcmd that produced the running
system. See section 4 for the full reconciliation.

`bootm 45000000` uses load address `0x45000000` — matching the BaseOS staging address
`0x45000000`, while the Android header declares kernel load `0x40080000`, ramdisk `0x42000000`
and tags `0x40000100` (**A**, header parse; the earlier report already established these).
The boot partition holds two `ANDROID!` images; their roles are resolved in section 7.

### 1.3 U-Boot boot flow diagram

```
BROM (AArch32)
  └─ Boot0 (eGON.BT0, LBA16 primary / LBA512 secondary)
       ├─ DDR training (LPDDR4 672 MHz candidate at 0x38; size see §3)
       └─ TOC1 package @ LBA 16+8
            ├─ item0 u-boot.bin   @ TOC offset 0x800      (AArch32, base 0x4a000000)
            ├─ item1 monitor.bin  @ TOC offset 0x100200   (AArch64 BL31, EL3)
            ├─ item2 dtbo.bin     @ TOC offset 0x11a000   (PMIC overlay only)
            └─ item3 dtb.bin      @ TOC offset 0x11ac00   (base FDT, 512 MiB)
                 │
                 ▼ U-Boot runs, reads p3 env (CRC-valid), selects storage
                 │  default bootcmd table offers setargs_mmc / setargs_nand
                 │  runtime chose MMC (boot_type=1 -> root=/dev/mmcblk0p5)
                 ▼
            sunxi_flash read 0x45000000 boot ; bootm 0x45000000
                 │  -> Android boot image at p4 offset 0x0 (primary, gzip kernel)
                 │  -> kernel @0x40080000, 512 B empty initrd @0x42000000
                 │  -> FDT fixups: memory 512M->1G, initrd, disp_reserve, bootargs
                 ▼
            SMC into vendor monitor (BL31, loaded 0x48000000)
                 │  BL31 validates next image (BL33 = A32 U-Boot, SPSR 0x1d3)
                 ▼
            BL31 final SMC service -> ERET to AArch64 kernel @0x40080000
                 │  SPSR_EL3 = 0x3c5 (EL1h), x0 = FDT @0x44000000
                 ▼
            Linux 4.9.170 #19 (root=/dev/mmcblk0p5)
```

---

## 2. DTB / DTBO relationship (question 2)

### 2.1 Four FDTs decompiled and compared

Full DTS decompilations were produced locally for all four trees (kept local-only). Hashes are
of the FDT `totalsize` crop.

| Tree | totalsize | SHA-256 prefix | `/memory@40000000` reg | nodes |
| --- | --- | --- | --- | --- |
| TOC1 `dtb.bin` | 139,822 | `7017a8f970234f75…` | start 0x40000000, size 0x20000000 (**512 MiB**) | 573 |
| Primary boot-image DTB @ partition 0x6b2800 | 137,770 | `e2f3ec2aa9ad78ce…` | 512 MiB | 573 |
| Secondary boot-image DTB @ partition 0x3805800 | 139,822 | `7017a8f970234f75…` = **byte-identical to TOC1** | 512 MiB | 573 |
| Live FDT (`/sys/firmware/fdt`, re-read this session) | 143,936 | dynamic (see below) | start 0x40000000, size 0x40000000 (**1 GiB**) | 573 |

Exact reg cells (big-endian 4×u32), read from raw bytes (**A**):
- TOC1 / primary / secondary: `00 00 00 00 40 00 00 00 00 00 00 00 20 00 00 00`
- live: `00 00 00 00 40 00 00 00 00 00 00 00 40 00 00 00`

The device's current `/sys/firmware/fdt` (read read-only via base64 to avoid ADB newline
translation) is 143,936 B and differs from the 2026-11-22 captured `live-fdt.dtb` in **exactly 3
properties**, all under `/chosen`: `baseos,boot-display`, `baseos,native-timing`, `bootargs`
(dynamic timing/panel/boot-reason values). Node count and all other properties are identical
(**A**). This both validates the earlier capture and confirms those three properties are
rewritten on each boot.

### 2.2 DTBO contents (all fragments resolved)

The DTBO has two real fragments plus fixup metadata (**A**, full decompile):

```
/ { compatible="board_manufacturer,board_model"; board_id=<0x20000>;
    board_rev=<0x20001>; another_hw_information="some_data"; soc_id=<0x68000000>; }
/fragment@1 { target=<0xffffffff>; }             # needs fixup
  /fragment@1/__overlay__ { #address-cells=<1>; #size-cells=<0>; no_suspend=<1>; status="okay";
    pmu { compatible="x-powers,axp1530"; reg=<0x36>; wakeup-source;
          standby_param { vcc-dram=<4>; };
          regulators { dcdc1/dcdc2/dcdc3/ldo1/ldo2 with regulator-*. }; } }
/fragment@2 { target=<0xffffffff>; }             # needs fixup
  /fragment@2/__overlay__ { cpu-supply=<phandle 1>; }
/__fixups__  { twi5="/fragment@1:target:0"; cpu0="/fragment@2:target:0"; }
/__symbols__ { pmu0, standby_param, reg_dcdc1..reg_aldo1, reg_dldo1 }
```

Applying the DTBO to each of the three base DTBs resolves the fixups identically in all three
(bases' `__symbols__`: `twi5 = /soc@03000000/twi@0x07081400`, `cpu0 = /cpus/cpu@0`):
- fragment@1 adds a `pmu` (axp1530) child under `twi5` and sets `#address-cells/#size-cells/no_suspend/status` there;
- fragment@2 sets `cpu-supply=<1>` on `cpu0`.

**The DTBO touches no node or property named memory, chosen, initrd, bootargs, display,
framebuffer or reserved-memory.** An exhaustive name scan over every DTBO node/property
returned zero hits (**A**). The PMIC it adds (`axp1530`) does not even match the PMIC present in
the base DTB (`axp2202` on the same `twi5` bus), suggesting this DTBO is a generic/vendor
overlay template rather than an active board patch on this boot — its application at runtime
is not proven here (**C**).

### 2.3 Answer: is 512 MiB → 1 GiB caused by the DTBO?

**No.** All three on-disk base DTBs already declare 512 MiB, and the DTBO contains no memory
modification. The expansion to 1 GiB must be injected between U-Boot loading the DTB and the
kernel parsing it. Section 3 locates the code that can do this.

### 2.4 Node-level diff: base DTB vs live FDT

Against all three bases the live tree adds/changes the same key items (**A**):

| Path / property | On-disk bases | Live |
| --- | --- | --- |
| `/memory@40000000:reg` | 512 MiB | 1 GiB |
| `/chosen:linux,initrd-start` | 0x0 | 0x42000000 |
| `/chosen:linux,initrd-end` | 0x0 | 0x42000200 |
| `/chosen:bootargs` | stock short string | full cmdline incl. `root=/dev/mmcblk0p5`, `disp_reserve=1382400,0x7bf24800`, `uboot_message=…`, `boot_type=1`, `cma=64M`, `loglevel=1 rcupdate…` |
| `/chosen:baseos,native-boot` / `native-timing` / `boot-display` | absent | present (dynamic per-boot values) |
| `/soc@03000000/disp@01000000:boot_fb0` | absent | `"7bf24800,2d0,1e0,20,b40,0,0,2d0,1e0"` |

The `baseos,*` properties are the exact three trailing additions that BaseOS-MultiFrontend's
native A32 loader documents writing into the final FDT (DESIGN.md, "What the kernel receives").
Their presence in this device's live FDT is strong evidence (**B**, bordering on A given the
exact names and semantics) that the **currently running chain used the BaseOS native loader**,
not a stock U-Boot-only path. See section 8 for the consequence.

---

## 3. Where 512 MiB → 1 GiB actually happens

Since the DTBO is not involved, the change comes from U-Boot's own FDT fixup path. Static string
and literal-pool analysis of `u-boot.bin` found the complete fixup vocabulary (**A**):

```
0x77a5b  fdt_fixup_memory_banks
0x79d32  update_fdt_dram_para
0x9be01  "update dtb dram start"  ... 0x9bf28 "update dtb dram  end"
         dram_clk, dram_type, dram_odt_en, dram_para1, dram_para2,
         dram_mr0..3, dram_tpr0..13              (full DRAM parameter set)
0x7ef7d  "DRAM bank"
0x7c79e  "## WARNING: fixup memory failed!"
0x826c4  "fdt_chosen: %s"      0x8298f "fdt_initrd: %s"
0x82a64  "failed on fdt_overlay_apply(): %s" / "base fdt does did not have a /__symbols__ node"
0x85973  "ERROR: arch-specific fdt fixup failed" / "ERROR: board-specific fdt fixup failed"
0x822dc  "Set disp_reserve env fail!" ; 0x9bd66 "disp_reserve" / 0x9bd73 " disp_reserve=%s"
0x3830   literal -> "sunxi_update_fdt_para_for_kernel"
0x3834   literal -> "##add mem rsv error: %s : %s\n" / 0x3838 "update dts\n"
```

Interpretation: U-Boot reads the actual DRAM geometry (Boot0 training result passed in the
`soc/dram_para` node — literal 0x59d0 → `"soc/dram_para"`, 0x59cc → `"dram_clk"`), then calls
`update_fdt_dram_para` / `fdt_fixup_memory_banks` to rewrite `/memory`, and separately performs
`fdt_chosen` (bootargs), `fdt_initrd` (initrd range) and `disp_reserve` fixups before `bootm`.
This is a coherent, vendor-standard chain that explains every live-vs-on-disk difference
observed in section 2.4. **Evidence level**: existence and names of the fixup path = **A**;
the specific call ordering and the exact function that writes the 0x40000000 size cell = **C**
(no symbol table in the raw U-Boot image and no early boot console trace were available).

Consequence for an LK port: never derive DDR size from the 512 MiB on-disk DTB, but also never
assume 1 GiB blindly — read the Boot0-trained geometry / `soc/dram_para` at hand-off, as the
stock chain does. The `baseos,native-timing` values in the live FDT are the native loader's own
DRAM timing census, a second independent source of the trained parameters.

---

## 4. NAND bootcmd vs MMC root contradiction (question 4)

The captured environment is internally consistent but not the environment that booted this
system. Four independent facts resolve the apparent contradiction:

1. **The stored env is one of several candidates.** U-Boot's default table contains both
   `run setargs_mmc boot_normal` (0x597c) and `run setargs_nand boot_normal` (0x5988),
   plus `setargs_mmc` / `setargs_nand` / `setargs_nor` and an `update bootcmd` path
   (0x9a841) (**A**). The `bootcmd` variable is rewritten at runtime according to the
   detected boot medium — `update bootcmd` is a named U-Boot routine in this binary.
2. **The selector is `boot_type`.** The rodata table contains `boot_type=%d` (0x59c0) and
   `androidboot.boot_type=%d` (0x59c4) (**A**), and the live cmdline carries
   `boot_type=1 androidboot.boot_type=1`. This device booted from MMC (SD), so the MMC
   branch of setargs was used, yielding `root=${mmc_root}=/dev/mmcblk0p5` — exactly what
   `/proc/cmdline` shows (**A**).
3. **A CRC-valid env only proves self-consistency.** The env report itself warns that a valid
   CRC does not prove which copy U-Boot selected nor that startup code did not modify bootcmd.
   Here the on-disk value is the NAND default; the executed value was the MMC override.
4. **There is no NAND device on this machine.** GPT shows an 8-partition eMMC/SD layout
   (`mmcblk0` p1..p7) and `nand_root=/dev/nand0p4` resolves to nothing in `/proc/partitions`.
   `sunxi_flash read … boot` is the generic sunxi command that reads the named GPT partition
   from the selected medium, i.e. `boot` = p4 on MMC in this boot (**A** from GPT + cmdline).

**Conclusion**: `bootcmd=run setargs_nand …` is the fallback/default stored in the env image;
the boot actually executed the MMC variant. There is no hardware contradiction — only a
difference between the persisted default and the runtime-selected override. Two practical rules
follow: (a) do not treat the p3 env `bootcmd` as the executed command without checking
`boot_type`; (b) an LK replacement must set its own bootargs explicitly rather than relying on
stored env.

---

## 5. monitor: A32 → A64 hand-off evidence (question 5)

### 5.1 monitor is vendor BL31, not a custom stub

`monitor.bin` is a full **ARM Trusted Firmware BL31** for `plat/sun50iw9p1` (**A**, strings):
- Header: `CHKv1.0`, name `monitor`, embedded SHA-looking string, `0x48000000` at +0x2c —
  matching the 16 MiB memblock-reserved monitor window and the BaseOS monitor load address.
- Entry @0xf8: `b` to header end, then `sctlr_el3`/`vbar_el3`/`cptr_el3` setup, `mrs/mpidr_el1`,
  and `b #0x158c8` (BL31 main) — classic AArch64 EL3 initialization. No static SMC instruction
  exists in the image; EL3 SMC handling is via the VBAR_EL3 vector table at 0x19000.
- Source-path strings: `bl31/bl31_main.c`, `plat/sun50iw9p1/bl31_sunxi_setup.c`,
  `plat/sun50iw9p1/plat_gic.c`, `plat/sun50iw9p1/plat_pm.c`, `bl31/context_mgmt.c`,
  `bl31/interrupt_mgmt.c`, `lib/aarch64/xlat_tables.c`.
- Hand-off strings: `bl31_plat_get_next_image_ep_info`, `NOTICE: set kernel entry`,
  `bl31_prepare_next_image_entry`, `NOTICE: BL3-1: Preparing for EL3 exit to %s world`,
  `NOTICE: BL3-1: Next image address = 0x%llx`, `NOTICE: BL3-1: Next image spsr = 0x%x`,
  `cm_prepare_el3_exit`, `cm_set_elr_el3`, `secure os exist`.
- Runtime services present: PSCI (`WARNING: Unimplemented PSCI Call`) and TSP/TSPD
  (`tspd_smc_handler`, `sunxi_tspd_cpu_on_init`) — a secure payload dispatcher exists.

### 5.2 The measured A32→A64 ABI (from BaseOS-MultiFrontend, cross-checked)

The cited BaseOS sources (`entry32.S`, `spl/verify-monitor.py`, `DESIGN.md`, fetched read-only
this session) establish the following contract for the **same monitor SHA** (**A** for the
source statements; **B** for carrying them to this exact device, since the verifier was run on
other H700 boards):

1. BL31 first ERET returns to the A32 BL33 at `entry=0x4a000000`, `SPSR_EL3=0x1d3`
   (AArch32 SVC, ARM state, IRQ/FIQ masked). This is why the stock BL33 is AArch32 U-Boot.
2. The native A32 loader ends with (verbatim `entry32.S`):
   `movw r0,#0xff04; movt r0,#0x8000` → `r0=0x8000ff04`,
   `r1=0x40080000` (kernel entry), `r2=0x44000000` (FDT), `r3=1`, `smc #0`.
   Before the SMC it performs a full set/way D-cache clean, clears SCTLR.M/C/I
   (`bic r0,#1/#4/#0x1000`), invalidates TLB+I-cache, `dsb sy`/`isb`, and masks interrupts.
3. After that SMC, the verifier observes a second ERET with
   `entry=0x40080000`, `SPSR_EL3=0x3c5` (AArch64 EL1h), `x0=0x44000000`, `x1..x3=0` —
   i.e. a correct Linux-style AArch64 entry.
4. `verify-monitor.py` must patch the monitor header with Boot0 DRAM parameters and board
   bytes (`monitor[0x30:0xb0]=boot0[0x38:0xb8]`, `monitor[0x24:0x2c]=boot0[0xf0:0xf8]`) and set
   a DTB pointer at 0xb0 before the monitor will run correctly. That patching is part of the
   chain; a bare "load monitor and jump" is not.

### 5.3 Investigation of `0x8000ff04`

- **Where it comes from**: it is the SMC function ID of a **vendor "jump to Linux" runtime
  service** implemented in this BL31 (not a generic jump API). It appears in BaseOS's A32
  `native_kernel_enter`, and the U-Boot binary independently contains the complementary
  `drm_base=0x%lx` / `drm_size=0x%lx` strings and `smc` thunks — but the numeric constant
  `0x8000ff04` does **not** appear anywhere in the local `u-boot.bin` or `monitor.bin` as a
  literal (**A**: byte search). It is baked into the BaseOS loader source, and the monitor
  accepts it as a registered SMC call.
- **Call convention**: AArch32 SMC, `r0` = service id 0x8000ff04, `r1` = target entry,
  `r2` = FDT pointer, `r3` = 1; further parameters via the standard SMC a0-a7 convention
  (U-Boot's `smc_call` at file 0x156d4 loads r0-r7 from a pointer block).
- **SPSR / EL / cache**: the monitor sets `SPSR_EL3=0x3c5` on exit (AArch64, EL1h, DAIF masked),
  `ELR_EL3=r1`. The caller must clean caches and disable MMU/caches first, as shown above.
- **Target address**: in all verified runs `r1=0x40080000` — the stock Linux load address. The
  BaseOS verifier only asserts a custom target when the manifest explicitly pins one, and its
  own diagnostic trampoline is still an AArch64 stub, not arbitrary user code.

**Hard limit (as required by the task): the Linux-start SMC is not a general bare-metal
jump interface.** The evidence proves only that this service reliably enters a payload that
obeys the stock Linux entry contract (address 0x40080000, FDT in x0, EL1h, caches/MMU state as
the stock kernel expects). Repointing `r1` to an arbitrary AArch64 payload requires re-proving
target address validity, SPSR/EL, register state, security/world state, cache and FDT ownership,
plus a minimal payload cold-boot test. No such proof exists today; no payload address is
approved by this report.

---

## 6. DRM SMC evidence (question 6, kernel side)

### 6.1 Recovered symbol positions (A)

The kallsyms table in the decompressed primary `kernel.Image` was recovered with
`vmlinux-to-elf` (89,675 symbols; `KALLSYMS_BASE_RELATIVE`). Key AArch64 virtual addresses:

| Symbol | VA |
| --- | --- |
| `optee_probe_drm_configure` | `0xffffff80086f663c` |
| `optee_smccc_smc` | `0xffffff80086f6098` |
| `optee_smccc_hvc` | `0xffffff80086f6284` |
| `sunxi_ion_probe` | `0xffffff80086e1244` |
| `sunxi_ion_probe_drm_info` | `0xffffff80086e0fbc` |
| `ion_secure_heap_create` | `0xffffff80086debb8` |

VA→file mapping was calibrated against `_text=0xffffff8008080000` and verified with correct
function prologues at `_stext`. `kernel.Image` itself is AArch64 (`ARMd` magic @0x38,
Linux 4.9.170 #19), and IKCONFIG already confirmed `CONFIG_ION_SUNXI/CONFIG_ION_OF/CONFIG_TEE/
CONFIG_OPTEE/CONFIG_CMA` in the earlier report (**A**).

All four DRM log strings exist byte-exactly in the image (**A**):
`drm config service not available: %X` @0xbf1bbb, `drm_base=0x%x` @0xbf1be0,
`drm_size=0x%x` @0xbf1bef, `tee_base=0x%x` @0xbf1bfe — adjacent to
`optee: initialized driver`, i.e. in the OP-TEE driver string pool.

### 6.2 SMC call site and static-call caveat

`optee_smccc_smc` decompiles at its symbol address to `brk #0x800` — it is the
**weak/alt-instructions placeholder**; the real SMC/HVC is selected at runtime from the
`/firmware/optee` `method="smc"` property (present in every DTB, **A**). Consequently there is
no static `smc` instruction inside `optee_probe_drm_configure`, and a naive BL-xref scan returns
no direct callers — this is expected for the OP-TEE invocation design, not evidence of absence.
The `OPTEE_SMC_GET_DRM_INFO` function ID (15 in the qiaoweibiao/T507_Kernel reference) reaches
the monitor via that runtime-selected optee invoke pointer. **Evidence level**: function
existence, addresses, strings = **A**; the exact instruction issuing SMC #15 = **C** (not
recovered here because it is runtime-dispatched; a Ghidra-level data-flow analysis would be
needed and is not claimed).

### 6.3 Runtime logs (read-only, this session)

Full `dmesg` (410 lines), `/proc/sys/kernel/printk` (`1 7 1 7`), `/proc/cmdline` and all ION
debugfs heaps were captured read-only this session. Result (**A**):
- The only OP-TEE lines are `optee: probing for conduit method from DT.` and
  `optee: api uid mismatch` — the standard OP-TEE UID handshake failed, so no `/dev/tee0`.
- **Neither** `drm_base/drm_size/tee_base` (success) **nor** `drm config service not available`
  (failure) appears anywhere in the full log. Early boot messages are suppressed by
  `loglevel=1`, so absence of these lines cannot decide whether the vendor SMC ran, succeeded,
  or failed. No SMC was issued and no ioctl executed by this audit.
- ION heaps: `sys_user` ~7.4 MB (display + vin allocations), `cma` with region
  `0x77c00000..0x7bc00000` (64 MiB, matches `CmaTotal`), `secure` heap present but empty.
  CMA debugfs path is not present on this build. The 16 MiB `0x7ec00000..0x7fc00000` region
  remains owner-unknown and excluded from any usable-RAM claim.

There is also a **second, independent DRM path in U-Boot**: literal pool entries
`drm_base=0x%lx` (0x3828) and `drm_size=0x%lx` (0x382c) plus `smc tee …` service strings
(0x8fea6 etc.) show the vendor bootloader itself queries DRM/TEE info via SMC. Both paths
converge on the same BL31 service set but are compiled in different images; do not assume one
proves the other ran.

---

## 7. BOL4 secondary image — resolved (question 7)

The secondary boot image's "kernel" component (8,406,862 B) is a **BOL4 container with an LZ4
block payload**, fully decoded this session (**A**):

```
offset 0x00: "BOL4"
offset 0x04: u32 LE compressed-payload length = 0x0080473e (8,406,846)
offset 0x08: u32 LE uncompressed length      = 0x010a7008 (17,461,256)
offset 0x0c: u32 LE checksum/stream marker    = 0xb4c47d39 (not zlib crc32/adler32; semantics unknown)
offset 0x10: standard LZ4 block (8,406,846 B) -- no LZ4 frame header
```

Decompressing bytes `0x10 .. 0x10+0x80473e` as a raw LZ4 block with `uncompressed_size=17,461,256`
yields a 17,461,256-byte AArch64 `ARMd` kernel whose SHA-256 is
`39cf8c3eb9a75ff0eda51b4074cd65860c5aa5386093bcaf69339819421d84b7` — **byte-identical to the
primary image's kernel** and carrying the same `4.9.170 #19` banner (**A**). `0x10 + 0x80473e =
0x80474e = blob length`, so the header accounts for the payload exactly.

Conclusion: the secondary image is not a different kernel and not an unknown format. It is the
**same kernel encoded as BOL4/LZ4**, consistent with BaseOS DESIGN.md's "bounded Android/BOL4
parsing" + "bounded LZ4 decoder" and with the vendor/older image retained in the tail of p4
when BaseOS wrote the gzip primary over p4. Its precise role (factory recovery vs. preserved
pre-patch payload) is still **C**; provenance against a specific Anbernic V1.0.6 release was not
byte-verified here.

---

## 8. LK take-over feasibility and option comparison (questions 8 & 9)

### 8.1 What the evidence supports

- The stock chain is **AArch32 U-Boot → AArch64 BL31 → AArch64 kernel**. The width change is
  entirely inside the vendor monitor; an LK that runs as a replacement BL33 must either be
  AArch32 (and use the same Linux-start SMC) or be entered by an AArch64-capable first stage.
- The live FDT already carries BaseOS-native markers, so on this device a native SPL path has
  in fact been exercised. BaseOS reports native SPL + vendor monitor + A32 BL33 working on
  other H700 boards (RG34XX SP 2 GiB LPDDR4, RG40XX V 1 GiB LPDDR3) with cold boot, display,
  suspend/wake and repeated boots — but the cited records do **not** show an RG34XX non-SP
  1 GiB LPDDR4 cold-boot acceptance of an arbitrary payload.
- Monitor entry is not free-standing: it needs the Boot0 DRAM parameters patched into its
  header, a DTB pointer, and the exact cache/MMU/SPSR contract of section 5.2. Any LK design
  that reuses it must reproduce all of that.

### 8.2 Option A — keep stock Boot0 + U-Boot + monitor, add an A32 loader and A64 payload

Replace or intercept the A32 BL33 that U-Boot/monitor returns to, keeping Boot0, TOC1, U-Boot,
monitor and GPT untouched. The added A32 loader would build the final FDT and issue the
`0x8000ff04` SMC to reach an AArch64 payload.

- **Pros**: smallest change surface; reuses proven DRAM training and the monitor's validated
  EL3 entry; recovery by re-flashing only the replaced component.
- **Cons**: still depends on the vendor Linux-start SMC with an unverified custom-target
  contract; must reproduce U-Boot's FDT fixups (memory/initrd/bootargs/disp_reserve) itself;
  A32 loader must be placed at 0x4a000000, colliding with U-Boot's own base; no proof that
  arbitrary `r1` targets work.
- **Verdict**: **feasible to study, not yet safe to ship** as a generic payload path.

### 8.3 Option B — A64 SPL + stock monitor + A32 BL33 (BaseOS native path)

Replace Boot0/SPL stage with a mainline-style AArch64 SPL that owns SD/PMIC/DRAM, loads the
stock monitor (patched with DRAM params), DTBO/DTB, and a RAM-only A32 BL33 loader; the loader
then uses the vendor SMC to enter AArch64.

- **Pros**: the only design with demonstrated end-to-end native Linux boots on sibling H700
  boards; strongest control over SD/recovery and DRAM; the live FDT on this very device already
  shows its three signature properties.
- **Cons**: takes over the Boot0/SPL responsibilities (highest-risk region); requires exact
  per-model DRAM-bank validation (RG34XX LPDDR4 672 MHz main candidate at 0x38, plus two
  alternates); requires reproducing the stock FDT byte-for-byte except three trailing
  properties; recovery complexity is highest.
- **Verdict**: **the best-supported architecture for a real port**, conditional on RG34XX
  non-SP cold-boot acceptance and a verified payload entry contract.

### 8.4 Option C — other evidence-supported variants

- **C1** Stock everything, add an LK/BL33 that never uses SMC and instead runs wholly in
  AArch32: supported by the 0x1d3 BL33 entry, but cannot host an AArch64 Linkslot payload.
- **C2** Keep stock chain but replace only the Android `boot` image payload with an AArch64
  payload that obeys the Linux entry contract (address 0x40080000, FDT at x0). This uses the
  monitor exactly as designed and avoids re-proving the SMC; the payload is a Linux-style
  kernel-like image rather than a conventional LK. **Lowest risk for a first AArch64 payload
  test**, but it is a payload-in-Linux-clothing, not a full LK takeover.
- Any design that proposes a new SMC, a new payload address, or skipping the monitor's DRAM
  patching is **not** supported by current evidence and is rejected.

### 8.5 Recommended architecture and risks

Recommend **Option B as the target architecture, with Option C2 as the first validation
step**: before any full LK port, boot a minimal AArch64 payload that only prints `CurrentEL`,
register arguments and a safe memory watermark, entered through the stock monitor using the
exact Linux contract. Only after repeated cold boots and a documented failure/recovery plan
should LK-specific HAL work begin.

Top risks, all currently open: (1) custom-target SMC not proven; (2) RG34XX non-SP native
cold boot not accepted; (3) DRAM bank/rank/width differences from sibling boards; (4) unknown
owner of `[0x7ec00000,0x7fc00000)` and top-of-RAM reserved fragments; (5) monitor header
patching and cache/MMU state must be exact; (6) no early-boot console trace exists to confirm
the U-Boot FDT fixup call order.

---

## 9. Open questions (unresolved, explicitly not claimed)

1. Exact instruction and function that writes the 1 GiB size cell into `/memory` (the fixup
   vocabulary is proven; the specific call site is not, no U-Boot symbols/trace available).
2. Whether this DTBO is actually applied at runtime (it targets an axp1530 PMIC while the base
   DTB has axp2202; no runtime overlay-apply log was captured).
3. Whether the vendor `GET_DRM_INFO` SMC executed at all on this boot, and if so its return
   values — both success and failure strings exist but neither was logged (loglevel=1).
4. Whether `r0=0x8000ff04` accepts an arbitrary AArch64 target and what exact security/EL/cache
   state it leaves — Linux-start evidence only.
5. Complete bit-exact provenance of the BOL4 secondary image against a named Anbernic release
   (format and kernel identity are resolved; release mapping is not).
6. Full compensation contract for the three `baseos,*` FDT additions on a stock-path boot:
   their presence here indicates a native loader booted this unit, but the earlier report's
   "which on-disk DTB U-Boot passed before fixup" remains undecided.
7. One device serial number was present in the re-read cmdline and is redacted here; no other
   identifiers are retained in this report.

---

### Evidence references (all read-only; local unless cited)

- `build/rg34xx-inventory-20261122-214708/`: `toc1-extracted/manifest.json`,
  `toc1-extracted/items/{u-boot,monitor,dtbo,dtb}.bin`, `boot-partition.bin`,
  `env-report.json`, `cmdline.txt`, `live-fdt.dtb`, `debugfs-memory-reservations.txt`,
  `tee-ion-runtime.txt`, `boot0-primary.json`.
- Local read-only analysis outputs (not uploaded): full DTS decompiles of the four FDTs,
  DTBO fragment report, node-level diffs, U-Boot string/xref report, U-Boot SMC disassembly,
  recovered kernel kallsyms (89,675 symbols) and DRM function disassembly, BOL4 decode,
  full 410-line dmesg capture, current `/sys/firmware/fdt` base64 readback.
- On-device read-only this session: `/proc/cmdline`, `/proc/sys/kernel/printk`, full `dmesg`,
  `/sys/firmware/fdt`, `/sys/kernel/debug/ion/heaps/*`, `/proc/meminfo`.
- Upstream read-only via HTTPS: `dengzhan007/BaseOS-MultiFrontend`
  `experiments/h700-boot-speed/native/entry32.S`, `spl/verify-monitor.py`, `spl/README.md`,
  `DESIGN.md`, `HANDOFF.md`; `marin-m/vmlinux-to-elf` `core/kallsyms.py`.
- Prior reports: `reports/rg34xx-boot-partition-deep-audit.md`, `docs/rg34xx-boot-handoff-audit.md`.
