# RG34XX BL31 RTC soft-entry TWI path — static audit

Date: 2026-11-27

Status: **The signed-index lookup has a static H616-family TWI0–TWI4/R-TWI0 base-map correlation. R-TWI0 is a strong conditional candidate (index 73), but the actual runtime selector is not captured; PMIC command and final platform action remain unresolved.**

This is a read-only host disassembly review of the exact monitor payload captured from the RG34XX non-SP inventory. It does not issue SMC, MMIO, I2C/TWI transactions, or suspend requests.

## 1. Pinned input

- Monitor load base used by the audit: `0x48000000`
- Size: 107,216 bytes
- SHA-256: `676bc0206e3c9a7f229d93ea7f3286095f669940ad91f3c4f3d37bdb18696c31`
- Source capture: [BL31 audit #94 artifact 15178074907](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45956263984/artifacts/15178074907)
- Static disassembly and host assertions: [BL31 audit #94 artifact 15178254258](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45956263984/artifacts/15178254258)
- Expanded signed-index/base-map audit: [BL31 audit #101 artifact 15182284425](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45966329671/artifacts/15182284425)

All addresses below refer to this exact payload and are file-base-plus-offset addresses.

## 2. RTC soft-entry call path

The SYSTEM_SUSPEND code distinguishes several context-controlled paths. In the RTC soft-entry branch, the code stages RTC context and branches to `0x48012670`; this is not the ordinary WFI loop.

The RTC continuation at `0x48012670` aggregates context fields including `+0x80`, `+0x88`, `+0x8c`, `+0x90`, `+0x98` and `+0xa4`, then calls `0x48012070`. If that helper returns, execution reaches a self-loop at `0x480126a4`. The self-loop is evidence of a non-returning/fail-stop continuation, but does not identify the physical action that preceded it.

At `0x48012070`:

- `W0` is saved as a mode/data argument.
- `W1` is forwarded to `0x480129f0`.
- `X2` is retained as the context/data pointer.
- It then reads a byte from context `+0xac` and dispatches among several transfer sequences.

The call `0x480129f0 → 0x48013ac8` initializes the controller using the forwarded `W1` value. The same context `+0xa4` is also read by `0x48012510`, which calls `0x480129f0`; an alternate wrapper at `0x480125d0` forwards context `+0xa4`, `+0x94`, and a pointer to `+0xac` to `0x48012070`.

Thus, `context + 0xa4` is a selector/index input to controller initialization in these static call paths. Its runtime value has not been captured.

## 3. Controller initialization and transfer object

At `0x48013ac8`, the decoded instruction sequence:

1. Forms candidate addresses `0x07022000` and `0x0701019c`, writes `0x33` to the former and `0x1000` then `0x10001` to the latter. The precise H700 register semantics of these writes have not been independently established.
2. Forms lookup origin `0x48014628`.
3. Loads a 64-bit value using `LDR X1, [X1, W0, SXTW #3]`. This sign-extends the 32-bit value in W0 and scales it by eight; the actual value from context `+0xa4` is unknown, so the selected location may be before or after `0x48014628`.
4. Uses the loaded value as a controller MMIO base and accesses offsets including `+0x0c`, `+0x14`, and `+0x18`.
5. Stores the loaded controller base into global slot `0x48015548`.

The helper at `0x48013a48` reloads the controller base from global slot `0x48015548`, programs fields at MMIO offsets `+0x0c`, `+0x10`, and `+0x20`, and polls for expected values including `0xf8` and `0x3a`. The transport routine at `0x48013b98` uses MMIO offsets `+0x08`, `+0x0c`, `+0x10`, `+0x14`, `+0x18`, `+0x1c`, and `+0x20`. It issues START/STOP/read/write-style operations and branches on status values including `0x08`, `0x18`, and `0x28`.

This is strong evidence of a TWI/I2C-master-style controller abstraction. It is not sufficient to name a bus instance, a PMIC slave address, a register map, or a particular power-off command.

## 4. Static data / runtime pointer discrepancy

A direct comparison of code references with the raw monitor bytes exposes an unresolved initialization boundary:

- Code at `0x48013ac8` uses `0x48014628` as the lookup origin for a signed, eight-byte-scaled index and subsequently treats the loaded value as an object pointer. The actual signed index is not captured. The eight qwords immediately following the lookup origin decode as scalar/encoded-looking values, for example `(0x0000000b, 0x000186a0)`, `(0x002dc6c0, 0x0000000c)`, and `(0x0000000f, 0x000186a0)`, not plausible mapped pointers. They are only the adjacent bytes at offsets `+0x00..+0x38`; **they are not proven to be the selected entries**. A negative signed selector can address locations before `0x48014628`. Therefore the binary proves the *use-time lookup instruction and pointer use*, not the runtime table layout.
- Code repeatedly loads the qword at `0x48014668` and passes it to helpers as a context pointer, but the raw file's initial value there is `2`, not a valid pointer.
- Global slot `0x48015548` also has raw initial value `2`, but `0x48013b14` explicitly overwrites it with the object pointer selected in the preceding instructions. This proves at least this pointer slot is runtime-populated before the controller helper consumes it.
- In the direct static address-reference review of this payload, no direct store to `0x48014628` or `0x48014668` was identified. This does not exclude indirect stores, runtime relocation/fixups, an external loader patch, or initialization outside the reviewed direct-reference pattern.

Therefore the adjacent raw qwords cannot be treated as the selected runtime pointer table, and the raw `0x48014668=2` value cannot be treated as the live context pointer. The function path establishes how these values are consumed, but not the runtime signed index, which memory location it selects, or who populates those locations.


## 4A. Pinned BaseOS native SPL patch audit (2026-11-27)

The pinned BaseOS source `dengzhan007/BaseOS-MultiFrontend` commit `cbe253a43f46cf551cc9a6c79a84015bde3686cf`, file `experiments/h700-boot-speed/spl/native-boot.c`, contains these direct monitor modifications:

- `memcpy(monitor + 0x30, parameters, 128)` — vendor DRAM parameters.
- `memcpy(monitor + 0x24, board, 8)` — board bytes.
- `*(u32 *)(monitor + 0xb0) = 0x4a200000` — DTB pointer.

A source scan of the pinned file found direct `monitor + ...` offsets exactly `0x24`, `0x30`, and `0xb0`. It found no direct SPL patch targeting `monitor + 0x14628` or `monitor + 0x14668`. This narrows the hypotheses but does not eliminate indirect writes, runtime mutation inside BL31, or another initialization contract not visible as a direct patch in this source file.

A reproducible host audit was added:

- `scripts/audit-rg34xx-bl31-twi-descriptors.py`
- `scripts/test-audit-rg34xx-bl31-twi-descriptors.py`
- The audit pins the monitor size/SHA, checks the selector loads, signed indexed lookup instruction, controller-base publication, nearby TWI/R-TWI qwords and BaseOS SPL patch offsets. It produces `TWI-DESCRIPTOR.md` and `twi-descriptor-report.json`.
- Workflow integration is in `.github/workflows/h700-bl31-memory-audit.yml`. BL31 audits [#100](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45966317624) and [#101](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45966329671) passed after extending the data scan to include both standard TWI and R-TWI base candidates; #101 also passed the complete six-address H616 mapping regression. Latest generated report: [artifact 15182284425](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45966329671/artifacts/15182284425).
- Earlier [#96](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45965474745) established the first passing descriptor-boundary audit; [#95](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45965350913) failed only because the new assertion expected `x19` at `0x480126d4`, while the matched instruction uses `x0`. This audit-script assertion was corrected and later runs passed. No firmware/product runtime code was modified.

**Interpretation update:** the static bytes at signed lookup indices 68–73 now form a coherent H616-family controller-base sequence (TWI0–TWI4/R-TWI0). This identifies the likely semantics of those entries, but not which index is selected at runtime. The context slot at `0x48014668` still has an unexplained raw initial value of 2, and the pinned SPL patch does not explain it.


## 4B. H616-family TWI base-map correlation (2026-11-27)

The signed lookup is not merely an opaque pointer-table pattern. A host scan of the exact matched monitor's lookup-origin neighborhood found this consecutive six-qword sequence:

| Signed lookup index if selected | Monitor data address | Raw qword | H616 family label |
|---:|---:|---:|---|
| 68 | `0x48014848` | `0x0000000005002000` | TWI0 |
| 69 | `0x48014850` | `0x0000000005002400` | TWI1 |
| 70 | `0x48014858` | `0x0000000005002800` | TWI2 |
| 71 | `0x48014860` | `0x0000000005002c00` | TWI3 |
| 72 | `0x48014868` | `0x0000000005003000` | TWI4 |
| 73 | `0x48014870` | `0x0000000007081400` | R-TWI0 |

The selected address is computed relative to lookup origin `0x48014628`: `0x48014628 + signed_index * 8`. For index 73, this yields `0x48014870`, whose qword is `0x07081400`. The sequence of six values matches the H616 User Manual v1.0 §9.1.5 TWI/R-TWI controller base table (p.582). The exact addresses and family labels are cross-reference evidence, not an assertion that the H700 hardware is register-identical in every respect.

The controller offsets used by the matched BL31 also align with the H616 TWI register layout:

| Offset | H616 register |
|---:|---|
| `+0x08` | DATA |
| `+0x0c` | CNTR |
| `+0x10` | STAT |
| `+0x14` | CCR |
| `+0x18` | SRST |
| `+0x1c` | EFR |
| `+0x20` | LCR |

Cross-reference sources:
- H616 User Manual v1.0, §9.1.5, TWI/R-TWI base addresses and register offsets: [PDF](https://www.scs.stanford.edu/~zyedidia/docs/allwinner/h616.pdf)
- H616 device tree R-TWI0 node at `0x07081400`: [sun50i-h616.dtsi](https://kernel.googlesource.com/pub/scm/linux/kernel/git/chenhuacai/linux-loongson/+/refs/heads/master/arch/arm64/boot/dts/allwinner/sun50i-h616.dtsi)
- RG34XX live FDT inventory: `reports/rg34xx-inventory-20261126-timer/live-fdt.json`; it places the AXP2202 PMU node under `/soc@03000000/twi@0x07081400/pmu`, compatible `x-powers,axp2202`.

**Conditional conclusion:** if the actual context `+0xa4` value at this call site is 73, BL31 selects the R-TWI0 base `0x07081400`, which is consistent with the live-FDT AXP2202 parent node. This three-way agreement (monitor lookup data, H616 family register map, target live FDT) makes R-TWI0 the strongest static candidate. However, the selector is a signed 32-bit runtime value and has not been captured. The audit therefore does **not** claim that the actual suspend path selected R-TWI0; indices 68–72 remain valid alternatives from the same static mapping.

BL31 audits [#100](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45966317624) and [#101](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45966329671) both passed after the controller-base scan and H616 label classification were added. #101 artifact: [15182284425](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45966329671/artifacts/15182284425).

## 5. Relationship to the live FDT

The uploaded live-FDT inventory reports an AXP2202 PMIC node beneath an H700 R-TWI-like node at `0x07081400`, compatible `x-powers,axp2202`. The H616 family reference maps R-TWI0 at `0x07081400`, and H616 DTS sources associate R-PIO PL0/PL1 with `s_i2c`.

The live FDT AXP2202 node agrees with the static H616-family lookup candidate at index 73 (`0x07081400`). This makes R-TWI0 the strongest static candidate, but does **not** prove that the actual suspend context selected index 73. The signed runtime value at context `+0xa4`, PMIC slave address, transaction capture, and target-specific schematic/readback remain unavailable.

## 6. Conclusions and next evidence

Confirmed statically:

- RTC soft-entry continuation `0x48012670` calls `0x48012070`; a return reaches `0x480126a4` self-loop.
- The continuation carries context `+0xa4` into the TWI-like controller initialization path.
- `0x48013ac8` performs a signed selector-indexed 64-bit load from lookup origin `0x48014628`, uses the loaded value as the controller MMIO base, and saves it to `0x48015548`.
- Monitor qwords at signed indices 68–73 correlate with H616 family TWI0–TWI4 and R-TWI0 base addresses; the R-TWI0 qword at index 73 is `0x07081400` and agrees with the live-FDT AXP2202 parent node. This is a conditional static map, not a captured runtime selection.
- `0x48013a48` and `0x48013b98` implement controller setup and transfer/status handling.

Unresolved:

1. Who initializes or relocates the context pointer at `0x48014668`?
2. What signed value does context `+0xa4` carry on the actual RG34XX suspend path (in particular, is it 73), and therefore which statically mapped TWI0–TWI4/R-TWI0 base is selected?
3. What PMIC slave address and command sequence are used?
4. What final PMIC/RTC/power action follows the transfer, and what consumes the GP_DATA2 phase markers?

Next work should remain host/static: trace the source/producer of context `+0xa4` and its signed value through the RTC soft-entry caller, or identify a trustworthy pre-suspend read-only capture. The static base mapping is already constrained; the missing fact is the runtime selector. Do not infer that selector from the FDT, do not guess a PMIC slave address, and do not issue a TWI transaction.

No hardware access or modification was performed. `H700_MEMORY_LAYOUT_APPROVED` remains unset and `hardware_applied=0`.
