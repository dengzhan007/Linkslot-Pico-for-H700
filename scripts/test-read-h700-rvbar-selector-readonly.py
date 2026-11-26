#!/usr/bin/env python3
"""Host regressions for the read-only H700 RVBAR selector probe."""
from __future__ import annotations
import importlib.util
from pathlib import Path

path = Path(__file__).with_name("read-h700-rvbar-selector-readonly.py")
spec = importlib.util.spec_from_file_location("rvbar_selector_readonly", path)
if spec is None or spec.loader is None:
    raise RuntimeError("cannot load RVBAR selector collector")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

def main() -> int:
    expected = {
        0: ("0x09010040", "0x09010040", True),
        1: ("0x09010040", "0x08100040", False),
        2: ("0x08100040", "0x08100040", True),
        3: ("0x09010040", "0x08100040", False),
        4: ("0x09010040", "0x08100040", False),
        5: ("0x09010040", "0x08100040", False),
        6: ("0x09010040", "0x08100040", False),
        7: ("0x09010040", "0x08100040", False),
    }
    for value, wanted in expected.items():
        got = module.classify(value)
        actual = (got["spl_rvbar_base"], got["bl31_psci_rvbar_base"], got["rules_match"])
        assert actual == wanted, (value, actual, wanted)
    for value in (-1, 8):
        try:
            module.classify(value)
        except ValueError:
            pass
        else:
            raise AssertionError("accepted invalid selector %d" % value)
    print("PASS: RVBAR selector parity across all eight low3 values")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
