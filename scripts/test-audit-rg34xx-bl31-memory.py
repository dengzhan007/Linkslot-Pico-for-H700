#!/usr/bin/env python3
"""Host tests for RG34XX BL31 static audit primitives."""
from __future__ import annotations
import importlib.util
import struct
from pathlib import Path

root = Path(__file__).resolve().parents[1]
script = root / "scripts/audit-rg34xx-bl31-memory.py"
spec = importlib.util.spec_from_file_location("rg34xx_bl31_audit", script)
if spec is None or spec.loader is None:
    raise RuntimeError("cannot load BL31 audit module")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

def encode_b(pc: int, target: int) -> int:
    delta = target - pc
    if delta % 4:
        raise ValueError("A64 branch targets must be 4-byte aligned")
    return 0x14000000 | ((delta >> 2) & 0x03FFFFFF)

def main() -> int:
    branch = encode_b(0x48000000, 0x480000F8)
    assert module.decode_a64_b(struct.pack("<I", branch), 0) == 0x480000F8
    try:
        module.decode_a64_b(struct.pack("<I", 0x94000000), 0)
    except ValueError:
        pass
    else:
        raise AssertionError("BL must not be accepted as the vendor-entry B instruction")

    assert module.classify_address(0x40080000) == "Linux kernel destination / proposed app candidate"
    assert module.classify_address(0x41FFFFFF) == "Linux kernel destination / proposed app candidate"
    assert module.classify_address(0x42000000) == "Linux initrd reservation"
    assert module.classify_address(0x7EC00000) == "unknown top-of-DRAM reservation"
    assert module.classify_address(0x80000000) is None

    # B at +0 branches over non-code bytes at +4 to RET at +8.
    synthetic = struct.pack("<I", encode_b(0x48000000, 0x48000008))
    synthetic += b"\xff\xff\xff\xff"
    synthetic += struct.pack("<I", 0xD65F03C0)
    refs, listing, count = module.scan_addresses(synthetic, 0x48000000)
    assert count == 2, count
    assert "48000000:" in listing and "48000008:" in listing
    assert "48000004:" not in listing
    assert not refs

    print("PASS: BL31 branch/range tests and control-flow data-skip regression")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
