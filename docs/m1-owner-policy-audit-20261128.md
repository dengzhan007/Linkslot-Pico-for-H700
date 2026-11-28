# M1 owner-policy audit: Profile A and dedicated RTC/WDT test workflow

Date: 2026-11-28

## Owner decisions

The owner selected **SPL Profile A** and accepted the **native RTC/WDT protocol only within an audited dedicated test-TF workflow**. These are policy decisions for the design and static-audit scope. They do not authorize execution, physical hardware application, firmware writes, or payload launch.

## A. SPL Profile A — what is and is not approved

Profile A means the project may preserve only those PMIC/TWI initialization operations that are proven necessary by the pinned stock H700/Boot0/SPL source and device description. Each proposed operation must have a provenance record containing:

- source file, immutable revision, and exact call path;
- controller instance/base and clock/reset/pinmux prerequisites;
- PMIC compatible, bus address, register address, access width, mask/value and read/write semantics;
- ordering relative to DRAM training and any earlier power-stage initialization;
- timeout, NACK/error handling, retry policy, and fail-closed behavior;
- evidence that the operation is necessary rather than inherited incidental behavior.

No guessed PMIC register writes, speculative voltage changes, unrestricted bus scans, or runtime power-management additions are allowed.

### Current evidence and limits

1. The matched RG34XX BL31/TOC1-DTB static audit establishes a specific suspend-side TWI selector dataflow: original TOC1 FDT `/aliases/pmu0` points to an AXP2202 PMU under `twi@0x07081400`; its parent `device_type` is `twi5`; the copied context selector resolves to index 5; the relocated lookup selects R-TWI0 base `0x07081400`. This is matched binary + original DTB static evidence, not a live bus trace.
2. The BL31 transfer code has TWI-master-like register operations, but the existing audit explicitly does **not** establish a PMIC slave address, PMIC register sequence, command payload, GP_DATA2 consumer, or final platform action. Therefore this evidence identifies the likely controller and PMIC node, not a reusable PMIC initialization routine.
3. The current M1 SPL preflight ELF is deliberately a **minimal read-only preflight fixture**. Its resolved config disables `SPL_I2C`, `SPL_DM_I2C`, `SPL_DM_PMIC`, `SPL_POWER`, and vendor PMIC paths; its post-link contract also rejects `i2c_read`, `i2c_write`, and `pmic_bus_write`. This remains correct for this isolated GPT/MMC read-only preflight because the preflight has no demonstrated PMIC/TWI dependency. It must not be mislabeled as the eventual Profile A DRAM/power-initialization SPL.
4. The linked M1 artifact does contain reachable clock/PLL and MMC initialization paths. The disassembly shows `board_init_f -> clock_init -> clock_init_safe / clock_init_uart`, followed by `baseos_early_mmc -> mmc_initialize -> mmc_init`. These paths include MMIO stores and controller operations; the conservative callgraph reports 160 reachable functions and 759 store-class instructions. Its own limitations explicitly exclude indirect calls, complete address/dataflow classification, and SMC/system-register semantics. Thus the link is not runtime-safe or execution-authorized.
5. The existing stock BaseOS native SPL patch evidence includes monitor parameter, board-byte and DTB-pointer patches; it does not by itself prove a PMIC initialization sequence or that DRAM rails can be adjusted safely from this M1 preflight stage.

### Required next static work for Profile A

- Trace the actual power/DRAM initialization owner across Boot0, the pinned BaseOS SPL source and the target AXP2202 DT node; determine whether PMIC writes occur before DRAM training, and in which stage.
- Identify the exact AXP2202 register map and board-specific regulator/voltage requirements from pinned source and board configuration. Do not infer values from the generic compatible string.
- Separate any required DRAM-power initialization from the later M1 read-only MMC/GPT preflight. Keep the latter's current no-I2C/no-PMIC profile unless a concrete dependency is proven.
- Produce a per-operation allowlist and a separate forbidden-access list. A Profile A source/ELF audit must prove every allowed bus transaction is reachable only in the intended initialization phase and that no unrelated PMIC operation was pulled in by Kconfig.
- Before any future execution proposal, perform a transitive callgraph and linked-ELF review of all TWI/MMIO sites, including indirect driver callbacks and error paths. Static audit alone does not authorize execution.

## B. Dedicated test-TF RTC/WDT protocol

The accepted scope is one dedicated, recoverable test-TF workflow. It is not the daily boot protocol and is not permission to modify the ordinary raw-capsule-v2 semantics.

### Current evidence

- The pinned BaseOS raw-capsule-v2 implementation is stateful: it reads RTC retention / GP14 / GP15 state, evaluates the attempt pair, configures and arms the watchdog, commits attempt state, then enters its SRAM copy/launch path.
- This means raw-capsule-v2 cannot be treated as an already-safe M1 one-shot mechanism. M1 must retain a separate protocol and must not silently inherit the v2 attempt-counter or launch semantics.
- The BL31 RTC soft-entry audit establishes the suspend-side TWI selector and controller candidate described above. It does not establish the RTC retention-register ownership or the exact GP14/GP15 consumer semantics in a dedicated SPL test workflow.
- The present M1 preflight ELF does not link RTC/WDT, PMIC/TWI or payload-launch functionality. It is compile/link-only with a zero FNV sentinel that rejects before firmware payload reads.

### Required audit before a dedicated test implementation

1. Enumerate every RTC backup/retention register touched by the pinned Boot0, BL31, BaseOS capsule and Linux sources; identify ownership, width, reset retention and any shared fields.
2. Trace GP14/GP15 reads/writes end-to-end, including attempt-pair encoding, ordering, validity markers, reset causes, and stale-state behavior.
3. Trace WDT clock source, reset domain, timeout encoding, enable/arm sequence, refresh requirements, reset behavior, and whether it remains active across the intended handoff.
4. Define a dedicated test marker with generation/version and integrity checks; stale, malformed, partially written or unknown states must fail closed to the normal recovery path.
5. Define cleanup and rollback for both successful and interrupted tests. No test marker may strand the device in a repeated reboot or watchdog loop.
6. Add host-only state-machine tests for cold boot, warm reset, watchdog reset, power loss during marker update, stale marker, malformed marker, timeout, and recovery.
7. Audit the final linked ELF and its transitive callbacks before proposing any device test. Keep physical execution unauthorized until a separate explicit approval.

## C. Current CI evidence

- Linkslot/LK feasibility run [#1073](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46138855460), commit `5f46a52a1c65708820b6df7fc99cb157d18426b0`: **completed successfully**. The compact review artifact is 551,255 bytes (GitHub artifact SHA-256 `dd5f65d63cce41ed240247fe68b397ddf505f2c87574fa31591714d48d4244cb`), replacing the previous oversized ~527 MiB artifact. The downloaded ZIP was inspected; its internal SHA256SUMS entries match the extracted files.
- Linked M1 contract: BSS use 5,032 / 16,384 bytes; 4 KiB scratch at `0x30340`; GPT table at `0x30000`; stack pointer `0x52a00`; 160 reachable functions and 759 reachable store-class instructions.
- Contract flags remain `execution_authorized=false`, `candidate_eligible=false`, `hardware_qualified=false`. Manifest flags remain `device_io_performed=false`, `media_write_performed=false`, `firmware_flash_performed=false`, `payload_executed=false`.
- H700 boot audit run [#1738](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46138911358) completed successfully on the preceding status-document commit; both Windows and host-tools jobs passed.
- The newer H700 boot audit triggered by the owner-decision record is tracked separately in the current project-status entry; do not count it as passed until completed.

## D. Non-negotiable gates

- `H700_MEMORY_LAYOUT_APPROVED=false`
- `hardware_applied=0`
- `candidate_eligible=false`
- No firmware payload execution or DRAM destination copy.
- No TF/Boot0/TOC1 writes or firmware flashing.
- DRAM source pages `0x40000000` and `0x60000000` remain excluded from scratch use.
- No M2 advancement based solely on this static audit or a green compile/link workflow.


## E. Additional pinned-source findings (2026-11-28 continuation)

The pinned BaseOS source was inspected directly at commit `cbe253a43f46cf551cc9a6c79a84015bde3686cf`, including `spl/build-early-mmc.py`, `spl/early-mmc.c`, `spl/native-boot.c`, `native/capsule/capsule.c`, `native/capsule/entry.S`, and `src/native-attempt.h`.

### E.1 Profile A must not be confused with the current preflight-only SPL

- The builder's documented sequence inserts `baseos_early_mmc()` before `sunxi_board_init()`; the mainline path is explicitly a pre-DRAM SD control. The current M1 preflight overlay deliberately stops after the read-only MMC/GPT/package validation path and never calls the later `sunxi_board_init()` continuation.
- The pinned builder shows that the persistent non-resume board-init path retains `i2c_init_board()` and `i2c_init(CONFIG_SYS_I2C_SPEED, CONFIG_SYS_I2C_SLAVE)`, while the resume path skips them. This is evidence that the upstream SPL board-init sequence has an I2C/TWI initialization phase whose ordering is conditional on resume state; it is not yet proof of the exact AXP2202 register writes or their necessity for DRAM rails.
- The current `native-boot.c` PMIC calls (`pmic_bus_read(0x20)`, `pmic_bus_read(0)`, `pmic_bus_read(0xf0)`, conditional `pmic_bus_write(0xf0, 0)`) are in `baseos_native_bootreason()`, which is called from `baseos_vendor_handoff()` after its `sunxi_board_init()` call. These are boot-reason / scratch-byte operations, not demonstrated DRAM-voltage initialization. Do not reuse them as the Profile A power-up sequence.
- Therefore the correct implementation split is: retain the current M1 read-only preflight's no-PMIC/no-TWI configuration; separately audit the I2C/TWI board-init path that precedes DRAM initialization in the future SPL profile. Profile A is a requirement for that future DRAM-capable path, not a reason to pull PMIC drivers into the current preflight ELF.

### E.2 Exact raw-capsule-v2 RTC/WDT addresses and behavior

The pinned raw-capsule-v2 source defines:

| Function | Address | Current source behavior |
|---|---:|---|
| RTC GP14 | `0x07000138` | attempt cookie |
| RTC GP15 | `0x0700013c` | attempt marker |
| RTC retention | `0x070001f8` | bit 16 selects resume in outer capsule |
| WDT control | `0x030090b0` | reload value `0x14af` |
| WDT config | `0x030090b4` | set/read back `1` |
| WDT mode | `0x030090b8` | set/read back `0xb1`; low-bit active check |

The raw outer capsule reads retention first. If not resuming, it reads the GP14/GP15 pair, requires the empty pair before the native attempt, validates stock/native fingerprints, refuses to take over an already-active watchdog, writes WDT config/mode/control with readback checks, then commits the attempt pair with barriers and readback. The A32 entry saves CPU context and transfers through the SRAM copy stub. The raw SPL adoption function performs two passes of read-only GP14/GP15 and WDT config/mode checks; it does not commit the attempt pair. The later native handoff reloads the watchdog control register.

The attempt-pair implementation in `src/native-attempt.h` is explicitly non-atomic: it uses ordered cookie-then-marker writes and detects observed changes by readback. Its own comment requires exclusive firmware ownership plus one serialized Linux health supervisor for acknowledgment. This protocol is not a generic free RTC scratch area.

**M1 implication:** do not copy this v2 state machine into M1. First define a distinct dedicated-test marker and owner, document whether the test path is allowed to touch retention or only GP14/GP15, and define interruption-safe cleanup and reset recovery. The accepted RTC/WDT decision authorizes this design/audit work only. It does not authorize writing these registers on a device.



### E.3 Target PMIC identity versus generic upstream H700 defconfig — unresolved compatibility gap

A cross-check exposed a board-specific mismatch that must be resolved before Profile A can be implemented:

- The captured RG34XX live-FDT inventory on the `reports` branch identifies `x-powers,axp2202` at `/soc@03000000/twi@0x07081400/pmu`, with the parent controller `twi5` / R-TWI0. This is target-specific captured DT evidence.
- The public U-Boot v2026.10 `configs/anbernic_rg35xx_h700_defconfig` instead enables `CONFIG_AXP717_POWER`, with AXP717 regulator settings. It is a generic RG35XX H700 defconfig, not proof of the RG34XX board's PMIC.
- The same pinned U-Boot v2026.10 `drivers/power/pmic/axp.c` compatible table contains several AXP families (including AXP717) but no `x-powers,axp2202` entry. Therefore simply enabling the stock AXP PMIC Kconfig path does not establish working AXP2202 support.
- The target FDT's regulator constraints (including CPU DCDC1 range and enabled/boot-on rails) are declarations, not measured PMIC register values or a validated power-up recipe. They must not be translated into guessed register writes.

**Disposition:** Profile A remains the approved policy, but the actual target-specific PMIC initialization implementation is **blocked pending source provenance**. First locate the vendor AXP2202 driver/register definitions and board-specific power sequence from the exact RG34XX firmware/source lineage; then compare its bus/controller and register semantics with the matched BL31 R-TWI0 path. Do not substitute AXP717, copy generic H700 voltage settings, or add a guessed AXP2202 driver. Until that source is found and audited, the current M1 preflight remains PMIC/TWI-disabled and the future DRAM-capable Profile A path remains unimplemented.



### E.4 Target-specific DTS confirmation and candidate Allwinner AXP2202 source

A later source search refined the PMIC identity finding:

- The Knulli H700 package's **RG34XX-specific** boot-package DTS at `package/boot/uboot-h700/rg34xx/boot_package/rg34xx.dts` (repository `knulli-cfw/knulli-linux`, branch `knulli-main`, commit `57afa662009641d181f57241bcc480119b131530`) declares `/soc@03000000/twi@0x07081400` as `device_type = "twi5"`, and its child PMU as `compatible = "x-powers,axp2202"`, `reg = <0x34>`. This independently matches the captured RG34XX live-FDT inventory. The target bus/address pair is therefore strongly grounded as **R-TWI0 at 0x07081400, PMIC address 0x34**.
- An Allwinner-authored AXP2202 SPL driver is available in `sam-yangjj/tina-v821-v1.3-brandy`, commit `34809037526678ccda1720cb4b4ec7ee32272c38`, files `brandy-2.0/spl/drivers/power/axp2202.c` and `brandy-2.0/spl/include/arch/axp2202_reg.h`. Under `CFG_SUNXI_TWI` it uses 400 kHz, runtime address 0x34 with fallback 0x35, and exposes AXP2202 register definitions. The driver has a helper `axp2202_set_ddr_voltage()` mapped to DCDC3; however, this source is for a different Allwinner platform, and no call site tying that helper to the RG34XX/H700 DRAM initialization was established.
- That candidate driver's `axp2202_set_necessary_reg()` writes charge-current, VBUS-current, ADC, VSYS-minimum, DCDC1 undervoltage-protection and module-enable registers. These are not automatically DRAM prerequisites. The helper's presence does not prove that every listed write belongs in our Profile A path.
- The same AXP2202 driver family appears in SyterKit, but the SyterKit implementation is an independent bare-metal reference, not the RG34XX vendor boot chain. Treat both as register-map/protocol cross-checks only, not as a source of board-specific voltage values.
- The generic U-Boot H700 defconfig's `CONFIG_AXP717_POWER` selection and the PMIC driver's compatible table remain a **binding/configuration mismatch to reconcile**, not proof that the physical PMIC is an incompatible chip. AXP2202/AXP717 register-family overlap is suggested by external driver headers, but no exact H700 binding equivalence has been proven.

**Revised disposition:** target PMIC identity, R-TWI0 controller and I2C address are now independently corroborated. The unresolved items are the exact RG34XX vendor SPL/Boot0 PMIC transaction sequence, whether/when DCDC3 or another rail must be adjusted before DRAM training, the precise AXP2202 silicon revision/address fallback behavior, and which initialization writes are necessary. Keep Profile A implementation blocked until those are resolved against the actual target firmware lineage. Do not copy the Tina/SyterKit voltage or initialization writes directly.

