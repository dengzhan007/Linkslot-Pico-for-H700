#!/usr/bin/env python3
"""Validate the published read-only RG34XX Boot0 captures against the parser."""

from __future__ import annotations

import argparse
import hashlib
import subprocess
import struct
import sys

from inspect_h700_boot0 import parse_boot0


REPORT_PATH = "reports/rg34xx-inventory-20261127-uart"
EXPECTED = {
    "boot0-primary.bin": {
        "capture_sha": "bf886995745a0eb9b5030296f9f6163035e164b3a82183716e67ff876319289f",
        "image_sha": "bf886995745a0eb9b5030296f9f6163035e164b3a82183716e67ff876319289f",
    },
    "boot0-secondary.bin": {
        "capture_sha": "5f143679e0d2c5293d22fee47a47ea6c17120c7a19e6007247c8b13710aa5ef9",
        "image_sha": "4701a85e4db7e8339b91b92f691a83cc954ec63a09369087c2e41d80eafa9f5d",
    },
}


def read_git_blob(ref: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{ref}:{path}"],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    return result.stdout


def find_table(report: dict, marker_offset: str) -> dict:
    for table in report["dram_ext_tables"]:
        if table["marker_offset"] == marker_offset:
            return table
    raise AssertionError(f"missing DRAM.ext table marker {marker_offset}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--reports-ref",
        default="refs/remotes/origin/reports",
        help="git ref containing the reports branch",
    )
    args = parser.parse_args()

    reports = {}
    raw_captures = {}
    for filename, expected in EXPECTED.items():
        path = f"{REPORT_PATH}/{filename}"
        data = read_git_blob(args.reports_ref, path)
        capture_sha = hashlib.sha256(data).hexdigest()
        if capture_sha != expected["capture_sha"]:
            raise AssertionError(
                f"{filename}: raw-capture SHA-256 {capture_sha} != "
                f"{expected['capture_sha']}"
            )
        report = parse_boot0(data)
        if report["sha256_full"] != expected["image_sha"]:
            raise AssertionError(
                f"{filename}: declared-image SHA-256 {report['sha256_full']} != "
                f"{expected['image_sha']}"
            )
        reports[filename] = report
        raw_captures[filename] = data

    primary = reports["boot0-primary.bin"]
    secondary = reports["boot0-secondary.bin"]

    assert primary["bytes"] == 0x20000, primary["bytes"]
    assert secondary["bytes"] == 0x10000, secondary["bytes"]
    for name, report in reports.items():
        active = report["dram_candidates"][0]
        assert active["dram_type"] == "LPDDR4", (name, active)
        assert active["clock_mhz_from_word0"] == 672, (name, active)

    primary_relocated = find_table(primary, "0x1348")
    secondary_table = find_table(secondary, "0x348")
    assert primary_relocated["valid_entry_count"] == 15
    # The 128 KiB primary container embeds a separate 64 KiB eGON Boot0
    # at +0x1000 and a nonstandard SPL2 component at +0x11000.
    primary_components = {item["offset"]: item for item in primary["embedded_egon_components"]}
    secondary_components = {item["offset"]: item for item in secondary["embedded_egon_components"]}
    assert set(primary_components) == {"0x0", "0x1000", "0x11000"}, primary_components
    assert set(secondary_components) == {"0x0"}, secondary_components
    assert primary_components["0x0"]["checksum_valid_standard_egon"] is True
    assert primary_components["0x1000"]["checksum_valid_standard_egon"] is True
    inner = primary_components["0x1000"]
    assert inner["declared_size"] == 0x10000
    assert inner["sha256_declared_component"] == "b6edbf39af0bf1f35a7e2fd991c8966b04d061e247c81985d2516f695464049b"
    assert inner["dram_ext_tables"][0]["marker_offset"] == "0x348"
    assert inner["dram_ext_tables"][0]["valid_entry_count"] == 15
    assert inner["active_dram_profile"]["words"] == primary_components["0x0"]["active_dram_profile"]["words"]

    spl = primary_components["0x11000"]
    assert spl["component_kind"] == "SPL2 component"
    assert spl["declared_size"] == 0xC000
    assert spl["checksum_valid_standard_egon"] is None
    assert spl["platform_string"] == "allwinner/sun50i-h700-anbernic-rg35xx-2024"

    primary_inner_bytes = raw_captures["boot0-primary.bin"][0x1000:0x11000]
    secondary_image_bytes = raw_captures["boot0-secondary.bin"][:0x10000]
    assert hashlib.sha256(primary_inner_bytes).hexdigest() == inner["sha256_declared_component"]
    assert hashlib.sha256(secondary_image_bytes).hexdigest() == "4701a85e4db7e8339b91b92f691a83cc954ec63a09369087c2e41d80eafa9f5d"
    byte_differences = [
        offset for offset, (left, right) in enumerate(zip(primary_inner_bytes, secondary_image_bytes))
        if left != right
    ]
    assert byte_differences == [0x0C, 0x0F, 0x34, 0xA3, 0xAC], byte_differences
    print("PASS: primary contains a valid nested 64 KiB Boot0 at +0x1000 and SPL2 component at +0x11000")
    print("PASS: primary inner Boot0 and secondary Boot0 differ only in checksum, debug byte, TPR6 and TPR12")
    print("REVIEW: embedded SPL2 platform label is rg35xx-2024; compatibility with RG34XX must be established, not assumed")

    # Target-specific SPL2 regulator table and PMIC I2C path.
    spl2 = raw_captures["boot0-primary.bin"][0x11000:0x1D000]
    assert len(spl2) == 0xC000
    expected_regulator_table = bytes.fromhex(
        "8001837ff40104060a46"
        "8002847ff40104060a46"
        "8004857ff40130070a46"
    )
    assert spl2[0xAAFC:0xAAFC + len(expected_regulator_table)] == expected_regulator_table
    assert struct.unpack_from("<I", spl2, 0x5F8)[0] == 0x52800680  # mov w0, #0x34 (I2C slave)
    assert struct.unpack_from("<I", spl2, 0x61C)[0] == 0x52800680  # same for write helper
    assert struct.unpack_from("<I", spl2, 0x4CE8)[0] == 0x52807580  # request 940 mV
    assert struct.unpack_from("<I", spl2, 0x4CEC)[0] == 0x9400158B  # DCDC2 setter
    assert struct.unpack_from("<I", spl2, 0x4CF4)[0] == 0x52808980  # request 1100 mV
    assert struct.unpack_from("<I", spl2, 0x4CF8)[0] == 0x9400158B  # DCDC3 setter
    assert struct.unpack_from("<I", spl2, 0x7AE0)[0] == 0x7100081F  # compare controller index 2
    assert struct.unpack_from("<I", spl2, 0x7B0C)[0] == 0xD2828000  # base low 0x1400
    assert struct.unpack_from("<I", spl2, 0x7B10)[0] == 0xF2A0E100  # base high 0x0708
    assert struct.unpack_from("<I", spl2, 0x7B14)[0] == 0xD65F03C0  # return
    assert struct.unpack_from("<Q", spl2, 0xB6B0 + 16)[0] == 0x27D30  # link-time read callback
    assert struct.unpack_from("<Q", spl2, 0xB6B0 + 24)[0] == 0x27C20  # link-time write callback
    assert struct.unpack_from("<Q", spl2, 0xB6B0 + 56)[0] == 2  # controller index -> R-TWI0
    assert struct.unpack_from("<Q", spl2, 0xB6B0 + 0x48 + 56)[0] == 2
    print("PASS: RG34XX SPL2 requests DCDC2=940 mV and DCDC3=1100 mV via PMIC I2C slave 0x34")
    print("PASS: regulator table maps DCDC2/DCDC3 to 0x84/0x85 and control masks 0x02/0x04")
    print("PASS: SPL2 controller-index 2 resolves to MMIO base 0x07081400 (R-TWI0); selected descriptor index remains externally initialized")
    # Nine 0x48-byte I2C descriptors are indexed by the context field [x18 + 264].
    descriptor_table = spl2[0xB6B0:0xB6B0 + 9 * 0x48]
    assert len(descriptor_table) == 9 * 0x48
    controller_indices = [
        struct.unpack_from("<Q", descriptor_table, index * 0x48 + 56)[0]
        for index in range(9)
    ]
    assert controller_indices == [2, 2, 0, 0, 0, 0, 0, 0, 0], controller_indices
    assert struct.unpack_from("<Q", descriptor_table, 16)[0] == 0x27D30
    assert struct.unpack_from("<Q", descriptor_table, 24)[0] == 0x27C20
    # Transaction helper reads the selector from x18+264 and indexes the table.
    assert struct.unpack_from("<I", spl2, 0x7738)[0] == 0xB9410A40
    assert struct.unpack_from("<I", spl2, 0x7740)[0] == 0x97FFFFCC
    # Table lookup computes nine entries and a 0x48-byte stride.
    for offset, expected_word in (
        (0x767C, 0x911BE042),
        (0x7680, 0x911AC000),
        (0x7688, 0xCB000042),
        (0x7698, 0x9343FC42),
        (0x76E4, 0x52800900),
        (0x76E8, 0x9B204C33),
    ):
        assert struct.unpack_from("<I", spl2, offset)[0] == expected_word, hex(offset)
    print("PASS: SPL2 I2C selector is an index into nine 0x48-byte descriptors; entries 0/1 map to R-TWI0")
    # The SPL2 DRAM init routine walks a fixed three-entry candidate table.
    # Candidate table VMA 0x5a7f4 maps to component offset 0xa7f4 at load VMA 0x50000.
    expected_candidates = (
        (672, 8, 0x08080808, 0x0E0E0E0E, 0x00000E0E, 0x7887BBBB,
         0x00000000, 0x00000001, 0x3E808080, 0x402F6633, 0x24272824, 0x06070706),
        (672, 7, 0x06060606, 0x0D0D0D0D, 0x00001919, 0x9988EEEE,
         0x00000000, 0x00000001, 0x2FB08080, 0x402F4489, 0x0E0E0F0E, 0x0F10110F),
        (672, 8, 0x08080808, 0x0E0E0E0E, 0x00000E0E, 0x7887BBBB,
         0x00000000, 0x00000001, 0x40808080, 0x402F6633, 0x1B1F1E1C, 0x06060606),
    )
    for index, expected in enumerate(expected_candidates):
        actual = struct.unpack_from("<12I", spl2, 0xA7F4 + index * 0x30)
        assert actual == expected, (index, actual, expected)
    # AArch64 SPL2 loop: indices 0..2, 0x30-byte stride, then per-type DCDC3 policy.
    for offset, expected_word in (
        (0x365C, 0x7100081F),  # cmp candidate_index, #2
        (0x3674, 0x52800600),  # mov candidate_stride, #0x30
        (0x3694, 0x94000092),  # save CPU context before candidate DRAM tests
        (0x3728, 0x7100201F),  # compare DRAM type 8
        (0x3730, 0x71001C1F),  # compare DRAM type 7
        (0x3734, 0x52809601),  # LPDDR3 DCDC3 1200 mV
        (0x3738, 0x5280BB80),  # other DRAM types DCDC3 1500 mV
        (0x3740, 0x94001AF9),  # DCDC3 setter
    ):
        assert struct.unpack_from("<I", spl2, offset)[0] == expected_word, hex(offset)
    print("PASS: SPL2 contains three 672 MHz DRAM candidates (LPDDR4, LPDDR3, LPDDR4 alternate)")
    # Candidate-specific routines invoke DRAM-controller calibration and memory scratch tests.
    for offset, expected_word in (
        (0x36B8, 0x97FFFC48),  # BL 0x527d8, candidate-specific controller test/config
        (0x36C4, 0x97FFFC6E),  # BL 0x5287c, candidate-specific scratch/test path
        (0x0990, 0xF2A06014),  # MMIO base construction for 0x03001540
        (0x28B0, 0xD2A80001),  # materialize DRAM scratch address 0x40000000
        (0x28B8, 0x940010D4),  # copy/read 64-byte scratch block
    ):
        assert struct.unpack_from("<I", spl2, offset)[0] == expected_word, hex(offset)
    print("PASS: candidate path calls DRAM controller test/config helpers and touches 0x40000000 scratch memory")
    print("PASS: SPL2 candidate loop uses indices 0..2 and 0x30-byte stride; DCDC3 policy is type-dependent")

    assert secondary_table["valid_entry_count"] == 15
    for name, table in (
        ("primary relocated DRAM.ext", primary_relocated),
        ("secondary DRAM.ext", secondary_table),
    ):
        assert table["entries"][13]["dram_type"] == "LPDDR3", (name, table)
        assert table["entries"][14]["dram_type"] == "LPDDR4", (name, table)
        assert table["entries"][13]["clock_mhz"] == 672, (name, table)
        assert table["entries"][14]["clock_mhz"] == 672, (name, table)

    print("PASS: both published RG34XX Boot0 raw captures and declared images pass SHA-256 and eGON checksum checks")
    print("PASS: active header parameter block in both copies reports LPDDR4 at 672 MHz")
    print("PASS: primary Boot0 DRAM.ext table is discovered at 0x1348 (15 profiles)")
    print("PASS: secondary Boot0 DRAM.ext table is discovered at 0x348 (15 profiles)")
    print("LIMIT: profile-table extraction does not prove runtime selection, PMIC writes, or DRAM map")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (AssertionError, subprocess.CalledProcessError) as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
