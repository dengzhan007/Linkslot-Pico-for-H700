# RG34XX H700 UART0 reference audit

## Purpose and confidence

This is a source cross-reference for a possible early LK console on the
**Anbernic RG34XX non-SP / Allwinner H700 (sun50iw9p1, H616-family)**.
It does not enable the UART, write registers, or claim that Linux and LK see
identical pin/clock state at handoff.

The strongest device-specific observation is the live RG34XX boot command
line collected on 2026-11-22:

```
earlyprintk=sunxi-uart,0x05000000
console=ttyS0,115200
```

That establishes that the currently running Linux boot chain identifies
UART0 at 0x05000000 and uses ttyS0 at 115200 baud. It does not prove the
physical connector is exposed or the register state is unchanged at BL33
entry.

## RG34XX boot-package DTS cross-reference

The Knulli RG34XX (non-SP) boot-package DTS has:

- UART node: /soc@03000000/uart@05000000
- compatible: allwinner,sun50i-uart
- register window: 0x05000000..0x050003ff
- status = "okay"
- serial0 alias points to that UART node
- clock phandle resolves to the uart0 peripheral-clock node
- pinctrl-0 resolves to the UART0 pin group.

Its matching sun50iw9p1-pinctrl.dtsi defines that group as:

| Signal | Pin | Function | Mux selector |
|---|---|---|---:|
| TX | PH0 | uart0 | 2 |
| RX | PH1 | uart0 | 2 |

The group also records pull-up and drive-strength values. These are firmware
configuration values from the Knulli board DTS, not a readback of the live
RG34XX PIO registers.

## H616 CCU register cross-reference

The mainline Linux H616 CCU driver records the UART0 bus clock gate as:

- CCU register offset: 0x90c
- bus clock parent: apb2
- UART0 bus gate: bit 0
- UART0 reset: bit 16

The H616 FreeBSD CCU source agrees on the reset bit at offset 0x90c, but
intentionally comments out all UART bus-gate entries and carries an inherited
warning about unusual UART gate behavior. That disagreement is a reason not
to blindly toggle the UART gate in an early LK driver.

The UART0 peripheral-clock phandle in the Knulli DTS is named uart0.
The exact source selection/rate and whether it is already enabled at the
vendor monitor-to-BL33 handoff have not been established from the current
RG34XX live inventory.

## UART IP register layout and inherited-state early console

The pinned `sun50iw9p1` vendor Linux driver in `qiaoweibiao/T507_Kernel`
provides an additional IP-level reference. Its register header identifies:

| Register | Offset |
|---|---:|
| RBR / THR / DLL | 0x00 |
| DLH / IER | 0x04 |
| IIR / FCR | 0x08 |
| LCR | 0x0C |
| LSR | 0x14 |
| USR | 0x7C |
| TFL / RFL | 0x80 / 0x84 |

The same driver's early-console callback polls `USR.TFNF` (bit 1, transmit
FIFO not full) and writes the next byte to `THR` at offset 0x00. That path
does not reprogram the CCU or PH0/PH1 pinmux. It assumes the firmware has
already configured a working UART and only supplies the mapped base address.

This makes an inherited-state, polling-only LK console a reasonable next
candidate: read the current UART state, then write only THR when USR.TFNF is
set. It must not reset the peripheral, alter LCR/DLL/DLH, toggle the UART
clock gate/reset, or change PH0/PH1 mux. The actual handoff state and the
whether the UART continues clocking at BL33 entry are still unverified, so
even this minimal console remains a candidate rather than an enabled driver.

The vendor source uses a 24 MHz `uartclk` in its non-EVB build branch; its
EVB branch derives the clock from clock providers and APB2. The device's
runtime bootargs only prove the requested Linux baud of 115200, not the
actual UART input clock or divisor at BL33 handoff. Do not infer a fixed
divisor from the source's fallback branch.

Sources:
- [sun50iw9p1 vendor UART register definitions](https://github.com/qiaoweibiao/T507_Kernel/blob/98fcc7d3f112e51b0edfb71536da89cb2115106f/drivers/tty/serial/sunxi-uart.h)
- [sun50iw9p1 vendor UART driver and inherited-state early console](https://github.com/qiaoweibiao/T507_Kernel/blob/98fcc7d3f112e51b0edfb71536da89cb2115106f/drivers/tty/serial/sunxi-uart.c)

## Safe next implementation boundary

The evidence supports treating UART0 as the first candidate for early
diagnostics, with these provisional parameters:

- MMIO window: 0x05000000, length 0x400
- serial format observed by Linux: 115200 baud, 8 data bits, no parity, 1 stop
  bit (8N1 is the normal interpretation, but is not directly captured as a
  register readback)
- board-DTS pin group candidate: PH0/PH1, mux function uart0, selector 2
- H616-family CCU reference: bus gate/reset register offset 0x90c, bits
  0 and 16 respectively.

Before adding any write-capable LK console code, obtain a read-only dump of
the UART, CCU and PIO registers at the earliest supported handoff, or prove
the vendor handoff contract from the loader/monitor source. Specifically
confirm the APB2/UART functional clock rate, UART divisor, gate/reset state,
PIO mux/pull/drive, and whether PH0/PH1 are shared with another active function.

Until then:

- do not touch CCU offset 0x90c;
- do not reconfigure PH0/PH1;
- do not assume that the Knulli DTS is the active BaseOS runtime FDT;
- do not add a UART driver to platform/h700 yet.

## 2026-11-27 LK translation-only UART0 aperture and runtime collector

The H700 AArch32 MMU hook now maps the 1 MiB section
`[0x05000000, 0x05100000)` as shared Device, read/write and XN, covering
UART0's `0x05000000/0x400` register aperture. This is only an address-
translation descriptor; it does not read or write UART registers, enable
the UART clock, alter reset or PH0/PH1 mux, or grant bus/security-firewall
permission. The section granularity also maps adjacent peripheral addresses,
but no driver is permitted to access them. Host regression requires both the
existing SYS_CFG/PIO/CCU section and the UART0 section before MMU enable.

The read-only inventory collector now emits `uart-runtime.txt` containing:
- only `console=`, `earlycon=` and `earlyprintk=` boot arguments (not the
  full cmdline or device serial number);
- `/proc/tty/driver/serial` and ttyS sysfs device links;
- PH0/PH1 matches from already-mounted pinctrl debugfs views;
- UART0/APB2 clock-summary matches from already-mounted clk debugfs;
- narrowly filtered UART0/ttyS kernel log lines.

The collector does not mount debugfs, open `/dev/mem`, access UART/CCU
registers, or change clock/reset/pinmux state. The resulting file is strictly
**Linux post-boot evidence**: it can identify which UART and pins Linux
currently claims, but it cannot establish the earlier BL31-to-BL33 state.
A missing debugfs view is reported as unavailable rather than worked around.

The first fresh inventory run on the actual RG34XX (non-SP) test unit is now
published as [reports/rg34xx-inventory-20261127-uart](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-inventory-20261127-uart)
(reports HEAD `7217907f57408b714c55a872c6afc5cfc34fdac9`). The 42-file capture
re-confirms the same TOC1 package (`e6c20e3a...`), monitor
(`676bc020...`) and TOC1 DTB (`c28c8738...`) as the prior inventory.

### 2026-11-27 target-device UART0 runtime result

- Filtered boot arguments: `earlyprintk=sunxi-uart,0x05000000`,
  `console=ttyS0,115200`.
- `/sys/class/tty/ttyS0` resolves to
  `/sys/devices/platform/soc/uart0` (dev `248:0`); ttyS1 resolves to uart1.
- Existing pinctrl debugfs reports PH0 and PH1 with function `uart0`;
  pinconf shows pull-up and 20 mA drive strength.
- Existing clock debugfs reports both `apb2` and `uart0` at 24 MHz.
- `/proc/tty/driver/serial` is absent/unreadable, so there is no UART driver
  line-status, divisor, or FIFO evidence in this capture.
- Six dmesg lines report `expect_func as:uart0_jtag` with muxsel(3) resolving
  in order to `jtag,jtag,uart0,jtag,uart0,jtag`. This sequence matches the
  reference sun50iw9p1 `sdc0_pins_c` group on PF0–PF5: at muxsel 3, PF0/PF1/
  PF3/PF5 are named `jtag` and PF2/PF4 are named `uart0`.
- `GPIO UNCLAIMED` in `pinmux-pins` describes the GPIO request state; it
  does not negate the separately reported `function uart0`.
- The report includes a redaction record and SHA256 manifest. The published
  scan reports zero device-serial, MAC-shaped, and workstation-path matches;
  the redacted live FDT still parses to 573 nodes.

This is a useful positive proof-of-life candidate observation on the exact
target device's **Linux post-boot runtime**, but it does not establish the
BL31-to-BL33 handoff state. In particular, current UART divisor/FIFO, the
actual PH0/PH1 physical test-pad identity and electrical connection, and
BL33-time clock/reset/pinmux/security-firewall access remain unknown.
The collector performed no `/dev/mem` access, no MMIO read/write, no debugfs
mount and no storage write. Do not add a UART driver or output path, touch
CCU offset `0x90c`, or alter PH0/PH1 mux on the basis of this report.
M1-S1 remains OPEN.

## Pinctrl `uart0_jtag` warning interpretation (2026-11-27)

The exact target-device FDT provenance is now available. A direct static parse
of the redacted `live-fdt.dtb` in
[reports/rg34xx-inventory-20261127-uart](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-inventory-20261127-uart)
(573 nodes) found:

- `/aliases/serial0` resolves to
  `/soc@03000000/uart@05000000`.
- The UART node is `allwinner,sun50i-uart`, `status = "okay"`, with
  `reg = <0x0 0x05000000 0x0 0x400>`. Its clock phandle `0x29` resolves
  to `/clocks/uart0`.
- `pinctrl-names = "default", "sleep"`. The default state references
  phandle `0x1a0` / `pinctrl@0300b000/uart0@0`, declaring PH0/PH1,
  function `uart0`, muxsel 2. The sleep state references phandle
  `0x2b` / `uart0@1`, also declaring PH0/PH1 and function `uart0`,
  with muxsel 7.
- The same live FDT contains
  `/soc@03000000/pinctrl@0300b000/sdc0@2` (phandle `0x95`), whose
  group declaration is PF0–PF5, `allwinner,function = "uart0_jtag"`,
  muxsel 3. This is the exact group-level source that was previously
  inferred only from the matching-family reference DTSI.

The six target dmesg warnings resolve in order to
`jtag,jtag,uart0,jtag,uart0,jtag`. The matching-family pin function table
names muxsel 3 on PF0/PF1/PF3/PF5 as `jtag`, but PF2/PF4 as `uart0`;
the vendor mapping code warns when the group-level expected name differs
from a per-pin function, then still constructs the mux map. The active FDT
group plus this source cross-check now explain the warning provenance much
more strongly than before. It is not evidence of PH0/PH1 JTAG arbitration.

**Scope boundary:** this is static FDT declaration plus Linux post-boot log
evidence. It does not prove that PF0–PF5 are electrically driven in that
configuration, does not read the live PIO registers, and does not prove the
UART clock/reset/pinmux/security state at BL33 entry. The exact deployed
4.9.170 kernel source is still not hash-identified. The independent Linux
pinctrl runtime view continues to report PH0/PH1 as function `uart0`.

The host FDT inspector now emits `serial_aliases`, `serial_nodes`,
`uart0_related_pinctrl_groups`, and an explicit static-only scope note.
Synthetic regression coverage exercises alias, reg, clock, default/sleep
pinctrl and the `uart0_jtag` group. H700 boot audit #1501 passed on Linux
and Windows. The uploaded `live-fdt.json` predates this parser extension;
the target values above were decoded directly from its accompanying
`live-fdt.dtb`. A subsequent inventory run will include the new JSON
fields automatically.

## 2026-11-27 pinned native SPL and exact BL31 UART handoff audit

The earlier bounded review below only covered the BaseOS A32 loader and SPL
adapter files; its negative result did not include inherited upstream U-Boot
SPL board/clock/GPIO startup or the vendor monitor. It therefore must not be
read as evidence that the full native boot chain contains no UART setup.

See the dedicated
[RG34XX UART0 SPL→BL31→BL33 audit](rg34xx-uart-bl31-handoff-audit.md).
In summary:

- Pinned U-Boot v2026.10 H616 `board_init_f()` runs `clock_init()`,
  `gpio_init()`, then `preloader_console_init()` before
  `sunxi_board_init()`. The H616 source maps console index 1 to PH0/PH1
  UART0, configures APB2 from OSC24M at 24 MHz, enables the selected UART
  gate and deasserts reset; `preloader_console_init()` invokes
  `serial_init()`. The exact reproduced SPL symbol map includes
  `clock_init_uart`, `preloader_console_init`, `serial_init` and
  `board_init_f`.
- The exact target monitor (107,216 bytes, SHA-256
  `676bc0206e3c9a7f229d93ea7f3286095f669940ad91f3c4f3d37bdb18696c31`)
  calls a UART gate selector before BL33 preparation. If a UART gate is
  already enabled, its helper caches that controller's base and bypasses
  clock/reset/divisor programming; otherwise it has a fallback APB2/16550
  initialization path. This is static analysis of the exact monitor bytes.
- The same target monitor calls an SPC/CCU/PRCM/DMA security routine in the
  BL33 preparation path. The H616 TF-A reference documents the intended
  non-secure setup but warns the SPC writes may be ineffective unless a
  specific fuse is set. The target fuse status and live permission state are
  unknown, and one extra target write at `0x030080e0` remains unexplained.

The source review of `native/entry32.S`, `native/loader.c`,
`spl/native-boot.c`, `spl/vendor-handoff.c` and `spl/handoff32.S`
remains accurate in its narrow scope: these files contain no UART driver or
console call, and the `serial` string in `native/loader.c` is the SID-derived
device serial argument, not a UART device. The review did **not** cover the
inherited U-Boot SPL board initialization or the vendor BL31 binary.

The target binary audit is host-only; no MMIO was read and no device code was
executed. It advances the static handoff model but does not close M1-S1.


## 2026-11-27 exact target TOC1 and BaseOS native-builder provenance boundary

The latest physical RG34XX **non-SP** capture pins the TOC1 package at SHA-256
`e6c20e3a54c51b9336c798e3a4fa1ca6c45b28c6f70456e2db9c252aa4ba4604`.
Its extracted stock-fallback `u-boot` item is 1,048,576 bytes with SHA-256
`4d60a22ff13eac00dba89e0ee7df52a49852c0d9d856bec545ce40126191d127`;
the monitor item is 107,216 bytes with SHA-256
`676bc0206e3c9a7f229d93ea7f3286095f669940ad91f3c4f3d37bdb18696c31`.

The live FDT `/chosen/uboot_message` reports
`2018.05(05/26/2026-13:55:09)`. **Do not interpret this as evidence that
the TOC1 U-Boot executed.** The active-loader provenance report establishes
that this item belongs to the stock compatibility/fallback path; on the
captured boot the native SPL places its A32 loader at `0x4a000000), replacing
the U-Boot image in the BL33 slot. BaseOS `native/build.py` explicitly
implements `vendor_version_message()`: it extracts the version/date/time
literals from the TOC1 `u-boot` component and uses the resulting string in
the generated bootargs. Thus the live `uboot_message` is build metadata copied
from the fallback component, not a runtime execution trace.

The pinned BaseOS-MultiFrontend source at
`cbe253a43f46cf551cc9a6c79a84015bde3686cf` **does include an `rg34xx`
prepared-source profile** in both `evidence/compatibility.json` and
`native/vendor-handoff.json`. That profile records a 1 GiB model and its own
LPDDR4/672 MHz Boot0 candidates; its prepared TOC1 package SHA is
`56371d984e29e74db63009ce27f829eb0a55ce9007ee5e40c52479b22418af3d`,
monitor SHA `676bc020…`, and panel profile vendor-U-Boot SHA
`0edfa88f7ef990ea4a423a2bbd4c0280006625b8c0cc5438cd2715527e891573`.
The `rg34xx` profile is real and must not be discarded as an SP-only source.

However, separate the levels of evidence:

- `native/build.py` uses the pinned upstream U-Boot v2026.10 archive as the
  source for building the mainline AArch64 SPL. This is a different component
  from the stock TOC1 U-Boot v2018.05 fallback item.
- `verify-fleet.py` defines physical `LIVE_TARGETS = {rg34xxsp, rg40xxv}`;
  its `--prepared-only` route supports the eleven prepared model profiles.
  `spl/inventory.py` likewise lists SP/RG40XXV/RGSP as authorized lab
  devices. Therefore the non-SP `rg34xx` profile has prepared/host evidence,
  not equivalent live hardware qualification in that experiment.
- The deployed non-SP active path itself is established by
  [the read-only active-loader provenance report](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-active-loader-provenance.md).
  Its exact build provenance is now also closed at host level by BaseOS
  [RG34XX native generation reproduction #45301188593](https://github.com/dengzhan007/BaseOS-MultiFrontend/actions/runs/45301188593).
  That workflow checks out the exact builder pin `cbe253a43f46cf551cc9a6c79a84015bde3686cf`,
  restores the prepared source bundle and verifies the 222,298,112-byte
  `boot-prefix.img` (SHA-256 `fb75bc4c835fa74ff41c0a1fdbe79b8871a960fc249716847fb7be529835c3c3`),
  then reconstructs the inventory slice, stock Boot0 and env with their
  pinned SHA-256 values. It rebuilds generation
  `022f0d8816c861ebbec05f0d0d7f513d6b974d9930661d20fa8ad51b7b4c0fee`,
  A32 loader, firmware and active image with exact hashes, and rebuilds a
  49,152-byte AArch64 SPL. The resulting 128 KiB capsule is byte-exact with
  the deployed RG34XX reference capsule in commit
  `339e1cd5f84a6178c496257f835beb3388b8d9f9` (zero differing bytes).
  The raw capsule host tests pass 36 cases and entry tests pass 2 cases.
  The workflow's manifest explicitly retains `hardware_qualified=false`
  and `durable_install_eligible=false`; it proves deterministic source/build
  provenance, not device qualification.
- The earlier 2026-11-22 16-combination generation attempt and the 2026-11-24
  note that the prepared prefix/Boot0 were unavailable are superseded by the
  later dedicated reproduction workflow. Do not treat the A/B test image's
  first 222,298,112 bytes as the prepared prefix: the exact prefix is restored
  from the pinned prepared bundle via `fetch-prepared.sh`, whose bundle SHA
  is `86a0186b5ac8ab907dc1d5c24220c9fcd94f38dd9dc3fddb7fb5cf59d4c7719f`.
  The current device's 128 KiB Boot0 primary capture is the complete capsule
  (`bf886995…`); the embedded 64 KiB stock Boot0 source is
  `b6edbf39…`. Likewise `56371d98…` is the TOC1 source package while
  `e6c20e3a…` is the separate recovery TOC. These are distinct artifacts,
  not unexplained build-input mismatches.


The same SPL README documents the monitor reference contract: monitor at
`0x48000000`, next stage AArch32 SVC at `0x4a000000` with SPSR `0x1d3`,
Boot0 DRAM parameters copied to monitor +`0x30` and an eight-byte board block
to +`0x24`. The active-loader report corroborates this entry shape on the
target. It does not prove the UART clock, pinmux, reset or security/firewall
state at that entry; the source review of the SPL/loader files remains
bounded and no UART initialization or `0x05000000` access was found in the
scoped files.


## Sources

- [RG34XX live inventory (reports branch)](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/docs/rg34xx-live-inventory-2026-11-22.md)
- [2026-11-27 exact RG34XX inventory snapshot and redacted live FDT](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-inventory-20261127-uart)
- [Knulli RG34XX boot-package DTS](https://github.com/knulli-cfw/knulli-linux/blob/knulli-main/package/boot/uboot-h700/rg34xx/boot_package/rg34xx.dts)
- [Allwinner sun50iw9p1 pinctrl DTSI reference](https://github.com/CrealityTech/sonic_pad_os/blob/7f37a7fbf4ad59907034715e3d967b9177de1cd4/lichee/linux-4.9/arch/arm64/boot/dts/sunxi/sun50iw9p1-pinctrl.dtsi)
- [Linux H616 CCU clock/reset map](https://github.com/intel/mOS/blob/197f50d406e2cc5a347f3cb58fb00b483dce6af7/drivers/clk/sunxi-ng/ccu-sun50i-h616.c)
- [FreeBSD H616 CCU clock/reset map and UART-gate caveat](https://github.com/freebsd/freebsd-src/blob/5c0715d55ed577eeb71078490d2c14df9fd98746/sys/dev/clk/allwinner/ccu_h616.c)
