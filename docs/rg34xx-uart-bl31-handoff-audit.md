# RG34XX UART0: SPL → BL31 → AArch32 SVC BL33 handoff audit

**Scope:** RG34XX non-SP / H700, exact native build pin and exact captured vendor monitor.
**Result:** static source/binary path supports UART0 being initialized before BL31 and reused by the vendor monitor before BL33. **M1-S1 remains OPEN**: physical pad identity, target fuse/SPC effect and live BL33-entry register state are not proven.

## 1. Evidence pins

| Object | Pin / result |
|---|---|
| BaseOS native builder | `cbe253a43f46cf551cc9a6c79a84015bde3686cf` |
| U-Boot v2026.10 peeled source commit | `5508406582f6f7120dce0c4b3059819a41788db2` |
| U-Boot v2026.10 source archive | SHA-256 `f943a88389c8fd70a6b0d061a88267465e143c7ca6ea2b75837ab20f5b60c739` |
| deployed/rebuilt vendor monitor | 107,216 B, SHA-256 `676bc0206e3c9a7f229d93ea7f3286095f669940ad91f3c4f3d37bdb18696c31` |
| RG34XX native-generation reproduction | [BaseOS run #45301188593](https://github.com/dengzhan007/BaseOS-MultiFrontend/actions/runs/45301188593) |
| host monitor-audit regression | [H700 boot audit #1510](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45999160104), Linux + Windows pass |

The monitor hash is the exact `monitor` item from the target's captured TOC1/native firmware. The disassembly offsets below are **offsets within this extracted 107,216-byte image**, not relocated monitor virtual addresses.

The host audit tool is `tools/audit_rg34xx_bl31_uart.py`; its regression tests are `tools/test_audit_rg34xx_bl31_uart.py`. The tool requires the exact 107,216-byte input and SHA-256 above. It validates fixed instruction words, AArch64 branch targets, MOVZ/MOVK-constructed MMIO addresses, and the SPC loop bounds. It does not execute code or access a device.

## 2. SPL performs the early UART0 setup before BL31

The pinned U-Boot v2026.10 `board_init_f()` sequence in
[`arch/arm/mach-sunxi/board.c`](https://github.com/u-boot/u-boot/blob/5508406582f6f7120dce0c4b3059819a41788db2/arch/arm/mach-sunxi/board.c)
is:

```text
sunxi_sram_init()
tzpc_init()
timer_init()
clock_init()
gpio_init()
spl_init()
preloader_console_init()
sunxi_board_init()
```

The BaseOS `spl/build-early-mmc.py` transformation replaces the normal
`spl_init()` call with `spl_early_init()` and inserts `baseos_early_mmc()`
before `sunxi_board_init()`. It does not remove the preceding clock, GPIO or
preloader-console calls.

### 2.1 Clock and pinmux

For `CONFIG_MACH_SUN50I_H616`, the pinned U-Boot source gives:

- `clock_init()` calls `clock_init_uart()`.
  [`clock.c`](https://github.com/u-boot/u-boot/blob/5508406582f6f7120dce0c4b3059819a41788db2/arch/arm/mach-sunxi/clock.c)
  and
  [`clock_sun50i_h6.c`](https://github.com/u-boot/u-boot/blob/5508406582f6f7120dce0c4b3059819a41788db2/arch/arm/mach-sunxi/clock_sun50i_h6.c)
  configure APB2 from OSC24M with N=1/M=1 (24 MHz), set the selected UART
  gate bit in CCU offset `0x90c`, and deassert its reset bit.
- For `CONFIG_CONS_INDEX == 1`, H616 `gpio_init()` assigns PH0 and PH1 to
  `SUN50I_H616_GPH_UART0` and enables pull-up on PH1.
  [`board.c`](https://github.com/u-boot/u-boot/blob/5508406582f6f7120dce0c4b3059819a41788db2/arch/arm/mach-sunxi/board.c).
  The H616 UART0 mux value is 2.
- [`serial.h`](https://github.com/u-boot/u-boot/blob/5508406582f6f7120dce0c4b3059819a41788db2/arch/arm/include/asm/arch-sunxi/serial.h)
  maps UART0 to `0x05000000` for the H6/H616 generation. The non-DM legacy
  console in
  [`sunxi-common.h`](https://github.com/u-boot/u-boot/blob/5508406582f6f7120dce0c4b3059819a41788db2/include/configs/sunxi-common.h)
  uses that UART0 base and a 24 MHz NS16550 input clock.
- `CONFIG_CONS_INDEX` is defined by
  [`drivers/serial/Kconfig`](https://github.com/u-boot/u-boot/blob/5508406582f6f7120dce0c4b3059819a41788db2/drivers/serial/Kconfig)
  with default 1 when the console index is specified. The archived reproduction
  artifact does not include the complete generated `.config`, so the default
  is a source-level configuration expectation, not a separately archived
  config-file readback.

### 2.2 Console initialization is linked into the exact native SPL

U-Boot `preloader_console_init()` calls `serial_init()` and then enables the
SPL console:
[`common/spl/spl.c`](https://github.com/u-boot/u-boot/blob/5508406582f6f7120dce0c4b3059819a41788db2/common/spl/spl.c).

The #45301188593 AArch64 SPL symbol map contains:

| Symbol | Image offset |
|---|---:|
| `board_init_f` | `0x20460` |
| `clock_init_uart` | `0x2087c` |
| `clock_init` | `0x2095c` |
| `baseos_early_mmc` | `0x23440` |
| `sunxi_board_init` | `0x24cb8` |
| `spl_early_init` | `0x24f78` |
| `preloader_console_init` | `0x25170` |
| `serial_init` | `0x2a374` |

The build log compiles the legacy/non-DM serial path:
`spl/drivers/serial/serial.o`, `serial_ns16550.o` and `ns16550.o`.
The #45301188593 symbol map contains `eserial1_device` and
`eserial1_init`, but no `eserial2_device` through `eserial6_device`.

In the pinned
[`serial_ns16550.c`](https://github.com/u-boot/u-boot/blob/5508406582f6f7120dce0c4b3059819a41788db2/drivers/serial/serial_ns16550.c),
`eserial1_device` is compiled only when `CFG_SYS_NS16550_COM1` is defined;
`default_serial_console()` selects it for `CONFIG_CONS_INDEX == 1`.
The U-Boot serial Kconfig defaults `SPECIFY_CONSOLE_INDEX` on for non-DM
SPL serial and gives `CONS_INDEX` a default of 1.
[`sunxi-common.h`](https://github.com/u-boot/u-boot/blob/5508406582f6f7120dce0c4b3059819a41788db2/include/configs/sunxi-common.h)
maps `CFG_SYS_NS16550_COM1` to `SUNXI_UART0_BASE`; the H6/H616
[`serial.h`](https://github.com/u-boot/u-boot/blob/5508406582f6f7120dce0c4b3059819a41788db2/arch/arm/include/asm/arch-sunxi/serial.h)
defines that base as `0x05000000`. The same console index controls the H616
PH0/PH1 GPIO branch and the UART gate/reset bit in `clock_init_uart()`.

Together, the exact link map, compiled legacy serial objects, Kconfig defaults
and source selection rules make `CONFIG_CONS_INDEX=1` / COM1 / UART0 the
strong build-level inference for this SPL. The generated `.config` was not
retained in the CI artifact, so this is **not** a direct config-file readback
or runtime trace. It does, however, narrow the linked console device to
COM1/UART0 rather than merely assuming that any UART might be used.


**H616 TZPC note:** the pinned U-Boot `tzpc_init()` header path is a no-op
for H616; do not count that call as proof of peripheral security permissions.
The vendor monitor's SPC/CCU security setup is a separate later stage.

## 3. The exact vendor monitor reuses the enabled UART before BL33

The target monitor's AArch64 code contains these fixed-offset calls:

| Image offset | Call target | Static interpretation |
|---|---:|---|
| `0x21a8` | `0xc0d4` | UART gate selector/helper |
| `0x21b4` | `0x2c78` | security/SPC setup |
| `0xc11c` | `0xbfa4` | UART base/configuration helper |

### 3.1 UART gate-selection and reuse branch

The helper at `0xc0d4` reads CCU UART gate/reset register
`0x0300190c`, scans the six UART gate bits from index 0 through 5, and
selects the lowest enabled index. It passes the selected index and a
“gate already enabled” flag to `0xbfa4`.

At `0xbfa4`:

- **Existing gate path:** when the flag is set, it computes
  `0x05000000 + index * 0x400`, stores the selected UART base in the
  monitor's internal global slot, and branches directly to the common return
  path. It does not reprogram CCU gate/reset or the UART divisor registers.
- **Fallback path:** when no UART gate bit is enabled, it configures the
  gate/reset state, reads/configures APB2 at `0x03001524`, and programs the
  UART registers. The fallback constants encode a 24 MHz input clock and
  115,200 baud divisor arithmetic.

One subtlety: the caller at `0x21a8` places values in argument registers
before calling `0xc0d4`, but the callee immediately overwrites them. Those
caller-side constants are **not** used as arguments by the decoded helper
and are deliberately not cited as evidence of UART configuration.

This makes the intended sequence clear: the SPL configures the UART first;
the monitor then detects an already-open UART gate and adopts that controller
instead of rewriting its clock/divisor state. This is a static branch/data-flow
conclusion from the exact monitor bytes, not a live register readback.

## 4. BL31 prepares non-secure peripheral access before A32 SVC BL33

The target monitor's BL33 preparation path calls the function at `0x2c78`
from offset `0x21b4`. Its instruction sequence constructs and writes:

| Register | Target address | Value | Static interpretation |
|---|---:|---:|---|
| SPC DEC_PORT_SET ports 0–13 | `0x03008004 + 0x10*n`, n=0..13 | `0xffffffff` | 14 decode-port set writes |
| CCU security switch | `0x03001f00` | `0x00000007` | security-switch configuration |
| R_PRCM security switch | `0x07010290` | `0x00000007` | security-switch configuration |
| DMA security register | `0x03002020` | `0x00000fff` | DMA channel security mask |
| Additional SPC register | `0x030080e0` | `0x00000001` | semantics unresolved |

The matching-family reference is ARM Trusted Firmware's H616 implementation at
pinned commit
`87cc230066271910096e025cf07376dba6c361d8`:
[`sunxi_security.c`](https://github.com/ARM-software/arm-trusted-firmware/blob/87cc230066271910096e025cf07376dba6c361d8/plat/allwinner/common/sunxi_security.c),
[`sunxi_bl31_setup.c`](https://github.com/ARM-software/arm-trusted-firmware/blob/87cc230066271910096e025cf07376dba6c361d8/plat/allwinner/common/sunxi_bl31_setup.c),
and
[`sunxi_spc.h`](https://github.com/ARM-software/arm-trusted-firmware/blob/87cc230066271910096e025cf07376dba6c361d8/plat/allwinner/sun50i_h616/include/sunxi_spc.h).

The TF-A reference explicitly describes these writes as preparing peripherals
for non-secure-world access, **but also warns that SPC writes may not take
effect unless a particular fuse is burnt (an erratum)**. The target monitor's
values are not byte-for-byte identical to the upstream reference (notably the
R_PRCM value and DMA mask), and `0x030080e0` is not explained by the public
H616 SPC header. Therefore the target binary proves that the monitor attempts
this setup; it does not prove the target fuse condition or effective
non-secure access at BL33 entry.

The target path later prepares the AArch32 SVC BL33 entry at
`0x4a000000` with SPSR `0x1d3`. The security sequence's placement in that
preparation path is strong static ordering evidence, not an execution trace.

## RG34XX non-SP physical-board equivalence and UART pad evidence

A signed manufacturer `Declaration Letter of Model Difference`, dated
2026-06-17, is available as FCC document 9400819 (SHA-256
`28138824031a0f7e4ff395ce800b8b4bb9062184b114bf60d55884d3130a8bcf`).
It names FCC ID `2AJU8-RG34XXSP`, lists `RG 34XXSP, RG 34XX, RG CubeXX,
RG 40XXV, RG 40XXH, RG SP, RG DS, RG DDS, RG DS Pro`, and states that the
models share the same PCB board, circuit, structure and internals, differing
only by model number and colour. The declaration is signed by the applicant's
manager. See the
[original one-page declaration PDF](https://device.report/m/28138824031a0f7e4ff395ce800b8b4bb9062184b114bf60d55884d3130a8bcf.pdf)
and its
[FCC exhibit entry](https://fccid.io/2AJU8-RG34XXSP/Letter/Declaration-Letter-of-Model-Difference-9400819).

The associated 2026-06-15 FCC Part 15.247 report CTA26060900901
(FCC ID `2AJU8-RG34XXSP`) identifies its test sample as RG34XXSP V1.0;
page 2 repeats the model-commonality assertion. See the
[40-page test report](https://device.report/m/01bdfd2898f5171e5885aa83c80a2fc5bfaf6bff027ac94b28bdfdfef077e12d.pdf).

The four-page `Internal Photos` exhibit is FCC document 9400823 (SHA-256
`3ff489bb32e9b47a56097c164a191453e16561d76c4ca3a50ebe3ddfc3cab1d2`).
Its page 3 shows the H700 mainboard front and page 4 the reverse side:
[original four-page PDF](https://device.report/m/3ff489bb32e9b47a56097c164a191453e16561d76c4ca3a50ebe3ddfc3cab1d2.pdf).
These are useful visual references for the declared common board family,
but neither side has PH0/PH1 test-pad, TX/RX, GND or net labels. The FCC
filing exposes its schematic as metadata-only, not as a downloadable
schematic/boardview.

### Independent community cross-check of the SP-side UART mapping (2026-11-27)

A read-only review of [fotlim/android_device_anbernic_rg34xxsp_h700_lineage](https://github.com/fotlim/android_device_anbernic_rg34xxsp_h700_lineage/tree/fa90b3a30f3d75bc9239609276b3bfa9e122788f) found a separate SP-device hardware record:

- `docs/HARDWARE.md` at blob `c0df1dcffa686ece164f62140195027240058636` records the debug port as UART0 on PH pins, 115200 8N1, based on that SP unit's stock firmware.
- `kernel/dtb/rocknix-rg34xx-sp-reference.dts` at blob `ca6b6c2db84f8a46e7711fb76381ea600f42943e` decodes `uart0-ph-pins` as `PH0\0PH1` with function `uart0`, maps `serial0` to `/soc/serial@5000000`, and sets `stdout-path = "serial0:115200n8"`.

This independently corroborates the H700-family PH0/PH1 ↔ UART0 function mapping and console rate on an RG34XX-SP software/board reference. It does **not** identify a test pad, TX/RX/GND solder point, pad voltage, or a traceable PH0/PH1-to-pad route. The repository is specifically for the SP model and is not a physical inspection of the target RG34XX non-SP board. It therefore adds pin/function corroboration only; it does not change M1-S1 qualification or authorize UART writes.

**Evidence classification:** a manufacturer/lab assertion of model-level PCB
commonality plus reference photos of an RG34XXSP sample. It is not direct
inspection of the specific non-SP board and does not identify a safe solder
point. Do not import test-pad locations from the SP community `device-facts.md`
or infer that a photographed via is UART0. The physical UART0 path remains
unqualified until the target non-SP PCB revision and a labelled/traceable
PH0/PH1-to-pad mapping are established.

## 5. What is now established, and what remains open

**Established by pinned source / exact binary:**

- H616 SPL has an early UART initialization path: APB2 24 MHz, UART gate/reset,
  PH0/PH1 UART0 mux for console index 1, and serial console initialization.
- The exact target monitor contains a UART gate selector and a reuse branch
  that avoids reprogramming an already-enabled UART.
- The exact target monitor invokes a multi-register SPC/CCU/PRCM/DMA security
  setup routine on the BL33 preparation path.
- The FDT/Linux runtime report independently confirms UART0/PH0/PH1 after
  Linux startup, but not at BL33 entry.

**Not established:**

- The complete generated SPL `.config` and the actual runtime value of every
  console option are not separately archived with the reproduction artifact.
- No live BL33-entry readback of CCU gate/reset, APB2, UART divisor/FIFO, PIO
  mux or SPC permission state exists.
- Target SPC fuse state and the effect of the `0x030080e0` write are unknown.
- The exact RG34XX PCB revision, UART0 test-pad identity, physical routing,
  voltage level and electrical connection are not established.
- No proof-of-life payload has been run; this audit does not authorize one.

**Gate conclusion:** UART0 is now a substantially better-supported
proof-of-life candidate and its SPL→BL31 handoff has a concrete static model.
M1-S1 remains **OPEN / NOT APPROVED** until the physical signal path and the
required access state are independently qualified. G2/G3 remain OPEN and
`H700_MEMORY_LAYOUT_APPROVED=false`, `hardware_applied=0`.

