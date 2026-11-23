# RG34XX H700 MMC board profile

This profile is an index of observed hardware topology from the read-only RG34XX capture in [round 16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/linkslot-h700-live-mmc-map-2026-11-23-round16.md). It is deliberately not an MMC driver and performs no register access.

## Controller identity

| Controller | MMIO | Live compatible | Observed Linux host | Capture result | Role |
| --- | ---: | --- | ---: | --- | --- |
| sdc0 | 0x04020000 | sunxi-mmc-v4p1x | mmc_host0 | SDHC, 4-bit, active mmcblk0 | Primary same-card storage |
| sdc1 | 0x04021000 | sunxi-mmc-v4p1x | mmc_host2 | SDIO DDR50, 4-bit | Wi-Fi/SDIO; not filesystem storage |
| sdc2 | 0x04022000 | sunxi-mmc-v4p6x | mmc_host1 | No card present; 4-bit | Candidate secondary SD slot |

The Linux host indices above are diagnostic observations only. They do not define controller identity. The live device tree aliases differ from Linux host enumeration, so code must identify controllers by MMIO base and compatible, not by `mmcN` suffixes.

## Storage policy

The initial Linkslot storage path should use **sdc0**. It owns the currently booted SDHC card; bootargs specify p5 as rootfs and identify p7 as UDISK. The live mount table confirms p7 is mounted at `/mnt/sdcard` (vfat), including `/mnt/sdcard/Roms` and `.userdata`, while p6 is mounted at `/data` (ext4). Sysfs `PARTNAME` values conflict with the bootargs and mount table for p6/p7, so the implementation must inspect the on-disk GPT and the actual volume layout rather than trust the sysfs label. RG34XX has no eMMC. Linux's `removable=0` / non-removable setting for sdc0 must not be interpreted as eMMC media.

sdc2 is kept as an optional second-slot candidate. It exposes a card-detect GPIO and a `vqmmc-supply` reference, but no card was inserted in the capture. Its GPIO provider cells and exact regulator/clock initialization still require decoding. The profile intentionally records these properties as present-but-unresolved and does not provide pin numbers, polarity, pull, drive, or regulator voltage to a driver.

sdc1 successfully enumerated an SDIO DDR50 device and is excluded from the first storage implementation.


## Live pin and clock observations

From the device's debugfs capture, sdc0 was in pinctrl state `default`, with PF0–PF5 muxed to `sdc0`. sdc1 was also in `default`, with PG0–PG5 muxed to `sdc1`. sdc2 was in state `sleep`; its default pinctrl map describes PC1, PC5, PC6, PC8–PC11, PC13–PC16 and PC0, while the sleep map disables those pins. This means the r2 capture does not show sdc2 default pinmux actively driving the empty slot.

The observed sdc0 and sdc1 pinconf entries are raw values `0x00002809` / `0x00000005`; the sdc2 default map mostly uses `0x00001e09` / `0x00000005`, with PC0 carrying `0x00000003`. These are vendor PIO encodings, not decoded electrical settings. Do not copy them into LK until the exact H700 PIO pinconf implementation has been inspected.

CCU debugfs showed `sdmmc0_mod` at 100 MHz and enabled, with `sdmmc0_bus` and `sdmmc0_rst` enabled. `sdmmc1_mod` was 200 MHz and enabled, with its bus/reset gates enabled. `sdmmc2_mod` showed an 800 kHz rate but was not enabled/prepared; its bus/reset gates were off. These module-clock rates are not card SDCLK rates: the Linux log reports sdc0's card bus at 50 MHz. The H616 CCU reference places MMC module clock config registers at `0x830/0x834/0x838` and the combined gate/reset register at `0x84c` (see the pinned [U-Boot clock definitions](https://github.com/u-boot/u-boot/blob/e578056c766d653270b14557c2a9d8daec414c17/arch/arm/include/asm/arch-sunxi/clock_sun50i_h6.h)).

## sdc0 pinmux and CCU register reference

The live inventory maps the active sdc0 pin group to PF0–PF5. The pinned H616 reference device tree at Linux commit `fa2933aac170dd19c6a7f0d7b81a8f537e40915c` declares the generic `mmc0_pins` group as PF0–PF5, function `mmc0`, drive-strength 30 mA and pull-up bias. The matching `pinctrl-sun50i-h616.c` pin descriptors identify function selector **ALT2 (value 2)** on each signal:

| Pin | MMC0 signal | H616 mux selector |
|---|---|---:|
| PF0 | D1 | 2 |
| PF1 | D0 | 2 |
| PF2 | CLK | 2 |
| PF3 | CMD | 2 |
| PF4 | D3 | 2 |
| PF5 | D2 | 2 |

This is the upstream H616 pin-function/reference configuration, not a decoded readback of the RG34XX's raw vendor pinconf words. Keep the live debugfs values `0x00002809` / `0x00000005` classified as unresolved; do not translate them into drive/pull values or write them back to PIO registers. The 30 mA/pull-up declaration is a useful board bring-up reference, not proof that those exact electrical settings were active at the time of the capture.

The H616 CCU reference gives the following MMC0 clock-control layout (offsets are relative to the CCU base):

| Register | Field | Value |
|---|---|---|
| `0x830` (`MMC0_CLK_CFG`) | M divider | bits 0–3 |
| `0x830` (`MMC0_CLK_CFG`) | N divider | bits 8–9 |
| `0x830` (`MMC0_CLK_CFG`) | parent mux | bits 24–25 |
| `0x830` (`MMC0_CLK_CFG`) | module clock gate | bit 31 |
| `0x84c` (`MMC_GATE_RESET`) | MMC0 bus gate | bit 0 |
| `0x84c` (`MMC_GATE_RESET`) | MMC0 reset deassert | bit 16 |

The Linux H616 CCU driver defines MMC parents as `osc24M`, `pll-periph0-2x`, and `pll-periph1-2x`, with a fixed post-divider of 2 for each MMC module clock. The pinned U-Boot sunxi MMC implementation instead describes the effective PLL6 input rate and accounts for the H616 hidden divider in its clock calculation. Keep the effective parent-rate semantics consistent when porting the divider calculation; do not copy the live debugfs `sdmmc0_mod = 100 MHz` rate as the card SDCLK. Linux separately reports the card bus at 50 MHz.

References:
- [H616 MMC0 pin group and controller node, Linux commit](https://github.com/torvalds/linux/blob/fa2933aac170dd19c6a7f0d7b81a8f537e40915c/arch/arm64/boot/dts/allwinner/sun50i-h616.dtsi)
- [H616 PF0–PF5 function selectors, Linux commit](https://github.com/torvalds/linux/blob/fa2933aac170dd19c6a7f0d7b81a8f537e40915c/drivers/pinctrl/sunxi/pinctrl-sun50i-h616.c)
- [H616 MMC module clock fields, Linux commit](https://github.com/torvalds/linux/blob/fa2933aac170dd19c6a7f0d7b81a8f537e40915c/drivers/clk/sunxi-ng/ccu-sun50i-h616.c)
- [Pinned U-Boot MMC clock calculation](https://github.com/u-boot/u-boot/blob/e578056c766d653270b14557c2a9d8daec414c17/drivers/mmc/sunxi_mmc.c)
- [Pinned U-Boot H6/H616 clock register definitions](https://github.com/u-boot/u-boot/blob/e578056c766d653270b14557c2a9d8daec414c17/arch/arm/include/asm/arch-sunxi/clock_sun50i_h6.h)

### MMC module-clock divider calculator (host-only)

`sunxi_mmc_module_clock.c` mirrors the pinned U-Boot H616 divider policy as a pure function. For a requested module rate up to 24 MHz it selects OSC24M; above 24 MHz it selects the effective PLL6 rate. It calculates `div = ceil(parent / target)`, folds factors of two into the two-bit N exponent while M exceeds 16, and encodes M as `M-1`. The returned rate is calculated as `parent / 2^N / M` and is rejected if it exceeds the requested rate or cannot fit N=0..3 and M=1..16.

With the test fixture's 24 MHz OSC and **600 MHz effective PLL6 input** (a fixture matching the U-Boot clock calculation model, not a measured constant for every boot), the tested examples are:

| Requested module rate | Parent | N | M | Calculated module rate | CCU register value |
|---:|---|---:|---:|---:|---:|
| 400 kHz | OSC24M 24 MHz | 2 | 15 | 400 kHz | `0x8000020e` |
| 25 MHz | effective PLL6 600 MHz | 1 | 12 | 25 MHz | `0x8100010b` |
| 50 MHz | effective PLL6 600 MHz | 0 | 12 | 50 MHz | `0x8100000b` |

These are module-clock examples, not direct card-SDCLK claims. The calculator does not enable the CCU bus gate, deassert reset, configure PIO, update CLKCR, issue a card command, or access storage. A future sdc0 driver must obtain the effective PLL6 rate from the actual platform clock state rather than assuming 600 MHz. Run `sh scripts/test-h700-sunxi-mmc-module-clock.sh` for the host regression and optional ASan/UBSan pass.

## sdc0 read-only driver constraints

The first controller implementation should target the `sdc0` register block at `0x04020000` only. The pinned [U-Boot sunxi MMC implementation](https://github.com/u-boot/u-boot/blob/e578056c766d653270b14557c2a9d8daec414c17/drivers/mmc/sunxi_mmc.c) provides a portable reference for command/response handling, polling, FIFO transfers through the AHB path, clock setup, and H616 FIFO threshold initialization. Its register map is in [sunxi_mmc.h](https://github.com/u-boot/u-boot/blob/e578056c766d653270b14557c2a9d8daec414c17/drivers/mmc/sunxi_mmc.h).

Important H616-specific constraint: CLKCR bit 31 is `MASK_DATA0`. Mainline Linux's H616 config sets `mask_data0 = true`; a July 2026 U-Boot change describes an H616 initial `UPCLK_ONLY` clock-update timeout and masks DATA0 around clock-off/on update sequences. This is a shared H616 clock-update behavior, not an sdc2-only feature. Implement it in the common clock-update path for sdc0 as well, while preserving the failure cleanup and not assuming that the old pinned U-Boot code already contains the fix. References: [mainline Linux H616 config](https://kernel.googlesource.com/pub/scm/linux/kernel/git/torvalds/linux/+/fa2933aac170dd19c6a7f0d7b81a8f537e40915c/drivers/mmc/host/sunxi-mmc.c#1197), [U-Boot H6-family fix](https://www.mail-archive.com/u-boot%40lists.denx.de/msg579824.html).

Initial bring-up should stay read-only: use identification clock first, send only card identification and read commands, perform PIO reads without enabling IDMA, and reject all write/erase commands in the block adapter. Start at a conservative transfer clock; only consider 50 MHz SDR25 after block-level read tests are repeatable. The live Linux 50 MHz result is a target reference, not proof the LK port can safely use that rate.

## What the C profile does

`h700_mmc_board_profile.c` exposes three immutable descriptors and lookup functions. The host regression checks:

- sdc0 is chosen as the primary same-card storage controller;
- sdc1 is identified as SDIO and not a filesystem controller;
- sdc2 remains an unpopulated secondary-slot candidate;
- the Linux host index mismatch is captured without being used for selection;
- unresolved GPIO and regulator references are not silently converted into guessed hardware settings.

The profile is safe to link into host diagnostics or target builds because it only returns static data. It does not map MMIO, enable clocks, change pins, power the card, issue SD commands, or read/write storage.

## Implementation sequence

1. Implement a minimal, **read-only** sdc0 block path and validate SD command initialization without issuing any write/erase commands.
2. Parse GPT read-only and locate p7 using the on-disk GPT rather than hardcoding the bootargs or kernel partition numbers.
3. Read-only mount the intended UDISK filesystem and run ROM enumeration/readback checks.
4. Only then add sdc2 as a separately controlled optional slot, after decoding its GPIO, clock and regulator provider details and obtaining a capture with a card inserted.
5. Keep FAT/exFAT save writes disabled until LFN directory creation, reader/writer synchronization, atomic replace, and durable block flush/barrier semantics are implemented and tested.

No SD card writes, boot-area writes, firmware construction, or payload execution are part of this profile.
