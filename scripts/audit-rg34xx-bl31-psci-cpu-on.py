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
SUSPEND_LEVEL_TABLE = 0x48017d70
SUSPEND_LEVEL_CALLBACKS = (0x48008b84, 0x48008ab8, 0x48008a00)
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
    # Legacy plat_pm_ops_t member dispatches.
    0x48009684: 0xf947e021, 0x48009688: 0xf9400021,
    0x48009690: 0xd63f0020,
    0x480080f8: 0xf9400835, 0x48008118: 0xd63f02a0,
    0x48008a84: 0xf9400c26, 0x48008a90: 0xd63f00c0,
    0x4800854c: 0xf9401003, 0x48008560: 0xd63f0060,
    0x48008918: 0xf9401421, 0x4800893c: 0xd63f0060,
    # Vendor SYSTEM_SUSPEND getter is the ninth (+0x40) member.
    0x48002544: 0x52a04020, 0x48002548: 0xd65f03c0,
    0x48009718: 0xf947e000, 0x4800971c: 0xf9402000,
    0x48009720: 0xb5000120, 0x48009744: 0xd63f0000,
    0x4800975c: 0x97fffd8f,
    # SYSTEM_SUSPEND special return handling and power-state entry.
    0x4800299c: 0x12800060, 0x48009768: 0x3100101f,
    0x48009770: 0x54000081, 0x48009774: 0x97ffe0a4,
    # Affinity-level suspend wrappers dispatch legacy affinst_suspend.
    0x48008a18: 0x71000a9f, 0x48008a84: 0xf9400c26,
    0x48008a90: 0xd63f00c0, 0x48008ad0: 0x7100069f,
    0x48008b34: 0xf9400c26, 0x48008b40: 0xd63f00c0,
    0x48008ba4: 0x39402c00, 0x48008c20: 0xf9400c36,
    0x48008c54: 0xd63f02c0,
    # Generic transition dispatcher: per-affinity callback table and loop.
    0x48008e4c: 0xf0000077, 0x48008e54: 0x9135c2f7,
    0x48008e64: 0x0b160281, 0x48008e68: 0x6b0102bf,
    0x48008e70: 0xf8767b60, 0x48008e74: 0xb4000100,
    0x48008e78: 0xf861dae4, 0x48008e88: 0xd63f0080,
    0x48008e94: 0x910006d6, 0x48008e98: 0x17fffff3,
    # SYSTEM_SUSPEND special entry prepares context and branches to SRAM.
    0x48001b5c: 0x94005173, 0x48001b64: 0xd2803700,
    0x48001b68: 0xa8c27bfd, 0x48001b6c: 0xf2a00040,
    0x48001b70: 0xd61f0000,
    # Ordinary power-down helper executes DSB; WFI; retry WFI.
    0x48015d70: 0xd5033f9f, 0x48015d74: 0xd503207f,
    0x48015d78: 0x17ffffff,
    # EL3 special path disables MMU/D-cache then walks cache hierarchy by set/way.
    0x48015fb8: 0xb40003e3, 0x48015fe4: 0xd51a000a,
    0x48015fec: 0xd5390001, 0x48016040: 0xd508764b,
    0x48016060: 0xd5087e4b, 0x48016080: 0xd5087a4b,
    0x480160a8: 0xd5390029, 0x4801612c: 0xd53e1000,
    0x48016130: 0x8a210000, 0x48016134: 0xd51e1000,
    0x48016138: 0xd5033fdf, 0x48016140: 0x17ffffda,
    # Slot 7 system-reset and slot 6 system-off helper calls.
    0x48002588: 0x97fffe50, 0x480025a8: 0x97fffe4c,
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
OPS_MEMBER_NAMES = (
    "affinst_standby",
    "affinst_on",
    "affinst_off",
    "affinst_suspend",
    "affinst_on_finish",
    "affinst_suspend_finish",
    "system_off",
    "system_reset",
    "get_sys_suspend_power_state",
)

STRINGS = (
    b"psci_plat_pm_ops",
    b"psci_plat_pm_ops->get_sys_suspend_power_state",
    b"psci_system_suspend",
    b"Sunxi System Reset",
    b"Sunxi System Reset: operation not handled",
    b"services/std_svc/psci/psci_main.c",
    b"services/std_svc/psci/psci_setup.c",
    b"psci_afflvl0_on",
)

DISASSEMBLY_WINDOWS = (
    ("ops_slot_0_raw", 0x48002510, 0x3c),
    ("ops_slot_1_cpu_on", 0x4800254c, 0x100),
    ("ops_slot_2_raw", 0x4800264c, 0x100),
    ("ops_slot_3_raw", 0x48002878, 0x180),
    ("ops_slot_4_raw", 0x48002680, 0x100),
    ("ops_slot_5_raw", 0x480026c4, 0x1b4),
    ("ops_slot_6_raw", 0x480025a0, 0x100),
    ("ops_slot_7_raw", 0x48002574, 0x100),
    ("ops_slot_8_raw", 0x48002544, 0x100),
    ("legacy_slot0_standby_dispatch", 0x48009680, 0x20),
    ("legacy_slot2_off_dispatch", 0x480080f0, 0x40),
    ("legacy_slot3_suspend_dispatch", 0x48008a74, 0x24),
    ("legacy_slot4_on_finish_dispatch", 0x48008544, 0x20),
    ("legacy_slot5_suspend_finish_dispatch", 0x48008910, 0x34),
    ("legacy_slot8_system_suspend_getter_dispatch", 0x48009714, 0x50),
    ("system_suspend_transition_dispatch", 0x48008d98, 0x140),
    ("suspend_afflvl0_wrapper", 0x48008b84, 0x100),
    ("suspend_afflvl1_wrapper", 0x48008ab8, 0x100),
    ("suspend_afflvl2_wrapper", 0x48008a00, 0x100),
    ("system_suspend_power_entry", 0x48001a04, 0x300),
    ("normal_suspend_powerdown_helper", 0x48015d70, 0x80),
    ("system_suspend_cache_teardown_and_trampoline", 0x48015fb8, 0x270),
    ("system_suspend_caller_full", 0x480096f8, 0xa0),

    ("ccu_save_helper", 0x48001790, 0x100),
    ("ccu_restore_helper", 0x480018d8, 0x100),
    ("restore_callsite_context", 0x48002838, 0x80),
    ("save_callsite_context", 0x48002978, 0x80),
    ("psci_dispatch_cpu_on_primary", 0x480085e0, 0x90),
    ("psci_dispatch_cpu_on_secondary", 0x48008690, 0x80),
    ("psci_dispatch_smc_range", 0x48007f00, 0x900),
    ("psci_suspend_dispatch_tail", 0x480087f4, 0x500),
    ("psci_post_suspend_dispatch_and_helpers", 0x48008cf0, 0x10d0),
    ("psci_platform_registration", 0x48009dc0, 0x80),
    ("power_system_off_helper", 0x48001ec8, 0x60),
    ("power_cpu_off_helper", 0x48001ed8, 0x60),
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
        "Slot semantics are mapped only where table offsets, dispatcher callsites, embedded strings and behavior provide binary evidence.",
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
    if len(table) != len(OPS_MEMBER_NAMES):
        raise RuntimeError("legacy PSCI platform-ops member count changed")
    if table[8] != 0x48002544:
        raise RuntimeError("get_sys_suspend_power_state slot 8 pointer changed")
    suspend_level_table = list(struct.unpack_from(
        "<3Q", raw, SUSPEND_LEVEL_TABLE - BASE
    ))
    if suspend_level_table != list(SUSPEND_LEVEL_CALLBACKS):
        raise RuntimeError("PSCI suspend affinity-level callback map changed")

    if decode_bl_target(0x48009dec, read_word(raw, 0x48009dec)) != 0x480029bc:
        raise RuntimeError("platform ops registration call changed")
    suspend_dispatch_words = {
        0x48008a18: 0x71000a9f,  # afflvl2 wrapper checks affinity level 2
        0x48008a84: 0xf9400c26,  # platform ops +0x18
        0x48008a90: 0xd63f00c0,  # call affinst_suspend
        0x48008ad0: 0x7100069f,  # afflvl1 wrapper checks affinity level 1
        0x48008b34: 0xf9400c26,
        0x48008b40: 0xd63f00c0,
        0x48008c20: 0xf9400c36,  # afflvl0 wrapper platform ops +0x18
        0x48008c54: 0xd63f02c0,
        0x48008e4c: 0xf0000077,  # callback table base page
        0x48008e54: 0x9135c2f7,  # callback table +0xd70
        0x48008e64: 0x0b160281,  # current level = start + index
        0x48008e68: 0x6b0102bf,  # compare current level with end
        0x48008e70: 0xf8767b60,  # state record for current level
        0x48008e74: 0xb4000100,  # skip null state record
        0x48008e78: 0xf861dae4,  # callback_table[current_affinity]
        0x48008e88: 0xd63f0080,  # indirect wrapper call
        0x48008e94: 0x910006d6,  # increment index
        0x48008e98: 0x17fffff3,  # loop
        0x4800299c: 0x12800060,  # affinst_suspend returns -4 after save path
        0x48009768: 0x3100101f,  # compare return against -4
        0x48009770: 0x54000081,  # branch if not equal
        0x48009774: 0x97ffe0a4,  # -4 path -> 0x48001a04
    }
    for address, expected in suspend_dispatch_words.items():
        actual = read_word(raw, address)
        if actual != expected:
            raise RuntimeError(
                "SYSTEM_SUSPEND transition mismatch at 0x%08x: got 0x%08x expected 0x%08x"
                % (address, actual, expected)
            )
    if decode_bl_target(0x48009774, read_word(raw, 0x48009774)) != 0x48001a04:
        raise RuntimeError("SYSTEM_SUSPEND -4 power-entry target changed")
    if decode_bl_target(0x48001b5c, read_word(raw, 0x48001b5c)) != 0x48016128:
        raise RuntimeError("SYSTEM_SUSPEND trampoline preparation helper changed")
    cache_control_words = {
        0x4801612c: 0xd53e1000,  # mrs x0, SCTLR_EL3
        0x48016130: 0x8a210000,  # bic x0, x0, #5 (clear M and C)
        0x48016134: 0xd51e1000,  # msr SCTLR_EL3, x0
        0x48016138: 0xd5033fdf,  # isb
        0x48016140: 0x17ffffda,  # enter CLIDR-based cache-level maintenance
        0x480160a8: 0xd5390029,  # mrs x9, CLIDR_EL1
        0x48015fb8: 0xb40003e3,  # cache-level loop
        0x48015fe4: 0xd51a000a,  # msr CSSELR_EL1, x10
        0x48015fec: 0xd5390001,  # mrs x1, CCSIDR_EL1
        0x48016040: 0xd508764b,  # dc isw, x11
        0x48016060: 0xd5087e4b,  # dc cisw, x11
        0x48016080: 0xd5087a4b,  # dc csw, x11
    }
    for address, expected in cache_control_words.items():
        if read_word(raw, address) != expected:
            raise RuntimeError("SCTLR/cache maintenance instruction changed at 0x%08x" % address)
    if read_word(raw, 0x48001b64) != 0xd2803700:
        raise RuntimeError("SYSTEM_SUSPEND SRAM target MOV changed")
    if read_word(raw, 0x48001b6c) != 0xf2a00040:
        raise RuntimeError("SYSTEM_SUSPEND SRAM target MOVK changed")
    if read_word(raw, 0x48001b70) != 0xd61f0000:
        raise RuntimeError("SYSTEM_SUSPEND indirect SRAM branch changed")
    sram_target = 0x1b8 | (0x2 << 16)
    if sram_target != 0x000201b8:
        raise RuntimeError("SYSTEM_SUSPEND SRAM target arithmetic changed")
    normal_powerdown_words = {
        0x48015d70: 0xd5033f9f,  # dsb sy
        0x48015d74: 0xd503207f,  # wfi
        0x48015d78: 0x17ffffff,  # b 0x48015d74
    }
    for address, expected in normal_powerdown_words.items():
        if read_word(raw, address) != expected:
            raise RuntimeError("ordinary suspend DSB/WFI loop changed at 0x%08x" % address)
    dispatch_words = {
        0x48009684: 0xf947e021,  # global ops pointer load
        0x48009688: 0xf9400021,  # slot 0 (+0x00)
        0x48009690: 0xd63f0020,  # slot 0 indirect call
        0x48008608: 0xf9400433,  # slot 1 (+0x08)
        0x480080f8: 0xf9400835,  # slot 2 (+0x10)
        0x48008118: 0xd63f02a0,
        0x48008a84: 0xf9400c26,  # slot 3 (+0x18)
        0x48008a90: 0xd63f00c0,
        0x4800854c: 0xf9401003,  # slot 4 (+0x20)
        0x48008560: 0xd63f0060,
        0x48008918: 0xf9401421,  # slot 5 (+0x28)
        0x4800893c: 0xd63f0060,
        0x48009718: 0xf947e000,  # SYSTEM_SUSPEND global ops pointer load
        0x4800971c: 0xf9402000,  # slot 8 (+0x40)
        0x48009720: 0xb5000120,  # null check
        0x48009744: 0xd63f0000,  # indirect getter call
        0x48002544: 0x52a04020,  # mov w0, #0x02010000
        0x48002548: 0xd65f03c0,
        0x48002588: 0x97fffe50,  # system_reset -> helper
        0x480025a8: 0x97fffe4c,  # system_off -> helper
        0x4800975c: 0x97fffd8f,  # getter result -> suspend transition
    }
    for address, expected in dispatch_words.items():
        actual = read_word(raw, address)
        if actual != expected:
            raise RuntimeError(
                "legacy PSCI dispatch mismatch at 0x%08x: got 0x%08x expected 0x%08x"
                % (address, actual, expected)
            )
    if decode_bl_target(0x48002588, read_word(raw, 0x48002588)) != 0x48001ec8:
        raise RuntimeError("system_reset helper target changed")
    if decode_bl_target(0x480025a8, read_word(raw, 0x480025a8)) != 0x48001ed8:
        raise RuntimeError("system_off helper target changed")
    if decode_bl_target(0x4800975c, read_word(raw, 0x4800975c)) != 0x48008d98:
        raise RuntimeError("SYSTEM_SUSPEND transition target changed")
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
            "table_bytes": len(table) * 8,
            "abi_family": "legacy Allwinner TF-A plat_pm_ops_t eight-member order plus vendor get_sys_suspend_power_state extension",
            "member_order": list(OPS_MEMBER_NAMES),
            "entries": ["0x%08x" % value for value in table],
            "members": [
                {"slot": index, "byte_offset": "0x%02x" % (index * 8),
                 "name": OPS_MEMBER_NAMES[index], "address": "0x%08x" % table[index]}
                for index in range(len(table))
            ],
            "slot_0": {"name": OPS_MEMBER_NAMES[0], "address": "0x%08x" % table[0],
                       "dispatch_load_pc": "0x48009688", "indirect_call_pc": "0x48009690"},
            "slot_1": {"name": OPS_MEMBER_NAMES[1], "address": "0x%08x" % table[1],
                       "dispatch_load_pc": "0x48008608", "indirect_branch_pc": "0x48008640"},
            "slot_2": {"name": OPS_MEMBER_NAMES[2], "address": "0x%08x" % table[2],
                       "dispatch_load_pc": "0x480080f8", "indirect_call_pc": "0x48008118"},
            "slot_3": {"name": OPS_MEMBER_NAMES[3], "address": "0x%08x" % table[3],
                       "dispatch_load_pc": "0x48008a84", "indirect_call_pc": "0x48008a90"},
            "slot_4": {"name": OPS_MEMBER_NAMES[4], "address": "0x%08x" % table[4],
                       "dispatch_load_pc": "0x4800854c", "indirect_call_pc": "0x48008560"},
            "slot_5": {"name": OPS_MEMBER_NAMES[5], "address": "0x%08x" % table[5],
                       "dispatch_load_pc": "0x48008918", "indirect_call_pc": "0x4800893c"},
            "slot_6": {"name": OPS_MEMBER_NAMES[6], "address": "0x%08x" % table[6],
                       "entry_helper_call_pc": "0x480025a8", "entry_helper_target": "0x48001ed8"},
            "slot_7": {"name": OPS_MEMBER_NAMES[7], "address": "0x%08x" % table[7],
                       "reset_log_function_pc": "0x48002584",
                       "entry_helper_call_pc": "0x48002588", "entry_helper_target": "0x48001ec8"},
            "slot_8": {"name": OPS_MEMBER_NAMES[8], "address": "0x%08x" % table[8],
                       "return_pc": "0x48002544", "return_value": "0x02010000",
                       "dispatch_load_pc": "0x4800971c", "indirect_call_pc": "0x48009744"},
            "registration_function": "0x480029bc",
            "registration_callsite": "0x48009dec",
        },
        "dispatch": {
            "global_pointer_address": "0x4801bfc0",
            "global_string": "psci_plat_pm_ops",
            "member_order": list(OPS_MEMBER_NAMES),
            "member_loads": {
                "slot_0": {"offset": "0x00", "pc": "0x48009688"},
                "slot_1": {"offset": "0x08", "pc": "0x48008608"},
                "slot_2": {"offset": "0x10", "pc": "0x480080f8"},
                "slot_3": {"offset": "0x18", "pc": "0x48008a84"},
                "slot_4": {"offset": "0x20", "pc": "0x4800854c"},
                "slot_5": {"offset": "0x28", "pc": "0x48008918"},
                "slot_8": {"offset": "0x40", "pc": "0x4800971c"},
            },
            "indirect_calls": {
                "slot_0": "0x48009690",
                "slot_2": "0x48008118",
                "slot_3": "0x48008a90",
                "slot_4": "0x48008560",
                "slot_5": "0x4800893c",
                "slot_8": "0x48009744",
            },
            "slot_1_indirect_branch_pc": "0x48008640",
            "system_suspend_entry_transition_pc": "0x4800975c",
            "system_suspend_entry_transition_target": "0x48008d98",
            "psci_system_suspend_string_present": True,
            "psci_plat_pm_ops_getter_string_present": True,
        },
        "disassembly_file": "PSCI-CALLBACK-DISASSEMBLY.txt",
        "power_management_path_candidates": {
            "slot_3_function": {
                "entry": "0x%08x" % table[3],
                "semantic_member": OPS_MEMBER_NAMES[3],
                "save_context_callsite": "0x48002998",
                "save_context_target": "0x48001790",
                "ccu_candidates": ["0x03001000", "0x03001a78", "0x03001a8c"],
                "save_call_within_slot3_function": table[3] <= 0x48002998 < 0x480029b8,
            },
            "slot_5_function": {
                "entry": "0x%08x" % table[5],
                "semantic_member": OPS_MEMBER_NAMES[5],
                "restore_context_callsite": "0x48002858",
                "restore_context_target": "0x480018d8",
                "tail_branch_callsite": "0x48002874",
                "tail_branch_target": "0x%08x" % decode_b_target(0x48002874, read_word(raw, 0x48002874)),
                "next_member": OPS_MEMBER_NAMES[4],
                "restore_call_within_slot5_function": table[5] <= 0x48002858 < 0x48002878,
            },
            "slot8_system_suspend_getter": {
                "entry": "0x%08x" % table[8],
                "member": OPS_MEMBER_NAMES[8],
                "return_instruction_pc": "0x48002544",
                "return_value": "0x02010000",
                "caller_function": "0x480096f8",
                "global_ops_pointer_load_pc": "0x48009718",
                "member_pointer_load_pc": "0x4800971c",
                "indirect_call_pc": "0x48009744",
                "next_suspend_dispatch_pc": "0x4800975c",
                "next_suspend_dispatch_target": "0x48008d98",
            },
            "system_suspend_transition": {
                "entry": "0x48008d98",
                "start_affinity_level_register": "w3",
                "end_affinity_level_register": "w4",
                "system_suspend_start_affinity_level": 0,
                "system_suspend_end_affinity_level": 2,
                "callback_table_address": "0x48017d70",
                "callback_table": [
                    {"affinity_level": 0, "wrapper": "0x48008b84",
                     "platform_member": "affinst_suspend", "platform_slot": 3,
                     "platform_dispatch_call_pc": "0x48008c54"},
                    {"affinity_level": 1, "wrapper": "0x48008ab8",
                     "platform_member": "affinst_suspend", "platform_slot": 3,
                     "platform_dispatch_call_pc": "0x48008b40"},
                    {"affinity_level": 2, "wrapper": "0x48008a00",
                     "platform_member": "affinst_suspend", "platform_slot": 3,
                     "platform_dispatch_call_pc": "0x48008a90"},
                ],
                "loop": "ascending from start level to end level; skip null state records; indirect-call callback_table[current_level]; stop on nonzero callback result",
                "special_return": -4,
                "special_return_handler_call_pc": "0x48009774",
                "special_return_handler_target": "0x48001a04",
                "power_handler_disassembly": "included as system_suspend_power_entry",
                "final_helper_call_pc": "0x48001b5c",
                "final_helper_target": "0x48016128",
                "final_indirect_branch_pc": "0x48001b70",
                "final_indirect_branch_register": "x0",
                "final_indirect_branch_target": "0x00000000000201b8",
                "final_target_region": "within the sun50iw9p1 SRAM A1 candidate region [0x20000,0x28000); live contents and code identity remain unverified",
                "el3_control_before_branch": {
                    "sctlr_el3_read_pc": "0x4801612c",
                    "sctlr_el3_clear_mask": "0x5",
                    "sctlr_el3_write_pc": "0x48016134",
                    "isb_pc": "0x48016138",
                    "mmu_enable_bit_cleared": True,
                    "data_cache_enable_bit_cleared": True,
                    "cache_maintenance_entry": "0x480160a8",
                },
                "cache_maintenance": {
                    "clidr_read_pc": "0x480160a8",
                    "cache_level_loop_pc": "0x48015fb8",
                    "csselr_write_pc": "0x48015fe4",
                    "ccsidr_read_pc": "0x48015fec",
                    "dc_isw_pc": "0x48016040",
                    "dc_cisw_pc": "0x48016060",
                    "dc_csw_pc": "0x48016080",
                    "interpretation": "EL3 cache-level and set/way maintenance code; exact runtime cache-level/operation path is not dynamically observed",
                },
                "ordinary_powerdown_path": {
                    "helper": "0x48015d70",
                    "dsb_pc": "0x48015d70",
                    "wfi_pc": "0x48015d74",
                    "wfi_loop_branch_pc": "0x48015d78",
                    "sequence": "DSB SY; WFI; branch back to WFI",
                },
            },
            "interpretation": "eight legacy affinst/system power callbacks followed by a vendor get_sys_suspend_power_state extension; dispatch offsets and the exact embedded getter string identify slot 8",
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
            "The matched BL31 registers a nine-entry platform-ops table at 0x480165f8 through 0x48009dec -> 0x480029bc.",
            "The first eight entries match the legacy Allwinner TF-A plat_pm_ops_t order: affinst_standby, affinst_on, affinst_off, affinst_suspend, affinst_on_finish, affinst_suspend_finish, system_off and system_reset.",
            "The ninth entry is the vendor get_sys_suspend_power_state extension: the exact member string is present, SYSTEM_SUSPEND loads table byte offset +0x40 and indirectly calls the function at 0x48002544.",
            "The getter returns encoded power state 0x02010000 and its caller passes that value to transition function 0x48008d98.",
            "Slots 0-5 are independently selected by matching table byte offsets and indirect call sites; slot 7 references Sunxi System Reset messages and calls reset helper 0x48001ec8, while slot 6 calls power-off helper 0x48001ed8.",
            "Slot 1 affinst_on reaches CPU_ON helper 0x48002d78, but initial BL31-to-BL33 core release is not established.",
            "Slot 3 affinst_suspend contains the conditional CCU/AON save helper; slot 5 affinst_suspend_finish contains the conditional restore helper and tail-branches to slot 4 affinst_on_finish.",
            "These are static binary ABI/control-flow findings. They do not prove suspend/resume, CPU_ON, system-off or system-reset execution on the tested device.",
            "CCU save/restore helpers do not establish a BL33 startup PLL transition or an exposed CPU DVFS API.",
            "The exact vendor TF-A source commit is not recovered; effective callback semantics are identified from table order, dispatcher offsets, embedded strings and function behavior.",
            "The live VER_REG selector value, secondary A53 reset/hold state and BL31-to-BL33 memory ownership remain runtime questions.",
            "No hardware MMIO, flash, boot-media write or payload execution was performed.",
        ],
    }

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n",
                        encoding="utf-8")
    disassembly_path = args.out.parent / "PSCI-CALLBACK-DISASSEMBLY.txt"
    disassembly_path.write_text(render_disassembly(raw), encoding="utf-8")
    markdown = [
        "# Matched BL31 legacy nine-member PSCI platform-ops audit", "",
        "- Monitor: 107,216 bytes; SHA-256 `%s`." % digest,
        "- Platform-ops table `0x480165f8` has nine 64-bit entries: affinst_standby, affinst_on, affinst_off, affinst_suspend, affinst_on_finish, affinst_suspend_finish, system_off, system_reset, get_sys_suspend_power_state.",
        "- Slots 0–5 are selected by dispatcher loads at table offsets +0x00, +0x08, +0x10, +0x18, +0x20 and +0x28; their load/call sites are pinned in the JSON report.",
        "- Slot 1 affinst_on reaches CPU_ON helper `0x48002d78`; slot 3 affinst_suspend saves CCU state through `0x48001790`; slot 5 affinst_suspend_finish restores via `0x480018d8` and tail-branches to slot 4 affinst_on_finish.",
        "- Slot 7 system_reset references `Sunxi System Reset` messages and calls helper `0x48001ec8`; slot 6 system_off calls helper `0x48001ed8`.",
        "- SYSTEM_SUSPEND loads slot 8 at byte offset +0x40 (PC `0x4800971c`), calls it indirectly (PC `0x48009744`), receives `0x02010000`, then enters transition `0x48008d98`.",
        "- Transition `0x48008d98` iterates affinity levels from start to end through callback map `0x48017d70`; the three wrappers `0x48008b84/0x48008ab8/0x48008a00` dispatch platform slot 3 `affinst_suspend`.",
        "- For system state `0x02010000`, slot 3's CCU-save branch returns `-4`; caller checks that sentinel and invokes power-entry target `0x48001a04` at `0x48009774`.",
        "- Ordinary suspend uses `0x48015d70`: `DSB SY` → `WFI` → branch back to WFI.",
        "- The special handler `0x48001a04` prepares context at `0x20000`, clears SCTLR_EL3 M/C via `0x48016128`, runs CLIDR/CCSIDR cache set/way maintenance, then computes target `0x201b8` and executes `br x0`.",
        "- The target `0x201b8` lies in the `sun50iw9p1` SRAM A1 map `[0x20000,0x28000)`; the actual live SRAM contents and resume-code copy/installation path are not captured, so the branch target's code identity remains unverified.",
        "- The effective ABI is the legacy Allwinner `plat_pm_ops_t` order with a vendor getter extension; the exact source commit is not identified.",

        "- Slot 1 affinst_on calls CPU_ON helper `0x48002d78` at `0x4800255c` when `W3 == 0`.",
        "- The exact embedded strings `psci_plat_pm_ops->get_sys_suspend_power_state` and `psci_system_suspend` are present in the matched monitor.",
        "- Raw slot 3 entry 0x48002878 contains the conditional CCU save call at 0x48002998 -> 0x48001790. Separate raw slot 5 entry 0x480026c4 contains the conditional CCU restore call at 0x48002858 -> 0x480018d8; its epilogue at 0x48002874 branches to raw slot 4 entry 0x48002680.",
        "- Full disassembly windows for all table entries and both CCU helpers are saved in PSCI-CALLBACK-DISASSEMBLY.txt for function-boundary/CFG review.",
        "- Slot 3 and slot 5 are identified as affinst_suspend and affinst_suspend_finish by the legacy dispatcher offsets; their branches are conditional and runtime execution/cold-boot reachability remain unproven. No BL33 startup clock transition or CPU DVFS support is established.",
        "- Low-level routine writes a per-core RVBAR and contains a CPU power/reset sequence; one SoC-selected variant uses `0x09010000`, `0x09010010`, `0x07000440`, `0x07000444` and `0x090100c0`.",
        "- RVBAR selector cross-check: BL31 uses VER_REG low3==0 for base 0x09010040, otherwise 0x08100040; pinned native SPL uses low3==2 for the alternate bank. Both agree only for selector values 0 and 2; the target value must be read.",
        "",
        "## Interpretation", "",
        "- The effective callback ABI is the legacy Allwinner `plat_pm_ops_t` order plus the vendor `get_sys_suspend_power_state` extension; do not apply modern `plat_psci_ops_t` slot offsets.",
        "- Callback semantics are mapped from binary table entries, dispatcher byte offsets, indirect calls, embedded strings and helper behavior; the exact vendor source commit remains unknown.",
        "- This proves the matched BL31 exposes a CPU_ON-capable callback. It does **not** prove the path runs automatically at initial BL31-to-BL33 handoff.",
        "- The exact SoC variant selected by `0x03000024 & 7` was not read on-device. Do not assume the H616-named register branch is the RG34XX H700 active branch.",
        "- The effective nine-member legacy callback layout is identified from this exact binary; the exact vendor TF-A source commit/build revision remains unknown.",
        "- Initial CPU1-CPU3 reset state and the moment Linux/Linkslot requests CPU_ON remain runtime questions.",
        "- No hardware MMIO, flash, boot-media write or payload execution occurred.", "",
    ]
    (args.out.parent / "PSCI-CPU-ON.md").write_text("\n".join(markdown), encoding="utf-8")
    print("PASS: PSCI suspend dispatch, DSB/WFI and EL3 cache teardown before SRAM branch")
    print("  table[1] = 0x%08x -> low-level 0x48002d78" % table[1])
    print("  callback disassembly = %s" % disassembly_path)
    print("  monitor SHA-256 = %s" % digest)
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
