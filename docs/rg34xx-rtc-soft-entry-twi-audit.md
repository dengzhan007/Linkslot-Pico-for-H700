# RG34XX BL31 RTC soft-entry TWI path — static audit

Date: 2026-11-27

Status: **TWI-like controller path is statically present; selected controller, bus, PMIC command and final platform action remain unresolved.**

This is a read-only host disassembly review of the exact monitor payload captured from the RG34XX non-SP inventory. It does not issue SMC, MMIO, I2C/TWI transactions, or suspend requests.

## 1. Pinned input

- Monitor load base used by the audit: `0x48000000`
- Size: 107,216 bytes
- SHA-256: `676bc0206e3c9a7f229d93ea7f3286095f669940ad91f3c4f3d37bdb18696c31`
- Source capture: [BL31 audit #94 artifact 15178074907](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45956263984/artifacts/15178074907)
- Static disassembly and host assertions: [BL31 audit #94 artifact 15178254258](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45956263984/artifacts/15178254258)

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
4. Uses the loaded value as an object base and accesses offsets including `+0x0c`, `+0x14`, and `+0x18`.
5. Stores the loaded value into global slot `0x48015548`.

The helper at `0x48013a48` reloads the object pointer from global slot `0x48015548`, programs fields at object offsets `+0x0c`, `+0x10`, and `+0x20`, and polls for expected values including `0xf8` and `0x3a`. The transport routine at `0x48013b98` uses object offsets `+0x08`, `+0x0c`, `+0x10`, `+0x14`, `+0x18`, `+0x1c`, and `+0x20` for DATA/control/status/clock/reset/extension/line-control-like fields. It issues START/STOP/read/write-style operations and branches on status values including `0x08`, `0x18`, and `0x28`.

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
- The audit pins the monitor size/SHA, checks the selector loads, signed indexed lookup instruction, object-pointer publication, adjacent raw qwords and BaseOS SPL patch offsets. It produces `TWI-DESCRIPTOR.md` and `twi-descriptor-report.json`.
- Workflow integration is in `.github/workflows/h700-bl31-memory-audit.yml`; [BL31 audit #96](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45965474745) passed the complete `audit-matched-bl31` job on audit-script HEAD `f4676657ff0864e9ecd01878c59886798ac1ea11`. Artifact: [15182980313](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45965474745/artifacts/15182980313).
- The initial run [#95](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45965350913) failed only because the new assertion expected `x19` at `0x480126d4`; the matched instruction uses `x0`. This audit-script assertion was corrected and the entire workflow passed on #96. No firmware/product code was modified.

**Interpretation update:** do not describe `0x48014628` as a proven initialized runtime pointer table. It is a lookup base used by the code; its runtime contents and the runtime context pointer at `0x48014668` remain unknown. The pinned SPL header patch does not explain either value.

## 5. Relationship to the live FDT

The uploaded live-FDT inventory reports an AXP2202 PMIC node beneath an H700 R-TWI-like node at `0x07081400`, compatible `x-powers,axp2202`. The H616 family reference maps R-TWI0 at `0x07081400`, and H616 DTS sources associate R-PIO PL0/PL1 with `s_i2c`.

This is a plausible family-level correspondence only. It does **not** prove that the descriptor selected through context `+0xa4` resolves to `0x07081400`; no selected object pointer, bus index, slave address, transaction capture, or target-specific schematic/readback is available.

## 6. Conclusions and next evidence

Confirmed statically:

- RTC soft-entry continuation `0x48012670` calls `0x48012070`; a return reaches `0x480126a4` self-loop.
- The continuation carries context `+0xa4` into the TWI-like controller initialization path.
- `0x48013ac8` performs a selector-indexed 64-bit load from lookup base `0x48014628`, uses the loaded value as an object base, and saves it to `0x48015548`; raw file bytes do not prove the runtime table contents.
- `0x48013a48` and `0x48013b98` implement controller setup and transfer/status handling.

Unresolved:

1. Who initializes or relocates the context pointer at `0x48014668`?
2. Who populates or transforms the indexed lookup data consumed from `0x48014628`, and what are its runtime contents/layout?
3. What signed value does context `+0xa4` carry on the actual RG34XX suspend path, and what absolute address does its scaled lookup select?
4. What controller base/object, bus number, slave address and command sequence are selected?
5. What final PMIC/RTC/power action follows the transfer, and what consumes the GP_DATA2 phase markers?

Next work should remain host/static: identify the runtime writer/loader patch or another authoritative memory image for the initialized data, then verify the selector-to-object mapping. Do not guess a bus or slave address and do not issue a TWI transaction.

No hardware access or modification was performed. `H700_MEMORY_LAYOUT_APPROVED` remains unset and `hardware_applied=0`.
