# Linkslot 1.0.36 → RG34XX/H700 live MMC mapping (round 16)

Date: 2026-11-23
Work branch: `porting/linkslot-1.0.36-feasibility`
PR: [#16 — Draft](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16)
Capture: `reports/h700-mmc-map-r2.tar.gz`
Collector: `scripts/collect-h700-mmc-map-readonly.sh` with the corrected `sdc*` platform-device glob
Device: RG34XX, `sun50iw9p1`, compatible `allwinner,h616` + `arm,sun50iw9p1`
Kernel: `4.9.170`, `sunxi-mmc v3.35`

## Outcome

The r2 capture resolves the live controller-to-host mapping and supplies the missing controller DT properties. The important distinction is:

| Platform controller | MMIO | Live DT compatible | Linux host | Role indicated by this capture |
| --- | ---: | --- | --- | --- |
| `sdc0` | `0x04020000` | `allwinner,sunxi-mmc-v4p1x` | `mmc_host0` | Current system/boot medium; `mmcblk0` |
| `sdc1` | `0x04021000` | `allwinner,sunxi-mmc-v4p1x` | `mmc_host2` | SDIO device; kernel log identifies an SDIO DDR50 card (consistent with Wi-Fi) |
| `sdc2` | `0x04022000` | `allwinner,sunxi-mmc-v4p6x` | `mmc_host1` | Removable-card candidate; has a card-detect GPIO; no card was inserted |

**The Linkslot external-card target is `sdc2` / MMIO `0x04022000`, not “Linux mmc2”.** Do not derive the host index or controller role from the DT alias number.

## Live host and alias mapping

The captured sysfs paths establish:

- `/sys/class/block/mmcblk0` → `/sys/devices/platform/soc/sdc0/mmc_host/mmc0/mmc0:1234`.
- `mmc_host0` → `sdc0` → DT node `/soc@03000000/sdmmc@04020000`.
- `mmc_host1` → `sdc2` → DT node `/soc@03000000/sdmmc@04022000`.
- `mmc_host2` → `sdc1` → DT node `/soc@03000000/sdmmc@04021000`.

DT aliases are `mmc0 → 0x04020000` and `mmc2 → 0x04022000`; `mmc1` is absent. The sdc1 node has no `OF_ALIAS_0`. The kernel class order nevertheless exposes sdc2 as `mmc_host1` and sdc1 as `mmc_host2`. This is direct device evidence that a driver or storage layer must bind by controller identity / OF node, not by assuming `mmcN == sdcN`.

## Controller properties captured from live DT

All three nodes are `status = "okay"` and `bus-width = <4>`. All expose `cap-sd-highspeed`. None exposes `no-1-8-v`, `resets`, or `reset-names`.

### sdc2 — target removable slot candidate

- `reg`: base `0x04022000`, size `0x1000`.
- `compatible`: `allwinner,sunxi-mmc-v4p6x` (distinct from sdc0/sdc1 v4p1x).
- `bus-width = <4>`.
- `cd-gpios` is present. Its raw big-endian cells are `0x4b, 4, 0x16, 6, 1, 3, 0xffffffff`. The GPIO provider's cell semantics must be decoded before treating the trailing values as polarity, mux, pull, or drive settings. The port/pin pair `4, 22` is consistent with the PE22 second-slot wiring in the ROCKNIX RG35XX H/RG34XX reference patch; it is not interpreted as “pin 6”.
- `vmmc-supply` is absent; `vqmmc-supply` references phandle `0x54`.
- The debugfs regulator summary lists `sdc2` as a consumer of `axp2202-aldo1`; its captured voltage is 1.8 V with a reported adjustable range of 0.5–3.5 V. Kernel log says “No vmmc regulator found” for sdc2, but does not report a missing vqmmc regulator.
- `pinctrl-names = "default", "sleep"`; `pinctrl-0` contains phandles `0x99` and `0x9a`.
- `clock-names`: `osc24m`, `pll_periph`, `mmc`, `ahb`, `rst`. Preserve the raw `clocks` property/provider interpretation until the clock provider's `#clock-cells` is decoded; do not hardcode these raw cells as clock rates.
- No card was present. Kernel log says “Got CD GPIO”, then `detmode:gpio polling`, and later powers the host clock off. No `mmcblk1` exists at capture time.

### sdc0 — current system medium

- `reg`: `0x04020000`, size `0x1000`; compatible v4p1x; `bus-width = <4>`.
- No `cd-gpios`, `vmmc-supply`, or `vqmmc-supply`; pinctrl default/sleep is present.
- Sysfs reports `removable = 0`; kernel reports `detmode: always in (non removable)`.
- Kernel enumerates `mmc0:1234 SA16G 14.5 GiB` as a **high-speed SDHC card**, then creates `mmcblk0`. Therefore this capture does **not** justify calling this medium eMMC: Linux marks it non-removable, but the reported card type is SDHC.
- The command line confirms `root=/dev/mmcblk0p5`.

The whole disk reports 30,375,936 sectors (14.48 GiB) with seven partitions. Sysfs sizes are p1 64 MiB, p2 32 MiB, p3 16 MiB, p4 64 MiB, p5 512 MiB, p6 128 MiB and p7 the remainder (~13.15 GiB). One naming discrepancy must be preserved: the cmdline `partitions=` string labels p6 as `appfs` and p7 as `UDISK`, while sysfs `PARTNAME` reports p6 as `UDISK` and p7 as `primary`. Do not silently treat those two naming sources as identical.

### sdc1 — SDIO device

- `reg`: `0x04021000`, size `0x1000`; compatible v4p1x; `bus-width = <4>`.
- No card-detect or supply properties; pinctrl default/sleep is present.
- Kernel log eventually reports `mmc2: new ultra high speed DDR50 SDIO card at address 0001`, at 50 MHz / 4-bit. This is consistent with the board's Wi-Fi SDIO device; it is not the removable game-card slot.

## Cross-checks against kernel log

- sdc0 initializes at 400 kHz, transitions to 50 MHz, switches to 4-bit SD-HS/SDR25, and creates `mmcblk0`.
- sdc2 detects its GPIO-based slot logic but finds no card and exposes no block device.
- sdc1 completes SDIO initialization as DDR50. Its transient CMD5/CMD52 voltage/timeout messages are followed by successful SDIO enumeration; they must not be confused with sdc2 card-detect behavior.

## Implications for the H700 LK driver

1. Start the removable-media backend with the `sdc2` hardware node at `0x04022000`, compatible `sunxi-mmc-v4p6x`; do not initialize sdc0 as the ROM card just because it owns `mmcblk0`.
2. Implement the H616/H700 clock, pinctrl, card-detect and regulator path explicitly. sdc2 is not configured like sdc0: it has a CD GPIO and a vqmmc provider, but no vmmc provider.
3. Decode the live GPIO provider and clock-provider bindings from the DT before setting pin function, polarity, pull, drive strength, clock parent/rate, or reset sequencing.
4. Audit and implement the H616 MMC2 `mask_data0` clock-update quirk referenced by the ROCKNIX patch. The live `v4p6x` compatible makes this a required controller-specific review, not a generic optional optimization.
5. Keep the first LK block-device probe strictly **read-only**. The current capture contains no card in sdc2, so it proves topology and configuration only; it does not prove successful card initialization, block reads, timing stability, or writes.

## Corrections to the preliminary r2 summary

- `sdc0` should be called the current non-removable-designated system medium, **not confirmed eMMC**; the kernel identifies it as SDHC.
- The raw `cd-gpios` cells contain `4, 0x16` (port 4 / pin 22) followed by provider-specific configuration values. The final `0x06` is not the pin number. PE22 is consistent with the board reference, but electrical semantics still require provider-cell decoding.
- The sysfs partition names p6/p7 disagree with the cmdline `partitions=` labels; both are recorded above.

## Safety boundary

The capture ran read-only. It did not open or read any block device, access CID/CSD, write a partition, or touch bootloader/firmware. It is not an SD/MMC functional test. No driver has yet been executed on hardware, no ROM storage path is connected, and the project remains **NO-GO for boot or persistence writes**.
