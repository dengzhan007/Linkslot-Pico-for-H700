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

## Sources

- [RG34XX live inventory (reports branch)](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/docs/rg34xx-live-inventory-2026-11-22.md)
- [Knulli RG34XX boot-package DTS](https://github.com/knulli-cfw/knulli-linux/blob/knulli-main/package/boot/uboot-h700/rg34xx/boot_package/rg34xx.dts)
- [Allwinner sun50iw9p1 pinctrl DTSI reference](https://github.com/CrealityTech/sonic_pad_os/blob/7f37a7fbf4ad59907034715e3d967b9177de1cd4/lichee/linux-4.9/arch/arm64/boot/dts/sunxi/sun50iw9p1-pinctrl.dtsi)
- [Linux H616 CCU clock/reset map](https://github.com/intel/mOS/blob/197f50d406e2cc5a347f3cb58fb00b483dce6af7/drivers/clk/sunxi-ng/ccu-sun50i-h616.c)
- [FreeBSD H616 CCU clock/reset map and UART-gate caveat](https://github.com/freebsd/freebsd-src/blob/5c0715d55ed577eeb71078490d2c14df9fd98746/sys/dev/clk/allwinner/ccu_h616.c)
