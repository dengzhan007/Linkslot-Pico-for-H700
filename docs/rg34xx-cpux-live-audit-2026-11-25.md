# RG34XX CPUX live read-only audit — 2026-11-25

**Target:** Anbernic RG34XX (non-SP), Allwinner H700 / sun50iw9p1, Cortex-A53 ×4  
**Capture time:** 2026-11-24 23:37:46 UTC / 2026-11-25 07:37:46 +08:00  
**Kernel:** Linux 4.9.170 #19 SMP PREEMPT aarch64  
**Evidence branch:** [reports](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports), directory [rg34xx-inventory-20261125-cpux](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-inventory-20261125-cpux)  
**Report's stated source:** porting/linkslot-1.0.45-feasibility @ aad78d4; read-only inventory v1; ADB shell UID 0. Binary captures remain on the acquisition PC.  
**Decision:** the CPU rail and live Linux clock-tree relationship are now substantially better evidenced, but **no RG34XX OPP is approved and no CPUX/PMIC write path is authorized**.

## 1. Primary findings

### 1.1 CPU topology and shared policy

The live CPU power section in live-fdt.json identifies four Cortex-A53 CPU nodes, CPU0–CPU3. Runtime Linux reports:

- online = 0-3, possible = 0-3, present = 0-3.
- Each per-CPU cpufreq directory reports affected_cpus = 0 1 2 3 and related_cpus = 0 1 2 3.
- The single policy0 also covers CPUs 0–3; driver is cpufreq-dt, governor is schedutil.
- Runtime available frequencies: 480, 720, 936, 1008, 1104, 1200, 1320, 1416 and 1512 MHz; min/max are 480/1512 MHz.
- During this capture the four cpuN/cpufreq nodes report 1416 MHz, while a later policy0 read reports 1320 MHz.

This is strong evidence that Linux treats the four CPUs as one shared frequency policy. It does **not** prove CPU1–CPU3 state at the earlier BL31→BL33 handoff. The handoff invariant remains open.

### 1.2 Live Linux clock tree

The captured /sys/kernel/debug/clk/clk_summary contains:

    pll_cpu       1512000000
      cpu         1512000000
        cpuapb     378000000
        axi        756000000

    pll_periph0    600000000

The CPU branch is pll_cpu → cpu; cpu equals the PLL rate in this snapshot, and CPUAPB is one quarter of it. pll_periph0 appears as a separate tree at 600 MHz with four prepare/enable users; it is not the current CPU parent in the captured tree.

A pinned same-version Linux source candidate explains why sample timing matters:

- `cpufreq-dt` registers `.get = cpufreq_generic_get`.
- In that source, `cpufreq_generic_get()` returns `clk_get_rate(policy->clk) / 1000`; `cpuinfo_cur_freq` calls this driver getter, so it reads the clock framework at the time of that sysfs read (not an independent hardware counter).
- For a target-based driver such as `cpufreq-dt`, `scaling_cur_freq` normally returns `policy->cur`, the last frequency recorded by cpufreq transition notifications. It is cached state and can lag the instantaneous clock.
- `clk_summary` also reports clock-framework rates, but inventory v1 reads it later than the cpufreq attributes. Under `schedutil`, the governor can change the shared CPU clock between those reads.

Therefore the present report supports the **clock parent relationship** and confirms a shared policy, but it does not supply a synchronized instantaneous-frequency sample. It is too early to conclude that cpufreq and the clock provider disagree persistently. The collector has now been extended to take six read-only, uptime-bracketed samples of `scaling_cur_freq`, `cpuinfo_cur_freq` and the CPUX clock-summary rows at 200 ms intervals; those new samples still need to be collected from the device.

The `enable_cnt=0 / prepare_cnt=0` values for `pll_cpu`/`cpu` are not proof that the CPU clock is gated off; clock-framework counters are not a reliable CPU PLL enable-state test here. The 600 MHz `pll_periph0` observation is useful when reviewing the vendor driver's alternate-parent sequence, but it does not prove BL33-entry parent/rate, timer continuity, or that switching to it is safe under LK.

The exact pinned vendor source sequence remains documented in [H700 CPUX clock-source audit](h700-cpux-clock-source-audit.md).

### 1.3 CPU supply is confirmed as AXP2202 DCDC1

The live FDT parser resolves CPU0's cpu-supply:

- phandle 0x4f
- /soc@03000000/twi@0x07081400/pmu/regulators@0/dcdc1
- regulator axp2202-dcdc1, status okay, always_on, boot_on
- declared range 500,000–1,540,000 µV.

Runtime Linux regulator evidence independently identifies the DCDC1 consumer as cpu0 and reports 1,160,000 µV in this sample. The /sys/class/regulator and regulator debugfs data are Linux regulator-framework observations, not a raw PMIC I²C register dump. The separate axp2202-cpusldo is 900 mV and has no CPU consumer in the captured regulator summary; do not mistake it for the CPU supply.

DCDC1 is now confirmed for this live FDT/runtime pairing, rather than remaining only a cross-reference candidate. A BL33-entry PMIC state and exact board-revision readback are still outstanding.

### 1.4 OPP declarations, runtime list and SID do not yet form an approved table

The live FDT declares a shared /opp_l_table referenced by all four CPU nodes. Relevant voltage-bearing examples include:

| Declared frequency | opp-supported-hw cell | Declared voltage |
|---:|---:|---:|
| 936 MHz | 0x30 | 900 mV |
| 1008 MHz | 0x01 / 0x02 / 0x0c / 0x30 | 920 / 900 / 1020 / 950 mV |
| 1104 MHz | 0x30 | 1000 mV |
| 1200 MHz | 0x01 / 0x02 / 0x0c / 0x30 | 980 / 960 / 1100 / 1050 mV |
| 1320 MHz | 0x30 | 1100 mV |
| 1416 MHz | 0x10 / 0x20 | 1150 / 1160 mV |
| 1512 MHz | 0x01 | 1120 mV |

These are static live-DT declarations, not a validated runtime OPP grid. Several duplicated/variant OPP nodes have no voltage properties; dmesg records _opp_supported_by_regulators rejecting entries at 600, 792, 1032, 1296 and 1344 MHz with minuV=0 / maxuV=0. Runtime scaling_available_frequencies differs from a simple reading of every DT node. The OPP core's selected speed-bin mask and the actually active OPP set are not proven by this inventory.

The bounded SID capture reports:

- speed cell bytes 00 6c, little-endian raw 0x6c00;
- thermal cell bytes 18 11 13 ab 19 1b 16 bb;
- four decoded 12-bit thermal fields: 2835 / 2841 / 2838 / 2842; factory-test temperature raw 280 (28.0 °C).

The pinned mainline reference classifier calls 0x6c00 candidate bin 5, without requiring SMCCC revision for that particular mapping. However, the raw value matches none of the pinned vendor cpufreq literal cases, the generic /sys/devices/soc0 identity is unavailable, and the parser explicitly leaves opp_authorization=not_approved. Candidate bin 5 is not an opp-supported-hw authorization and must not be turned into a release OPP mask.

The 1416 MHz / 1160 mV runtime observation is numerically consistent with the DT 1416 MHz 0x20 / 1160 mV variant, but the cpufreq-vs-clk_summary divergence and unresolved vendor speed-bin semantics make this a correlation only—not proof that bit 5 was selected or that the pair is safe.

### 1.5 Exact-version cpufreq/OPP source check — candidate only

The device reports Linux `4.9.170`. The pinned vendor tree [qiaoweibiao/T507_Kernel@98fcc7d3f112e51b0edfb71536da89cb2115106f](https://github.com/qiaoweibiao/T507_Kernel/tree/98fcc7d3f112e51b0edfb71536da89cb2115106f) has the same top-level kernel version in its Makefile (`4.9.170`), but no build ID or image hash ties the running kernel to that exact source revision. Treat it as a same-version source candidate, not proof of the binary's source.

Pinned file evidence:

| Source | Blob SHA | Finding |
|---|---|---|
| `Makefile` | `532275df0dc8dc01ed7ee1c6ddd95bd36f5ba1a7` | Linux 4.9.170 |
| `drivers/cpufreq/sun50i-cpufreq-nvmem.c` | `e121a4ddf28adb55e0b6bf78b450bd783b8b27bc` | Reads the two-byte NVMEM speed cell into a `u32 *`, does not validate `len`, then switches on `*efuse_value`; unknown values fall through to supported-hardware mask `0x4` |
| `drivers/base/power/opp/of.c` | `d92e7d3a4c02f722dc4d62cd460e316b403ebced` | `_opp_is_supported()` checks that `opp-supported-hw & supported_hw` is nonzero |
| `drivers/base/power/opp/core.c` | `e494a938a9f1fcbeabb9c5469f87e82b4aa430ec` | `dev_pm_opp_set_supported_hw()` stores the driver-provided version mask |
| `drivers/cpufreq/cpufreq-dt.c` | `fbb278b7eed6bb291ab087ab4ee365acaa6036f3` | `cpufreq-dt` uses OPP-derived frequency table, `dev_pm_opp_set_rate()`, and `.get = cpufreq_generic_get` |
| `drivers/cpufreq/cpufreq.c` | `8cb2f419a070a9bdc1a80328191df1c19583e8e4` | `cpufreq_generic_get()` reads `clk_get_rate()`; `scaling_cur_freq` uses cached `policy->cur` for this target-based driver |

For raw SID `0x6c00`, the pinned vendor switch has no matching literal and would take its default branch, assigning `supported_hw = 0x4`—**assuming the unsafe 32-bit read happens to return the two-byte value as expected**. The read is not safe to assume: the NVMEM API returns a two-byte allocation for this DTS cell, while the cpufreq consumer dereferences a `u32 *` without checking `len`.

Applying the pinned OPP bitwise filter to the voltage-bearing entries in this live DT, mask `0x4` would select candidate entries at **480, 600, 792, 1008, 1200 and 1344 MHz**. The actual Linux `scaling_available_frequencies` capture is **480, 720, 936, 1008, 1104, 1200, 1320, 1416 and 1512 MHz**. These sets do not match. This is material evidence that the pinned same-version source cannot be treated as the exact running cpufreq/OPP implementation (or that another firmware/runtime mechanism changes the table); it is not a basis for inferring the live mask.

Do not use either the mainline diagnostic candidate bin 5 or the pinned vendor fallback mask 0x4 as the RG34XX OPP authorization. The active runtime mask remains unknown pending exact kernel-source/build identification and a reconciliation of the live OPP table with runtime frequencies.

### 1.6 Thermal telemetry is not safe for automatic DVFS

The kernel log explicitly reports:

    sunxi-thermal 5070400.thermal_sensor: avcc is not calibrated

The SID thermal calibration bytes exist and decode into plausible 12-bit fields, but the thermal driver did not accept AVCC calibration. Current thermal-zone readings (CPU ~45.5 °C, GPU ~46.4 °C) are diagnostic only. CPU-zone trips are 60/70/115 °C; other zones do not expose trip metadata in this capture.

Therefore calibration_status=not_certified. Do not use these temperatures for an automatic frequency controller, thermal cap certification, or Boost enablement.

## 2. Memory ownership observations relevant to the broader port

The capture reports one 1 GiB memory range [0x40000000, 0x80000000), 46 reserved ranges, and the following noteworthy regions:

- 0x48000000–0x49000000: 16 MiB monitor/BL31 candidate reservation.
- 0x77c00000–0x7bc00000: 64 MiB CMA reservation from cma=64M.
- 0x7ec00000–0x7fc00000: 16 MiB un-attributed reservation; keep excluded.
- Framebuffer at 0x7bf24800, 720×480×32bpp, stride 2880, 1,382,400 bytes, end-exclusive 0x7c076000.

The live FDT has no reserved-memory node; CMA debugfs is absent on this Linux 4.9 build, although meminfo reports CmaTotal=64 MiB. The memory map is an observed Linux runtime snapshot, not authorization to allocate these regions to LK. The un-attributed 16 MiB region and all secure/monitor/DMA ownership questions remain blockers for the BL33 memory gate.

## 3. What this closes and what it does not

| Item | Result |
|---|---|
| CPU0 cpu-supply phandle | **Confirmed:** AXP2202 DCDC1 in captured live FDT |
| CPU0–CPU3 Linux cpufreq grouping | **Confirmed:** one shared policy, all four online in this Linux snapshot |
| Current Linux CPU clock parent | **Confirmed at Linux snapshot:** pll_cpu → cpu; pll_periph0 is separate |
| pll_periph0 Linux rate | **Observed:** 600 MHz, 4/4 prepare/enable users |
| Physical CPU rate | **Unresolved:** cpufreq and clk_summary values diverge |
| SID speed raw value | **Captured:** 0x6c00; mainline candidate bin 5 only |
| Vendor speed-bin interpretation / OPP mask | **Unresolved** |
| DCDC1 runtime framework observation | **Captured:** enabled / 1160 mV / cpu0 consumer in this sample |
| Raw PMIC voltage register at BL33 entry | **Not captured** |
| Thermal calibration | **Not certified:** AVCC calibration rejected by kernel |
| CPU1–CPU3 state at BL31→BL33 | **Unresolved** |
| Timer continuity during PLL relock | **Unresolved** |
| Approved RG34XX OPP grid | **None** |
| DVFS register writes | **Not implemented / not authorized** |

## 4. Next actions

1. Reconcile the exact shipping Linux 4.9.170 cpufreq/OPP implementation with the live OPP table and raw 0x6c00; do not borrow the mainline bin label or the vendor's unsafe 2-byte-to-u32* consumer as authorization.
2. Determine why scaling_cur_freq/cpuinfo_cur_freq diverge from clk_summary pll_cpu; collect a synchronized, repeated read sequence with timestamps and identify the implementation behind each sysfs field.
3. Keep the shared CPUX PLL write path disabled until BL31→BL33 CPU1–CPU3 state, single-core handoff, pll_periph0 fallback behavior and timer continuity are proven.
4. Resolve why the thermal driver rejects AVCC calibration or establish a separately verified sensor path before implementing closed-loop thermal policy.
5. Keep the runtime C service POLICY_ONLY, hardware_applied=0, and the Linkslot CPU Performance row display-only/unavailable.

## 5. Source report files

- [Inventory README and conclusions](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-inventory-20261125-cpux/README.md)
- [Live FDT parser output](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-inventory-20261125-cpux/live-fdt.json)
- [CPU / regulator / clock / thermal capture](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-inventory-20261125-cpux/cpu-power-runtime.txt)
- [SID cell decode](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-inventory-20261125-cpux/sid-cell-report.json)
- [Memory map report](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-inventory-20261125-cpux/memory-map-report.json)
- [Filtered hardware boot log](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-inventory-20261125-cpux/dmesg-hardware-lines.txt)
