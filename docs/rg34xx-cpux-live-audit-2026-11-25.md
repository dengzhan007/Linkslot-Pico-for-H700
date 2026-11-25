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

The live FDT declares a shared `/opp_l_table` referenced by all four CPU nodes. The uploaded `live-fdt.json` was generated by the earlier FDT parser, which only emitted the base `opp-microvolt` property. Its reported voltage cells are therefore **base-property observations only**; it did not emit `opp-microvolt-<suffix>` variants or the OPP table's `nvmem-cells` / `nvmem-cell-names`. Do not interpret an empty `voltage_microvolt_cells` array as proof that the raw DT node has no named voltage property. The parser has now been extended to collect those fields, but the report must be regenerated from the device's original live FDT to populate them.

| Declared frequency | opp-supported-hw cell | Declared voltage |
|---:|---:|---:|
| 936 MHz | 0x30 | 900 mV |
| 1008 MHz | 0x01 / 0x02 / 0x0c / 0x30 | 920 / 900 / 1020 / 950 mV |
| 1104 MHz | 0x30 | 1000 mV |
| 1200 MHz | 0x01 / 0x02 / 0x0c / 0x30 | 980 / 960 / 1100 / 1050 mV |
| 1320 MHz | 0x30 | 1100 mV |
| 1416 MHz | 0x10 / 0x20 | 1150 / 1160 mV |
| 1512 MHz | 0x01 | 1120 mV |

These are static live-DT declarations, not a validated runtime OPP grid. The parser-v1 JSON is incomplete for nodes that use named properties such as `opp-microvolt-a1`, `opp-microvolt-b0` or `opp-microvolt-c1`; a blank base-property cell in that JSON is not sufficient to conclude that no voltage declaration exists. The kernel log records `_opp_supported_by_regulators` rejecting some OPP entries at 600, 792, 1032, 1296 and 1344 MHz with `minuV=0 / maxuV=0`, but the parser-v1 output cannot map those log lines to every variant node. Runtime `scaling_available_frequencies` remains the authoritative list exported by this Linux instance; the OPP core's selected speed-bin mask and voltage-property suffix are not proven by this inventory.

The bounded SID capture reports:

- speed cell bytes 00 6c, little-endian raw 0x6c00;
- thermal cell bytes 18 11 13 ab 19 1b 16 bb;
- four decoded 12-bit thermal fields: 2835 / 2841 / 2838 / 2842; factory-test temperature raw 280 (28.0 °C).

The pinned mainline reference classifier labels `0x6c00` as candidate bin 5. That is a diagnostic classifier only. Subsequent source review found multiple vendor cpufreq implementations that disagree on the same raw value: one Linux 4.9.170 candidate has no explicit `0x6c00` case and falls back to `supported_hw=0x4`; another Linux 4.9.170 candidate explicitly maps it to `supported_hw=0x1`; a Linux 4.9.191 candidate also maps it to bit 0 but uses a length-checked byte-wise NVMEM reader. The raw value therefore does **not** have one universally established vendor meaning. The generic `/sys/devices/soc0` identity is unavailable, and the SID report explicitly leaves `opp_authorization=not_approved`. Neither mainline candidate bin 5 nor either vendor mask is an RG34XX OPP authorization.

The 1416 MHz / 1160 mV runtime observation is numerically consistent with the DT 1416 MHz 0x20 / 1160 mV variant, but the cpufreq-vs-clk_summary divergence and unresolved vendor speed-bin semantics make this a correlation only—not proof that bit 5 was selected or that the pair is safe.

### 1.5 cpufreq/OPP source variants and parser-v1 limitations — none is proven to be the running binary

The device reports Linux `4.9.170`. The running kernel has no captured build ID or image hash that maps it to a specific public source commit. Several candidate trees have been checked, and they do **not** agree on how raw SID `0x6c00` is classified.

| Candidate source | Kernel version / source | `0x6c00` behavior | Safety / applicability |
|---|---|---|---|
| [qiaoweibiao/T507_Kernel@98fcc7d3f112e51b0edfb71536da89cb2115106f](https://github.com/qiaoweibiao/T507_Kernel/tree/98fcc7d3f112e51b0edfb71536da89cb2115106f) | 4.9.170; driver blob `e121a4ddf28adb55e0b6bf78b450bd783b8b27bc` | No explicit case; takes default `supported_hw=0x4` | Reads a two-byte NVMEM cell through `u32 *` and ignores `len`; unsafe candidate |
| [lineage-on-allwinner/android_kernel_allwinner_4.9@ce0c8395b269a6748a10f4917a0e1d06bd4978d0](https://github.com/lineage-on-allwinner/android_kernel_allwinner_4.9/tree/ce0c8395b269a6748a10f4917a0e1d06bd4978d0) | 4.9.170; driver blob `9adc22b6782589b80e58ed86e5f97ff231cfda59` | Explicitly maps `0x6c00` to `supported_hw=0x1` | Also reads the two-byte cell as `u32 *` without validating `len`; unsafe candidate |
| [CrealityTech/sonic_pad_os@7f37a7fbf4ad59907034715e3d967b9177de1cd4](https://github.com/CrealityTech/sonic_pad_os/tree/7f37a7fbf4ad59907034715e3d967b9177de1cd4) | 4.9.191; driver blob `b524aa53e1bc047624d0bf62ce8504f959095fe7` | Explicitly maps `0x6c00` to `supported_hw=0x1` | Uses a bounded `u8 *` NVMEM read, checks the returned length and assembles the value byte-wise; not the reported kernel version |
| [Upstream Linux reference decoder](https://github.com/torvalds/linux/blob/8d7fe9885c62fc9a95bf55d9db244d9672a8aadc/drivers/cpufreq/sun50i-cpufreq-nvmem.c) | Mainline reference, not the target's build | Diagnostic candidate bin 5 | Not an OPP authorization or target-specific decode |

The two 4.9.170 sources have the same kernel release number but different source behavior. This proves that `4.9.170` alone is not enough to identify the SID mapping. The two 4.9.170 candidates also share a potential out-of-bounds read: their NVMEM provider returns a two-byte cell allocation while the cpufreq consumer dereferences it as a 32-bit value without checking `len`. Do not copy either implementation or rely on its raw-mask result.

#### Named OPP voltage properties

A separate RG34XX-H device-tree reference, [MustardOS `device/rg34xx-h/package/sunxi.dts`](https://github.com/MustardOS/internal/blob/aef38ed9a870419a0fe4f351dac1bac58a84ba5e/device/rg34xx-h/package/sunxi.dts), contains an `opp_l_table` with table-level `nvmem-cells` / `nvmem-cell-names = "speed"`, as well as OPP nodes with named voltage properties such as `opp-microvolt-a1`, `opp-microvolt-b0`, `opp-microvolt-c0/c1/c2`. Its pinned file blob is `64cad2f59b9596dfe6d31e0b52d4f682b1081b4c`. This is a board-family reference, not proof that every named property is present in the exact uploaded live FDT.

The OPP core source in the 4.9.170 candidate supports `dev_pm_opp_set_prop_name()`; when a property name is configured, it looks first for `opp-microvolt-<name>` and falls back to base `opp-microvolt`. The 4.9.191 Creality candidate demonstrates a driver path that chooses `a0/a1/a2` or `b0/b1/b2` voltage-property suffixes for particular speed-cell groups using a separate `Icpu` cell. In that candidate, raw `0x6c00` takes the version-mask path instead of the named suffix path. This illustrates why both the raw speed cell and any companion NVMEM inputs / driver implementation matter.

**Parser v1 limitation:** the uploaded `live-fdt.json` was produced before the parser captured `opp-microvolt-<suffix>` properties and the OPP table's own NVMEM references. Its `voltage_microvolt_cells` field represents only the base property. An empty array is not evidence that an OPP has no voltage declaration. The project parser and synthetic tests have now been extended to emit:
- `voltage_microvolt_variants` keyed by suffix;
- OPP table-level `nvmem_cells` and `nvmem_cell_names`.

The changes are on the current porting branch, but the uploaded report must be regenerated from the original live FDT to populate them. The original `live-fdt.dtb` was not uploaded with the report, so parser-v2 cannot reconstruct the missing properties from the old JSON alone.

#### Why the source-to-frequency comparison is not an OPP conclusion

For the T507 candidate only, applying its default `supported_hw=0x4` to the **base voltage properties visible in parser-v1 output** yields a preliminary set of 480/600/792/1008/1200/1344 MHz. The Lineage 4.9.170 candidate's `supported_hw=0x1` yields a different preliminary set from those same base properties. Neither calculation includes named-voltage variants or proves that the candidate source corresponds to the running kernel. Neither reproduces the captured `scaling_available_frequencies` list.

The 1008 MHz / 950 mV, 1104 MHz / 1000 mV and 1416 MHz / 1160 mV Linux regulator observations are consistent with some base DT entries, but this is only correlation. Do not derive an active mask by combining those voltage samples with an unverified source branch.

The active runtime mask and OPP property suffix remain unknown pending all of the following: exact running kernel source/build identification, a parser-v2 live FDT report, table-level NVMEM cell identification and a reconciliation of the result with the runtime cpufreq table.

### 1.6 Thermal telemetry is not safe for automatic DVFS

The kernel log explicitly reports:

    sunxi-thermal 5070400.thermal_sensor: avcc is not calibrated

The SID thermal calibration bytes exist and decode into plausible 12-bit fields, but the thermal driver did not accept AVCC calibration. Current thermal-zone readings (CPU ~45.5 °C, GPU ~46.4 °C) are diagnostic only. CPU-zone trips are 60/70/115 °C; other zones do not expose trip metadata in this capture.

Therefore calibration_status=not_certified. Do not use these temperatures for an automatic frequency controller, thermal cap certification, or Boost enablement.

### 1.7 Parser-v2 live FDT and synchronized sampler result

The follow-up capture at **2026-11-25 00:41:21 UTC** is archived in [reports/rg34xx-inventory-20261125-cpux-v2](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-inventory-20261125-cpux-v2). Its original `live-fdt.dtb` hash is identical to v1; parser output is now complete for named voltage variants and OPP-table NVMEM references.

- `/opp_l_table` has `nvmem-cells = <&speed>` and `nvmem-cell-names = "speed"`; parser-v2 emits `opp-microvolt-a1/b0/b1/c0/c1/c2` properties.
- The boot log drops 600/792/1032/1296/1344 MHz because regulator lookup returns `minuV=0 maxuV=0`. The Linux frequency list remains 480/720/936/1008/1104/1200/1320/1416/1512 MHz. This identifies an unavailable voltage-property lookup; it does not identify the correct SID mask or authorize the dropped OPPs.
- Eight additional samples at 150 ms spacing show `scaling_cur_freq=480 MHz` continuously for at least 1.1 seconds, while `cpuinfo_cur_freq` reports 1008/1104 MHz, `pll_cpu` stays at 1008/1104 MHz and DCDC1 reports 950/1000 mV. The capture cannot distinguish a policy/governor constraint from a clock-driver rejection. This is not evidence of the CPU physically running at 480 MHz.
- CPU0–CPU3 remain in one Linux shared policy; DCDC1 is the CPU supply. AVCC thermal calibration is still rejected by the kernel.

The capture remains read-only and does not authorize any DVFS, PMIC or clock register write. The original raw FDT binary stays on the acquisition PC; only parser-v2 JSON was uploaded.


### 1.8 The old 480 MHz cached-frequency observation requires reinterpretation

A follow-up source review of the pinned public candidate
[qiaoweibiao/T507_Kernel@98fcc7d3f112e51b0edfb71536da89cb2115106f](https://github.com/qiaoweibiao/T507_Kernel/tree/98fcc7d3f112e51b0edfb71536da89cb2115106f)
shows that Linux 4.9's `cpuinfo_cur_freq` read can alter the software cache
that `scaling_cur_freq` reports:

- The candidate `cpufreq-dt` driver uses `.target_index=set_target` and
  `.get=cpufreq_generic_get`; set_target calls `dev_pm_opp_set_rate()`, while
  the getter returns `clk_get_rate(policy->clk)`.
- For this target-based driver, cpufreq core's `show_scaling_cur_freq()`
  returns cached `policy->cur`; it does not call the getter.
- `show_cpuinfo_cur_freq()` calls `__cpufreq_get()`. When the hardware/clock
  getter returns a non-zero value different from `policy->cur`, that core
  path calls `cpufreq_out_of_sync()`, updates `policy->cur` from the getter
  result and schedules a policy update.

Thus the v2 ordered read `scaling_cur_freq` → `cpuinfo_cur_freq` may read a
stale cached value first and then repair that cache with the getter. Since the
v2 collector did not re-read `scaling_cur_freq` after the getter, the
reported 480 MHz cached reading does **not** establish a persistent PLL or
clock-provider fault. This is source-level behavior in a candidate 4.9.170
tree; the exact shipping kernel binary has not yet been matched to it.

The same v2 capture gives CPU thermal zone0 temperature 46.637°C and a first
passive trip at 60°C. The pinned candidate `power_allocator_throttle()`
returns to non-passive / maximum-power mode below its switch-on trip. This
reduces the likelihood that CPU-zone power_allocator was throttling at that
sample, but no cooling-device state or zone binding was captured, and the
candidate source is not proven to be the target binary. No conclusion is made
about other cooling devices or limits.

The latest read-only collector update captures cpufreq path identity, the
`cpuinfo_cur_freq` getter's before/after effect on cached `scaling_cur_freq`,
thermal cooling-device `cur_state/max_state` and zone-to-device bindings,
plus DCDC1/CPUX clock samples. A fresh device capture is required.

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
| Synchronized instantaneous CPU rate | **Not yet captured:** cpufreq and clk_summary were read at different times; `cpuinfo_cur_freq` uses a clock-framework getter in the pinned cpufreq-dt candidate, while `scaling_cur_freq` is cached policy state |
| SID speed raw value | **Captured:** 0x6c00; candidate classifications differ by vendor source tree |
| Vendor speed-bin interpretation / OPP mask / voltage suffix | **Unresolved:** 4.9.170 candidates disagree; parser-v1 missed named voltage and table-NVMEM metadata |
| DCDC1 runtime framework observation | **Captured:** enabled / 1160 mV / cpu0 consumer in this sample |
| Raw PMIC voltage register at BL33 entry | **Not captured** |
| Thermal calibration | **Not certified:** AVCC calibration rejected by kernel |
| CPU1–CPU3 state at BL31→BL33 | **Unresolved** |
| Timer continuity during PLL relock | **Unresolved** |
| Approved RG34XX OPP grid | **None** |
| DVFS register writes | **Not implemented / not authorized** |

## 4. Next actions

1. Identify the exact running Linux 4.9.170 cpufreq source/build and reconcile its SID mask / voltage-property suffix with the now-complete parser-v2 live FDT and runtime frequency list. Do not borrow any candidate mapping as authorization.
2. Re-run the updated read-only collector. Check whether reading `cpuinfo_cur_freq` reconciles cached `scaling_cur_freq`, resolve `policy0`/per-CPU cpufreq path identities, and inspect active thermal cooling-device state/bindings before considering any policy explanation.
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

### 5.1 Parser-v2 follow-up report

- [Inventory v2 README](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-inventory-20261125-cpux-v2/README.md)
- [Parser-v2 live FDT](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-inventory-20261125-cpux-v2/live-fdt.json)
- [Synchronized CPU/regulator/clock sampler](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-inventory-20261125-cpux-v2/cpu-power-runtime.txt)
- [SID decode](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-inventory-20261125-cpux-v2/sid-cell-report.json)

### 5.2 cpufreq getter and cooling-device follow-up

The updated collector and safety tests are in commit
[`b421b077e5f27c95bf61679762b70a86d66131cb`](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/b421b077e5f27c95bf61679762b70a86d66131cb).
The H700 AArch32/AArch64 compile/regression workflow
[#45526953772](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45526953772)
and boot audit tools workflow
[#45526953774](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45526953774)
passed. Both capture additions are read-only; they do not write cpufreq,
thermal, regulator, clock or NVMEM sysfs controls.


### 5.4 OPP candidate analyzer and CI status

The read-only analyzer and regression tests were added to `tools/analyze_h700_opp_candidates.py` and `tools/test_analyze_h700_opp_candidates.py`. It models absent/empty `opp-supported-hw`, one-cell mask intersection and selected voltage suffix/base fallback, and compares candidate OPP frequency sets with the actual runtime list.

The first CI attempts exposed a malformed regex, then a malformed test method block. Both were corrected. At commit [`60d82ee21832f7f701c341eb4de72fae73e5a797`](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/60d82ee21832f7f701c341eb4de72fae73e5a797), H700 compile/host regression [#45531974987](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45531974987) and Linux/Windows boot audit tools [#45531974867](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45531974867) are green. Device parser-v3 output is still required before running the analyzer against real OPP nodes.