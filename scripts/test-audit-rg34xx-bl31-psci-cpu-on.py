#!/usr/bin/env python3
"""Host tests for BL31 PSCI CPU_ON instruction target decoding."""
from __future__ import annotations
import importlib.util
from pathlib import Path

root = Path(__file__).resolve().parents[1]
path = root / "scripts/audit-rg34xx-bl31-psci-cpu-on.py"
spec = importlib.util.spec_from_file_location("bl31_psci_cpu_on_audit", path)
if spec is None or spec.loader is None:
    raise RuntimeError("cannot load PSCI CPU_ON audit")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

def main() -> int:
    assert module.decode_bl_target(0x48009dec, 0x97ffe2f4) == 0x480029bc
    assert module.decode_bl_target(0x4800255c, 0x94000207) == 0x48002d78
    assert module.decode_bl_target(0x48002d8c, 0x97ffffe9) == 0x48002d30
    assert module.decode_bl_target(0x48002d90, 0x97ffffe2) == 0x48002d18
    try:
        module.decode_bl_target(0x1000, 0x14000008)
    except RuntimeError:
        pass
    else:
        raise AssertionError("B instruction was incorrectly accepted as BL")
    print("PASS: PSCI registration, callback and helper BL target decoding")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
