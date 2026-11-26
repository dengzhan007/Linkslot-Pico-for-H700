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
    assert module.decode_bl_target(0x48002998, 0x97fffb7e) == 0x48001790
    assert module.decode_bl_target(0x48002858, 0x97fffc20) == 0x480018d8
    assert module.decode_b_target(0x48002874, 0x17ffff83) == 0x48002680
    assert module.EXPECTED_TABLE[3] == 0x48002878
    assert module.EXPECTED_TABLE[4] == 0x48002680
    assert module.EXPECTED_TABLE[5] == 0x480026c4
    # Slot pointer values are verified, but non-CPU_ON callback semantics
    # remain unclassified until the exact vendor TF-A ABI is identified.
    assert module.EXPECTED_TABLE[7] == 0x48002574
    window_map = {name: (start, size) for name, start, size in module.DISASSEMBLY_WINDOWS}
    assert "ops_slot_0_raw" in window_map
    assert window_map["ops_slot_0_raw"][0] == 0x48002510
    assert window_map["psci_dispatch_cpu_on_primary"][0] <= 0x48008608
    assert window_map["psci_dispatch_cpu_on_secondary"][0] <= 0x480086a0
    assert window_map["psci_platform_registration"][0] <= 0x48009dec
    assert window_map["power_system_off_helper"][0] == 0x48001ec8
    assert window_map["power_cpu_off_helper"][0] == 0x48001ed8
    assert "ops_slot_3_raw" in window_map
    assert window_map["ops_slot_3_raw"][0] <= 0x48002998 < window_map["ops_slot_3_raw"][0] + window_map["ops_slot_3_raw"][1]
    assert window_map["ops_slot_5_raw"][0] <= 0x48002858 < window_map["ops_slot_5_raw"][0] + window_map["ops_slot_5_raw"][1]
    assert window_map["ops_slot_5_raw"][1] >= 0x1b4
    assert window_map["restore_callsite_context"][0] <= 0x48002858
    assert window_map["save_callsite_context"][0] <= 0x48002998
    assert window_map["ccu_save_helper"][0] == 0x48001790
    assert window_map["ccu_restore_helper"][0] == 0x480018d8
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
