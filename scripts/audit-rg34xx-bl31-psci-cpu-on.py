#!/usr/bin/env python3
"""Validate the matched RG34XX BL31 PSCI platform CPU_ON callback path."""
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
    0x4800254c: 0x35000103, 0x4800255c: 0x94000207,
    0x48002d18: 0xd2800480, 0x48002d1c: 0xf2a06000,
    0x48002d20: 0xb9400000, 0x48002d24: 0xf240081f,
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

def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--monitor", type=Path, required=True)
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
    if decode_bl_target(0x48002d8c, read_word(raw, 0x48002d8c)) != 0x48002d30:
        raise RuntimeError("per-core RVBAR helper call changed")
    if decode_bl_target(0x48002d90, read_word(raw, 0x48002d90)) != 0x48002d18:
        raise RuntimeError("SoC selection helper call changed")

    missing = [value.decode() for value in STRINGS if value not in raw]
    if missing:
        raise RuntimeError("expected embedded TF-A PSCI strings missing: %s" % missing)

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
            "slot_7": {"name": "pwr_domain_on_finish (standard plat_psci_ops_t order)",
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
            "standard_tf_a_capability_condition": "pwr_domain_on slot +8 and pwr_domain_on_finish slot +56 are both non-null",
            "slot_7_on_finish_address": "0x48002574",
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
            "The standard TF-A PSCI setup advertises PSCI_CPU_ON_AARCH64 (FID 0xc4000003) when both pwr_domain_on and pwr_domain_on_finish are non-null; this matched table contains both.",
            "This proves BL31 contains and exposes a CPU_ON-capable platform callback; it does not prove that the callback executes during initial BL31-to-BL33 handoff.",
            "The audit does not establish live CPU1-CPU3 reset state before Linux or Linkslot requests PSCI CPU_ON.",
            "The exact vendor callback ABI/source revision is not recovered; this is a pinned binary plus standard ABI-position correlation, not exact vendor source identity.",
            "No hardware MMIO, flash, boot-media write or payload execution was performed.",
        ],
    }

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n",
                        encoding="utf-8")
    markdown = [
        "# Matched BL31 PSCI CPU_ON callback audit", "",
        "- Monitor: 107,216 bytes; SHA-256 `%s`." % digest,
        "- Platform ops table: `0x480165f8`; slot +8 points to callback `0x4800254c`.",
        "- PSCI dispatch loads the global `psci_plat_pm_ops` pointer, reads its +8 slot, moves that function pointer to `x5`, then branches through `x5`.",
        "- Callback `0x4800254c` calls low-level CPU_ON routine `0x48002d78` at `0x4800255c` when `W3 == 0`.",
        "- Low-level routine writes a per-core RVBAR and contains a CPU power/reset sequence; one SoC-selected variant uses `0x09010000`, `0x09010010`, `0x07000440`, `0x07000444` and `0x090100c0`.",
        "",
        "## Interpretation", "",
        "- In the standard TF-A `plat_psci_ops_t` layout, the second function pointer is `pwr_domain_on`; embedded PSCI source strings and dispatcher references support the platform-ops interpretation.",
        "- This proves the matched BL31 exposes a CPU_ON-capable callback. It does **not** prove the path runs automatically at initial BL31-to-BL33 handoff.",
        "- The exact SoC variant selected by `0x03000024 & 7` was not read on-device. Do not assume the H616-named register branch is the RG34XX H700 active branch.",
        "- The exact vendor callback ABI/source revision is not recovered; this is a binary and standard ABI-position correlation, not exact source identity.",
        "- Initial CPU1-CPU3 reset state and the moment Linux/Linkslot requests CPU_ON remain runtime questions.",
        "- No hardware MMIO, flash, boot-media write or payload execution occurred.", "",
    ]
    (args.out.parent / "PSCI-CPU-ON.md").write_text("\n".join(markdown), encoding="utf-8")
    print("PASS: matched BL31 PSCI ops table, dispatch, CPU_ON callback and reset sequence")
    print("  table[1] = 0x%08x -> low-level 0x48002d78" % table[1])
    print("  monitor SHA-256 = %s" % digest)
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
