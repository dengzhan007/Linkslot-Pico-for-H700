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

    assert module.label(0x03001000) == "CPUX_CCU"
    assert module.label(0x03001520) == "CPUX_CCU"
    assert module.label(0x03001FFF) == "CPUX_CCU"
    assert module.label(0x03002000) == "SID_SYS"
    assert module.label(0x0300B000) == "PIO"
    assert module.label(0x0300B124) == "PIO"
    assert module.label(0x0300B130) == "PIO"
    assert module.label(0x0300B3FF) == "PIO"
    assert module.label(0x0300B400) == "SID_SYS"
    assert module.label(0x09010000) == "C0_CPUX_CFG"
    assert module.label(0x090100C4) == "C0_CPUX_CFG"
    assert module.label(0x09020000) is None
    assert module.pio_register(0x0300B100) == ("PH_CFG1", "PH10 mux field")
    assert module.pio_register(0x0300B110) == ("PH_DRV0", "PH10 drive field")
    assert module.pio_register(0x0300B118) == ("PH_PUL0", "PH10 pull field")
    assert module.pio_register(0x0300B124) == ("PI_CFG1", "PI12 mux field")
    assert module.pio_register(0x0300B130) == ("PI_DAT0", "PI12 data bit")
    assert module.pio_register(0x0300B134) == ("PI_DRV0", "PI12 drive field")
    assert module.pio_register(0x0300B13C) == ("PI_PUL0", "PI12 pull field")
    assert module.pio_register(0x0300B400) is None

    # The address seed and high half may be separated by more than the old
    # 16-instruction window. Keep this regression to prevent missing such
    # PIO/CCU accesses in long register-setup sequences.
    long_sequence = [
        0xD2962004, 0xD2962203, 0xD2962302,  # x4/x3/x2 low address seeds
        *([0xD503201F] * 24),                 # force >16-instruction tracking
        0xF2A06004, 0xF2A06003, 0xF2A06002,  # add 0x0300 high halves
        0xB9400080, 0xB9000080,              # load/store [x4] -> PIO+0x100
        0xB9400060, 0xB9000060,              # load/store [x3] -> PIO+0x110
        0xB9400040, 0xB9000040,              # load/store [x2] -> PIO+0x118
        0xD65F03C0
    ]
    long_raw = b"".join(struct.pack("<I", word) for word in long_sequence)
    long_hits = module.instruction_hits(long_raw)
    expected_pio_ops = {
        ("ldr", "0x0300b100"), ("str", "0x0300b100"),
        ("ldr", "0x0300b110"), ("str", "0x0300b110"),
        ("ldr", "0x0300b118"), ("str", "0x0300b118"),
    }
    actual_pio_ops = {
        (item["mnemonic"], item["address"]) for item in long_hits
        if item["window"] == "PIO"
    }
    assert expected_pio_ops <= actual_pio_ops, (expected_pio_ops, actual_pio_ops, long_hits)
    long_context = next(
        item["context"] for item in long_hits
        if item["mnemonic"] == "ldr" and item["address"] == "0x0300b100"
    )
    assert any("mov x4, #0xb100" in row for row in long_context), long_context

    print("PASS: whole-payload BL31 overlapping decode and literal-word MMIO tests")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
