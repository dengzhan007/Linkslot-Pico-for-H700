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
    # AArch64 instruction words: LDR/STR W0,[X0,#0xa4] and
    # LDR/STR X0,[X0,#0x668], respectively.
    import struct
    synthetic = struct.pack(
        "<IIIIIIII",
        0xB940A400,
        0xFFFFFFFF,  # invalid/unallocated word must not stop the scan
        0xB900A400,  # str w0, [x0, #0xa4]
        0xF9433400,  # ldr x0, [x0, #0x668]
        0xF9033400,  # str x0, [x0, #0x668]
        0xF9005020,  # str x0, [x1, #0xa0] overlaps +0xa4
        0x29140440,  # stp w0, w1, [x2, #0xa0] second word overlaps +0xa4
        0xA9098440,  # stp x0, x1, [x2, #0x98] second qword overlaps +0xa4
    )
    writer_scan = MODULE.scan_direct_context_writes(synthetic)
    assert writer_scan["scanned_aligned_word_slots"] == 8
    assert len(writer_scan["context_plus_a4_direct_load_candidates"]) == 1
    assert len(writer_scan["context_plus_a4_direct_store_candidates"]) == 1
    assert len(writer_scan["context_plus_a4_overlapping_store_candidates"]) == 4
    assert len(writer_scan["global_slot_plus_668_load_candidates"]) == 1
    assert len(writer_scan["global_slot_plus_668_store_candidates"]) == 1
    assert len(writer_scan["global_slot_plus_668_overlapping_store_candidates"]) == 1
    assert 0x4801571C in MODULE.REVIEWED_NON_RTC_SELECTOR_OVERLAPS

    # Alias chain: ADRP x20,0x48014000 -> LDR x0,[x20,#0x668]
    # (context pointer) -> STR w1,[x0,#0xa4]; followed by a global-slot
    # store through the absolute ADRP-derived address.
    alias_chain = struct.pack(
        "<IIIII",
        0x900000B4,
        0xF9433680,
        0xB900A401,
        0xF9033680,
        0xD65F03C0,
    )
    alias_scan = MODULE.scan_context_pointer_alias_writes(alias_chain)
    assert len(alias_scan["global_slot_pointer_load_aliases"]) == 1
    assert len(alias_scan["context_plus_a4_alias_store_candidates"]) == 1
    assert len(alias_scan["global_slot_plus_668_alias_store_candidates"]) == 1

    # A conditional branch must preserve the fall-through alias chain.
    conditional_chain = struct.pack(
        "<IIII",
        0x900000B4,  # adrp x20, 0x48014000
        0x34000082,  # cbz w2, +0x10
        0xF9433680,  # ldr x0, [x20, #0x668]
        0xB900A401,  # str w1, [x0, #0xa4]
    )
    conditional_scan = MODULE.scan_context_pointer_alias_writes(conditional_chain)
    assert len(conditional_scan["global_slot_pointer_load_aliases"]) == 1
    assert len(conditional_scan["context_plus_a4_alias_store_candidates"]) == 1

    # CFG scan follows direct BL targets and transfers x0-x7 aliases to
    # the callee. It also detects a write to the global pointer slot.
    cfg_call_chain = struct.pack(
        "<IIIIIIIII",
        0x900000B4,  # adrp x20, 0x48014000 at BASE
        0xF9433680,  # ldr x0, [x20, #0x668]
        0x94000002,  # bl BASE+0x10
        0xD65F03C0,  # ret (caller continuation)
        0xB900A401,  # callee: str w1, [x0, #0xa4]
        0xD65F03C0,  # ret
        0x900000B5,  # adrp x21, 0x48014000
        0xF90336A0,  # str x0, [x21, #0x668]
        0xD65F03C0,  # ret
    )
    cfg_scan = MODULE.scan_context_pointer_alias_cfg_writes(cfg_call_chain)
    assert cfg_scan["seed_count"] == 2
    assert len(cfg_scan["global_slot_pointer_load_aliases"]) == 1
    assert len(cfg_scan["context_plus_a4_alias_store_candidates"]) == 1
    assert len(cfg_scan["global_slot_plus_668_alias_store_candidates"]) == 1

    # Alternate ADRP page + SUB computes the same global slot address.
    alternate_page_chain = struct.pack(
        "<IIIII",
        0xB00000B4,  # adrp x20, 0x48015000 (linked at BASE)
        0xD1266294,  # sub x20, x20, #0x998 -> 0x48014668
        0xF9400280,  # ldr x0, [x20]
        0xB900A401,  # str w1, [x0, #0xa4]
        0xD65F03C0,
    )
    alternate_scan = MODULE.scan_context_pointer_alias_cfg_writes(alternate_page_chain)
    assert alternate_scan["seed_count"] == 1
    assert len(alternate_scan["global_slot_pointer_load_aliases"]) == 1
    assert len(alternate_scan["context_plus_a4_alias_store_candidates"]) == 1

    # ADRP + shifted ADD + ADD reaches the slot without naming its page directly.
    shifted_add_chain = struct.pack(
        "<IIIIII",
        0xF0000094,  # adrp x20, 0x48013000 (linked at BASE)
        0x91400694,  # add x20, x20, #1, lsl #12
        0x9119A294,  # add x20, x20, #0x668
        0xF9400280,  # ldr x0, [x20]
        0xB900A401,  # str w1, [x0, #0xa4]
        0xD65F03C0,
    )
    shifted_scan = MODULE.scan_context_pointer_alias_cfg_writes(shifted_add_chain)
    assert shifted_scan["seed_count"] == 1
    assert len(shifted_scan["global_slot_pointer_load_aliases"]) == 1
    assert len(shifted_scan["context_plus_a4_alias_store_candidates"]) == 1

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
