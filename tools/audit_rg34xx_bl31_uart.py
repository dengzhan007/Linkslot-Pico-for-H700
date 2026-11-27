#!/usr/bin/env python3
"""Static, hash-pinned UART/security handoff audit for the RG34XX H700 BL31 monitor.

This tool reads an extracted monitor image on the host. It never opens a device,
maps MMIO, executes monitor code, or claims that the audited paths ran on hardware.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import struct
import sys
from pathlib import Path
from typing import Any

EXPECTED_SIZE = 107_216
EXPECTED_SHA256 = "676bc0206e3c9a7f229d93ea7f3286095f669940ad91f3c4f3d37bdb18696c31"

# Fixed offsets are relative to the extracted monitor payload, not runtime PCs.
WORDS = {
    0x21A8: 0x940027CB,  # BL 0xc0d4: UART gate selector/helper
    0x21B4: 0x940002B1,  # BL 0x2c78: SPC/security setup
    0x2C78: 0xD2900080,  # MOVZ x0, #0x8004
    0x2C7C: 0xD2901C81,  # MOVZ x1, #0x80e4
    0x2C80: 0xF2A06000,  # MOVK x0, #0x300, LSL #16
    0x2C84: 0x12800002,  # MOVN w2, #0 (0xffffffff)
    0x2C88: 0xF2A06001,  # MOVK x1, #0x300, LSL #16
    0x2C8C: 0xB8010402,  # STR w2, [x0], #0x10
    0x2C90: 0xEB01001F,  # CMP x0, x1
    0x2C94: 0x54FFFFC1,  # B.NE 0x2c8c
    0x2C98: 0xD283E001,  # MOVZ x1, #0x1f00
    0x2C9C: 0x528000E0,  # MOVZ w0, #7
    0x2CA0: 0xF2A06001,  # MOVK x1, #0x300, LSL #16
    0x2CA4: 0xB9000020,  # STR w0, [x1]
    0x2CA8: 0xD2805201,  # MOVZ x1, #0x290
    0x2CAC: 0xF2A0E021,  # MOVK x1, #0x701, LSL #16
    0x2CB0: 0xB9000020,  # STR w0, [x1]
    0x2CB4: 0xD2840400,  # MOVZ x0, #0x2020
    0x2CB8: 0xF2A06000,  # MOVK x0, #0x300, LSL #16
    0x2CBC: 0x5281FFE1,  # MOVZ w1, #0xfff
    0x2CC0: 0xB9000001,  # STR w1, [x0]
    0x2CC4: 0xD2901C00,  # MOVZ x0, #0x80e0
    0x2CC8: 0xF2A06000,  # MOVK x0, #0x300, LSL #16
    0x2CCC: 0x52800021,  # MOVZ w1, #1
    0x2CD0: 0xB9000001,  # STR w1, [x0]
    0xBFB0: 0x340000A2,  # CBZ w2, 0xbfc4 (no gate: reconfigure UART)
    0xBFB4: 0xD2A0A000,  # MOVZ x0, #0x05000000
    0xBFB8: 0x8B21C001,  # ADD x1, x0, w1, SXTW
    0xBFBC: 0xF905C501,  # STR x1, [x8, #0xb88] (cached UART base)
    0xBFC0: 0x14000039,  # B 0xc0a4 (reuse path: no UART reprogramming)
    0xBFC4: 0xD2832182,  # MOVZ x2, #0x190c
    0xBFC8: 0x11004003,  # ADD w3, w0, #0x10
    0xBFCC: 0xF2A06002,  # MOVK x2, #0x300, LSL #16
    0xC000: 0xD282A480,  # MOVZ x0, #0x1524
    0xC008: 0xF2A06000,  # MOVK x0, #0x300, LSL #16
    0xC040: 0x5286C000,  # MOVZ w0, #0x3600
    0xC044: 0x72A02DC0,  # MOVK w0, #0x16e, LSL #16
    0xC04C: 0xD2A0A005,  # MOVZ x5, #0x05000000
    0xC054: 0x52800064,  # MOVZ w4, #3
    0xC058: 0x11438400,  # ADD w0, w0, #0xe1, LSL #12 (8 * 115200)
    0xC05C: 0x52840003,  # MOVZ w3, #0x2000
    0xC060: 0x72A00383,  # MOVK w3, #0x1c, LSL #16 (16 * 115200)
    0xC064: 0xF905C501,  # STR x1, [x8, #0xb88]
    0xC06C: 0x1AC30800,  # UDIV w0, w0, w3 (16550 divisor)
    0xC084: 0xB9000423,  # STR w3, [x1, #4]
    0xC088: 0xB8256840,  # STR w0, [x2, x5] (UART base + index*0x400)
    0xC09C: 0xB9000C24,  # STR w4, [x1, #0xc]
    0xC0A0: 0xB9000820,  # STR w0, [x1, #8]
    0xC0D4: 0xD2832180,  # MOVZ x0, #0x190c
    0xC0DC: 0xF2A06000,  # MOVK x0, #0x300, LSL #16
    0xC0E8: 0xB9400001,  # LDR w1, [x0] (UART gate bitmap)
    0xC100: 0x7100181F,  # CMP w0, #6
    0xC11C: 0x97FFFFA2,  # BL 0xbfa4
}

def _u32(data: bytes, offset: int) -> int:
    if offset < 0 or offset + 4 > len(data):
        raise ValueError(f"instruction at 0x{offset:x} lies outside monitor")
    return struct.unpack_from("<I", data, offset)[0]


def _sign_extend(value: int, bits: int) -> int:
    sign = 1 << (bits - 1)
    return (value ^ sign) - sign


def decode_bl_target(word: int, pc: int) -> int:
    """Decode AArch64 BL (imm26 << 2) using an image-relative PC."""
    if word & 0xFC000000 != 0x94000000:
        raise ValueError(f"0x{word:08x} is not an AArch64 BL")
    imm26 = _sign_extend(word & 0x03FFFFFF, 26)
    return pc + (imm26 << 2)


def decode_cbz_target(word: int, pc: int) -> int:
    """Decode AArch64 CBZ/CBNZ (imm19 << 2), either W or X register."""
    if word & 0x7E000000 != 0x34000000:
        raise ValueError(f"0x{word:08x} is not an AArch64 CBZ/CBNZ")
    imm19 = _sign_extend((word >> 5) & 0x7FFFF, 19)
    return pc + (imm19 << 2)


def decode_movz_movk_pair(data: bytes, movz_offset: int, movk_offset: int) -> int:
    """Decode a MOVZ followed by MOVK into the same W/X register."""
    movz = _u32(data, movz_offset)
    movk = _u32(data, movk_offset)
    if movz & 0x7F800000 != 0x52800000:
        raise ValueError(f"0x{movz_offset:x} is not MOVZ")
    if movk & 0x7F800000 != 0x72800000:
        raise ValueError(f"0x{movk_offset:x} is not MOVK")
    rd_z, rd_k = movz & 0x1F, movk & 0x1F
    if rd_z != rd_k:
        raise ValueError("MOVZ/MOVK destination registers differ")
    sf_z, sf_k = (movz >> 31) & 1, (movk >> 31) & 1
    if sf_z != sf_k:
        raise ValueError("MOVZ/MOVK register widths differ")
    imm_z = (movz >> 5) & 0xFFFF
    imm_k = (movk >> 5) & 0xFFFF
    hw_z, hw_k = (movz >> 21) & 3, (movk >> 21) & 3
    if hw_z != 0:
        raise ValueError("expected low-half MOVZ")
    width = 64 if sf_z else 32
    if width == 32 and (hw_k > 1):
        raise ValueError("invalid 32-bit MOVK shift")
    return (imm_z | (imm_k << (16 * hw_k))) & ((1 << width) - 1)


def audit_monitor(data: bytes, *, require_identity: bool = True) -> dict[str, Any]:
    if len(data) != EXPECTED_SIZE:
        raise ValueError(f"expected {EXPECTED_SIZE} monitor bytes, got {len(data)}")
    digest = hashlib.sha256(data).hexdigest()
    if require_identity and digest != EXPECTED_SHA256:
        raise ValueError(f"monitor SHA-256 mismatch: {digest}")

    for offset, expected in WORDS.items():
        actual = _u32(data, offset)
        if actual != expected:
            raise ValueError(
                f"monitor instruction mismatch at 0x{offset:x}: "
                f"expected 0x{expected:08x}, got 0x{actual:08x}"
            )

    calls = {
        "uart_gate_selector": decode_bl_target(_u32(data, 0x21A8), 0x21A8),
        "security_setup": decode_bl_target(_u32(data, 0x21B4), 0x21B4),
        "uart_configuration_helper": decode_bl_target(_u32(data, 0xC11C), 0xC11C),
    }
    if calls != {
        "uart_gate_selector": 0xC0D4,
        "security_setup": 0x2C78,
        "uart_configuration_helper": 0xBFA4,
    }:
        raise ValueError(f"unexpected BL targets: {calls}")

    addresses = {
        "spc_decode_port_set_first": decode_movz_movk_pair(data, 0x2C78, 0x2C80),
        "spc_decode_port_set_end": decode_movz_movk_pair(data, 0x2C7C, 0x2C88),
        "ccu_security_switch": decode_movz_movk_pair(data, 0x2C98, 0x2CA0),
        "r_prcm_security_switch": decode_movz_movk_pair(data, 0x2CA8, 0x2CAC),
        "dma_security_register": decode_movz_movk_pair(data, 0x2CB4, 0x2CB8),
        "additional_spc_register": decode_movz_movk_pair(data, 0x2CC4, 0x2CC8),
        "uart_gate_reset": decode_movz_movk_pair(data, 0xC0D4, 0xC0DC),
        "uart_apb2_clock": decode_movz_movk_pair(data, 0xC000, 0xC008),
        "uart0_base": _u32(data, 0xC04C) & 0xFFFFFFFF,
    }
    expected_addresses = {
        "spc_decode_port_set_first": 0x03008004,
        "spc_decode_port_set_end": 0x030080E4,
        "ccu_security_switch": 0x03001F00,
        "r_prcm_security_switch": 0x07010290,
        "dma_security_register": 0x03002020,
        "additional_spc_register": 0x030080E0,
        "uart_gate_reset": 0x0300190C,
        "uart_apb2_clock": 0x03001524,
    }
    for name, expected in expected_addresses.items():
        if addresses[name] != expected:
            raise ValueError(f"{name} address mismatch: 0x{addresses[name]:08x}")
    if _u32(data, 0x2C9C) != 0x528000E0 or _u32(data, 0x2CBC) != 0x5281FFE1:
        raise ValueError("SPC security values changed")
    if decode_cbz_target(_u32(data, 0xBFB0), 0xBFB0) != 0xBFC4:
        raise ValueError("UART gate reuse/fallback branch changed")
    if decode_bl_target(_u32(data, 0xBFC0), 0xBFC0) != 0xC0A4:
        raise ValueError("UART gate reuse path no longer bypasses UART reconfiguration")
    clock_hz = decode_movz_movk_pair(data, 0xC040, 0xC044)
    baud_times_16 = decode_movz_movk_pair(data, 0xC05C, 0xC060)
    if clock_hz != 24_000_000 or baud_times_16 != 16 * 115_200:
        raise ValueError("UART fallback clock/baud divisor constants changed")
    if _u32(data, 0xC058) != 0x11438400:
        raise ValueError("UART fallback rounding term changed")

    port_count = (addresses["spc_decode_port_set_end"] -
                  addresses["spc_decode_port_set_first"]) // 0x10
    if port_count != 14:
        raise ValueError(f"unexpected SPC decode-port count: {port_count}")

    spc_ports = [
        f"0x{addresses['spc_decode_port_set_first'] + i * 0x10:08x}"
        for i in range(port_count)
    ]
    return {
        "monitor": {"bytes": len(data), "sha256": digest,
                    "identity_match": digest == EXPECTED_SHA256},
        "execution_scope": "static instruction audit only; no monitor code executed",
        "bl33_preparation_calls": {k: f"0x{v:x}" for k, v in calls.items()},
        "uart": {
            "gate_register": f"0x{addresses['uart_gate_reset']:08x}",
            "selector": "scans gate bits 0..5 and chooses the lowest enabled UART",
            "existing_gate_path": (
                "selects UART base 0x05000000 + index*0x400 and caches the pointer; "
                "branches around UART clock/reset/divisor programming"
            ),
            "fallback_path": {
                "apb2_register": f"0x{addresses['uart_apb2_clock']:08x}",
                "default_clock_hz": clock_hz,
                "baud_rate": 115200,
                "divisor_formula": "(clock_hz + 8*baud_rate) / (16*baud_rate)",
                "uart0_base": f"0x{addresses['uart0_base']:08x}",
                "register_writes": "16550 register offsets 0x0, 0x4, 0x8, 0xc, 0x10",
            },
            "scope_note": (
                "The reuse path does not write UART/CCU registers. This audit does not "
                "read live PIO mux, gate, reset or UART divisor state."
            ),
        },
        "security_setup": {
            "spc_decode_port_count": port_count,
            "spc_decode_port_set_registers": spc_ports,
            "spc_write_value": "0xffffffff",
            "ccu_security_switch": f"0x{addresses['ccu_security_switch']:08x}",
            "ccu_security_value": "0x00000007",
            "r_prcm_security_switch": f"0x{addresses['r_prcm_security_switch']:08x}",
            "r_prcm_security_value": "0x00000007",
            "dma_security_register": f"0x{addresses['dma_security_register']:08x}",
            "dma_security_value": "0x00000fff",
            "additional_spc_register": f"0x{addresses['additional_spc_register']:08x}",
            "additional_spc_value": "0x00000001",
            "additional_spc_semantics": "unresolved from the pinned public H616 SPC header",
            "scope_note": (
                "The sequence is statically called on the BL33 preparation path. "
                "The H616 TF-A reference warns SPC writes may not take effect unless "
                "a specific fuse is burnt; target fuse/status and live permissions are unknown."
            ),
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("monitor", type=Path, help="extracted 107,216-byte RG34XX monitor.bin")
    parser.add_argument("--allow-unpinned-hash", action="store_true",
                        help="skip the expected SHA check (tests/controlled research only)")
    args = parser.parse_args()
    try:
        data = args.monitor.read_bytes()
        report = audit_monitor(data, require_identity=not args.allow_unpinned_hash)
    except (OSError, ValueError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 1
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
