#!/usr/bin/env python3
"""Host regressions for BL31 whole-payload CPU/MMIO candidate scanner."""
from __future__ import annotations
import importlib.util
import struct
from pathlib import Path

root = Path(__file__).resolve().parents[1]
path = root / "scripts/audit-rg34xx-bl31-full-image-cpu-mmio.py"
spec = importlib.util.spec_from_file_location("bl31_full_image_mmio", path)
if spec is None or spec.loader is None:
    raise RuntimeError("cannot load whole-image MMIO scanner")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

def main() -> int:
    # MOVZ X1,#0x901,LSL#16; MOVK X1,#0x10; LDR W0,[X1]; RET
    raw = b"".join(struct.pack("<I", x) for x in (
        0xD2A12021, 0xF2800201, 0xB9400020, 0xD65F03C0
    ))
    hits = module.instruction_hits(raw)
    assert any(
        item["mnemonic"] == "ldr" and item["address"] == "0x09010010"
        for item in hits
    ), hits

    literal_data = struct.pack("<II", 0x58000040, 0x090100C4)
    words = module.literal_hits(literal_data)
    assert len(words) == 1 and words[0]["value"] == "0x090100c4", words

    assert module.label(0x09010000) == "C0_CPUX_CFG"
    assert module.label(0x090100C4) == "C0_CPUX_CFG"
    assert module.label(0x09020000) is None
    print("PASS: whole-payload BL31 overlapping decode and literal-word MMIO tests")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
