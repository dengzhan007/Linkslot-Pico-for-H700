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

The first Linkslot storage target is **sdc0 at MMIO `0x04020000`**. Round 16 directly bound `/sys/class/block/mmcblk0` through `mmc_host0` to DT node `/soc@03000000/sdmmc@04020000`; the kernel enumerated a 14.5 GiB SDHC card and mounted p7 at `/mnt/sdcard`. This confirms the controller used by the currently running system/game volume; it is not inferred from the Linux `mmcblk0` number alone.

The p6/p7 names need care: sysfs `PARTNAME` reports p6 as `UDISK` and p7 as `primary`, while the bootargs `partitions=` string labels p6 `appfs` and p7 `UDISK`. The live mount table resolves the use: p6 is `/data` (ext4), while p7 is the vfat volume at `/mnt/sdcard` containing `Roms`, `SYSTEM`, `.userdata` and other card directories. The unified H700 bootstrap therefore selects the on-disk GPT name `primary` for p7; it must not choose p6 from the stale/conflicting `UDISK` label. The H700 driver has not yet read the physical GPT or VBR, so the live Linux mount observation and the host GPT test remain separate evidence. RG34XX's sdc0 is Linux-designated non-removable but is reported as SDHC, not eMMC.

sdc2 is kept as an optional second-slot candidate. It exposes a card-detect GPIO and a `vqmmc-supply` reference, but no card was inserted in the capture. Its GPIO provider cells and exact regulator/clock initialization still require decoding. The profile intentionally records these properties as present-but-unresolved and does not provide pin numbers, polarity, pull, drive, or regulator voltage to a driver.

sdc1 successfully enumerated an SDIO DDR50 device and is excluded from the first storage implementation.

## Live pin and clock observations

From the device's debugfs capture, sdc0 was in pinctrl state `default`, with PF0–PF5 muxed to `sdc0`. sdc1 was also in `default`, with PG0–PG5 muxed to `sdc1`. sdc2 was in state `sleep`; its default pinctrl map describes PC1, PC5, PC6, PC8–PC11, PC13–PC16 and PC0, while the sleep map disables those pins. This means the r2 capture does not show sdc2 default pinmux actively driving the empty slot.

The observed pinconf values are vendor packed configurations, and the exact `sun50iw9p1` encoding is now decoded. The vendor `include/linux/pinctrl/pinconf-generic.h` packs values as `(argument << 8) | parameter`: `PIN_CONFIG_DRIVE_STRENGTH` is parameter 9, `PIN_CONFIG_BIAS_PULL_UP` is 5, and `PIN_CONFIG_BIAS_PULL_DOWN` is 3. Its `sunxi_pconf_set()` converts drive mA to the two-bit hardware field with `mA / 10 - 1`.

- sdc0/sdc1 `0x00002809` = 40 mA drive; `0x00000005` = pull-up.
- sdc2 default `0x00001e09` = 30 mA drive; `0x00000005` = pull-up.
- sdc2 PC0 `0x00000003` = pull-down.

This is grounded in the `sun50iw9p1` vendor implementation pinned at [T507_Kernel @ 98fcc7d](https://github.com/qiaoweibiao/T507_Kernel/tree/98fcc7d3f112e51b0edfb71536da89cb2115106f): `drivers/pinctrl/sunxi/pinctrl-sunxi.c`, `pinctrl-sun50iw9p1.c`, and `include/linux/pinctrl/pinconf-generic.h`. The same SoC-family pin table maps PF0–PF5 to sdc0 ALT2 on D1/D0/CLK/CMD/D3/D2. The project helper now uses the live sdc0 electrical settings, 40 mA and pull-up. This remains source-backed decoding plus host register-model validation, not a live LK MMIO test.

CCU debugfs showed `sdmmc0_mod` at 100 MHz and enabled, with `sdmmc0_bus` and `sdmmc0_rst` enabled. `sdmmc1_mod` was 200 MHz and enabled, with its bus/reset gates enabled. `sdmmc2_mod` showed an 800 kHz rate but was not enabled/prepared; its bus/reset gates were off. These module-clock rates are not card SDCLK rates: the Linux log reports sdc0's card bus at 50 MHz. The H616 CCU reference places MMC module clock config registers at `0x830/0x834/0x838` and the combined gate/reset register at `0x84c` (see the pinned [U-Boot clock definitions](https://github.com/u-boot/u-boot/blob/e578056c766d653270b14557c2a9d8daec414c17/arch/arm/include/asm/arch-sunxi/clock_sun50i_h6.h)).

## sdc0 pinmux and CCU register reference

The live inventory maps the active sdc0 pin group to PF0–PF5. The exact SoC-family vendor table `pinctrl-sun50iw9p1.c` maps PF0–PF5 to `sdc0` at **ALT2 (value 2)**; the captured vendor pinconf specifies 40 mA drive and pull-up. The generic mainline H616 DT's 30 mA setting is not used for this RG34XX profile:

| Pin | MMC0 signal | H616 mux selector |
|---|---|---:|
| PF0 | D1 | 2 |
| PF1 | D0 | 2 |
| PF2 | CLK | 2 |
| PF3 | CMD | 2 |
| PF4 | D3 | 2 |
| PF5 | D2 | 2 |

The PF0–PF5 mux and electrical settings are now source-backed for the `sun50iw9p1` family: ALT2, 40 mA and pull-up. The helper writes only the six relevant fields, preserving PF6/PF7 and all other register bits. This is a source decode plus host register-model validation, not a live LK MMIO test.

The same-family vendor clock source independently confirms the MMC0 offsets: `clk-sun50iw9.h` defines `SMHC0_CFG = 0x830` and `SMHC_GATE = 0x84c`; `clk-sun50iw9.c` binds `sdmmc0_mod` to `SMHC0_CFG`, and `sdmmc0_bus` / `sdmmc0_rst` to `SMHC_GATE` bits 0 / 16. The H616 CCU reference gives the following matching MMC0 clock-control layout (offsets are relative to the CCU base):

| Register | Field | Value |
|---|---|---|
| `0x830` (`MMC0_CLK_CFG`) | M divider | bits 0–3 |
| `0x830` (`MMC0_CLK_CFG`) | N divider | bits 8–9 |
| `0x830` (`MMC0_CLK_CFG`) | parent mux | bits 24–25 |
| `0x830` (`MMC0_CLK_CFG`) | module clock gate | bit 31 |
| `0x84c` (`MMC_GATE_RESET`) | MMC0 bus gate | bit 0 |
| `0x84c` (`MMC_GATE_RESET`) | MMC0 reset deassert | bit 16 |

The Linux H616 CCU driver provides a register-field comparison, but the RG34XX `sun50iw9p1` vendor clock source is authoritative for the target parent topology: the SMHC parent list is `hosc`, `pll_periph0x2`, and `pll_periph1x2`. The vendor DTS assigns `pll_periph0` and `pll_periph1` 600 MHz and defines their `x2` outputs as fixed factors, so this is not the same as assuming the H616 U-Boot `PLL6` 600 MHz effective input. Do not copy the live debugfs `sdmmc0_mod = 100 MHz` rate as the card SDCLK. Linux separately reports the card bus at 50 MHz.

References:
- [H616 MMC0 pin group and controller node, Linux commit](https://github.com/torvalds/linux/blob/fa2933aac170dd19c6a7f0d7b81a8f537e40915c/arch/arm64/boot/dts/allwinner/sun50i-h616.dtsi)
- [sun50iw9p1 vendor PF0–PF5 pin map and pinconf implementation](https://github.com/qiaoweibiao/T507_Kernel/tree/98fcc7d3f112e51b0edfb71536da89cb2115106f/drivers/pinctrl/sunxi)
- [sun50iw9p1 PIO base, clk_pio consumer and sdc0_pins_a electrical profile](https://github.com/qiaoweibiao/T507_Kernel/blob/98fcc7d3f112e51b0edfb71536da89cb2115106f/arch/arm64/boot/dts/sunxi/sun50iw9p1-pinctrl.dtsi)
- [sun50iw9p1 APB1/PIO clock parent mapping and APB1_CFG offset](https://github.com/qiaoweibiao/T507_Kernel/blob/98fcc7d3f112e51b0edfb71536da89cb2115106f/drivers/clk/sunxi/clk-sun50iw9.c)
- [sun50iw9p1 MMC0/SMHC clock and gate/reset register definitions](https://github.com/qiaoweibiao/T507_Kernel/blob/98fcc7d3f112e51b0edfb71536da89cb2115106f/drivers/clk/sunxi/clk-sun50iw9.h)
- [H616 MMC module clock fields, Linux commit](https://github.com/torvalds/linux/blob/fa2933aac170dd19c6a7f0d7b81a8f537e40915c/drivers/clk/sunxi-ng/ccu-sun50i-h616.c)
- [Pinned U-Boot MMC clock calculation](https://github.com/u-boot/u-boot/blob/e578056c766d653270b14557c2a9d8daec414c17/drivers/mmc/sunxi_mmc.c)
- [Pinned U-Boot H6/H616 clock register definitions](https://github.com/u-boot/u-boot/blob/e578056c766d653270b14557c2a9d8daec414c17/arch/arm/include/asm/arch-sunxi/clock_sun50i_h6.h)

### MMC module-clock divider calculator (host-only)

`sunxi_mmc_module_clock.c` is a pure H616/U-Boot-style divider candidate, not yet the RG34XX high-speed clock implementation. For a requested module rate up to 24 MHz it selects HOSC/OSC24M; above 24 MHz it uses the H616 effective-PLL input model. It calculates `div = ceil(parent / target)`, folds factors of two into the two-bit N exponent while M exceeds 16, and encodes M as `M-1`. The returned rate is calculated as `parent / 2^N / M` and is rejected if it exceeds the requested rate or cannot fit N=0..3 and M=1..16.

With the test fixture's 24 MHz HOSC and **600 MHz effective PLL6 input** (a fixture matching the pinned U-Boot/H616 calculator model, not an RG34XX clock measurement), the pure-function test vectors are:

| Requested module rate | Parent | N | M | Calculated module rate | CCU register value |
|---:|---|---:|---:|---:|---:|
| 400 kHz | OSC24M 24 MHz | 2 | 15 | 400 kHz | `0x8000020e` |
| 25 MHz | effective PLL6 600 MHz | 1 | 12 | 25 MHz | `0x8100010b` |
| 50 MHz | effective PLL6 600 MHz | 0 | 12 | 50 MHz | `0x8100000b` |

These are module-clock examples, not direct card-SDCLK claims. The calculator does not enable the CCU bus gate, deassert reset, configure PIO, update CLKCR, issue a card command, or access storage. More importantly, the RG34XX `sun50iw9p1` vendor DTS supplies the MMC `pll_periph` input from `pll_periph1x2`; that is a fixed factor of 2 from `pll_periph1`. Its SMHC parent list is `hosc`, `pll_periph0x2`, `pll_periph1x2` (mux indices 0, 1, 2), which is not the same as assuming the H616 U-Boot `PLL6` 600 MHz effective input. The current H700 platform wrapper therefore uses only the confirmed HOSC path at or below 24 MHz and deliberately rejects higher rates. The 400 kHz read-only bring-up has no PLL-rate prerequisite; the full host sdc0→GPT→FAT→VFS regressions run with `pll6_effective_hz = 0`. Do not use the 25/50 MHz H616 vectors as RG34XX clock settings. Run `sh scripts/test-h700-sunxi-mmc-module-clock.sh` for the pure H616 candidate regression and optional ASan/UBSan pass.

### SD CSD capacity decoder (host-only)

`sunxi_sd_csd.c` decodes the controller's four-word R2 CSD response (word 0 holds bits 127..96; word 3 holds bits 31..0). It supports CSD structure 0 for SDSC and structure 1 for SDHC/SDXC, rejects SDUC/reserved structures, checks the caller's block-addressing classification, and returns capacity in both 512-byte sectors and bytes. Structure 0 uses C_SIZE, C_SIZE_MULT and READ_BL_LEN; structure 1 uses the 22-bit C_SIZE field and the fixed 512-byte block size. SDSC capacity is bounded by the 32-bit byte-addressed CMD17 argument range.

The synthetic RG34XX regression vector uses C_SIZE 29,663, which decodes to **30,375,936 sectors** and **15,552,479,232 bytes**, matching the live Linux block inventory. This validates the bit-position and capacity arithmetic only; the CSD bytes have not yet been captured from the physical SD card. Run `sh scripts/test-h700-sunxi-sd-csd.sh`.

### CMD17 FAT adapter and GPT partition mount

`h700_sunxi_mmc_fat.c` adapts the selected SD card to the existing FAT16/FAT32/exFAT read callback. Each requested sector is issued as a separate CMD17 operation and copied from an explicitly 4-byte-aligned 512-byte bounce buffer. This is important because the filesystem reader's byte-oriented local arrays are not required by C to have the alignment demanded by the PIO FIFO routine. The callback reports only complete sectors and rejects 32-bit LBA/count overflow or requests beyond the card capacity.

`h700_sunxi_mmc_fat_mount_gpt_partition()` accepts a validated GPT partition record, checks its LBA range against the card capacity and the current 32-bit FAT API, then calls `fat_mount_partition()` with the exact GPT extent. The resulting volume remains read-only. The host test mounts a synthetic FAT32 p7 (`primary`) VBR through the same CMD17 data path and verifies its partition/BPB lengths, rejects out-of-card GPT bounds, tests unaligned multi-sector destinations, and asserts no SD write command was emitted.

Run `sh scripts/test-h700-sunxi-mmc-fat.sh`.

### Unified sdc0 → GPT → FAT → VFS bootstrap (host-only)

`h700_fat_vfs_scan_partition_and_install()` is the single read-only startup entry point for a selected GPT name. It performs the existing H616 sdc0 reference pin/CCU setup and SD v2 identification, reads and validates primary GPT, locates the exact name requested by the caller (normally `primary` on the tested RG34XX inventory), initializes the caller-owned persistent CMD17 block device, bounds the FAT mount to that GPT entry, and registers the read-only C backend with `linkslot_vfs_install()`.

The API takes caller-owned process-lifetime provider and block-device objects plus a GPT scratch array of at least 128 entries and a result structure for card/GPT diagnostics. The scratch array is used synchronously and need only remain valid for the call. It must be invoked before Libretro `retro_init()`. Card power and valid MMIO mappings remain board-provided prerequisites; the current 400 kHz path uses the confirmed 24 MHz HOSC and does not need an MMC PLL-rate measurement. This routine does not guess regulator or slot wiring. The block-device object must not alias the provider object and both must remain alive for the lifetime of open VFS handles.

The new full host test models an 8192-sector SDHC card with p6 `UDISK` and p7 `primary`, then calls the unified bootstrap once and opens/reads synthetic `Games/GBA/TITLE.GBA` and `BIOS/BIOS.BIN` through the installed VFS table. This proves the complete simulated software call chain and that the write-command count remains zero; it is not a live RG34XX card test. Run `sh scripts/test-h700-sdc0-fat-vfs-bootstrap.sh`.

### Libretro VFS v3 read-only FAT provider (host-only)

`h700_fat_vfs_backend.c` converts the mounted FAT volume into the C callback table declared by `platform/h700/runtime/include/linkslot_vfs.h`. The installer mounts one validated GPT partition through CMD17 and publishes the backend with `linkslot_vfs_install()`. Fixed arrays hold at most 16 open files and 8 open directories. The mount root exposes only existing directories whose names match the current allowed top-level roots; file access is constrained to an absolute path under the configured mount prefix. Relative paths are rejected until a caller can provide an explicit base directory.

This first backend is read-only by design. It implements get_path/open/close/size/tell/seek/read/stat/opendir/readdir/dirent callbacks; write/flush/remove/rename/truncate/mkdir return failure. The provider struct, MMC device and mounted FAT volume must be static/process-lifetime objects. The current implementation assumes VFS callbacks are serialized and does not claim SMP or reader/writer synchronization.

Run `sh scripts/test-h700-fat-vfs-backend.sh` for the mock-C-ABI integration regression. The target startup path still needs to invoke the installer before Libretro `retro_init()`; this test does not claim the physical RG34XX frontend is wired or bootable.

### GPT -> CMD17 -> FAT32 directory/file scan (host-only)

A higher-level fake-card test now builds a synthetic FAT32 p7 with a root `ROMS` directory, a `BIOS.BIN` file, and `ROMS/TITLE.GBA`. It mounts that volume using the GPT partition record and the same CMD17 PIO callback, enumerates the subdirectory, opens both files case-insensitively, and reads back their four-byte payloads. The test confirms the generic `fat_ro.c` directory traversal and file reader work over the new H700 block adapter, and that no SD write command is emitted.

This is still host emulation. The FAT sectors and directory contents are synthetic, and the test does not mount or enumerate the physical RG34XX card. Run `sh scripts/test-h700-sunxi-mmc-fat-scan.sh`.



`h700_sunxi_sdc0_readonly_scan.c` is the top-level orchestration API. Given mapped PIO/CCU/MMC callbacks and caller-supplied timeouts, it configures PF0–PF5 with the decoded RG34XX `sun50iw9p1` profile (ALT2, 40 mA, pull-up), enables MMC0 bus/reset, requests the 400 kHz identification clock, identifies and selects the SD v2 card, creates the CMD17-only block reader, and parses the primary GPT into caller-owned partition storage. The result preserves card identification and GPT error codes for diagnostics.

It configures APB1 to the 24 MHz OSC24M source, applies the known PF0–PF5 sdc0 pin settings and MMC0 gate/reset, but still requires the caller to have powered the SD card. It does not set a guessed regulator voltage or change unrelated/unknown PIO fields, configure a high-speed timing mode, access another MMC controller, or expose any storage write method. It changes only the supplied PIO/CCU/controller registers and sends SD identification plus CMD17 read commands.

The integrated fake-H700 regressions model PIO, CCU, MMC command/response registers, FIFO, and synthetic SDHC images. The current unified bootstrap test checks the orchestration order, CSD-derived capacity, 400 kHz module clock, PF0–PF5 ALT2/40 mA/pull-up settings, GPT p6 `UDISK` / p7 `primary` labels, FAT32 p7 mount, ROM/BIOS reads through the registered Libretro VFS v3 table, and absence of write commands. This remains host simulation, not physical RG34XX validation. Run `sh scripts/test-h700-sdc0-readonly-scan.sh`.

### End-to-end synthetic GPT over CMD17 (host-only)

A separate integration regression now exercises the complete software path: the GPT parser calls the `h700_block_reader`, which issues CMD17 through the FIFO-backed PIO reader, whose fake controller returns sectors from a synthetic 40-sector SDHC image. The GPT contains p6 named `UDISK` and p7 named `primary`; the test verifies both names and 1-based GPT slot numbers are discovered from the on-disk entries. It also asserts that six CMD17 reads service MBR, primary header, and both entry-array sectors in CRC and parse passes, with no command carrying the write bit.

This closes the host-side interface wiring gap, but remains synthetic: no physical RG34XX LBA0, GPT header, or partition sector has yet been read. Run `sh scripts/test-h700-sunxi-mmc-gpt-integration.sh`.

### CMD17 to GPT block-reader adapter (host-only)

`h700_sunxi_mmc_block_device.c` converts a successfully identified SD card into the existing `h700_block_reader` interface. It copies the sector capacity and block/byte-addressing classification returned by the card's OCR/CSD, fixes the sector size at 512 bytes, and forwards each GPT sector request to the read-only CMD17 PIO function. Errors and out-of-range LBAs are propagated as failed sector reads. The adapter exposes no write, erase, flush, or raw-command method.

The GPT parser now explicitly aligns its sector/header scratch buffers to 4 bytes, matching the PIO reader's buffer contract. The host regression verifies metadata propagation, data forwarding through the FIFO, SDHC LBA versus SDSC byte-address conversion, and out-of-range rejection. It is a simulated block-device test; no physical sector has been read from the RG34XX yet.

Run `sh scripts/test-h700-sunxi-mmc-gpt-adapter.sh`.

### H616 sdc0 PF0-PF5 pin configuration (host-only)

`sunxi_h616_sdc0_pins.c` implements the RG34XX `sun50iw9p1` sdc0 pin group. The PIO bank stride is `0x24`; Port F is bank index 5, so PF0–PF5 share MUX0 at PIO offset `0xb4`, DRV0 at `0xc8`, and PULL0 at `0xd0` (offsets relative to PIO base `0x0300b000`). The vendor `pinctrl-sun50iw9p1.c` descriptor assigns ALT2 to `sdc0` on each of PF0–PF5. The live pinconf specifies 40 mA drive (hardware field 3) and pull-up (field 1).

The helper updates only PF0–PF5 fields, preserves PF6/PF7 and all unrelated drive/pull fields, sets 40 mA drive and pull-up before switching the mux to ALT2, and validates readback. The vendor DTS at `sun50iw9p1-pinctrl.dtsi` defines the PIO MMIO base as `0x0300b000`, supplies the pinctrl consumer with `clk_pio`, and defines `sdc0_pins_a` with muxsel 2, drive 3 and pull 1. In the pinned `clk-sun50iw9p1` clock driver, the logical `pio` clock uses `apb1` as its parent; `APB1_CFG` is CCU offset `0x520`, with M in bits 0–1, N in bits 8–9, and mux in bits 24–25. Parent index 0 is HOSC (24 MHz), while M=0 and N=0 both encode divide-by-1. Therefore the unified bring-up calls `sunxi_h616_pio_enable_apb1_24m()` before PIO access to select HOSC with M/N at /1. The helper rejects a platform that does not report a 24 MHz oscillator and verifies clock-field readback. This is confirmed by the same SoC-family source, but remains host-tested only and has not been run against the device's PIO registers.

Run `sh scripts/test-h700-h616-sdc0-pins.sh` for register-layout, field-preservation, write-order, and readback-error tests.

### H616 sdc0 CCU and identification-clock integration (host-only)

`sunxi_h616_sdc0_platform.c` binds the HOSC-only identification-clock path and shared H616-family `MASK_DATA0` clock-update helper to the vendor-confirmed CCU-relative offsets. The caller maps the RG34XX `sun50iw9p1` CCU base `0x03001000` and provides read/write callbacks. `sunxi_h616_sdc0_enable_bus()` sets only MMC0 bus gate bit 0 and reset-deassert bit 16 in CCU `0x84c`. `sunxi_h616_sdc0_set_clock_hz()` currently accepts only rates up to 24 MHz, disables card output, writes `MMC0_CLK_CFG` at `0x830`, clears the controller divider, and reenables the card clock through the shared `MASK_DATA0` update path. The 400 kHz regression expects module-clock register value `0x8000020e`.

The high-speed path above 24 MHz is deliberately rejected until both the `sun50iw9p1` SMHC parent/divider mapping and RG34XX sampling/output timing are implemented and validated. In particular, the H616 PLL6 calculator is not yet a valid source for the H700 `pll_periph1x2` parent. The clock helper does not itself program PIO mux/bias/drive, card power, or physical MMIO; pin configuration is handled by the separate PIO helper using the decoded RG34XX settings (ALT2, 40 mA, pull-up). All helpers remain host-tested only, and actual register access on the device is still unverified.

Run `sh scripts/test-h700-h616-sdc0-platform.sh` for host register-model tests covering CCU gate/reset, exact 400 kHz divider value, H616 update sequencing, timeout cleanup, and deliberate rejection of unvalidated high-speed requests.

### SD memory-card identification state machine (host-only)

`sunxi_sd_card_init.c` adds a closed, read-only command sequence for SD v2 memory cards: CMD0, CMD8, repeated CMD55/ACMD41, CMD2, CMD3, CMD9, CMD7, and CMD16 only for byte-addressed SDSC. It performs controller soft/FIFO/DMA reset, initializes timeout and H616 FIFO-threshold registers, keeps the bus at 1-bit mode, requests a 400 kHz identification clock through a platform callback, waits at least 1 ms for the required power-up clock cycles, negotiates OCR, records CID/RCA/CSD, decodes capacity, selects the card, and waits for DAT0 busy to clear. It rejects invalid CMD8 echo, unsupported CSD/addressing combinations, and SD cards that do not complete OCR power-up. SDUC and MMC/eMMC are outside the current supported profile.

The current clock callback configures the module clock and card clock from the confirmed 24 MHz HOSC path for 400 kHz identification and uses the shared H616-family `MASK_DATA0` update protocol. It does not require a PLL-rate measurement. Board code must still provide valid sdc0/CCU/PIO MMIO mappings, enable CCU bus/reset resources, configure PF0–PF5 and power the card; those board operations are not performed by the card-identification state machine itself. The implementation stays at 400 kHz after initialization; higher transfer clocks remain disabled until the H700 `pll_periph1x2` path and timing are validated.

The host regression simulates the controller registers and SD responses. It covers the complete SDHC command order and the RG34XX capacity vector, ACMD41 readiness polling, SDSC byte addressing plus CMD16, CMD8 rejection, CMD55/ACMD41 rejection, OCR/command/busy timeouts, and the invariant that no command sets the write bit. It is not evidence of a physical card negotiation. Run `sh scripts/test-h700-sunxi-sd-card-init.sh`.

### Single-sector read primitive (host-only, not a complete MMC driver)

`sunxi_mmc_pio_read.c` adds a deliberately narrow CMD17 operation for one 512-byte sector. It uses the AHB-accessed CPU FIFO path and keeps IDMA disabled; it never exposes arbitrary command submission and has no write, erase, or multi-block-write operation. The caller must already have completed controller initialization, card identification/OCR negotiation, RCA selection, card selection, bus-width setup, and clock setup. It then supplies the known card capacity and whether the card uses block addressing (SDHC/SDXC) or byte addressing (SDSC).

The block argument is the sector LBA for SDHC/SDXC and LBA multiplied by 512 for SDSC. Each request validates the LBA, 32-bit command-argument range, buffer capacity, and 4-byte alignment. The FIFO path drains exactly 128 words, checks the controller RINT error mask, and waits separately for COMMAND_DONE and DATA_OVER. Timeout/error recovery resets controller/FIFO/DMA state and executes the H616 UPCLK_ONLY clock update with MASK_DATA0 asserted and cleared on success or timeout.

The host regression models the register block and FIFO. It covers CMD17 flags (including absence of the write bit), SDHC/SDSC addressing, 512-byte transfer, response return, FIFO ordering/barriers, invalid buffers/ranges, FIFO/data timeouts, controller errors, and recovery timeout. Run `sh scripts/test-h700-sunxi-mmc-pio-read.sh`.

**Evidence boundary:** the host-tested read primitive and full software path have not yet been run against the physical controller. Round 16 confirms the live controller mapping and Linux's successful use of sdc0, but no H700/LK CMD17 operation has read a physical sector. Physical MMIO mapping, PIO/APB1 bring-up, card power and on-device validation remain outstanding; the PF0–PF5 pinconf values are now source-decoded.

## sdc0 read-only driver constraints

The first controller implementation targets the confirmed `sdc0` register block at `0x04020000` only. The pinned [U-Boot sunxi MMC implementation](https://github.com/u-boot/u-boot/blob/e578056c766d653270b14557c2a9d8daec414c17/drivers/mmc/sunxi_mmc.c) provides a portable reference for command/response handling, polling, FIFO transfers through the AHB path, clock setup, and H616 FIFO threshold initialization. Its register map is in [sunxi_mmc.h](https://github.com/u-boot/u-boot/blob/e578056c766d653270b14557c2a9d8daec414c17/drivers/mmc/sunxi_mmc.h).

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

## FAT/exFAT partition-bound validation

The FAT backend now has an explicit `fat_mount_partition()` entry point. The caller passes a GPT/MBR partition start LBA and its exact sector count; the FAT BPB's TotalSectors must not exceed that extent. FAT metadata is validated before mounting: bytes-per-sector and cluster size, reserved area, FAT count/size, data region, FAT entry capacity, FAT16/FAT32 cluster-range classification, and FAT32 root-cluster bounds. The legacy whole-device `fat_mount()` path also propagates GPT/MBR partition lengths when it discovers a volume, rather than treating a VBR as unbounded.

The exFAT mount path reads VolumeLength from the VBR and checks it against the containing partition, the 32-bit LBA limit, FAT copies and FAT size, cluster heap end, cluster count, and root-cluster range before reading the root cluster to locate the allocation bitmap/up-case table. This project currently uses 32-bit sector LBAs, so exFAT volumes with a nonzero high 32-bit VolumeLength are rejected rather than truncated.

Host tests cover FAT16, FAT32 and exFAT valid extents, BPB/VolumeLength extending one sector beyond the partition, an undersized FAT, an invalid FAT32 root cluster, exFAT heap overflow, MBR extent propagation, and GPT p7 extent propagation. Run `sh scripts/test-h700-fat-partition-bounds.sh`. These are synthetic metadata regressions; the actual RG34XX p7 boot sector has not yet been read by the H700 path.

## Implementation sequence

1. Implement a minimal, **read-only** sdc0 block path and validate SD command initialization without issuing any write/erase commands.
2. Parse GPT read-only and locate p7 using the on-disk GPT rather than hardcoding the bootargs or kernel partition numbers.
3. Use the live mapping to select GPT p7 (`PARTNAME=primary`, mounted by Linux at `/mnt/sdcard` despite the bootargs `UDISK` label), then read-only mount and run ROM enumeration/readback checks through the H700 path.
4. Only then add sdc2 as a separately controlled optional slot, after decoding its GPIO, clock and regulator provider details and obtaining a capture with a card inserted.
5. Keep FAT/exFAT save writes disabled until LFN directory creation, reader/writer synchronization, atomic replace, and durable block flush/barrier semantics are implemented and tested.

No SD card writes, boot-area writes, firmware construction, or payload execution are part of this profile.
