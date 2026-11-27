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

**Parser-v3 superseding result (2026-11-25 03:19:48 UTC):** The [v3 report](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-inventory-20261125-cpux-v3) now includes OPP property-presence, all named voltage variants and the table `speed` NVMEM reference. Across 38 OPP entries and five identical runtime frequency lists, the corrected analyzer finds one exact structural model: **no table-level `supported_hw` array + `b1` voltage suffix**. All 14 `-ab` nodes have `supported_hw_present=false`; all nine runtime frequencies (480/720/936/1008/1104/1200/1320/1416/1512 MHz) have `b1` variants, while the five boot-rejected rates (600/792/1032/1296/1344 MHz) do not. No one-cell table mask yields an exact witness.

The nine `b1` voltages are 900/900/900/950/1000/1050/1100/1150/1160 mV. Linux clock / regulator-framework samples at 1416/1150, 1104/1000 and 1200/1050 MHz/mV match these declarations. This is a unique table/runtime structural fingerprint, not proof of the shipping binary's source path or a mapping of SID `0x6c00` to `b1`; it also does not simulate the regulator selector table. **No OPP is approved.**

The following older parser-v1 paragraphs are historical context; their claim that named voltage properties are missing is superseded by the v3 report.

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

### 1.5 cpufreq/OPP source variants and v4 running-kernel provenance

The device reports Linux `4.9.170`. The live kernel Build ID is now matched exactly to the decompressed kernel inside the stock `mmcblk0p4` Android boot image (see §1.11), so the binary provenance gate is closed. The matching binary has also been reconstructed as an AArch64 ELF with 89,675 recovered symbols. No public source commit has been matched to this image. Public 4.9.170 candidate trees still disagree on raw SID `0x6c00`; their behavior must not be substituted for the recovered target binary.

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

v3's analyzer identifies a unique **static DT/runtime structural model**: no table-level `supported_hw` mask plus the `b1` voltage suffix reproduces the nine-rate list. This is a hypothesis about OPP-table filtering, not proof of the running driver's branch-specific behavior. The recovered probe has call sites for both `dev_pm_opp_set_prop_name()` and `dev_pm_opp_set_supported_hw()`, but its local mask slot is initialized to zero before speed-bin decoding. Call-site presence alone does not prove the mask setter executes on the `b`-prefix branch; that branch may leave the slot zero and skip the setter. Decode branch-to-call reachability before accepting or rejecting no-mask + `b1`. Do not infer `0x6c00 → b1` from the 2-byte SID capture alone.

#### Public source cross-check after parser-v3

The corrected v3 static-table model was compared against additional public `sun50iw9` cpufreq-NVMEM implementations. A closer **driver-family candidate** was found in AvaotaSBC's Allwinner BSP: it has the same SID offsets `0x20/0x28`, the same `pvalue`/`icpu` transforms, the same SoC-version bits, and both named-property and supported-hardware OPP setters. However, its public `sun50iw9` speed-bin switch, property-name outputs and length-safe NVMEM read still differ from the recovered target binary; it is not a source match. The earlier **no table mask + `b1`** result remains a static model only, not a confirmed target state:

| Public source candidate | NVMEM inputs and `0x6c00` path | Why it does not match the recovered target binary |
|---|---|---|
| [AvaotaSBC/linux, Allwinner BSP](https://github.com/AvaotaSBC/linux/blob/7c2a33957cb82e522885992ec26331d1c1c6db28/bsp/drivers/cpufreq/sun50i-cpufreq-nvmem.c#L134-L190) | Strong structural-family match: SoC bits `sunxi_get_soc_ver() & 7`; SID offsets `0x20` / `0x28`; first value `& 0xff` then `*32`; second value `& 0x3ff`; named-property plus one-cell supported-hardware setters in probe | Its public `sun50iw9` switch emits only `aN` names and `1 << value` masks with a different speed-code set; its NVMEM helper checks `len` and assembles bytes safely, unlike the target's 32-bit load from a 2-byte cell. Strong lineage candidate, not exact source |
| [JasonYANG170/Mini-LinuxPC-Pro, Linux 5.4](https://github.com/JasonYANG170/Mini-LinuxPC-Pro/blob/cabc0f177920738c41973655cfd73b1c205c5545/kernel/linux-5.4/drivers/cpufreq/sun50i-cpufreq-nvmem.c#L98-L145) | Same `PVALUE_OFFSET=0x20`, `ICPU_OFFSET=0x28`, SoC-version bits, `pvalue` and `icpu` transforms in `sun50iw9_nvmem_xlate` | Its public switch maps a different set of speed codes and emits `aN` only; Linux 5.4 source is not proven to build the 4.9.170 target |
| [qiaoweibiao/T507_Kernel, Linux 4.9.170](https://github.com/qiaoweibiao/T507_Kernel/blob/98fcc7d3f112e51b0edfb71536da89cb2115106f/drivers/cpufreq/sun50i-cpufreq-nvmem.c#L42-L110) | Reads only `speed`; no explicit `0x6c00` case, so its switch default sets `supported_hw=0x4`; it does not call `dev_pm_opp_set_prop_name()` | v3 shows no table-level supported-hw match and `b1` is required |
| [lineage-on-allwinner/android_kernel_allwinner_4.9, Linux 4.9.170](https://github.com/lineage-on-allwinner/android_kernel_allwinner_4.9/blob/ce0c8395b269a6748a10f4917a0e1d06bd4978d0/drivers/cpufreq/sun50i-cpufreq-nvmem.c#L42-L120) | Reads only `speed`; explicitly maps `0x6c00` to `supported_hw=0x1`; no named-property setter in this source | v3's exact no-mask + `b1` model is different |
| [CrealityTech/sonic_pad_os, Linux 4.9.191](https://github.com/CrealityTech/sonic_pad_os/blob/7f37a7fbf4ad59907034715e3d967b9177de1cd4/lichee/linux-4.9/drivers/cpufreq/sun50i-cpufreq-nvmem.c#L101-L143) | The `sun50iw9` path requires both `speed` and `Icpu`; `0x6c00` sets mask 1. The `b1` name is generated only for other speed-code branches (`0x2c00/0x7c00`) with Icpu data | v4 live OPP table exposes only `speed`, not `Icpu`; this source's probe would need the missing cell and its `0x6c00` path is not `b1` |
| [Radxa allwinner-bsp `sun50iw9` variant](https://github.com/radxa/allwinner-bsp/blob/2045a3ca2a01f088c0314dc924bda59d154e363e/drivers/cpufreq/sun50i-cpufreq-nvmem.c#L146-L205) | Its speed-only `sun50iw9` data path falls through to property `a0` plus a one-cell supported-hw mask for `0x6c00` | It sets a property name and a hardware mask; it does not produce no-mask + `b1` |

The Creality device tree also declares both `speed` and `Icpu` at the OPP table ([DTS lines 139–143](https://github.com/CrealityTech/sonic_pad_os/blob/7f37a7fbf4ad59907034715e3d967b9177de1cd4/lichee/linux-4.9/arch/arm64/boot/dts/sunxi/sun50iw9p1.dtsi#L139-L143)), unlike the uploaded RG34XX live FDT's single `speed` cell. The SID offsets `0x20/0x28` in the Avaota/JasonYANG family code are separate direct SID-module reads, not extra `Icpu` NVMEM cells. The target binary's two direct SID calls therefore fit this family more closely than the previous public-source candidates. Yet the target's observed branch constants/output strings and NVMEM read width still differ, so the source repository remains unidentified. Do not infer the final mask or active OPP set from a family-compatible DTS alone.

### 1.6 Thermal telemetry is not safe for automatic DVFS

The kernel log explicitly reports:

    sunxi-thermal 5070400.thermal_sensor: avcc is not calibrated

The SID thermal calibration bytes exist and decode into plausible 12-bit fields, but the thermal driver did not accept AVCC calibration. v3 reports CPU 47.69 °C, GPU 47.85 °C, VE/DDR 47.61 °C and battery 30.0 °C. CPU-zone trips are 60/70/115 °C. The captured cooling devices are `thermal-cpufreq-0` state 0/8 and `thermal-devfreq-0` state 0/5, both bound to CPU thermal zone0; this snapshot does not show active cooling-state throttling. AVCC calibration remains rejected, so these temperatures are unsuitable for closed-loop DVFS.

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

### 1.9 Parser-v3 cpufreq getter reconciliation and cooling-device evidence

The v3 read-only capture at 2026-11-25 03:19:48 UTC is archived at [reports/rg34xx-inventory-20261125-cpux-v3](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-inventory-20261125-cpux-v3). It adds eight ordered reads of `scaling_cur_freq`, `cpuinfo_cur_freq`, a second getter, CPUX `clk_summary`, DCDC1 framework voltage and uptime.

- Sample 2: `scaling_cur_freq` starts at 1008 MHz; `cpuinfo_cur_freq` returns 1104 MHz; the following `scaling_cur_freq` reads return 1104 MHz. The same sample's `pll_cpu` is 1104 MHz and DCDC1 framework readback is 1000 mV.
- Sample 4: the cpufreq getter reads 1104 MHz, but the later clock tree reports 1200 MHz and DCDC1 1050 mV. The cross-attribute reads are non-atomic.
- Samples 5/6 also show cached scaling moving from 1008 to 1104 MHz only after the later getter/read sequence; samples 7/8 repeat the 1008 → 1104 MHz reconciliation.

This v3 evidence supersedes the v2 interpretation of a continuous 480 MHz cached-frequency window. The v2 value was a stale/non-atomic cpufreq sysfs observation; v3 does not support a persistent 480 MHz clamp or an actual clock-driver rejection. It does not identify the exact running binary or make all reads atomic.

CPU zone0 is 47.69 °C with passive trips at 60/70 °C and critical at 115 °C. `cooling_device0` (`thermal-cpufreq-0`) is at 0/8 and `cooling_device1` (`thermal-devfreq-0`) is at 0/5; both are bound to CPU thermal zone0. This capture does not show active CPU cooling-state throttling. AVCC remains uncalibrated.

The five boot-chain captures (Boot0 primary/secondary, TOC1, env partition and original live FDT) match the v1/v2 capture hashes. Memory-map findings are unchanged: 1 GiB RAM, 46 reserved ranges, 64 MiB CMA, framebuffer `[0x7bf24800,0x7c076000)`, and un-attributed `[0x7ec00000,0x7fc00000)` remains excluded.


### 1.10 Parser-v4 kernel provenance and live OPP corroboration

The v4 read-only capture at 2026-11-25 04:05:51 UTC is archived at [reports/rg34xx-inventory-20261125-cpux-v4](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-inventory-20261125-cpux-v4). It closes the missing build-identity / binding evidence gap, but it does not identify a public source commit:

- Running kernel: Linux `4.9.170 #19 SMP PREEMPT`, built 2026-05-26 13:58:08 CST with Linaro GCC 5.3.1 (2016.05); build host string is `flower@flower-B85M-D2V`.
- `/sys/kernel/notes` begins with a GNU build-id note (namesz=4, descsz=20, type=GNU): **`a1d211f847a5fb69fe9898e15fbccdd3ea8d7b68`**. The SHA-256 of the complete notes file is `6065c1a7ae894047f0e902551811b39d0c5a11b5afd894083eae46497d56e12d`. Exact Build ID / build-host searches did not surface a public matching source or image in the repositories checked; this is a fingerprint, not a source mapping.
- The kernel config confirms `CONFIG_CPUFREQ_DT=y`, `CONFIG_CPUFREQ_DT_PLATDEV=y`, `CONFIG_PM_OPP=y`, `CONFIG_NVMEM=y`, `CONFIG_NVMEM_AXP=y` and `CONFIG_NVMEM_SUN50I_SID=y`. Filtered kallsyms contains `sun50i_cpufreq_nvmem_probe`, `sun50i_cpufreq_init`, `sun50i_cpufreq_match_list`, `dev_pm_opp_set_prop_name` and `dev_pm_opp_set_supported_hw`; no matching loadable modules are listed, consistent with the relevant driver being built in.
- Both `/sys/bus/platform/devices/sun50i-cpufreq-nvmem` and `cpufreq-dt` expose their corresponding `driver` symlink and `DRIVER=` uevent. This confirms the platform devices are bound, and is stronger than seeing symbols or a device-tree node alone. It still does not reveal the driver's selected property string or its SID decoder.
- The v4 live FDT continues to declare a single table NVMEM input, `nvmem-cell-names = "speed"`, with raw SID `0x6c00`. All five runtime cpufreq lists still agree on exactly nine rates: 480/720/936/1008/1104/1200/1320/1416/1512 MHz. These are exactly the rates whose `-ab` OPP entries contain a `b1` voltage; their declared `b1` values are 900/900/900/950/1000/1050/1100/1150/1160 mV. The five excluded rates (600/792/1032/1296/1344 MHz) still have no matching `b1` variant. This is now a strong runtime-to-`b1` fingerprint, but it is not direct proof that the source calls `dev_pm_opp_set_prop_name("b1")`, nor that SID `0x6c00` is the rule used to select it.
- Six new getter/clock samples preserve the non-atomic cpufreq-cache behavior. For example, sample 2 goes from cached 1008 MHz to getter/PLL 1104 MHz with DCDC1 framework readback 1000 mV; sample 1 moves from cached/getter 1200 MHz to a later 1320 MHz read with 1100 mV. These voltage/frequency pairs agree numerically with the corresponding `b1` entries. They remain Linux clock/regulator-framework observations, not direct PMIC selector or CCU register reads at BL33 entry.
- CPU thermal zone0 is 51.74 °C in this capture; passive trips remain 60/70 °C and critical 115 °C. The kernel still reports `avcc is not calibrated`; the raw thermal SID bytes do not certify sensor calibration.

The public Creality 4.9.191 `sun50iw9` implementation remains excluded: it requires `Icpu` as well as `speed`, while the live table exposes only `speed`. The complete target CFG now establishes the actual `a/b/c` property-name branches and mask-only branches, including the fact that b0/b1 skips the supported-hardware setter. Avaota/JasonYANG remain the closest public family references, but neither matches the target's constants, names and control flow. The exact source repository/commit is still unidentified.


**Result:** the running cpufreq/OPP path and its exact shipping kernel binary are identified. The complete probe CFG confirms that the `0x6c00/0x7c00` b-prefix branch leaves the mask zero and skips `dev_pm_opp_set_supported_hw`; the target OPP core then drops every `opp-supported-hw` node when no table mask is registered. The parsed v4 live FDT has b1 voltage variants on exactly the nine runtime rates and no b1 variant on the five rejected rates. Therefore **no-mask + b1 is the high-confidence software selection model**. The remaining live-value uncertainty is whether the 32-bit operand read from the two-byte cell equals a b-group constant and whether `sunxi_get_soc_ver() & 7` is nonzero (selecting b1 rather than b0). Keep `opp_authorization=not_approved`, `POLICY_ONLY`, `hardware_applied=0`, and all DVFS writes disabled.

### 1.11 Stock boot partition is now matched to the running kernel

The user exported `/dev/mmcblk0p4` read-only and uploaded [rg34xx-boot-p4-metadata.json](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-boot-p4-metadata.json). This is the 64 MiB Android boot partition (`ANDROID!` magic), with the kernel payload beginning at the 2048-byte page boundary:

- Partition size: `67,108,864` bytes; SHA-256: `41cfaa8f9ade4a5e05a89188abb7db154243e38b8e19c53b730f0a45ff325516`.
- Android boot header: `kernel_size=7,018,071` bytes, `page_size=2,048` bytes. The gzip-compressed kernel payload SHA-256 is `cc4b5769cb8c9dcf8d013602e57c494a1153eb6714d9786cf847e39e3ecdd3af`.
- The decompressed kernel contains GNU Build ID `a1d211f847a5fb69fe9898e15fbccdd3ea8d7b68`, an exact byte-for-byte identifier match to the running kernel's `/sys/kernel/notes` value from v4. This closes the **live kernel ↔ stock p4 kernel image identity** gate. It does not identify the source repository or explain the OPP property setter.
- An aligned ELF-note scan locates GNU Build ID `fe7e68cd7aa14945d71f36e4fbf8412d959f4cc1` at decompressed-kernel offset `0x9082b8`, and the live-kernel ID `a1d211f847a5fb69fe9898e15fbccdd3ea8d7b68` at `0xc8fc50`. The first ID's owning image/section is still not attributed; do not assign it to a component without further ELF-note context.

A useful independent cross-check exists in [pvaibhav/BaseOS `all-targets-survey.jsonl`](https://github.com/pvaibhav/BaseOS/blob/63137afeda1c0bc4ff771be6f03012b7f8147aca/experiments/h700-boot-speed/evidence/all-targets-survey.jsonl). The newly uploaded verified report records decompressed kernel length `17,461,256` bytes and SHA-256 `39cf8c3eb9a75ff0eda51b4074cd65860c5aa5386093bcaf69339819421d84b7`; its first 16 hex characters match the survey's published prefix `39cf8c3eb9a75ff0`. This is an independent 64-bit-prefix corroboration; the exact live-kernel ↔ p4 identity proof remains the full GNU Build ID match above.

The `vmlinux-to-elf` reconstruction has now succeeded. The local AArch64 `ET_EXEC` ELF recovers 89,675 symbols at link base `0xffffff8008080000`; the `.kernel` section is validly mapped and the target probe / OPP helper entry points decode as legal AArch64 instructions. The bounded disassembly is archived in [rg34xx-cpufreq-driver-disasm.txt](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-cpufreq-driver-disasm.txt). The next offline step is no longer ELF recovery: it is a full basic-block/control-flow decode of the entire probe plus the exact target OPP-core support and named-voltage parsers. Keep the boot image and reconstructed ELF local.

**Interpretation boundary (first-pass excerpt; superseded by §1.14):** the Build ID match identifies the shipping binary, not the source repository. Full CFG and OPP-core results now establish exact branch mapping, b0/b1 mask=0, setter guards and no-mask+b1 filtering. The actual 32-bit NVMEM comparison operand, live SoC revision bits and hardware safety gates remain open. `opp_authorization=not_approved`, `POLICY_ONLY`, `hardware_applied=0`, and every DVFS write remain disabled.

### 1.12 Matched shipping cpufreq binary: first disassembly findings (2026-11-25)

The uploaded verified p4 kernel report and bounded driver disassembly now allow direct inspection of the exact kernel image that produced the live Build ID. This is stronger than comparing public source candidates by kernel version.

- Reconstructed ELF: AArch64 ET_EXEC; 89,675 recovered kallsyms; link base 0xffffff8008080000; code is in .kernel (section 1, sh_addr=0xffffff8008080000, sh_offset=0x240, sh_size=0xd08298). The report validates instruction decoding and in-section branch targets. Target symbols have st_size=0, so function boundaries remain inferred and current excerpts are limited to 60 instructions plus selected call-site scans.
- sun50i_cpufreq_nvmem_probe calls sunxi_get_soc_ver(), calls sunxi_get_module_param_from_sid() twice with keys 0x20 and 0x28, obtains the CPU0 operating-points-v2 table, and requests its NVMEM cell named speed.
- nvmem_cell_read() receives the cell length through an output pointer. The probe then loads the returned buffer with an AArch64 32-bit ldr w1, [x20]. The live FDT defines the speed cell as 2 bytes, and the captured SID bytes are 00 6c (little-endian 0x6c00). The recovered call path does not show a length guard before this 32-bit load. The upper two bytes beyond the declared cell length are unknown, so we cannot yet prove that the actual 32-bit comparison operand equals 0x6c00. The confirmed issue is a 32-bit read beyond the NVMEM cell's declared two-byte length; this alone does not establish a fault or a read outside the allocator's rounded kmalloc slab object. Do not copy this read pattern into the new HAL.
- The probe performs ten explicit speed comparisons (the previous nine-value inventory omitted `0x5000`): `0x2000`, `0x2400`, `0x2800`, `0x2c00`, `0x5000`, `0x5400`, `0x5c00`, `0x6c00`, `0x7400`, `0x7c00`, plus a default branch. The complete map is in §1.14 and `rg34xx-cpufreq-full-cfg.json`. In particular `0x6c00/0x7c00` -> mask 0, prefix `b`, suffix `(sunxi_get_soc_ver() & 7) ? 1 : 0`; the b path skips the mask setter. `0x2000/0x2800` -> mask 0 + `a1`; `0x5c00` -> mask 0 + conditional `c0/c1/c2`; mask-only groups are `0x2400/0x5000/0x5400/0x7400` -> 1, `0x2c00` -> 2, and default -> 8.
- The probe's per-CPU loop calls `dev_pm_opp_set_prop_name()` only when the generated name buffer is nonempty, and calls `dev_pm_opp_set_supported_hw()` only when the local mask is nonzero. Full CFG proves a/b/c branches set only the name; mask branches set only the mask, so the two setters are mutually exclusive in one probe pass. Any setter error goes to cleanup and prevents `cpufreq-dt` registration; the device is registered only after the per-CPU loop completes.
- The complete CFG and OPP-core disassembly now resolve the earlier ambiguity: b0/b1 explicitly skips the setter, and when no mask is registered `_of_add_opp_table_v2` drops OPP nodes with an `opp-supported-hw` property while retaining property-less nodes. `opp_parse_supplies` tries `opp-microvolt-b1` and falls back to base `opp-microvolt` when the named property is absent.

The 2-byte-cell/32-bit-load width mismatch is confirmed in the matched shipping binary. The NVMEM helper allocates the declared cell length (2 bytes) using `GFP_KERNEL|__GFP_ZERO`; the 32-bit `ldr` reads four bytes. The extra bytes fall within the allocator's rounded slab object in the analyzed path, so this does not by itself establish a fault or an access outside that object. Whether the high half is guaranteed zero depends on this kernel's allocator zeroing behaviour; do not infer the actual comparison operand from the two-byte SID capture alone.
### 1.13 New closest public driver-family candidate (2026-11-25)

GitHub code search surfaced [AvaotaSBC/linux at `7c2a33957cb82e522885992ec26331d1c1c6db28`](https://github.com/AvaotaSBC/linux/blob/7c2a33957cb82e522885992ec26331d1c1c6db28/bsp/drivers/cpufreq/sun50i-cpufreq-nvmem.c), plus the closely related [JasonYANG170/Mini-LinuxPC-Pro Linux 5.4 version](https://github.com/JasonYANG170/Mini-LinuxPC-Pro/blob/cabc0f177920738c41973655cfd73b1c205c5545/kernel/linux-5.4/drivers/cpufreq/sun50i-cpufreq-nvmem.c). Their `sun50iw9_nvmem_xlate()` contains the same unusual combination now visible in the matched shipping binary:

- `sunxi_get_soc_ver() & 7` supplies the SoC revision bits.
- `sunxi_get_module_param_from_sid()` reads offset `0x20` into `pvalue`, then masks to 8 bits and multiplies by 32; a second read at offset `0x28` is masked with `0x3ff` into `icpu`.
- The decoded values and NVMEM `speed` cell jointly select the OPP table property name and hardware mask.
- The probe installs both `dev_pm_opp_set_prop_name()` and `dev_pm_opp_set_supported_hw()` and aborts/unwinds on setter failure before creating `cpufreq-dt`.

This is a strong **code-family/lineage fingerprint**, not a source identity match. The public Avaota `sun50iw9` branch emits only `aN` names and masks `1 << value`, with a different speed set (`0x2000`, `0x2100`, `0x2400`, `0x2c00`, `0x5000`, `0x5400`, `0x6000`, `0x5c00`, `0x5d00`). The target full CFG compares ten explicit speed values plus default, emits `a/b/c` conditional names, and uses mutually exclusive name-only branches (mask 0) and mask-only branches `{1,2,8}`. Its b0/b1 path is confirmed to skip the supported-hardware setter. The public helper validates NVMEM cell length and assembles up to four bytes safely; the target loads 32 bits from a cell declared as two bytes. The exact target source commit remains unidentified.

**Source-identification result:** the full basic-block comparison is now complete. The target shares the Avaota/JasonYANG family inputs (`soc_ver&7`, SID offsets `0x20/0x28`, `pvalue`/`icpu` transforms) but uses a different ten-value speed switch, `a/b/c` naming, mask values `{0,1,2,8}`, and its own per-CPU setter/cleanup flow. Treat those public trees as lineage references, not source matches.

### 1.14 Full probe CFG, OPP-core and NVMEM audit (2026-11-25, offline static)

This section closes the open items left by 1.12/1.13 using the same local symbolized ELF only (no device, no `/sys`, no `/dev/mem`, no `sunxi_get_soc_ver` invocation, no re-export of p4). Bounded text/JSON is uploaded to `reports/rg34xx-cpufreq-full-cfg.txt`, `reports/rg34xx-cpufreq-full-cfg.json` and `reports/rg34xx-opp-core-disasm.txt`; the ELF and boot image stay local.

**Function boundary.** `sun50i_cpufreq_nvmem_probe` = `0xffffff8008672ccc` .. `0xffffff8008673148` (exclusive), **287 instructions, 69 strict basic blocks and 106 edges** in the machine-readable CFG. Because `st_size=0`, the end is the `ret` that closes the epilogue at `0xffffff8008673144`, whose successor is a distinct FUNC (`find_deepest_state`) that is not any intra-function branch target. All instructions in `[start,end)` decode and are covered by contiguous basic blocks. The 16-region view in the text report is a human-readable grouping, not the strict CFG block count. This supersedes the earlier 60-instruction excerpt.

**Nine-value switch → mask / name / setter (complete).** The loaded 32-bit `speed` word is compared against exactly nine constants; every branch converges to a masked 0/1/2/8 slot and optionally a generated `"%s%d"` name:

| speed | dispatch | mask | prefix | suffix | set_prop_name | set_supported_hw |
| --- | --- | ---: | --- | --- | --- | --- |
| `0x2000` | `0x…72eb4` | 0 | `a` | 1 (fixed) | yes | no |
| `0x2400` | `0x…72ec8` | 1 | — | — | no | yes |
| `0x2800` | `0x…72eb4` | 0 | `a` | 1 (fixed) | yes | no |
| `0x2c00` | `0x…72e64` | 2 | — | — | no | yes |
| `0x5000` | `0x…72ec8` | 1 | — | — | no | yes |
| `0x5400` | `0x…72ec8` | 1 | — | — | no | yes |
| `0x5c00` | `0x…72ed0` | 0 | `c` | 0/1/2 (cond.) | yes | no |
| `0x6c00` | `0x…72f24` | 0 | `b` | 0/1 (cond.) | yes | no |
| `0x7400` | `0x…72ec8` | 1 | — | — | no | yes |
| `0x7c00` | `0x…72f24` | 0 | `b` | 0/1 (cond.) | yes | no |
| any other | `0x…72f40` | 8 | — | — | no | yes |

- **b0/b1 confirmed mask=0 and skips `set_supported_hw`.** The b-group block at `0x…72f24` contains no `str w0,[x29,#0x74]`; only `0x…72f44` writes the mask slot. The b path reaches `snprintf` and branches to `0x…72f48`, bypassing `0x…72f44`. The guard `0x…72fd8 ldr w1,[x29,#0x74]; cbz w1,0x…73070` therefore always skips `dev_pm_opp_set_supported_hw` on the b-prefix path. So the shipping binary *does* keep mask 0 for b0/b1, which resolves the 1.12 open question in favour of the analyzer's **no-mask + b1** model.
- **a-group suffix is fixed 1** (`0x…72eb8 mov w4,#1`), not conditional. Only the c-group (`0x5c00`) and b-group (`0x6c00`/`0x7c00`) suffix are conditional: b suffix = `(sunxi_get_soc_ver() & 7) ? 1 : 0`; c suffix = 0 when `soc_ver&7==0`, else 1 if `SID[0x28] ∈ [0x26,0x2d]` and `(SID[0x20]<<5) ∈ [0xa60,0x13a0]`, else 2.
- **Zero-guards.** `0x…72fa8 ldrb w1,[x29,#0xd0]; cbz w1,0x…72fd8` skips `set_prop_name` when the name buffer is empty (mask-group paths never `snprintf`). `0x…72fd8 … cbz w1,0x…73070` skips `set_supported_hw` when mask==0 (a/b/c paths). Thus a/b/c paths set only the name, mask-group paths set only the mask — the two setters are never both reached in one pass.
- **Setter failure and registration.** `0x…72ff8 cbz w0,0x…72f74` continues the per-CPU loop on success; on failure it prints `"Failed to set supported hardware"` and jumps to `0x…73070` (unwind, no `cpufreq-dt`). `platform_device_register_full("cpufreq-dt")` at `0x…7303c` is reached only after the CPU loop completes, and on error prints and unwinds via `0x…730ec`.

**OPP-core behaviour (task B).** `opp_table->supported_hw` is a u32 array at offset `0x160`, count at `0x168`, prop-name string at `0x170`; `dev_pm_opp_set_supported_hw` uses `kmemdup(4*count)` and `dev_pm_opp_set_prop_name` uses `kstrdup`. The `supported-hw` predicate is inlined in `_of_add_opp_table_v2` (`0x…84bdac4`–`0x…84bdb78`): when `supported_hw==NULL`, every node carrying an `opp-supported-hw` property is dropped (only property-less nodes survive); when a mask is registered, property-less nodes are still accepted and a node is kept only if some `(mask[i] & bit[i]) != 0`. Named voltage falls back from `opp-microvolt-<name>` to `opp-microvolt`. Therefore, **mask-not-registered + b1** is the high-confidence model for this target. The separately uploaded parser-v3/v4 `live-fdt.json` is derived from the target's original live FDT and exposes all 38 OPP entries, each `supported_hw_present` flag and each named voltage variant: the `-ab` nodes have no `opp-supported-hw`; their `b1` variants exist on exactly 480/720/936/1008/1104/1200/1320/1416/1512 MHz, while the five boot-rejected rates lack b1. Applying the recovered no-mask predicate and b1 property selection therefore reproduces the nine-rate runtime list. The raw DTB binary is not archived, but the structured parser output contains the required property-presence and voltage evidence.

**NVMEM audit (task C).** `nvmem_cell_read` allocates `cell->bytes` bytes (from `[cell+0xc]`) with `GFP_KERNEL|__GFP_ZERO`; for a 2-byte `speed` cell only 2 bytes are allocated and read by the helper. The probe's `ldr w1,[x20]` then reads 4 bytes: this is a **logical over-read beyond the declared cell length** (the `&len` output is ignored). The extra two bytes stay inside the same slab object, so this is not by itself a fault or a read outside the allocator's rounded object; the risk is that a non-zero high half would miss the constant matches and fall to the default mask=8. Not asserted as a crash. Do not copy this read pattern into the new HAL.

**Source-family comparison (task D).** Against AvaotaSBC/linux `sun50iw9_nvmem_xlate` the target shares the family fingerprint (`sunxi_get_soc_ver() & 7`, SID `0x20` → `<<5`, SID `0x28` → `& 0x3ff`, both setters before `cpufreq-dt`), but differs concretely: constant set (target adds `0x2800`/`0x6c00`/`0x7400`/`0x7c00`, drops `0x2100`/`0x6000`/`0x5d00`), naming (`a/b/c` with conditional suffix vs `aN` only), mask values (`{0,1,2,8}` vs `1<<value`), and control flow (EPROBE_DEFER classification, per-CPU setter loop, `put_*` cleanup). Conclusion: **same lineage, not the same implementation**; identical SID offsets alone do not prove a source match.

**What this closes / leaves open.** Closes: full probe CFG and ten explicit speed comparisons; exact branch-to-name/mask/setter flow; b0/b1 mask=0; OPP-core supported-hw predicate and named-voltage fallback; parsed 38-node live-FDT tie-out to the nine-rate list; NVMEM read-width risk scope. Still open: the actual 32-bit `speed` comparison operand (the FDT cell is two bytes but the probe loads four), the live `sunxi_get_soc_ver()&7` value (b1 requires nonzero), exact matching source commit, and all hardware safety/authorization gates. No OPP/DVFS write is authorized; the CPU service stays `POLICY_ONLY`, `hardware_applied=0`.

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
| cpufreq getter/cache reconciliation | **Captured in v3/v4:** getter-triggered cache refresh is repeated; reads remain non-atomic |
| SID speed raw value | **Captured:** 0x6c00; candidate classifications differ by vendor source tree |
| OPP table/runtime-list structural fingerprint | **High-confidence software match (see §1.14):** shipping probe keeps mask 0 and skips `set_supported_hw` on b0/b1; OPP core drops nodes with `opp-supported-hw` when mask is absent; parser-v4 structured live FDT contains all 38 nodes and b1 variants exactly at the nine runtime rates. Raw DTB binary is not archived but is not needed for this structured property/rate reconciliation. Actual four-byte speed operand and SoC revision bits remain unmeasured. |
| DCDC1 runtime framework observation | **Captured:** v3/v4 frequency-voltage pairs (including 1320/1100 and 1104/1000 MHz/mV) align with `b1`; framework-only, not raw PMIC readback |
| Raw PMIC voltage register at BL33 entry | **Not captured** |
| Thermal cooling / calibration | **Cooling states observed at 0/8 and 0/5;** no active state in this snapshot. AVCC calibration remains rejected, so temperatures are not certified for closed-loop control |
| CPU1–CPU3 state at BL31→BL33 | **Unresolved** |
| Timer continuity during PLL relock | **Unresolved** |
| Approved RG34XX OPP grid | **None** |
| DVFS register writes | **Not implemented / not authorized** |

## 4. Next actions

1. **Done (see §1.14):** full probe CFG (287 instructions, 69 strict blocks, 106 edges, ten explicit speed values plus default), per-branch mask/name mapping, both setter guards/error paths, `cpufreq-dt` registration gate, target OPP-core supported-hw predicate and named-voltage fallback are decoded offline. The parser-v4 structured live FDT includes all 38 OPP entries and confirms b1 variants on exactly the nine runtime frequencies. **Next:** safely establish the actual four-byte speed-cell comparison operand and live `sunxi_get_soc_ver()&7`; prove BL31→BL33 CPU1–CPU3 state/timer continuity; obtain safe raw PMIC/CCU readback and a calibrated temperature source. Do not re-export p4 or rebuild ELF.
2. Obtain a safe, read-only BL33-entry CPUX / PMIC state record only after the entry ABI and MMIO access prerequisites are proven. Distinguish Linux framework regulator readings from raw AXP2202 registers and Linux clk_summary from direct CCU readback. Do not issue test writes.
3. Prove BL31→BL33 CPU1–CPU3 state, single-core execution ownership, shared CPUX PLL coordination, pll_periph0 fallback behavior and timer continuity before any shared-PLL transition design.
4. Resolve AVCC/thermal calibration rejection or establish an independently validated temperature sensor path.
5. Keep the runtime C service `POLICY_ONLY`, `hardware_applied=0`, and the Linkslot CPU Performance row display-only/unavailable. No Balanced/Performance/Boost transition is authorized.


The v4 provenance report is now archived at [reports/rg34xx-inventory-20261125-cpux-v4](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-inventory-20261125-cpux-v4). The collector is validated on Linux and Windows and across the three H700 target compile modes. It reads kernel metadata, sysfs links and filtered symbol names only; it does not touch CPUFreq controls, PMIC/PLL/CCU registers, SID contents beyond the existing two bounded cells, or vendor SMC/ioctl interfaces.

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


### 5.3 Parser-v3 inventory and analysis

- [v3 inventory README](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-inventory-20261125-cpux-v3/README.md)
- [Parser-v3 live FDT](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-inventory-20261125-cpux-v3/live-fdt.json)
- [CPU / regulator / clock / thermal getter reconciliation](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-inventory-20261125-cpux-v3/cpu-power-runtime.txt)
- [SID cell decode](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-inventory-20261125-cpux-v3/sid-cell-report.json)
- [Memory map report](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-inventory-20261125-cpux-v3/memory-map-report.json)
- [Filtered hardware boot log](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-inventory-20261125-cpux-v3/dmesg-hardware-lines.txt)

### 5.5 Kernel provenance / OPP corroboration v4

- [v4 report README](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-inventory-20261125-cpux-v4/README.md)
- [Kernel build ID, config and driver bindings](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-inventory-20261125-cpux-v4/kernel-driver-provenance.txt)
- [Stock p4 boot-image metadata](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-boot-p4-metadata.json)
- [Verified decompressed-kernel hash and aligned GNU Build ID note offsets](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-boot-p4-verified.json)
- [Matched cpufreq/OPP driver disassembly](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-cpufreq-driver-disasm.txt)
- [Parser-v4 live FDT and OPP table](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-inventory-20261125-cpux-v4/live-fdt.json)
- [CPUFreq / clock / regulator / thermal samples](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-inventory-20261125-cpux-v4/cpu-power-runtime.txt)
- [SID cell decode](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-inventory-20261125-cpux-v4/sid-cell-report.json)
- [Filtered hardware boot log](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-inventory-20261125-cpux-v4/dmesg-hardware-lines.txt)

### 5.4 OPP candidate analyzer and CI status

The read-only analyzer and regression tests were added to `tools/analyze_h700_opp_candidates.py` and `tools/test_analyze_h700_opp_candidates.py`. It models absent/empty `opp-supported-hw`, one-cell mask intersection and selected voltage suffix/base fallback, and compares candidate OPP frequency sets with the actual runtime list.

The analyzer was expanded to all 32 mask bits, alternative version cells and exact set-cover witness search; the corrected implementation commit is [`da41806e47dbd2e0d57e23e9ff6ff405a5514e29`](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/da41806e47dbd2e0d57e23e9ff6ff405a5514e29). H700 compile/host regression [#45537478198](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45537478198) and Linux/Windows boot audit [#45537478202](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45537478202) passed. Running it against v3 produces one exact `no-mask + b1` match and no exact one-cell mask witness. This is an OPP-selection fingerprint only; it does not identify the shipping driver, decode SID or approve writes.

## 6. CPU reset/RVBAR at initial BL33 handoff (2026-11-28)

The 2026-11-26 read-only probe in
[reports/rg34xx-rvbar-selector-20261126](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-rvbar-selector-20261126)
did not read back the selector. Root ADB worked, but both a bounded
32-bit pread(/dev/mem, 0x03000024) and an independent dd read failed
with EFAULT / Bad address; the probe produced zero bytes, no JSON file,
and no selector value. No kernel module, debugfs mount, boot argument,
security relaxation, or MMIO write was attempted.

A separate static audit of the matched 128 KiB Boot0 primary image, pinned
BaseOS native SPL and matched vendor BL31 now narrows the interpretation:

- Boot0's helper calls only for CPU2 and CPU3, conditional on bit 29 of
  0x03006248; the helper writes C0_RST_CTRL & ~(1 << core), which keeps
  the chosen reset bit asserted under H616 reference semantics.
- CPU1 is not passed to the helper. The H616 reference reset default
  0x13ff0101 leaves CPU0 deasserted and CPU1–CPU3 asserted. The SPL source
  only writes the primary RVBAR pair; the matched BL31 direct-MMIO scan
  recovers CPU0 mode setup but no direct secondary reset/RVBAR writes.
- This supports a probable, not proven, CPU0-only initial BL33 handoff.
  H700 C0_RST_CTRL equivalence, bit-29 meaning/value, ROM/secure/indirect
  paths, and live C0_RST_CTRL/RVBAR values remain unknown. The unusual
  Boot0 write to 0x09010044 is not resolved and must not be modified.
- The matched BL31 has a concrete later CPU_ON implementation, not merely
  symbol strings: legacy ops-table slot 1 `affinst_on` at `0x4800255c`
  branches to helper `0x48002d78` when `W3 == 0`. The helper computes
  per-core register addresses, writes per-core RVBAR and performs the
  selected CPU power/reset sequence. One selector branch uses
  `0x09010000/0x09010010`, `0x07000440/0x07000444` and `0x090100c0`; the
  other uses the `0x08100000` region (direct reference `0x08100020`).
- The active helper branch is selected using `VER_REG 0x03000024 & 7`.
  The read-only probe failed, so the actual RG34XX branch is unknown. The
  fixed-address full-image scanner's empty `cpu_reset_mmio_accesses` list
  does not capture the helper's computed per-core accesses.
- This is a later PSCI CPU_ON callback; there is no evidence it is invoked
  automatically during the first BL31→BL33 handoff, and non-secure call
  availability remains unverified. Linux later showing possible/present/
  online CPUs 0–3 is consistent with a later OS CPU_ON sequence but does
  not establish the initial handoff state.

The static audit is being rerun with an explicit reset-bit unit test in the
[BL31 static audit workflow](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/workflows/h700-bl31-memory-audit.yml).
Do not repeat /dev/mem attempts on this image. Next evidence must come from
a trusted early-boot capture or a bounded read through an already-mapped
kernel resource. Until such readback exists, keep Linkslot/LK CPU0-only,
do not call PSCI CPU_ON, and keep all hardware gates closed.

