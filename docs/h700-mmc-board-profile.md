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

The initial Linkslot storage path should use **sdc0**. It owns the currently booted SDHC card; bootargs specify p5 as rootfs and identify p7 as UDISK. RG34XX has no eMMC. Linux's `removable=0` / non-removable setting for sdc0 must not be interpreted as eMMC media.

sdc2 is kept as an optional second-slot candidate. It exposes a card-detect GPIO and a `vqmmc-supply` reference, but no card was inserted in the capture. Its GPIO provider cells and exact regulator/clock initialization still require decoding. The profile intentionally records these properties as present-but-unresolved and does not provide pin numbers, polarity, pull, drive, or regulator voltage to a driver.

sdc1 successfully enumerated an SDIO DDR50 device and is excluded from the first storage implementation.

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
