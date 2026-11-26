#!/usr/bin/env python3
"""Validate matched RG34XX BL31 PSCI CPU_ON and emit power-ops disassembly."""
from __future__ import annotations
import argparse
import hashlib
import json
import struct
from pathlib import Path

BASE = 0x48000000
SIZE = 107216
SHA = "676bc0206e3c9a7f229d93ea7f3286095f669940ad91f3c4f3d37bdb18696c31"
TABLE = 0x480165f8
EXPECTED_TABLE = [
    0x48002510, 0x4800254c, 0x4800264c, 0x48002878, 0x48002680,
    0x480026c4, 0x480025a0, 0x48002574, 0x48002544,
]
WORDS = {
    0x480029bc: 0xa9bf7bfd, 0x480029c0: 0x900000a1,
    0x480029c4: 0x9117e021, 0x480029cc: 0xf9000001,
    0x48009dec: 0x97ffe2f4,
    0x48008600: 0x900001a1, 0x48008604: 0xf947e021,
    0x48008608: 0xf9400433, 0x48008628: 0xaa1303e5,
    0x48008640: 0xd61f00a0,
    0x4800869c: 0x900001a1, 0x480086a0: 0xf947e021,
    0x480086a4: 0xf9400433, 0x480086c4: 0xaa1303e5,
    0x480086dc: 0xd61f00a0,
    0x48002858: 0x97fffc20, 0x48002874: 0x17ffff83,
    0x48002998: 0x97fffb7e,
    0x4800254c: 0x35000103, 0x4800255c: 0x94000207,
    0x48002d18: 0xd2800480, 0x48002d1c: 0xf2a06000,
    0x48002d20: 0xb9400000, 0x48002d24: 0xf240081f,
    0x48002d28: 0x1a9f17e0, 0x48002d40: 0x34000080,
    0x48002d44: 0x52840100, 0x48002d48: 0x72a02400,
    0x48002d4c: 0x14000003, 0x48002d50: 0x52800100,
    0x48002d54: 0x72a02040, 0x48002d58: 0x0b000021,
    0x48002d5c: 0x531d7020, 0x48002d60: 0x11001001,
    0x48002d64: 0xb9000002, 0x48002d68: 0xb900003f,
    0x48002d30: 0xa9bf7bfd, 0x48002d3c: 0x97fffff7,
    0x48002d64: 0xb9000002, 0x48002d68: 0xb900003f,
    0x48002d78: 0xa9bf7bfd, 0x48002d7c: 0x2a0003e4,
    0x48002d80: 0xaa0103e0, 0x48002d84: 0x2a0403e1,
    0x48002d8c: 0x97ffffe9, 0x48002d90: 0x97ffffe2,
    0x48002d94: 0x340004e0,
    0x48002db0: 0xb94000a1, 0x48002db8: 0x0a030021,
    0x48002dbc: 0xb90000a1, 0x48002dc4: 0xb94000c1,
    0x48002dcc: 0xb90000c1, 0x48002dd8: 0xb9400027,
    0x48002de0: 0xb9000020, 0x48002df4: 0xb9400001,
    0x48002dfc: 0xb9000003, 0x48002e00: 0xb94000c0,
    0x48002e08: 0xb90000c0, 0x48002e0c: 0xb94000a0,
    0x48002e14: 0xb90000a0, 0x48002e20: 0xb9400001,
    0x48002e28: 0xb9000002,
}
STRINGS = (
    b"psci_plat_pm_ops",
    b"services/std_svc/psci/psci_main.c",
    b"services/std_svc/psci/psci_setup.c",
    b"psci_afflvl0_on",
)

DISASSEMBLY_WINDOWS = (
    ("ops_slot_1_cpu_on", 0x4800254c, 0x100),
    ("ops_slot_2_raw", 0x4800264c, 0x100),
    ("ops_slot_3_raw", 0x48002878, 0x180),
    ("ops_slot_4_raw", 0x48002680, 0x100),
    ("ops_slot_5_raw", 0x480026c4, 0x1b4),
    ("ops_slot_6_raw", 0x480025a0, 0x100),
    ("ops_slot_7_raw", 0x48002574, 0x100),
    ("ops_slot_8_raw", 0x48002544, 0x100),
    ("ccu_save_helper", 0x48001790, 0x100),
    ("ccu_restore_helper", 0x480018d8, 0x100),
    ("restore_callsite_context", 0x48002838, 0x80),
    ("save_callsite_context", 0x48002978, 0x80),
)

def read_word(raw: bytes, address: int) -> int:
    offset = address - BASE
    if offset < 0 or offset + 4 > len(raw):
        raise RuntimeError("address outside payload: 0x%08x" % address)
    return struct.unpack_from("<I", raw, offset)[0]

def decode_bl_target(pc: int, instruction: int) -> int:
    if (instruction & 0xfc000000) != 0x94000000:
        raise RuntimeError("expected AArch64 BL at 0x%08x" % pc)
    immediate = instruction & 0x03ffffff
    if immediate & (1 << 25):
        immediate -= 1 << 26
    return pc + (immediate << 2)

def decode_b_target(pc: int, instruction: int) -> int:
    if (instruction & 0xfc000000) != 0x14000000:
        raise RuntimeError("expected AArch64 B at 0x%08x" % pc)
    immediate = instruction & 0x03ffffff
    if immediate & (1 << 25):
        immediate -= 1 << 26
    return pc + (immediate << 2)

def render_disassembly(raw: bytes) -> str:
    try:
        from capstone import Cs, CS_ARCH_ARM64, CS_MODE_ARM
    except ImportError as exc:
        raise RuntimeError("Capstone is required for callback disassembly") from exc
    decoder = Cs(CS_ARCH_ARM64, CS_MODE_ARM)
    lines = [
        "# Matched BL31 PSCI/CCU callback windows",
        "# Slots other than the CPU_ON slot are intentionally not semantically labelled.",
        "",
    ]
    for label, start, length in DISASSEMBLY_WINDOWS:
        offset = start - BASE
        if offset < 0 or offset + length > len(raw):
            raise RuntimeError("disassembly window outside monitor: %s" % label)
        lines.append("## %s [0x%08x, 0x%08x)" % (label, start, start + length))
        for insn in decoder.disasm(raw[offset:offset + length], start):
            operands = (" " + insn.op_str) if insn.op_str else ""
            lines.append("0x%08x: %-8s%s" % (insn.address, insn.mnemonic, operands))
        lines.append("")
    return "\n".join(lines) + "\n"

def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--monitor", type=Path, required=True)
    parser.add_argument("--baseos-root", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()

    raw = args.monitor.read_bytes()
    digest = hashlib.sha256(raw).hexdigest()
    if len(raw) != SIZE or digest != SHA:
        raise RuntimeError("BL31 identity mismatch; refusing PSCI audit")

    for address, expected in WORDS.items():
        actual = read_word(raw, address)
        if actual != expected:
            raise RuntimeError(
                "instruction mismatch at 0x%08x: got 0x%08x expected 0x%08x"
                % (address, actual, expected)
            )

    offset = TABLE - BASE
    table = list(struct.unpack_from("<9Q", raw, offset))
    if table != EXPECTED_TABLE:
        raise RuntimeError("platform ops table changed: %s" %
                           [hex(value) for value in table])

    if decode_bl_target(0x48009dec, read_word(raw, 0x48009dec)) != 0x480029bc:
        raise RuntimeError("platform ops registration call changed")
    if decode_bl_target(0x4800255c, read_word(raw, 0x4800255c)) != 0x48002d78:
        raise RuntimeError("CPU_ON callback target changed")
    if table[3] != 0x48002878:
        raise RuntimeError("platform-ops slot 3 pointer changed")
    if table[4] != 0x48002680:
        raise RuntimeError("platform-ops slot 4 pointer changed")
    if table[5] != 0x480026c4:
        raise RuntimeError("platform-ops slot 5 pointer changed")
    if decode_bl_target(0x48002998, read_word(raw, 0x48002998)) != 0x48001790:
        raise RuntimeError("slot-3 CCU context-save call changed")
    if decode_bl_target(0x48002858, read_word(raw, 0x48002858)) != 0x480018d8:
        raise RuntimeError("slot-5 CCU context-restore call changed")
    if decode_b_target(0x48002874, read_word(raw, 0x48002874)) != table[4]:
        raise RuntimeError("slot-5 tail branch to slot-4 target changed")
    if decode_bl_target(0x48002d8c, read_word(raw, 0x48002d8c)) != 0x48002d30:
        raise RuntimeError("per-core RVBAR helper call changed")
    if decode_bl_target(0x48002d90, read_word(raw, 0x48002d90)) != 0x48002d18:
        raise RuntimeError("SoC selection helper call changed")

    missing = [value.decode() for value in STRINGS if value not in raw]
    if missing:
        raise RuntimeError("expected embedded TF-A PSCI strings missing: %s" % missing)

    # Compare the BL31 die selector against the exact pinned native SPL rule.
    spl_path = args.baseos_root.resolve() / "experiments/h700-boot-speed/spl/native-boot.c"
    spl_source = spl_path.read_text(encoding="utf-8")
    spl_selector = "ulong rvbar = (readl((void *)0x03000024) & 7) == 2 ? 0x08100040 : 0x09010040;"
    if spl_selector not in spl_source:
        raise RuntimeError("pinned BaseOS native SPL RVBAR selector changed")
    primary_bank = (0x01202008 << 3) & 0xffffffff
    alternate_bank = (0x01020008 << 3) & 0xffffffff
    if primary_bank != 0x09010040 or alternate_bank != 0x08100040:
        raise RuntimeError("decoded BL31 per-core RVBAR bank arithmetic changed")
    bl31_bank = {value: (primary_bank if value == 0 else alternate_bank) for value in range(8)}
    spl_bank = {value: (alternate_bank if value == 2 else primary_bank) for value in range(8)}
    matching_selector_values = [value for value in range(8) if bl31_bank[value] == spl_bank[value]]
    mismatching_selector_values = [value for value in range(8) if bl31_bank[value] != spl_bank[value]]
    if matching_selector_values != [0, 2] or mismatching_selector_values != [1, 3, 4, 5, 6, 7]:
        raise RuntimeError("BL31/SPL RVBAR selector parity classification changed unexpectedly")

    report = {
        "schema": "rg34xx-bl31-psci-cpu-on-static-audit-v1",
        "monitor": {"bytes": len(raw), "sha256": digest, "base": "0x48000000"},
        "platform_ops": {
            "table_address": "0x480165f8",
            "entries": ["0x%08x" % value for value in table],
            "slot_0": "cpu_standby (standard plat_psci_ops_t order)",
            "slot_1": {"name": "pwr_domain_on (standard plat_psci_ops_t order)",
                       "address": "0x%08x" % table[1]},
            "slot_2": {"name": "pwr_domain_off (standard plat_psci_ops_t order)",
                       "address": "0x%08x" % table[2]},
            "slot_3": {"name": "unclassified (exact vendor plat_psci_ops_t ABI not pinned)",
                       "address": "0x%08x" % table[3]},
            "slot_4": {"name": "unclassified (exact vendor plat_psci_ops_t ABI not pinned)",
                       "address": "0x%08x" % table[4]},
            "slot_5": {"name": "unclassified (exact vendor plat_psci_ops_t ABI not pinned)",
                       "address": "0x%08x" % table[5]},
            "slot_7": {"name": "unclassified (exact vendor plat_psci_ops_t ABI not pinned)",
                       "address": "0x%08x" % table[7]},
            "registration_function": "0x480029bc",
            "registration_callsite": "0x48009dec",
        },
        "dispatch": {
            "global_pointer_address": "0x4801bfc0",
            "global_string": "psci_plat_pm_ops",
            "ops_pointer_load_pc": "0x48008604",
            "slot_plus_8_load_pc": "0x48008608",
            "callback_move_pc": "0x48008628",
            "indirect_branch_pc": "0x48008640",
            "second_dispatch": "0x480086a0..0x480086dc",
            "standard_tf_a_psci_cpu_on_aarch64_fid": "0xc4000003",
            "capability_advertisement_proof": "not established: exact vendor plat_psci_ops_t ABI and pwr_domain_on_finish slot are not pinned",
            "slot_7_unclassified_address": "0x48002574",
        },
        "disassembly_file": "PSCI-CALLBACK-DISASSEMBLY.txt",
        "power_management_path_candidates": {
            "slot_3_function": {
                "entry": "0x%08x" % table[3],
                "save_context_callsite": "0x48002998",
                "save_context_target": "0x48001790",
                "ccu_candidates": ["0x03001000", "0x03001a78", "0x03001a8c"],
                "save_call_within_slot3_function": table[3] <= 0x48002998 < 0x480029b8,
            },
            "slot_5_function": {
                "entry": "0x%08x" % table[5],
                "restore_context_callsite": "0x48002858",
                "restore_context_target": "0x480018d8",
                "tail_branch_callsite": "0x48002874",
                "tail_branch_target": "0x%08x" % decode_b_target(0x48002874, read_word(raw, 0x48002874)),
                "restore_call_within_slot5_function": table[5] <= 0x48002858 < 0x48002878,
            },
            "interpretation": "the function entered through raw slot 3 contains the conditional CCU save call; the separate function entered through raw slot 5 contains the conditional CCU restore call and tail-branches to raw slot 4. Slots 3/4/5 remain semantically unclassified because the exact vendor ABI is not pinned",
        },
        "callback": {
            "address": "0x4800254c",
            "call_pc": "0x4800255c",
            "low_level_target": "0x48002d78",
            "condition": "W3 == 0 reaches low-level CPU_ON sequence; W3 != 0 skips it",
        },
        "low_level_cpu_on": {
            "address": "0x48002d78",
            "argument_use": "W0 target core index; X1 non-secure entry address",
            "rvbar_helper": "0x48002d30",
            "soc_selector": "0x48002d18",
            "rvbar_selection": {
                "selector_register": "0x03000024 (SYS_CFG VER_REG; H616 family reference)",
                "selector_mask": "0x7",
                "bl31_rule": "low3 == 0 -> 0x09010040 + core*8; low3 != 0 -> 0x08100040 + core*8",
                "native_spl_rule": "low3 == 2 -> 0x08100040; otherwise -> 0x09010040",
                "matching_low3_values": matching_selector_values,
                "mismatching_low3_values": mismatching_selector_values,
                "device_low3_value": "UNKNOWN; requires read-only target sampling",
                "native_spl_source_commit": "cbe253a43f46cf551cc9a6c79a84015bde3686cf",
            },
            "registers": {
                "C0_RST_CTRL": "0x09010000",
                "C0_CTRL_REG0": "0x09010010",
                "AON_POWERON_RESET": "0x07000440",
                "AON_CLAMP": "0x07000444",
                "DBG_REG0": "0x090100c0",
            },
            "sequence": [
                "program per-core RVBAR low and clear high word",
                "assert target core reset",
                "assert target power-on reset",
                "set target AArch64 mode",
                "apply target power",
                "release target output clamp",
                "deassert target power-on reset",
                "deassert target core reset",
                "assert target DBGPWRUPD",
            ],
        },
        "embedded_strings": [value.decode() for value in STRINGS],
        "interpretation": [
            "The matched binary registers a platform operation table whose second pointer slot is loaded by PSCI dispatcher code and invoked through an indirect branch.",
            "In the standard TF-A plat_psci_ops_t layout, the second member is pwr_domain_on; its callback reaches code that programs a per-core RVBAR and sequences CPU power/reset.",
            "The table's slot 1 callback reaches the CPU_ON low-level routine, but PSCI feature advertisement is not inferred because the exact vendor plat_psci_ops_t ABI and pwr_domain_on_finish slot are not pinned.",
            "This proves BL31 contains and exposes a CPU_ON-capable platform callback; it does not prove that the callback executes during initial BL31-to-BL33 handoff.",
            "The audit does not establish live CPU1-CPU3 reset state before Linux or Linkslot requests PSCI CPU_ON.",
            "The function entered through raw slot 3 at 0x48002878 contains the CCU save call at 0x48002998; the separate function entered through raw slot 5 at 0x480026c4 contains the CCU restore call at 0x48002858 and tail-branches at 0x48002874 to raw slot 4 at 0x48002680.",
            "The two helper calls are in separate registered raw platform-ops entry regions. Exact semantic callback names, runtime invocation, and cold-boot reachability remain unresolved; they do not establish a BL33 startup clock transition or CPU DVFS API.",
            "BL31 and pinned BaseOS SPL select the same RVBAR bank only when VER_REG low3 is 0 or 2; values 1 or 3-7 would make their selector rules diverge. The live value is unknown.",
            "The exact vendor callback ABI/source revision is not recovered; this is a pinned binary plus standard ABI-position correlation, not exact vendor source identity.",
            "No hardware MMIO, flash, boot-media write or payload execution was performed.",
        ],
    }

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n",
                        encoding="utf-8")
    disassembly_path = args.out.parent / "PSCI-CALLBACK-DISASSEMBLY.txt"
    disassembly_path.write_text(render_disassembly(raw), encoding="utf-8")
    markdown = [
        "# Matched BL31 PSCI CPU_ON callback audit", "",
        "- Monitor: 107,216 bytes; SHA-256 `%s`." % digest,
        "- Platform ops table: `0x480165f8`; slot +8 points to callback `0x4800254c`.",
        "- PSCI dispatch loads the global `psci_plat_pm_ops` pointer, reads its +8 slot, moves that function pointer to `x5`, then branches through `x5`.",
        "- Callback `0x4800254c` calls low-level CPU_ON routine `0x48002d78` at `0x4800255c` when `W3 == 0`.",
        "- Raw slot 3 entry 0x48002878 contains the conditional CCU save call at 0x48002998 -> 0x48001790. Separate raw slot 5 entry 0x480026c4 contains the conditional CCU restore call at 0x48002858 -> 0x480018d8; its epilogue at 0x48002874 branches to raw slot 4 entry 0x48002680.",
        "- Full disassembly windows for all table entries and both CCU helpers are saved in PSCI-CALLBACK-DISASSEMBLY.txt for function-boundary/CFG review.",
        "- Slot 3/5 helper-call ownership by raw table-entry code regions is established; semantic callback names, runtime execution and cold-boot reachability remain unresolved. No BL33 startup clock transition or CPU DVFS support is established.",
        "- Low-level routine writes a per-core RVBAR and contains a CPU power/reset sequence; one SoC-selected variant uses `0x09010000`, `0x09010010`, `0x07000440`, `0x07000444` and `0x090100c0`.",
        "- RVBAR selector cross-check: BL31 uses VER_REG low3==0 for base 0x09010040, otherwise 0x08100040; pinned native SPL uses low3==2 for the alternate bank. Both agree only for selector values 0 and 2; the target value must be read.",
        "",
        "## Interpretation", "",
        "- In the standard TF-A `plat_psci_ops_t` layout, the second function pointer is `pwr_domain_on`; embedded PSCI source strings and dispatcher references support the platform-ops interpretation.",
        "- The slot-1 callback reaches the CPU_ON low-level sequence. PSCI feature advertisement is not inferred until the exact vendor TF-A ABI and pwr_domain_on_finish slot are identified.",
        "- This proves the matched BL31 exposes a CPU_ON-capable callback. It does **not** prove the path runs automatically at initial BL31-to-BL33 handoff.",
        "- The exact SoC variant selected by `0x03000024 & 7` was not read on-device. Do not assume the H616-named register branch is the RG34XX H700 active branch.",
        "- The exact vendor callback ABI/source revision is not recovered; this is a binary and standard ABI-position correlation, not exact source identity.",
        "- Initial CPU1-CPU3 reset state and the moment Linux/Linkslot requests CPU_ON remain runtime questions.",
        "- No hardware MMIO, flash, boot-media write or payload execution occurred.", "",
    ]
    (args.out.parent / "PSCI-CPU-ON.md").write_text("\n".join(markdown), encoding="utf-8")
    print("PASS: matched BL31 PSCI ops table, dispatch, CPU_ON callback and reset sequence")
    print("  table[1] = 0x%08x -> low-level 0x48002d78" % table[1])
    print("  callback disassembly = %s" % disassembly_path)
    print("  monitor SHA-256 = %s" % digest)
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
