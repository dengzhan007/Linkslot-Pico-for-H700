#!/usr/bin/env python3
"""Unit tests for the BL31 DRAM-context helper arithmetic."""
import importlib.util
from pathlib import Path

PATH = Path(__file__).with_name("audit-rg34xx-bl31-dram-context.py")
SPEC = importlib.util.spec_from_file_location("dram_context_audit", PATH)
MOD = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MOD)

def main():
    # Captured RG34XX primary LPDDR4 geometry: 15 rows, 10 columns, 1 GiB.
    words = [0] * 32
    words[0], words[1] = 672, 8
    words[6], words[7], words[30] = 0x30FA, 0x04000000, 0x02008C60
    result = MOD.derive(words)
    assert result["helper_result"] == "0x00000400"
    assert result["word7_after_helper"] == "0x04000040"
    assert result["midpoint_selector_mib"] == 0x200
    assert result["second_source_window"] == ["0x60000000", "0x60000140"]

    # A candidate with a zero high halfword at word[30] takes the same branch
    # when the low three bits remain clear.
    words[30] = 0x00008C60
    assert MOD.derive(words)["helper_result"] == "0x00000400"

    # Verify the low-nibble branch and the optional 3/4 scaling are modeled.
    assert MOD.helper_result(0x30FA, 0x04000001, 0x02008C61) != 0x400
    print("PASS: BL31 geometry helper yields the 1-GiB midpoint for captured RG34XX parameters")

if __name__ == "__main__":
    main()
