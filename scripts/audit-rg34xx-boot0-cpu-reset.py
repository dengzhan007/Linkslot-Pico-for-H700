#!/usr/bin/env python3
"""Read-only static audit of RG34XX Boot0 CPU reset and primary BL31 handoff."""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import struct
from pathlib import Path

from capstone import CS_ARCH_ARM, CS_MODE_THUMB, Cs
from capstone.arm import ARM_OP_IMM, ARM_OP_MEM

BOOT0_LOAD_BASE = 0x20000
PRIMARY_BYTES = 131072
PRIMARY_SHA256 = "bf886995745a0eb9b5030296f9f6163035e164b3a82183716e67ff876319289f"
RESET_DEFAULT = 0x13FF0101
SID_BASE = 0x03006000
SID_SIZE = 0x1000


def u32(raw: bytes, offset: int) -> int:
    return struct.unpack_from("<I", raw, offset)[0]


def normalize(value: str) -> str:
    return re.sub(r"\s+", "", value.lower())


def disassemble(raw: bytes, start: int, end: int) -> dict[int, object]:
    md = Cs(CS_ARCH_ARM, CS_MODE_THUMB)
    md.detail = True
    offset = start - BOOT0_LOAD_BASE
    if offset < 0 or end > BOOT0_LOAD_BASE + len(raw):
        raise RuntimeError("Boot0 disassembly range is outside captured image")
    return {ins.address: ins for ins in md.disasm(raw[offset:end-BOOT0_LOAD_BASE], start)}


def require(insns: dict[int, object], address: int, mnemonic: str, fragment: str = "") -> object:
    ins = insns.get(address)
    if ins is None:
        raise RuntimeError("missing Boot0 instruction at 0x%08x" % address)
    if ins.mnemonic.lower() != mnemonic.lower():
        raise RuntimeError("Boot0 instruction at 0x%08x is %s, expected %s" % (
            address, ins.mnemonic, mnemonic
        ))
    if fragment and normalize(fragment) not in normalize(ins.op_str):
        raise RuntimeError("Boot0 operands at 0x%08x changed: %s" % (address, ins.op_str))
    return ins


def require_reg_imm(ins, register: str, immediate: int) -> None:
    if len(ins.operands) < 2:
        raise RuntimeError("missing immediate operand at 0x%08x" % ins.address)
    if ins.reg_name(ins.operands[0].reg) != register:
        raise RuntimeError("unexpected destination register at 0x%08x: %s" % (
            ins.address, ins.op_str
        ))
    if ins.operands[1].type != ARM_OP_IMM or int(ins.operands[1].imm) != immediate:
        raise RuntimeError("unexpected immediate at 0x%08x: %s" % (
            ins.address, ins.op_str
        ))


def literal_target(ins) -> int:
    if len(ins.operands) < 2 or ins.operands[1].type != ARM_OP_MEM:
        raise RuntimeError("expected PC-relative literal load at 0x%08x" % ins.address)
    mem = ins.operands[1].mem
    if ins.reg_name(mem.base) != "pc":
        raise RuntimeError("expected PC-relative literal load at 0x%08x" % ins.address)
    return ((ins.address + 4) & ~3) + int(mem.disp)


def branch_target(ins) -> int:
    if not ins.operands or ins.operands[0].type != ARM_OP_IMM:
        raise RuntimeError("expected immediate branch target at 0x%08x" % ins.address)
    return int(ins.operands[0].imm)


def cpu_reset_bit_asserted(reset_value: int, core: int) -> bool:
    """Decode the H616-reference C0_RST_CTRL convention: zero asserts reset."""
    if isinstance(reset_value, bool) or not isinstance(reset_value, int):
        raise TypeError("reset_value must be an integer")
    if isinstance(core, bool) or not isinstance(core, int) or not 0 <= core <= 3:
        raise ValueError("core must be an integer from 0 through 3")
    return (reset_value & (1 << core)) == 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reports-root", type=Path, required=True)
    parser.add_argument("--baseos-root", type=Path, required=True)
    parser.add_argument("--bl31-report", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()

    reports = args.reports_root.resolve()
    baseos = args.baseos_root.resolve()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)

    boot0_dir = reports / "rg34xx-inventory-20261126-timer"
    raw = (boot0_dir / "boot0-primary.bin").read_bytes()
    meta = json.loads((boot0_dir / "boot0-primary.json").read_text(encoding="utf-8"))
    digest = hashlib.sha256(raw).hexdigest()
    if len(raw) != PRIMARY_BYTES or meta.get("sha256_full") != PRIMARY_SHA256 or digest != PRIMARY_SHA256:
        raise RuntimeError("primary Boot0 image no longer matches the pinned 128 KiB capture")
    if not meta.get("checksum_valid"):
        raise RuntimeError("primary Boot0 eGON checksum validation is not true")

    # Boot0 is loaded at 0x20000; code address 0x231e0 maps to file offset 0x31e0.
    helper = disassemble(raw, 0x231E0, 0x23216)
    caller = disassemble(raw, 0x23224, 0x2323E)

    literal_pairs = (
        (0x231E0, 0x090100C4),
        (0x231F6, 0x09010044),
        (0x23204, 0x09010000),
    )
    literal_map = {}
    for pc, expected in literal_pairs:
        load = require(helper, pc, "ldr", "pc")
        address = literal_target(load)
        actual = u32(raw, address - BOOT0_LOAD_BASE)
        if actual != expected:
            raise RuntimeError("Boot0 literal at 0x%08x is 0x%08x, expected 0x%08x" % (
                address, actual, expected
            ))
        literal_map["0x%08x" % pc] = {
            "literal_address": "0x%08x" % address,
            "literal_value": "0x%08x" % actual,
        }

    # DBG_REG1 bit clear, RVBAR0_H bit set (semantics unresolved), C0_RST_CTRL bit clear.
    for pc, mnemonic, fragment in (
        (0x231EA, "ldr", "r3, [r2]"),
        (0x231EC, "mvns", "r5, r4"),
        (0x231EE, "ands", "r3, r5"),
        (0x231F0, "str", "r3, [r2]"),
        (0x231FA, "ldr", "r2, [r3]"),
        (0x231FC, "orrs", "r4, r2"),
        (0x231FE, "str", "r4, [r3]"),
        (0x23208, "ldr", "r2, [r3]"),
        (0x2320A, "ands", "r5, r2"),
        (0x2320C, "str", "r5, [r3]"),
    ):
        require(helper, pc, mnemonic, fragment)

    gate_load = require(caller, 0x2322A, "ldr", "pc")
    gate_literal_address = literal_target(gate_load)
    gate_value = u32(raw, gate_literal_address - BOOT0_LOAD_BASE)
    if gate_value != 0x03006248:
        raise RuntimeError("Boot0 CPU-reset gate literal changed: 0x%08x" % gate_value)
    if not (SID_BASE <= gate_value < SID_BASE + SID_SIZE):
        raise RuntimeError("Boot0 CPU-reset gate address no longer falls in the documented H616 SID window")

    require(caller, 0x2322C, "ldr", "r3, [r3]")
    shift = require(caller, 0x2322E, "lsls", "r3, r3")
    if int(shift.operands[-1].imm) != 2:
        raise RuntimeError("Boot0 gate shift is no longer two bits")
    branch = require(caller, 0x23230, "bpl")
    if branch_target(branch) != 0x2323E:
        raise RuntimeError("Boot0 CPU-reset gate skip target changed")
    require_reg_imm(require(caller, 0x23232, "movs"), "r0", 2)
    call2 = require(caller, 0x23234, "bl")
    require_reg_imm(require(caller, 0x23238, "movs"), "r0", 3)
    call3 = require(caller, 0x2323A, "bl")
    if branch_target(call2) != 0x231E0 or branch_target(call3) != 0x231E0:
        raise RuntimeError("Boot0 CPU2/CPU3 reset helper targets changed")

    # H616 reference semantics: C0_RST_CTRL[3:0] is CPU[3:0] reset.
    reset_low_nibble = RESET_DEFAULT & 0xF
    if reset_low_nibble != 0x1:
        raise RuntimeError("unexpected H616 C0_RST_CTRL default low nibble")
    h616_reference_reset_state = {
        "cpu%d" % core: (
            "asserted" if cpu_reset_bit_asserted(RESET_DEFAULT, core) else "deasserted"
        )
        for core in range(4)
    }
    if h616_reference_reset_state != {
        "cpu0": "deasserted",
        "cpu1": "asserted",
        "cpu2": "asserted",
        "cpu3": "asserted",
    }:
        raise RuntimeError("unexpected H616 C0_RST_CTRL reset-default interpretation")

    source_path = baseos / "experiments/h700-boot-speed/spl/native-boot.c"
    spl = source_path.read_text(encoding="utf-8")
    expected_spl = (
        "ulong rvbar = (readl((void *)0x03000024) & 7) == 2 ? 0x08100040 : 0x09010040;",
        "writel(0x48000000, (void *)rvbar);",
        "writel(0, (void *)(rvbar + 4));",
    )
    for fragment in expected_spl:
        if fragment not in spl:
            raise RuntimeError("native SPL primary RVBAR handoff changed: " + fragment)
    if re.search(r"rvbar\s*\+\s*(?:8|0xc|0x10|0x14|0x18|0x1c)\b", spl, re.IGNORECASE):
        raise RuntimeError("native SPL unexpectedly writes per-core RVBAR offsets")
    if re.search(r"0x09010000|0x0901004c|0x09010054|0x0901005c|0x090100c4", spl, re.IGNORECASE):
        raise RuntimeError("native SPL source now directly writes secondary CPU reset/debug/RVBAR registers")

    bl31 = json.loads(args.bl31_report.read_text(encoding="utf-8"))
    mmio_refs = bl31.get("cpu_startup_mmio_references", [])
    reset_refs = bl31.get("cpu_reset_mmio_accesses", [])
    if reset_refs:
        raise RuntimeError("BL31 now contains tracked direct CPU reset/secondary RVBAR MMIO references")
    c0_mode_refs = [
        item for item in mmio_refs
        if item.get("kind") == "memory-effective-address" and item.get("address") == "0x09010010"
    ]
    if not c0_mode_refs:
        raise RuntimeError("expected BL31 CPUCFG C0_CTRL_REG0 AArch64-mode setup was not recovered")

    report = {
        "schema": "rg34xx-boot0-cpu-reset-static-audit-v2",
        "primary_boot0": {
            "bytes": len(raw),
            "sha256": digest,
            "load_base": "0x00020000",
            "eGON_checksum_valid": True,
            "routine_address": "0x000231e0",
            "routine_file_offset": "0x000031e0",
            "caller_address": "0x00023224",
        },
        "boot0_cpu_reset_helper": {
            "debug_register": "0x090100c4 (DBG_REG1)",
            "debug_power_request_action": "clear C_DBGPWRUPREQ[core]",
            "rvbar_high_register": "0x09010044 (manual names RVBARADDR0_H / CPU0 high word)",
            "rvbar_high_action": "OR the one-hot core mask; semantic relationship to per-core reset is unresolved",
            "reset_register": "0x09010000 (C0_RST_CTRL)",
            "reset_action": "write C0_RST_CTRL & ~(1 << core), asserting reset for selected CPU[3:0] bit",
            "invoked_for": [2, 3],
            "condition_register": "0x03006248",
            "condition_register_block": "SID peripheral window (H616 family reference)",
            "condition_register_block_base": "0x03006000",
            "condition_register_offset": "0x248",
            "condition_register_block_size": "0x1000",
            "condition_register_field_definition": "not present in the H616 public register tables; do not assign a fuse/strap meaning",
            "condition": "bit 29 set; LSLS #2 moves original bit29 to N, BPL skips when clear",
        },
        "h616_register_reference": {
            "source": "H616 User Manual v1.0, section 3.2.5.1, page 36",
            "register": "0x09010000 C0_RST_CTRL",
            "reset_default": "0x13ff0101",
            "cpu_reset_bits_3_to_0": "0x1",
            "cpu0": "deasserted by reset default",
            "cpu1_cpu2_cpu3": "asserted by reset default",
            "bit_semantics": "0 assert reset; 1 deassert reset",
            "reference_url": "https://linux-sunxi.org/images/1/1d/H616_User_Manual_V1.0.pdf",
            "caveat": "H700 CPUCFG equivalence and live register state require on-device readback; H616 manual is a family reference",
        },
        "native_spl": {
            "source_commit": "cbe253a43f46cf551cc9a6c79a84015bde3686cf",
            "writes_primary_rvbar_pair_only": True,
            "does_not_directly_write_c0_rst_ctrl": True,
            "does_not_directly_write_secondary_rvbar_offsets": True,
        },
        "vendor_bl31": {
            "cpu_cfg_accesses": mmio_refs,
            "direct_cpu_reset_or_secondary_rvbar_accesses": reset_refs,
            "observed_cpu0_mode_register_address": "0x09010010 (C0_CTRL_REG0)",
            "interpretation": "Only primary CPU0 mode setup is statically tracked; no direct C0_RST_CTRL / DBG_REG1 / CPU1-CPU3 RVBAR access was recovered",
        },
        "secondary_core_assessment": {
            "status": "PROBABLE_NOT_DEVICE_VERIFIED",
            "cpu1": {
                "boot0_reset_helper_invoked": False,
                "h616_reference_reset_default": h616_reference_reset_state["cpu1"],
            },
            "cpu2": {
                "boot0_reset_helper_invoked_conditionally": True,
                "condition": "bit29 of 0x03006248",
                "helper_reset_action": "clear reset bit; cannot deassert reset under H616 reference semantics",
                "h616_reference_reset_default": h616_reference_reset_state["cpu2"],
            },
            "cpu3": {
                "boot0_reset_helper_invoked_conditionally": True,
                "condition": "bit29 of 0x03006248",
                "helper_reset_action": "clear reset bit; cannot deassert reset under H616 reference semantics",
                "h616_reference_reset_default": h616_reference_reset_state["cpu3"],
            },
            "h616_reference_reset_state": h616_reference_reset_state,
            "native_spl_secondary_rvbar_or_reset_writes_found": False,
            "direct_bl31_secondary_reset_or_rvbar_writes_found": False,
            "gate_bit29_live_value": "NOT_CAPTURED",
            "c0_rst_ctrl_live_value": "NOT_CAPTURED",
            "h700_register_equivalence": "NOT_VERIFIED",
            "assessment_boundary": "Static evidence suggests CPU1-CPU3 remain held off at initial BL33 handoff, but ROM/secure/indirect paths and live H700 register state are not proven.",
        },
        "limitations": [
            "The file named boot0-secondary.bin is a second Boot0 image copy/slot, not CPU1 firmware.",
            "The reset default is a hardware reset value, not live readback at BL31->BL33 handoff.",
            "The 0x03006248 feature-bit runtime value is not captured; CPU2/CPU3 helper runs only when bit29 is set.",
            "0x03006248 falls inside the H616 SID address window at offset 0x248, but the public H616 register list does not define this word or bit29. Its exact H700 meaning is unknown; only a masked single-bit read should be considered, never a full SID dump.",

            "The write to 0x09010044 is unusual: H616 documentation calls it CPU0 RVBARADDR0_H, but this helper ORs a CPU2/CPU3 one-hot mask into it. Its purpose is unresolved.",
            "The scan follows direct statically decoded instructions and direct MMIO references; indirect/secure/ROM behavior is not fully modeled.",
            "No hardware MMIO, CPU state, reset bits, or RVBAR values were read. No firmware, Boot0, TOC1, SD or payload was written or executed.",
        ],
    }
    (out / "boot0-cpu-reset-report.json").write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    markdown = [
        "# RG34XX Boot0 / BL31 CPU startup static audit",
        "",
        "- Primary Boot0: %d bytes, SHA-256 %s; eGON checksum valid." % (len(raw), digest),
        "- CPU reset helper: 0x231e0 (file offset 0x31e0); caller 0x23224.",
        "- Helper clears DBG_REG1.C_DBGPWRUPREQ[core], ORs one-hot bit into 0x09010044, then writes C0_RST_CTRL & ~(1 << core).",
        "- Caller reads 0x03006248; bit 29 selects helper calls for core 2 and core 3. Core 1 is not passed to this helper.",
        "- Gate address 0x03006248 is in the H616 SID window [0x03006000,0x03007000), offset 0x248; the public register tables do not define this word/bit29.",

        "- H616 C0_RST_CTRL reset default 0x13ff0101: CPU0 reset bit=1 (deasserted); CPU1-3 bits=0 (asserted). This is a family reference, not H700 live readback.",
        "- The helper reset write keeps selected CPU2/CPU3 reset bits asserted; it never sets/deasserts C0_RST_CTRL bits. Its call is gated by bit29 at 0x03006248; that value is unknown.",
        "- CPU1 is not passed to the helper. Under H616 reference semantics its reset-default bit remains asserted unless an unreviewed stage releases it.",
        "- Native SPL programs the primary RVBAR pair only; BL31 statically references CPU0 mode register 0x09010010, with no direct reset/secondary RVBAR access recovered.",
        "",
        "## Required interpretation",
        "",
        "- Static evidence makes CPU1-CPU3 hold-off PROBABLE for this Boot0/SPL/BL31 source set: CPU1 is not touched by the helper; CPU2/CPU3 are only passed to a helper that clears reset bits; and the H616 family reset default asserts bits 1-3. This is not an H700 device verification.",
        "- Treat CPU1-CPU3 hold-off as PROBABLE, not device-verified. Before executing a diagnostic BL33, collect read-only C0_RST_CTRL (0x09010000), C0_CPU_STATUS (0x09010080), RVBARADDR0-3 and DBG_REG0/1 via an existing trusted environment.",
        "- Do not enable SMP. Keep the single-core LK gate open until live readback or a reviewed explicit Boot0/SPL hold-off patch proves the other cores remain in reset for the full LK lifetime.",
        "- 0x09010044 is documented as CPU0 RVBAR high, but the Boot0 helper sets bits 2/3 there; this mismatch is unresolved and must be traced before modifying that register.",
        "- H616 User Manual v1.0 section 3.2.5.1 page 36 and section 3.2.5.17 page 41: https://linux-sunxi.org/images/1/1d/H616_User_Manual_V1.0.pdf",
        "- No hardware MMIO or write operation was performed.",
        "",
    ]
    (out / "BOOT0-CPU-STARTUP.md").write_text("\n".join(markdown), encoding="utf-8")
    print("\n".join(markdown))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
