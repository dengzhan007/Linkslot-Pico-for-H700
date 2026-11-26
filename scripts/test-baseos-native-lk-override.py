#!/usr/bin/env python3
"""Host regression for the pinned BaseOS native LK BL33 override patcher."""
from __future__ import annotations

import ast
import importlib.util
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PATCHER_PATH = ROOT / "scripts/patch-baseos-native-lk-override.py"
SPEC = importlib.util.spec_from_file_location("baseos_lk_override_patcher", PATCHER_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("cannot load BaseOS LK override patcher")
PATCHER = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = PATCHER
SPEC.loader.exec_module(PATCHER)


def branch_word(pc: int, target: int) -> int:
    delta = target - (pc + 8)
    if delta & 3:
        raise ValueError("A32 branch target must be word aligned")
    return 0xEA000000 | ((delta >> 2) & 0x00FFFFFF)


def valid_raw_image(size: int = 0x2000) -> bytes:
    data = bytearray(size)
    struct.pack_into("<I", data, 0, branch_word(PATCHER.IMAGE_BASE, PATCHER.IMAGE_BASE + 0x1024))
    for index in range(8):
        offset = PATCHER.VECTOR_OFFSET + index * 4
        pc = PATCHER.IMAGE_BASE + offset
        struct.pack_into("<I", data, offset, branch_word(pc, PATCHER.IMAGE_BASE + 0x1100))
    return bytes(data)


BASEOS_BUILD_FIXTURE = '''import os
import struct
from pathlib import Path

def fnv(data):
    result = 2166136261
    for byte in data: result = (result ^ byte) * 16777619 & 0xffffffff
    return result


def build(inventory, output, persistent=False, raw_capsule=False, kernel_probes=False):
    inputs = [Path(__file__)]
    source_boot0 = (inventory / "boot0.bin").read_bytes()
    generation = b"fixture-generation"
    blob = bytearray((output / "loader.bin").read_bytes())
    match = None
    assert bool(match) == kernel_probes
    blob[4:0x640] = b"H" * 0x63c
    blob += bytes(-len(blob) % 512)
    assert len(blob) <= 0xe0000
    report = dict(durable_install_eligible=False)
    if persistent or raw_capsule:
        report["source_boot0_sha256"] = "fixture"
'''


def main() -> int:
    checks = 0
    raw = valid_raw_image()
    PATCHER._validate_h700_lk_raw(raw)
    checks += 1

    for bad, label in (
        (valid_raw_image(0x8001), "oversize"),
        (valid_raw_image()[:0x1000], "missing vectors"),
    ):
        try:
            PATCHER._validate_h700_lk_raw(bad)
        except ValueError:
            checks += 1
        else:
            raise AssertionError(f"{label} image unexpectedly accepted")

    bad_header = bytearray(valid_raw_image())
    bad_header[0x10] = 1
    try:
        PATCHER._validate_h700_lk_raw(bytes(bad_header))
    except ValueError:
        checks += 1
    else:
        raise AssertionError("non-zero reserved header unexpectedly accepted")

    bad_branch = bytearray(valid_raw_image())
    struct.pack_into("<I", bad_branch, 0, 0)
    try:
        PATCHER._validate_h700_lk_raw(bytes(bad_branch))
    except ValueError:
        checks += 1
    else:
        raise AssertionError("invalid entry branch unexpectedly accepted")

    patched = PATCHER.patch_source(BASEOS_BUILD_FIXTURE)
    ast.parse(patched)
    assert PATCHER.PATCH_MARKER in patched
    assert "inputs.append(lk_bl33_override_path)" in patched
    assert "blob = bytearray(lk_bl33_override_data)" in patched
    assert "blob.extend(bytes(0x8000 - len(blob)))" in patched
    assert "'loader_extent_preserved': len(blob) == 0x8000" in patched
    assert patched.index("inputs.append(lk_bl33_override_path)") < patched.index(
        'source_boot0 = (inventory / "boot0.bin").read_bytes()'
    )
    assert patched.index("blob = bytearray(lk_bl33_override_data)") < patched.index(
        'blob[4:0x640] = b"H" * 0x63c'
    )
    checks += 1

    if PATCHER.patch_source(patched) != patched:
        raise AssertionError("patch application is not idempotent")
    checks += 1

    try:
        PATCHER.patch_source(BASEOS_BUILD_FIXTURE.replace('    source_boot0 = (inventory / "boot0.bin").read_bytes()\n', ""))
    except RuntimeError:
        checks += 1
    else:
        raise AssertionError("missing pinned-source anchor unexpectedly accepted")

    print(f"PASS: {checks} BaseOS native LK override checks")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
