#!/usr/bin/env python3
"""Read only SYS_CFG VER_REG[2:0] and compare SPL/BL31 RVBAR banks."""
from __future__ import annotations
import argparse
import json
import os
import struct
import sys

REGISTER = 0x03000024
PRIMARY = 0x09010040
ALTERNATE = 0x08100040
SOURCE_COMMIT = "cbe253a43f46cf551cc9a6c79a84015bde3686cf"

def classify(selector: int) -> dict:
    if not 0 <= selector <= 7:
        raise ValueError("selector must be in range 0..7")
    spl = ALTERNATE if selector == 2 else PRIMARY
    bl31 = PRIMARY if selector == 0 else ALTERNATE
    return {
        "selector_low3": selector,
        "spl_rvbar_base": "0x%08x" % spl,
        "bl31_psci_rvbar_base": "0x%08x" % bl31,
        "rules_match": spl == bl31,
    }

def read_selector(device: str = "/dev/mem") -> int:
    if not hasattr(os, "pread"):
        raise OSError("os.pread unavailable; this probe requires Linux/Android")
    if hasattr(os, "geteuid") and os.geteuid() != 0:
        raise PermissionError("root required to read /dev/mem")
    flags = os.O_RDONLY | getattr(os, "O_CLOEXEC", 0)
    fd = os.open(device, flags)
    try:
        raw = os.pread(fd, 4, REGISTER)
    finally:
        os.close(fd)
    if len(raw) != 4:
        raise OSError("short read at SYS_CFG VER_REG")
    # Only return bits [2:0]. Never print or persist the full register word.
    return struct.unpack("<I", raw)[0] & 7

def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--device", default="/dev/mem")
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args()
    try:
        selector = read_selector(args.device)
    except (OSError, PermissionError) as exc:
        print("ERROR: read-only VER_REG[2:0] read failed: %s" % exc, file=sys.stderr)
        return 2
    result = classify(selector)
    result.update({
        "register": "SYS_CFG_VER_REG",
        "register_address": "0x03000024",
        "read_method": "single 32-bit pread using O_RDONLY",
        "baseos_spl_source_commit": SOURCE_COMMIT,
        "matched_bl31_monitor_sha256": "676bc0206e3c9a7f229d93ea7f3286095f669940ad91f3c4f3d37bdb18696c31",
        "raw_register_word_emitted": False,
        "mmio_writes_performed": False,
        "scope_note": "sample taken after Linux boot; does not prove pre-BL33 CPU reset/status state",
    })
    if args.json:
        print(json.dumps(result, sort_keys=True))
    else:
        print("H700 RVBAR selector (read-only)")
        print("VER_REG[2:0] = %d" % selector)
        print("native SPL bank = %s" % result["spl_rvbar_base"])
        print("BL31 PSCI bank  = %s" % result["bl31_psci_rvbar_base"])
        print("SPL/BL31 rules  = %s" % ("MATCH" if result["rules_match"] else "MISMATCH"))
        print("raw word output = no; MMIO writes = no")
        print("NOTE: running-Linux sample, not a pre-BL33 CPU-state snapshot")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
