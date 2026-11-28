#!/usr/bin/env python3
"""Unit tests for the captured H700 Boot0 DRAM geometry cross-check."""
from __future__ import annotations
import importlib.util
from pathlib import Path
module_path = Path(__file__).resolve().with_name("audit-rg34xx-boot0-dram-geometry.py")
spec = importlib.util.spec_from_file_location("boot0_dram_geometry_audit", module_path)
if spec is None or spec.loader is None:
    raise RuntimeError("cannot load Boot0 geometry audit module")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
build = module.build

def fixture(digest: str, size: int, debug: int) -> dict:
    return {
        "sha256_full": digest, "bytes": size, "checksum_valid": True, "debug_byte": debug,
        "dram_candidates": [{"name": "primary", "words": [
            "0x000002a0", "0x00000008", "0x08080808", "0x0e0e0e0e",
            "0x00000e0e", "0x7887bbbb", "0x000030fa", "0x04000000",
            *["0x00000000"] * 24,
        ]}],
    }

def main() -> None:
    p = fixture("bf886995745a0eb9b5030296f9f6163035e164b3a82183716e67ff876319289f", 131072, 3)
    s = fixture("4701a85e4db7e8339b91b92f691a83cc954ec63a09369087c2e41d80eafa9f5d", 65536, 0)
    mem = {"memblock": {"memory": [{"start": "0x40000000", "size_bytes": 1073741824}]}}
    r = build(p, s, mem)
    assert r["copies_agree_on_geometry"] is True
    assert r["boot0_geometry_matches_linux_runtime_capacity"] is True
    g = r["boot0_copies"][0]
    assert (g["rows"], g["columns"], g["ranks"], g["bus_width_bits"]) == (15, 10, 1, 32)
    assert g["capacity_bytes"] == 0x40000000
    assert r["bl31_second_source_window_candidate"]["start"] == "0x60000000"
    assert r["bl31_second_source_window_candidate"]["end_exclusive"] == "0x60000140"
    assert r["runtime_context_selector_captured"] is False
    assert r["memory_layout_approved"] is False
    bad = fixture("4701a85e4db7e8339b91b92f691a83cc954ec63a09369087c2e41d80eafa9f5d", 65536, 0)
    bad["dram_candidates"][0]["words"][7] = "0x02000000"
    try:
        build(p, bad, mem)
    except ValueError:
        pass
    else:
        raise AssertionError("mismatched Boot0 geometry must fail closed")
    print("Boot0 DRAM geometry cross-check tests passed")

if __name__ == "__main__":
    main()
