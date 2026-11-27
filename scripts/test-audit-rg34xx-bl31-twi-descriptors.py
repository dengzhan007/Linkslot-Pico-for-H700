#!/usr/bin/env python3
"""Host regressions for the RTC TWI descriptor audit helpers."""
from __future__ import annotations

import importlib.util
from pathlib import Path

SCRIPT = Path(__file__).with_name("audit-rg34xx-bl31-twi-descriptors.py")
SPEC = importlib.util.spec_from_file_location("twi_descriptor_audit", SCRIPT)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("unable to load TWI descriptor audit module")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def main() -> int:
    raw = bytes.fromhex("8877665544332211")
    assert MODULE.read_u64(raw, MODULE.BASE) == 0x1122334455667788
    assert MODULE.classify_pointer(0x48002510) == "monitor-internal-address"
    assert MODULE.classify_pointer(0x07081400) == "soc-mmio-candidate"
    assert MODULE.classify_pointer(0x05002000) == "soc-mmio-candidate"
    assert MODULE.H616_TWI_BASES == {
        0x05002000: "TWI0",
        0x05002400: "TWI1",
        0x05002800: "TWI2",
        0x05002C00: "TWI3",
        0x05003000: "TWI4",
        0x07081400: "R-TWI0",
    }
    assert MODULE.classify_pointer(2) == "small-scalar-or-offset"
    assert MODULE.classify_pointer(0x000186A00000000B) == "unclassified-scalar-or-encoded-value"
    source = """
      memcpy(monitor + 0x30, params, 128);
      memcpy(monitor + 0x24, board, 8);
      *(u32 *)(monitor + 0xb0) = dtb;
    """
    assert MODULE.parse_monitor_patch_offsets(source) == [0x24, 0x30, 0xB0]
    print("PASS: qword reader, address classification and SPL patch-offset parser")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
