#!/usr/bin/env python3
"""Validate matched RG34XX BL31 PSCI CPU_ON and emit power-ops disassembly."""
from __future__ import annotations
import argparse
import hashlib
import json
import struct
from pathlib import Path

BASE = 0x48000000
SIZE = 107216
SHA = "676bc0206e3c9a7f229d93ea7f3286095f669940ad91f3c4f3d37bdb18696c31"
TABLE = 0x480165f8
EXPECTED_TABLE = [
    0x48002510, 0x4800254c, 0x4800264c, 0x48002878, 0x48002680,
    0x480026c4, 0x480025a0, 0x48002574, 0x48002544,
]
SUSPEND_LEVEL_TABLE = 0x48017d70
SUSPEND_LEVEL_CALLBACKS = (0x48008b84, 0x48008ab8, 0x48008a00)
SRAM_SUSPEND_COPY_SOURCE = 0x4800c220
SRAM_SUSPEND_COPY_END = 0x48015630
SRAM_SUSPEND_COPY_DEST = 0x00020000
SRAM_SUSPEND_ENTRY_SOURCE = 0x4800c3d8
SRAM_SUSPEND_ENTRY = 0x000201b8
SRAM_SUSPEND_STACK_SETUP_SOURCE = 0x480140a0
SRAM_SUSPEND_STACK_TOP = 0x00057ff0
SRAM_SUSPEND_SCRATCH_SOURCE_START = 0x48015410
SRAM_SUSPEND_SCRATCH_SOURCE_LAST = 0x48015568
SUSPEND_CHECKSUM_CONFIG_BASE_ADDR = 0x4801a000
SUSPEND_CHECKSUM_CONFIG_LENGTH_ADDR = 0x4801a004
SUSPEND_CHECKSUM_CONFIG_ENABLE_ADDR = 0x4801a008
SUSPEND_CHECKSUM_CONFIG_LABEL_ADDR = 0x4801a010
SUSPEND_CHECKSUM_INITIAL_BASE = 0x40000000
SUSPEND_CHECKSUM_INITIAL_LENGTH = 0x100
SUSPEND_CHECKSUM_INITIAL_ENABLE = 1
SUSPEND_CHECKSUM_INITIAL_END = SUSPEND_CHECKSUM_INITIAL_BASE + SUSPEND_CHECKSUM_INITIAL_LENGTH
SUSPEND_CHECKSUM_INITIAL_WORD_COUNT = SUSPEND_CHECKSUM_INITIAL_LENGTH // 4
ARM_SVC_HANDLER_ADDRESS = 0x480035d0
ARM_SVC_HANDLER_RECORD_ADDRESS = 0x48018ca8
ARM_SVC_NAME_POINTER_ADDRESS = 0x48018c98
ARM_SVC_NAME_ADDRESS = 0x48016ad9
ARM_SVC_CHECKSUM_FIDS = (0x80000064, 0xc0000064)
RTC_GP_DATA1_ADDRESS = 0x07000104
RETENTION_BUFFER_ADDRESS = 0x4801b400
RETENTION_BUFFER_BYTES = 0x280
RETENTION_BUFFER_END = RETENTION_BUFFER_ADDRESS + RETENTION_BUFFER_BYTES
RETENTION_WORD_COUNT = RETENTION_BUFFER_BYTES // 4
RETENTION_SEGMENT_BYTES = 0x140
RETENTION_SEGMENT_WORDS = RETENTION_SEGMENT_BYTES // 4
RETENTION_FIXED_DRAM_BASE = 0x40000000
RETENTION_SELECTOR_SHIFT = 17
RETENTION_BANK_STRIDE_SHIFT = 20
WORDS = {
    0x480029bc: 0xa9bf7bfd, 0x480029c0: 0x900000a1,
    0x480029c4: 0x9117e021, 0x480029cc: 0xf9000001,
    0x48009dec: 0x97ffe2f4,
    0x48008600: 0x900001a1, 0x48008604: 0xf947e021,
    0x48008608: 0xf9400433, 0x48008628: 0xaa1303e5,
    0x48008640: 0xd61f00a0,
    0x4800869c: 0x900001a1, 0x480086a0: 0xf947e021,
    0x480086a4: 0xf9400433, 0x480086c4: 0xaa1303e5,
    0x480086dc: 0xd61f00a0,
    # Legacy plat_pm_ops_t member dispatches.
    0x48009684: 0xf947e021, 0x48009688: 0xf9400021,
    0x48009690: 0xd63f0020,
    0x480080f8: 0xf9400835, 0x48008118: 0xd63f02a0,
    0x48008a84: 0xf9400c26, 0x48008a90: 0xd63f00c0,
    0x4800854c: 0xf9401003, 0x48008560: 0xd63f0060,
    0x48008918: 0xf9401421, 0x4800893c: 0xd63f0060,
    # Vendor SYSTEM_SUSPEND getter is the ninth (+0x40) member.
    0x48002544: 0x52a04020, 0x48002548: 0xd65f03c0,
    0x48009718: 0xf947e000, 0x4800971c: 0xf9402000,
    0x48009720: 0xb5000120, 0x48009744: 0xd63f0000,
    0x4800975c: 0x97fffd8f,
    # SYSTEM_SUSPEND special return handling and power-state entry.
    0x4800299c: 0x12800060, 0x48009768: 0x3100101f,
    0x48009770: 0x54000081, 0x48009774: 0x97ffe0a4,
    # Affinity-level suspend wrappers dispatch legacy affinst_suspend.
    0x48008a18: 0x71000a9f, 0x48008a84: 0xf9400c26,
    0x48008a90: 0xd63f00c0, 0x48008ad0: 0x7100069f,
    0x48008b34: 0xf9400c26, 0x48008b40: 0xd63f00c0,
    0x48008ba4: 0x39402c00, 0x48008c20: 0xf9400c36,
    0x48008c54: 0xd63f02c0,
    # Generic transition dispatcher: per-affinity callback table and loop.
    0x48008e4c: 0xf0000077, 0x48008e54: 0x9135c2f7,
    0x48008e64: 0x0b160281, 0x48008e68: 0x6b0102bf,
    0x48008e70: 0xf8767b60, 0x48008e74: 0xb4000100,
    0x48008e78: 0xf861dae4, 0x48008e88: 0xd63f0080,
    0x48008e94: 0x910006d6, 0x48008e98: 0x17fffff3,
    # SYSTEM_SUSPEND special entry prepares context and branches to SRAM.
    0x48001b5c: 0x94005173, 0x48001b64: 0xd2803700,
    0x48001b68: 0xa8c27bfd, 0x48001b6c: 0xf2a00040,
    0x48001b70: 0xd61f0000,
    # Ordinary power-down helper executes DSB; WFI; retry WFI.
    0x48015d70: 0xd5033f9f, 0x48015d74: 0xd503207f,
    0x48015d78: 0x17ffffff,
    # SYSTEM_SUSPEND copies its standby image and auxiliary context to SRAM.
    0x48001a08: 0xf0000041, 0x48001a0c: 0x900000a2,
    0x48001a10: 0x91088021, 0x48001a14: 0x9118c042,
    0x48001a1c: 0xcb010042, 0x48001a20: 0xd2a00040,
    0x48001a28: 0xd2a00053, 0x48001a2c: 0x94002415,
    0x48001a34: 0xf00001c1, 0x48001a38: 0xd2801000,
    0x48001a3c: 0x912e4021, 0x48001a40: 0xd2800602,
    0x48001a44: 0xf2a00040, 0x48001a48: 0x9400240e,
    0x48001a4c: 0x900000a0, 0x48001a50: 0x91345000,
    0x48001a54: 0xb900c660,
    0x48001a58: 0xb00000c0, 0x48001a5c: 0x39402000,
    0x48001a60: 0xb900c260, 0x48001a64: 0xb00000c0,
    0x48001a68: 0xb9400000, 0x48001a6c: 0xb900b260,
    0x48001a70: 0xb00000c0, 0x48001a74: 0xb9400400,
    0x48001a78: 0xb900b660, 0x48001a7c: 0xd00000c0,
    0x48001a80: 0xb9474800, 0x48001a84: 0xb900ca60,
    0x48001a88: 0x94000225, 0x48001a8c: 0xb900ce60,
    0x48001a90: 0xd2800601, 0x48001a94: 0xd2801002,
    0x48001a98: 0xb9436a80, 0x48001a9c: 0xf2a90001,
    0x48001aa0: 0xb900d260, 0x48001aa4: 0xaa1303e0,
    0x48001aa8: 0x940023f6,
    0x48001aac: 0xd2802081, 0x48001ab0: 0xd00000c0,
    0x48001ab4: 0xf2a0e001, 0x48001ab8: 0x91100000,
    0x48001abc: 0xb9000020,
    0x48001ac0: 0xb9436a80, 0x48001ac4: 0x3100041f,
    0x48001ac8: 0x540003c0, 0x48001acc: 0x7100081f,
    0x48001ad0: 0x540000a1, 0x48001ad4: 0xb940ce60,
    0x48001ad8: 0x121f7800, 0x48001adc: 0x321f0000,
    0x48001ae0: 0x14000017, 0x48001ae4: 0xd00000d4,
    0x48001ae8: 0xb9437a80, 0x48001aec: 0x35000080,
    0x48001af0: 0xb940ce60, 0x48001af4: 0x32000000,
    0x48001af8: 0x14000011, 0x48001afc: 0xd00000c1,
    0x48001b00: 0xd2802680, 0x48001b04: 0x910e0021,
    0x48001b08: 0xd2801002, 0x48001b0c: 0xf2a00040,
    0x48001b10: 0x940023dc,
    0x48001b14: 0xb9437a80, 0x48001b18: 0xd00000c1,
    0x48001b1c: 0x910c2021, 0x48001b20: 0xd2800c02,
    0x48001b24: 0xb901b660, 0x48001b28: 0xd2801a80,
    0x48001b2c: 0xf2a00040, 0x48001b30: 0x940023d4,
    0x48001b34: 0xb940ce60, 0x48001b38: 0x121e7400,
    0x48001b3c: 0xb900ce60, 0x48001b40: 0xd00000c0,
    0x48001b44: 0x394db000, 0x48001b48: 0x340000a0,
    0x48001b4c: 0xd2a00041, 0x48001b50: 0xb940cc20,
    0x48001b54: 0x321e0000, 0x48001b58: 0xb900cc20,
    # EL3 arm_svc command dispatcher and checksum descriptor mutation.
    0x480035dc: 0xaa0103f4, 0x480035ec: 0xaa0203f5,
    0x480035f0: 0xaa0303e2,
    0x48003720: 0x52800c81, 0x48003724: 0x72b00001,
    0x48003728: 0x6b01001f, 0x4800372c: 0x54002300,
    0x480039f8: 0x52800c81, 0x480039fc: 0x72b80001,
    0x48003a00: 0x6b01001f, 0x48003a04: 0x54000c40,
    0x48003b8c: 0x2a1503e1, 0x48003b90: 0x2a1403e0,
    0x48018c98: 0x48016ad9, 0x48018ca8: 0x480035d0,
    # Checksum descriptor writer and direct callers.
    0x48001c3c: 0x97ffff72, 0x48002440: 0x97fffdcd,
    0x480022fc: 0x900000c3, 0x48002300: 0x39002060,
    0x48002304: 0x900000c0, 0x48002308: 0xb9000001,
    0x4800230c: 0x900000c0, 0x48002310: 0xb9000402,
    0x48002314: 0x52800000, 0x48002318: 0xd65f03c0,
    0x48003b94: 0x97fff9da,
    0x48011928: 0x97ffff7a,
    0x48010f98: 0xd2940102,
    0x48010fa0: 0xf2a08fe2,
    0x48010ff0: 0xd2960600,
    0x48010ff4: 0xd2963405,
    0x48010ff8: 0xf2a08fe0,
    0x48010ffc: 0xf2a08fe5,
    0x48011000: 0xd2963004,
    0x48011004: 0xd29a3003,
    0x48011008: 0xf2a08fe4,
    0x4801100c: 0xf2a08fe3,
    0x48011014: 0xd29c3002,
    0x48011018: 0xf2a08fe2,
    0x4801101c: 0xd29e3001,
    0x48011024: 0xf2a08fe1,
    0x48011078: 0xd2960c03,
    0x4801107c: 0xd2960600,
    0x48011080: 0xf2a08fe3,
    0x48011084: 0xf2a08fe0,
    0x48011088: 0xd2963602,
    0x48011090: 0xf2a08fe2,
    0x48011094: 0xd2940406,
    0x4801109c: 0xf2a08fe6,
    0x480110a0: 0xd2940485,
    0x480110ac: 0xf2a08fe5,
    0x480110a4: 0xd2940504,
    0x480110b4: 0xf2a08fe4,
    0x48011100: 0xd2960002,
    0x48011108: 0xf2a08fe2,
    0x48011174: 0xd2940a00,
    0x4801117c: 0xf2a08fe0,
    0x48011184: 0xd2964a01,
    0x48011188: 0xf2a08fe1,
    0x480115c0: 0xd294a002,
    0x480115c8: 0xf2a08fe2,
    0x480115cc: 0xd2960183,
    0x480115e0: 0xf2a08fe3,
    0x480115e8: 0xd2940101,
    0x480115f0: 0xf2a08fe1,
    0x48011618: 0xd2964a02,
    0x48011620: 0xf2a08fe2,
    0x4801161c: 0xd2960703,
    0x48011624: 0xf2a08fe3,
    0x48011670: 0xd2966403,
    0x48011678: 0xf2a08fe3,
    0x48011674: 0xd2960c04,
    0x4801167c: 0xf2a08fe4,
    0x48011680: 0xd2940282,
    0x48011688: 0xf2a08fe2,
    0x48011684: 0xd2940409,
    0x4801168c: 0xf2a08fe9,
    0x48011694: 0xd2940487,
    0x4801169c: 0xf2a08fe7,
    0x480116a4: 0xd2940505,
    0x480116ac: 0xf2a08fe5,
    0x480116d0: 0xd2966482,
    0x480116d8: 0xf2a08fe2,
    0x480116f8: 0xd2940a00,
    0x48011700: 0xf2a08fe0,
    0x48011330: 0x97ffff9c,
    0x48011338: 0x940000a2,
    0x48011640: 0x97fffe56,
    0x4801166c: 0x97ffff3d,
    0x48011370: 0x97fff31e,
    0x48011378: 0x97ffed9c,
    0x480113bc: 0x97fff3c9,
    0x480113f0: 0x97fff444,
    0x4801140c: 0x97fff43d,
    0x48011438: 0x97fff5da,
    0x48011454: 0x97fff5d3,
    0x48011480: 0x97fff020,
    0x480114a4: 0x97fff49f,
    0x480114f4: 0x97ffff8b,
    0x4801150c: 0x97fff5a5,
    0x48011534: 0x97fff36b,
    0x48011554: 0x97fff3eb,
    0x4801157c: 0x97fff469,
    0x4800c490: 0x940015ac, 0x4800c658: 0x94001540,
    0x48011710: 0xd2802082, 0x48011714: 0xb9401c01,
    0x48011718: 0xf2a0e002, 0x4801171c: 0xb9400045,
    0x48011720: 0x37000421, 0x48011724: 0x52a80007,
    0x48011728: 0x53117c21, 0x4801172c: 0x2a0703e8,
    0x48011730: 0x128009e3, 0x48011734: 0x0b0150e7,
    0x48011738: 0x52800001, 0x4801173c: 0x1400000d,
    0x48011740: 0xb9400406, 0x48011744: 0x531e7422,
    0x48011748: 0x0b080044, 0x4801174c: 0x0b0200a2,
    0x48011750: 0x710010df, 0x48011754: 0x54000be0,
    0x48011758: 0xb9400084, 0x4801175c: 0xb9000044,
    0x48011760: 0x11000421, 0x48011764: 0x11000463,
    0x48011768: 0x7102803f, 0x4801176c: 0x540001c0,
    0x48011770: 0x71013c3f, 0x48011774: 0x54fffe69,
    0x48011778: 0xb9400404, 0x4801177c: 0x0b0308e2,
    0x48011780: 0x0b0108a6, 0x48011784: 0x7100109f,
    0x48011788: 0x54000b80, 0x4801178c: 0xb9400042,
    0x48011790: 0x11000421, 0x48011794: 0x7102803f,
    0x48011798: 0x11000463, 0x4801179c: 0xb90000c2,
    0x480117a0: 0x54fffe81, 0x480117a4: 0xd2940405,
    0x480118d0: 0x53037c24, 0x480118d4: 0x531e0829,
    0x480118d8: 0x531b0082, 0x480118dc: 0x0b0108a6,
    0x480118e0: 0x0b041924, 0x480118e4: 0x0b080042,
    0x480118e8: 0x0b020082, 0x480118ec: 0xb9400044,
    0x480118f0: 0xb90000c4, 0x480118f4: 0x17ffff9b,
    0x480118f8: 0x53037c64, 0x480118fc: 0x531e0822,
    0x48011900: 0x531b0089, 0x48011904: 0x0b070042,
    0x48011908: 0x0b0108a6, 0x4801190c: 0x0b041924,
    0x48011910: 0x0b020082, 0x48011914: 0xb9400044,
    0x48011918: 0xb90000c4, 0x4801191c: 0x17ffff91,
    0x48011938: 0xd2806203, 0x48011940: 0xf2a0e023,
    0x48011944: 0xaa0103e0, 0x48011954: 0xb9400061,
    0x48011960: 0x32180021, 0x48011964: 0xb9000061,
    0x48011968: 0xb9400041, 0x4801196c: 0x121a6421,
    0x48011970: 0xb9000041, 0x48011974: 0x97fffe6b,
    0x48011978: 0x52800001, 0x4801197c: 0x340007a0,
    0x48011980: 0xb9401e67, 0x48011984: 0xd35078ea,
    0x48011988: 0x37f80227, 0x48011990: 0x97ffeb68,
    0x480119ac: 0x54000920, 0x480119bc: 0x540008a0,
    0x480119d0: 0x36d80880, 0x480119d4: 0xd2802081,
    0x480119d8: 0xb9401e60, 0x480119dc: 0xf2a0e001,
    0x480119e0: 0xb9400026, 0x480119e4: 0x37000440,
    0x480119e8: 0x52a80001, 0x480119ec: 0x53117c00,
    0x480119f0: 0x2a0103e7, 0x480119f4: 0x128009e4,
    0x480119f8: 0x0b005020, 0x480119fc: 0x52800002,
    0x48011a00: 0x1400000e, 0x48011a08: 0xb9400661,
    0x48011a0c: 0x531e7443, 0x48011a10: 0x0b070065,
    0x48011a14: 0x0b0300c3, 0x48011a18: 0x7100103f,
    0x48011a1c: 0x54000320, 0x48011a20: 0xb9400061,
    0x48011a24: 0xb90000a1, 0x48011a28: 0x11000442,
    0x48011a2c: 0x11000484, 0x48011a30: 0x7102805f,
    0x48011a34: 0x540001c0, 0x48011a38: 0x71013c5f,
    0x48011a3c: 0x54fffe69, 0x48011a40: 0xb9400661,
    0x48011a44: 0x0b040803, 0x48011a48: 0x0b0208c5,
    0x48011a4c: 0x7100103f, 0x48011a50: 0x540002c0,
    0x48011a54: 0xb94000a1, 0x48011a58: 0x11000442,
    0x48011a5c: 0x7102805f, 0x48011a60: 0x11000484,
    0x48011a64: 0xb9000061, 0x48011a68: 0x54fffe81,
    0x48011ae0: 0x97ffeb32,
    0x48011b10: 0xa9bf7bfd, 0x48011b14: 0xaa0003e1,
    0x48011b18: 0x52800000, 0x48011b1c: 0x910003fd,
    0x48011b20: 0x97ffff86, 0x48011b24: 0x52800000,
    0x48011b28: 0xa8c17bfd, 0x48011b2c: 0xd65f03c0,
    0x48011b30: 0xd65f03c0, 0x48011b38: 0xd65f03c0,
    0x48011b40: 0xa9bf7bfd, 0x48011b44: 0x910003fd,
    0x48011b48: 0x97fffffc, 0x48011b50: 0x17ffff74,
    0x48011b4c: 0xa8c17bfd, 0x48011b58: 0xa9bf7bfd,
    0x48011b5c: 0x910003fd, 0x48011b60: 0x97ffffec,
    0x48011b64: 0x97fffff3, 0x48011b68: 0x52800000,
    0x48011b6c: 0xa8c17bfd,
    # Retained DRAM snapshot and inverse restore helpers.
    # Relocated standby entry, in-image scratch/BSS and copied helper calls.
    0x4800aa80: 0xd2800003, 0x4800aa84: 0xeb03005f,
    0x4800aa88: 0x540000a0, 0x4800aa8c: 0x38636824,
    0x4800aa90: 0x38236804, 0x4800aa94: 0x91000463,
    0x4800aa98: 0x17fffffb, 0x4800aa9c: 0xd65f03c0,
    0x4800c3d8: 0xb0000040, 0x4800c3dc: 0xb0000041,
    0x4800c3e0: 0x91104000, 0x4800c3e4: 0x9115a021,
    0x4800c3e8: 0x3800141f, 0x4800c3ec: 0xeb01001f,
    0x4800c3f0: 0x54ffffc9, 0x4800c3fc: 0x94001f29,
    0x4800c400: 0x94000006, 0x4800c404: 0x94000033,
    0x4800c408: 0x94000084,
    # Standby image additive checksum and resume integrity comparison.
    0x4800c43c: 0xb940c260, 0x4800c440: 0x34000080,
    0x4800c444: 0xaa1303e0, 0x4800c448: 0x940015cc,
    0x4800c44c: 0xb900ba60,
    0x4800c6c0: 0xaa1503e0, 0x4800c6c4: 0x9400152d,
    0x4800c6c8: 0xf9433661, 0x4800c6cc: 0xb940b822,
    0x4800c6d0: 0xb900bea0, 0x4800c6d4: 0xb940bc20,
    0x4800c6d8: 0x6b00005f, 0x4800c6dc: 0x54fffcc0,
    0x4800c6e0: 0x52800020, 0x4800c6e4: 0x72b10000,
    0x4800c6e8: 0xb9000280, 0x4800c6ec: 0x14000000,
    0x48011b78: 0x29560c01, 0x48011b7c: 0x52800000,
    0x48011b80: 0x0b030023, 0x48011b84: 0x2a0103e1,
    0x48011b88: 0xeb01007f, 0x48011b8c: 0x540000c9,
    0x48011b90: 0xb8404422, 0x48011b94: 0xeb01007f,
    0x48011b98: 0x0b020000, 0x48011b9c: 0x54ffffa8,
    0x48011ba0: 0xd65f03c0, 0x48011ba4: 0xd65f03c0,
    0x4800c50c: 0xd503207f, 0x4800c550: 0xd503207f,
    0x4800c53c: 0x94001e2d, 0x4800c568: 0x94001e60,
    0x4800c460: 0xd282a002, 0x4800c468: 0xf2a06002,
    0x4800c470: 0xb9400041, 0x4800c474: 0x12067421,
    0x4800c478: 0xb9000041, 0x4800c47c: 0x940016df,
    0x4800c580: 0xd2803f00, 0x4800c584: 0x52a00023,
    0x4800c588: 0xf2a0e000, 0x4800c594: 0xd2803f84,
    0x4800c598: 0xb9000003, 0x4800c59c: 0xf2a0e004,
    0x4800c5a8: 0xb940c426, 0x4800c5b8: 0xb9000086,
    0x4800c680: 0xd282a002, 0x4800c684: 0xf2a06002,
    0x4800c690: 0xb9400041, 0x4800c694: 0x32080421,
    0x4800c698: 0xb9000041, 0x4800c69c: 0x94001657,
    0x4800c6a8: 0xd63f02c0,
    # BL31 wake-return continuation stored into RTC soft-entry context.
    0x48015d14: 0x1000fb77, 0x48015d18: 0xd2820141,
    0x48015d1c: 0xd53e1000, 0x48015d20: 0xaa010000,
    0x48015d24: 0xd51e1000, 0x48015d28: 0xd5033fdf,
    0x48015d2c: 0x97fffeb6, 0x48015d30: 0x10019680,
    0x48015d34: 0xd51ec000, 0x48015d38: 0xd5033fdf,
    0x48015d3c: 0xd50344ff, 0x48015d40: 0xd50040bf,
    0x48015d44: 0xd53800a0, 0x48015d48: 0x97ffd930,
    0x48015d4c: 0xd2800020, 0x48015d50: 0x97ffd894,
    0x48015d54: 0x97ffd20f, 0x48015d58: 0x97ffcd60,
    0x48015d5c: 0xaa1703e2, 0x48015d60: 0xaa0003e1,
    0x48015d64: 0xd2800000, 0x48015d68: 0x97ffcdd9,
    0x48015d6c: 0x17fffed7,
    # Copied code installs its SRAM-C stack.
    0x48011ff8: 0x531e7400, 0x48011ffc: 0x1400081f,
    0x4800c464: 0x52800140, 0x4800c688: 0x52800140,
    0x48014078: 0xf1000400, 0x4801407c: 0x54ffffe8,
    0x48014080: 0xd65f03c0,
    0x480140a0: 0x58000081, 0x480140a4: 0x9100003f,
    0x480140a8: 0xd65f03c0,
    # EL3 special path disables MMU/D-cache then walks cache hierarchy by set/way.
    0x48015fb8: 0xb40003e3, 0x48015fe4: 0xd51a000a,
    0x48015fec: 0xd5390001, 0x48016040: 0xd508764b,
    0x48016060: 0xd5087e4b, 0x48016080: 0xd5087a4b,
    0x480160a8: 0xd5390029, 0x4801612c: 0xd53e1000,
    0x48016130: 0x8a210000, 0x48016134: 0xd51e1000,
    0x48016138: 0xd5033fdf, 0x48016140: 0x17ffffda,
    # OEM standby clock-save/restore, AON tag and PLL relock.
    0x48011bb0: 0xf2a06005, 0x48011bc4: 0xb94000a7,
    0x48011bd4: 0x0a0100e1, 0x48011be0: 0xb9041441,
    0x48011c28: 0x528063e2, 0x48011c2c: 0x72a06002,
    0x48011c74: 0xb94000c1, 0x48011c78: 0x29029007,
    0x48011c7c: 0x529fe064, 0x48011c80: 0x12067421,
    0x48011c84: 0x72a00064, 0x48011c88: 0x32080021,
    0x48011c8c: 0xb90000c1,
    # mode-2 branch: CCU/ R_PRCM low-power muxes and RTC XO_CTRL.
    0x48011d8c: 0xd2a0e024, 0x48011d90: 0xf2a06005,
    0x48011d94: 0xd2800203, 0x48011d98: 0xf2a0e023,
    0x48011d9c: 0xd2802c02, 0x48011da0: 0xf2a0e002,
    0x48011da4: 0xb94000a1, 0x48011db4: 0xb9400081,
    0x48011db8: 0x12067421, 0x48011dbc: 0x32070021,
    0x48011dc0: 0xb9000081, 0x48011dc4: 0xb9400061,
    0x48011dc8: 0x12067421, 0x48011dcc: 0x32070021,
    0x48011dd0: 0xb9000061, 0x48011dd4: 0xb9400041,
    0x48011de0: 0xb9400041, 0x48011dec: 0xb940cc01,
    0x48011c54: 0x12067463, 0x48011c5c: 0x32080063,
    0x48011c64: 0xb9000103, 0x48011c68: 0xd2804888,
    0x48011c6c: 0xf2a0e028, 0x48011c70: 0x52b4e003,
    0x48011c90: 0xb94000a1, 0x48011c94: 0x12067421,
    0x48011c98: 0x32080021, 0x48011c9c: 0xb90000a1,
    0x48011ca8: 0x0a040021, 0x48011cac: 0x120078a5,
    0x48011cb0: 0xb9000045, 0x48011cb4: 0xb9001c01,
    0x48011cbc: 0x12027800, 0x48011cc0: 0xb9000040,
    0x48011cc4: 0xb9400100, 0x48011cc8: 0x12005c00,
    0x48011ccc: 0x2a030000, 0x48011cd0: 0xb9000100,
    0x48011d2c: 0x9400063d, 0x48011d6c: 0x9400062d,
    0x48011d88: 0xd282a205, 0x48011da8: 0x12067421,
    0x48011dac: 0x32070021, 0x48011db0: 0xb90000a1,
    0x48011dd8: 0x32010021, 0x48011ddc: 0xb9000041,
    0x48011de4: 0x121e7821, 0x48011de8: 0xb9000041,
    0x48011df0: 0x17ffffc0,
    0x48011e10: 0x9400089a, 0x48011e48: 0x12027884,
    0x48011e4c: 0xb9000004, 0x48011e54: 0x0a030083,
    0x48011e58: 0x2a020062, 0x48011e5c: 0xb9000002,
    0x48011e64: 0x32010042, 0x48011e68: 0xb9000002,
    0x48011e70: 0x32030042, 0x48011e74: 0xb9000002,
    0x48011e78: 0xb9400020, 0x48011e7c: 0x36e7ffe0,
    0x48011e94: 0x52800280, 0x48011ec0: 0x9400004e,
    # GIC pending, IR FIFO/status and GPIO wake-pin helpers.
    0x48013194: 0xd2824003, 0x4801319c: 0xf2a06043,
    0x480131a0: 0x531e7401, 0x480131a4: 0x52800022,
    0x480131a8: 0x1ac42040, 0x480131ac: 0xb8614861,
    0x480131b0: 0x0a010000, 0x480131b4: 0xd65f03c0,
    0x480131bc: 0xd2800609, 0x480131c0: 0xd2800406,
    0x480131c8: 0xf2a0e089, 0x480131cc: 0xf2a0e086,
    0x480131d8: 0xb9400128, 0x480131dc: 0x12001d05,
    0x480131e0: 0xd3483901, 0x480131e4: 0xb9000125,
    0x48013200: 0xb94000c4, 0x4801320c: 0x39001024,
    0x48013214: 0xb9000043,
    0x48013588: 0x39400001, 0x48013594: 0x39400402,
    0x48013598: 0x51010442, 0x480135a0: 0x39400801,
    0x480135a4: 0x5100c060, 0x480135b4: 0x2a024000,
    0x480135d8: 0x53107c01, 0x480135e0: 0x7100283f,
    0x480135ec: 0x72a0e044, 0x480135f0: 0x72a06002,
    0x480135f8: 0x1a843042, 0x480135fc: 0x1ac02060,
    0x48013600: 0x531e7421,
    0x48013608: 0x0b020021, 0x4801360c: 0xb9400021,
    0x48013610: 0x6a01001f, 0x48013614: 0x1a9f07e0,
    0x48013620: 0x39400001, 0x48013654: 0x53107c26,
    0x48013660: 0x72a06003, 0x48013664: 0x0b060cc2,
    0x48013668: 0x710028df, 0x4801366c: 0xb9400c09,
    0x48013674: 0x1a883068, 0x48013678: 0x12003c23,
    0x4801367c: 0x531e7444,
    0x48013684: 0x11004087, 0x48013690: 0x1ac32129,
    0x480136a4: 0x0b030903, 0x480136a8: 0x0b020902,
    0x480136b4: 0xb94000e4, 0x480136c4: 0x0a250085,
    0x480136d4: 0xb90000e5, 0x480136dc: 0xb9401007,
    0x480136f4: 0xb9000044, 0x48013714: 0xb90000c2,
    0x48013724: 0xb9000060,
    # Wake-source aggregator call sites.
    0x48013e88: 0x97fffde6, 0x48013ec8: 0x97fffdd6,
    0x48013f14: 0x97fffca9, 0x48013f48: 0x97fffd90,
    0x48013f54: 0x97fffda1, 0x48013f8c: 0x97fffd7f,
    0x48013f94: 0x97fffd91, 0x48013fe4: 0x97fffc6b,
    0x48014050: 0x97fffc5a, 0x4800c4a0: 0x940015c2,
    0x4800c648: 0x940015ec,
    # Slot 7 system-reset and slot 6 system-off helper calls.
    0x48002588: 0x97fffe50, 0x480025a8: 0x97fffe4c,
    0x48002858: 0x97fffc20, 0x48002874: 0x17ffff83,
    0x48002998: 0x97fffb7e,
    0x4800254c: 0x35000103, 0x4800255c: 0x94000207,
    0x48002d18: 0xd2800480, 0x48002d1c: 0xf2a06000,
    0x48002d20: 0xb9400000, 0x48002d24: 0xf240081f,
    0x48002d28: 0x1a9f17e0, 0x48002d40: 0x34000080,
    0x48002d44: 0x52840100, 0x48002d48: 0x72a02400,
    0x48002d4c: 0x14000003, 0x48002d50: 0x52800100,
    0x48002d54: 0x72a02040, 0x48002d58: 0x0b000021,
    0x48002d5c: 0x531d7020, 0x48002d60: 0x11001001,
    0x48002d64: 0xb9000002, 0x48002d68: 0xb900003f,
    0x48002d30: 0xa9bf7bfd, 0x48002d3c: 0x97fffff7,
    0x48002d64: 0xb9000002, 0x48002d68: 0xb900003f,
    0x48002d78: 0xa9bf7bfd, 0x48002d7c: 0x2a0003e4,
    0x48002d80: 0xaa0103e0, 0x48002d84: 0x2a0403e1,
    0x48002d8c: 0x97ffffe9, 0x48002d90: 0x97ffffe2,
    0x48002d94: 0x340004e0,
    0x48002db0: 0xb94000a1, 0x48002db8: 0x0a030021,
    0x48002dbc: 0xb90000a1, 0x48002dc4: 0xb94000c1,
    0x48002dcc: 0xb90000c1, 0x48002dd8: 0xb9400027,
    0x48002de0: 0xb9000020, 0x48002df4: 0xb9400001,
    0x48002dfc: 0xb9000003, 0x48002e00: 0xb94000c0,
    0x48002e08: 0xb90000c0, 0x48002e0c: 0xb94000a0,
    0x48002e14: 0xb90000a0, 0x48002e20: 0xb9400001,
    0x48002e28: 0xb9000002,
}
OPS_MEMBER_NAMES = (
    "affinst_standby",
    "affinst_on",
    "affinst_off",
    "affinst_suspend",
    "affinst_on_finish",
    "affinst_suspend_finish",
    "system_off",
    "system_reset",
    "get_sys_suspend_power_state",
)

STRINGS = (
    b"psci_plat_pm_ops",
    b"psci_plat_pm_ops->get_sys_suspend_power_state",
    b"psci_system_suspend",
    b"Sunxi System Reset",
    b"Sunxi System Reset: operation not handled",
    b"services/std_svc/psci/psci_main.c",
    b"services/std_svc/psci/psci_setup.c",
    b"psci_afflvl0_on",
)

DISASSEMBLY_WINDOWS = (
    ("ops_slot_0_raw", 0x48002510, 0x3c),
    ("ops_slot_1_cpu_on", 0x4800254c, 0x100),
    ("ops_slot_2_raw", 0x4800264c, 0x100),
    ("ops_slot_3_raw", 0x48002878, 0x180),
    ("ops_slot_4_raw", 0x48002680, 0x100),
    ("ops_slot_5_raw", 0x480026c4, 0x1b4),
    ("ops_slot_6_raw", 0x480025a0, 0x100),
    ("ops_slot_7_raw", 0x48002574, 0x100),
    ("ops_slot_8_raw", 0x48002544, 0x100),
    ("legacy_slot0_standby_dispatch", 0x48009680, 0x20),
    ("legacy_slot2_off_dispatch", 0x480080f0, 0x40),
    ("legacy_slot3_suspend_dispatch", 0x48008a74, 0x24),
    ("legacy_slot4_on_finish_dispatch", 0x48008544, 0x20),
    ("legacy_slot5_suspend_finish_dispatch", 0x48008910, 0x34),
    ("legacy_slot8_system_suspend_getter_dispatch", 0x48009714, 0x50),
    ("system_suspend_transition_dispatch", 0x48008d98, 0x140),
    ("suspend_afflvl0_wrapper", 0x48008b84, 0x100),
    ("suspend_afflvl1_wrapper", 0x48008ab8, 0x100),
    ("suspend_afflvl2_wrapper", 0x48008a00, 0x100),
    ("system_suspend_power_entry", 0x48001a04, 0x300),
    ("system_suspend_sram_memcpy", 0x4800aa80, 0x20),
    ("sram_resume_entry_source", 0x4800c3d8, 0x40),
    ("sram_resume_power_and_rtc_source", 0x4800c418, 0x2f0),
    ("sram_resume_stack_setup_source", 0x480140a0, 0x18),
    ("sram_resume_return_target", 0x48015d14, 0x90),
    ("cpux_axi_clock_transition_helper", 0x48011ff8, 0x100),
    ("cpux_axi_wait_target", 0x48014078, 0x100),
    ("system_suspend_helper_48011b40", 0x48011b40, 0x100),
    ("system_suspend_helper_48011ba8", 0x48011ba8, 0x100),
    ("system_suspend_helper_48011df8", 0x48011df8, 0x100),
    ("system_suspend_helper_48011b58", 0x48011b58, 0x100),
    ("system_suspend_helper_48011b78", 0x48011b78, 0x100),
    ("system_suspend_helper_48013df0", 0x48013df0, 0x100),
    ("system_suspend_helper_48013ee8", 0x48013ee8, 0x100),
    ("system_suspend_helper_48012670", 0x48012670, 0x100),
    ("system_suspend_helper_48011ba8_full", 0x48011ba8, 0x240),
    ("system_suspend_helper_48011df8_full", 0x48011df8, 0x180),
    ("system_suspend_helper_48013ee8_full", 0x48013ee8, 0x180),
    ("system_suspend_helper_48011b38", 0x48011b38, 0x18),
    ("system_suspend_helper_48011920", 0x48011920, 0x180),
    ("system_suspend_helper_48011b10", 0x48011b10, 0x30),
    ("system_suspend_helper_48011b30", 0x48011b30, 0x20),
    ("system_suspend_helper_48012070", 0x48012070, 0x100),
    ("system_suspend_helper_480126b0", 0x480126b0, 0x100),
    ("system_suspend_helper_48012868", 0x48012868, 0x100),
    ("system_suspend_helper_48013490", 0x48013490, 0x100),
    ("system_suspend_helper_480125d0", 0x480125d0, 0x100),
    ("system_suspend_helper_48013620", 0x48013620, 0x100),
    ("system_suspend_helper_48013738", 0x48013738, 0x100),
    ("system_suspend_helper_480131b8", 0x480131b8, 0x100),
    ("system_suspend_helper_48013588", 0x48013588, 0x100),
    ("system_suspend_helper_480135d8", 0x480135d8, 0x80),
    ("system_suspend_helper_48012510", 0x48012510, 0x100),
    ("system_suspend_helper_48013190", 0x48013190, 0x80),
    ("system_suspend_helper_48011ba8_complete", 0x48011ba8, 0x280),
    ("system_suspend_helper_48011df8_complete", 0x48011df8, 0x240),
    ("system_suspend_helper_48011d80_tail", 0x48011d80, 0x80),
    ("system_suspend_helper_48011920_complete", 0x48011920, 0x220),
    ("system_suspend_helper_48013df0_complete", 0x48013df0, 0x180),
    ("system_suspend_helper_480131b8_ir_capture", 0x480131b8, 0x180),
    ("system_suspend_helper_48013588_gpio_name", 0x48013588, 0x60),
    ("system_suspend_helper_480135d8_gpio_read", 0x480135d8, 0x60),
    ("system_suspend_helper_48013620_gpio_test", 0x48013620, 0x120),
    ("system_suspend_helper_480129f0", 0x480129f0, 0x100),
    ("system_suspend_helper_48012a08", 0x48012a08, 0x100),
    ("system_suspend_helper_48012aa0", 0x48012aa0, 0x100),
    ("system_suspend_helper_48013b98", 0x48013b98, 0x100),
    ("normal_suspend_powerdown_helper", 0x48015d70, 0x80),
    ("system_suspend_cache_teardown_and_trampoline", 0x48015fb8, 0x270),
    ("system_suspend_caller_full", 0x480096f8, 0xa0),
    ("system_suspend_descriptor_writer", 0x480022fc, 0x20),
    ("arm_svc_checksum_descriptor_handler", 0x480035d0, 0x6b0),
    ("system_suspend_setup_caller", 0x48002400, 0x80),
    ("system_suspend_descriptor_handoff", 0x48002284, 0x40),
    ("bl31_boot_memory_map_setup", 0x48003374, 0xb0),
    ("system_suspend_retention_helper_48011710", 0x48011710, 0x210),
    ("retention_bank_selector_helper", 0x4800c730, 0x70),
    ("retention_rtc_fallback_setup_helper", 0x4800c7a8, 0x88),
    ("retention_restore_gate_helper", 0x48011320, 0x40),
    ("retention_restore_dram_clock_helper", 0x480111a0, 0x180),
    ("retention_restore_dram_state_helper", 0x480115c0, 0x150),
    ("retention_restore_lower_helper_48010f98", 0x48010f98, 0x208),
    ("retention_restore_lower_helper_48011360", 0x48011360, 0x1d0),

    ("ccu_save_helper", 0x48001790, 0x100),
    ("ccu_restore_helper", 0x480018d8, 0x100),
    ("restore_callsite_context", 0x48002838, 0x80),
    ("save_callsite_context", 0x48002978, 0x80),
    ("psci_dispatch_cpu_on_primary", 0x480085e0, 0x90),
    ("psci_dispatch_cpu_on_secondary", 0x48008690, 0x80),
    ("psci_dispatch_smc_range", 0x48007f00, 0x900),
    ("psci_suspend_dispatch_tail", 0x480087f4, 0x500),
    ("psci_post_suspend_dispatch_and_helpers", 0x48008cf0, 0x10d0),
    ("psci_platform_registration", 0x48009dc0, 0x80),
    ("power_system_off_helper", 0x48001ec8, 0x60),
    ("power_cpu_off_helper", 0x48001ed8, 0x60),
)

def read_word(raw: bytes, address: int) -> int:
    offset = address - BASE
    if offset < 0 or offset + 4 > len(raw):
        raise RuntimeError("address outside payload: 0x%08x" % address)
    return struct.unpack_from("<I", raw, offset)[0]

def decode_bl_target(pc: int, instruction: int) -> int:
    if (instruction & 0xfc000000) != 0x94000000:
        raise RuntimeError("expected AArch64 BL at 0x%08x" % pc)
    immediate = instruction & 0x03ffffff
    if immediate & (1 << 25):
        immediate -= 1 << 26
    return pc + (immediate << 2)

def decode_imm19_branch_target(pc: int, instruction: int) -> int:
    if (instruction & 0x7E000000) != 0x34000000:
        raise RuntimeError("instruction at 0x%08x is not CBZ/CBNZ" % pc)
    immediate = (instruction >> 5) & 0x7FFFF
    if immediate & (1 << 18):
        immediate -= 1 << 19
    return (pc + (immediate << 2)) & 0xFFFFFFFF


def decode_cond_branch_target(pc: int, instruction: int) -> int:
    if (instruction & 0xFF000010) != 0x54000000:
        raise RuntimeError("instruction at 0x%08x is not B.cond" % pc)
    immediate = (instruction >> 5) & 0x7FFFF
    if immediate & (1 << 18):
        immediate -= 1 << 19
    return (pc + (immediate << 2)) & 0xFFFFFFFF


def decode_b_target(pc: int, instruction: int) -> int:
    if (instruction & 0xfc000000) != 0x14000000:
        raise RuntimeError("expected AArch64 B at 0x%08x" % pc)
    immediate = instruction & 0x03ffffff
    if immediate & (1 << 25):
        immediate -= 1 << 26
    return pc + (immediate << 2)

def render_disassembly(raw: bytes) -> str:
    try:
        from capstone import Cs, CS_ARCH_ARM64, CS_MODE_ARM
    except ImportError as exc:
        raise RuntimeError("Capstone is required for callback disassembly") from exc
    decoder = Cs(CS_ARCH_ARM64, CS_MODE_ARM)
    lines = [
        "# Matched BL31 PSCI/CCU callback windows",
        "Slot semantics are mapped only where table offsets, dispatcher callsites, embedded strings and behavior provide binary evidence.",
        "",
    ]
    for label, start, length in DISASSEMBLY_WINDOWS:
        offset = start - BASE
        if offset < 0 or offset + length > len(raw):
            raise RuntimeError("disassembly window outside monitor: %s" % label)
        lines.append("## %s [0x%08x, 0x%08x)" % (label, start, start + length))
        for insn in decoder.disasm(raw[offset:offset + length], start):
            operands = (" " + insn.op_str) if insn.op_str else ""
            lines.append("0x%08x: %-8s%s" % (insn.address, insn.mnemonic, operands))
        lines.append("")
    return "\n".join(lines) + "\n"

def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--monitor", type=Path, required=True)
    parser.add_argument("--baseos-root", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()

    raw = args.monitor.read_bytes()
    digest = hashlib.sha256(raw).hexdigest()
    if len(raw) != SIZE or digest != SHA:
        raise RuntimeError("BL31 identity mismatch; refusing PSCI audit")

    for address, expected in WORDS.items():
        actual = read_word(raw, address)
        if actual != expected:
            raise RuntimeError(
                "instruction mismatch at 0x%08x: got 0x%08x expected 0x%08x"
                % (address, actual, expected)
            )

    offset = TABLE - BASE
    table = list(struct.unpack_from("<9Q", raw, offset))
    if table != EXPECTED_TABLE:
        raise RuntimeError("platform ops table changed: %s" %
                           [hex(value) for value in table])
    if len(table) != len(OPS_MEMBER_NAMES):
        raise RuntimeError("legacy PSCI platform-ops member count changed")
    if table[8] != 0x48002544:
        raise RuntimeError("get_sys_suspend_power_state slot 8 pointer changed")
    suspend_level_table = list(struct.unpack_from(
        "<3Q", raw, SUSPEND_LEVEL_TABLE - BASE
    ))
    if suspend_level_table != list(SUSPEND_LEVEL_CALLBACKS):
        raise RuntimeError("PSCI suspend affinity-level callback map changed")

    if decode_bl_target(0x48009dec, read_word(raw, 0x48009dec)) != 0x480029bc:
        raise RuntimeError("platform ops registration call changed")
    suspend_dispatch_words = {
        0x48008a18: 0x71000a9f,  # afflvl2 wrapper checks affinity level 2
        0x48008a84: 0xf9400c26,  # platform ops +0x18
        0x48008a90: 0xd63f00c0,  # call affinst_suspend
        0x48008ad0: 0x7100069f,  # afflvl1 wrapper checks affinity level 1
        0x48008b34: 0xf9400c26,
        0x48008b40: 0xd63f00c0,
        0x48008c20: 0xf9400c36,  # afflvl0 wrapper platform ops +0x18
        0x48008c54: 0xd63f02c0,
        0x48008e4c: 0xf0000077,  # callback table base page
        0x48008e54: 0x9135c2f7,  # callback table +0xd70
        0x48008e64: 0x0b160281,  # current level = start + index
        0x48008e68: 0x6b0102bf,  # compare current level with end
        0x48008e70: 0xf8767b60,  # state record for current level
        0x48008e74: 0xb4000100,  # skip null state record
        0x48008e78: 0xf861dae4,  # callback_table[current_affinity]
        0x48008e88: 0xd63f0080,  # indirect wrapper call
        0x48008e94: 0x910006d6,  # increment index
        0x48008e98: 0x17fffff3,  # loop
        0x4800299c: 0x12800060,  # affinst_suspend returns -4 after save path
        0x48009768: 0x3100101f,  # compare return against -4
        0x48009770: 0x54000081,  # branch if not equal
        0x48009774: 0x97ffe0a4,  # -4 path -> 0x48001a04
    }
    for address, expected in suspend_dispatch_words.items():
        actual = read_word(raw, address)
        if actual != expected:
            raise RuntimeError(
                "SYSTEM_SUSPEND transition mismatch at 0x%08x: got 0x%08x expected 0x%08x"
                % (address, actual, expected)
            )
    if decode_bl_target(0x48009774, read_word(raw, 0x48009774)) != 0x48001a04:
        raise RuntimeError("SYSTEM_SUSPEND -4 power-entry target changed")
    copy_length = SRAM_SUSPEND_COPY_END - SRAM_SUSPEND_COPY_SOURCE
    mapped_entry = SRAM_SUSPEND_COPY_DEST + (
        SRAM_SUSPEND_ENTRY_SOURCE - SRAM_SUSPEND_COPY_SOURCE
    )
    mapped_stack_setup = SRAM_SUSPEND_COPY_DEST + (
        SRAM_SUSPEND_STACK_SETUP_SOURCE - SRAM_SUSPEND_COPY_SOURCE
    )
    scratch_start = SRAM_SUSPEND_COPY_DEST + (
        SRAM_SUSPEND_SCRATCH_SOURCE_START - SRAM_SUSPEND_COPY_SOURCE
    )
    scratch_end_exclusive = SRAM_SUSPEND_COPY_DEST + (
        SRAM_SUSPEND_SCRATCH_SOURCE_LAST - SRAM_SUSPEND_COPY_SOURCE
    ) + 1
    if copy_length != 0x9410:
        raise RuntimeError("SRAM standby image copy length changed")
    if mapped_entry != SRAM_SUSPEND_ENTRY:
        raise RuntimeError("SRAM standby entry/source mapping changed")
    if mapped_stack_setup != 0x27e80:
        raise RuntimeError("SRAM standby stack-setup mapping changed")
    if (scratch_start, scratch_end_exclusive) != (0x291f0, 0x29349):
        raise RuntimeError("relocated SRAM scratch/BSS range changed")
    copy_calls = (
        (0x48001a2c, 0x4800aa80),
        (0x48001a48, 0x4800aa80),
        (0x48001aa8, 0x4800aa80),
        (0x48001b10, 0x4800aa80),
        (0x48001b30, 0x4800aa80),
    )
    for call_pc, target in copy_calls:
        if decode_bl_target(call_pc, read_word(raw, call_pc)) != target:
            raise RuntimeError("SRAM image/context memcpy target changed at 0x%08x" % call_pc)
    copy_instruction_words = {
        0x48001a08: 0xf0000041,  # ADRP source page
        0x48001a0c: 0x900000a2,  # ADRP source end page
        0x48001a10: 0x91088021,  # source +0x220
        0x48001a14: 0x9118c042,  # end +0x630
        0x48001a1c: 0xcb010042,  # end - source
        0x48001a20: 0xd2a00040,  # destination 0x20000
        0x48001a28: 0xd2a00053,  # retained SRAM base
        0x48001a34: 0xf00001c1,  # auxiliary source page 0x4803c000
        0x48001a38: 0xd2801000,  # copy size 0x80 low bits
        0x48001a3c: 0x912e4021,  # source +0xb90
        0x48001a40: 0xd2800602,  # copy length 0x30
        0x48001a44: 0xf2a00040,  # destination 0x20080
        0x48001a90: 0xd2800601,  # header source offset 0x30
        0x48001a94: 0xd2801002,  # copy length 0x80
        0x48001a98: 0xb9436a80,  # context global
        0x48001a9c: 0xf2a90001,  # source base 0x48000000
        0x48001aa4: 0xaa1303e0,  # destination retained x19=0x20000
        0x48001aac: 0xd2802081,  # RTC/entry address offset setup
        0x48001ab4: 0xf2a0e001,  # 0x07000104
        0x48001b00: 0xd2802680,  # conditional destination 0x20134
        0x48001b04: 0x910e0021,  # conditional source +0x380
        0x48001b08: 0xd2801002,  # conditional copy length 0x80
        0x48001b0c: 0xf2a00040,  # destination high half
        0x48001b18: 0xd00000c1,  # second conditional data source
        0x48001b1c: 0x910c2021,  # source +0x308
        0x48001b20: 0xd2800c02,  # second conditional copy length 0x60
        0x48001b28: 0xd2801a80,  # destination low 0xd4
        0x48001b2c: 0xf2a00040,  # destination high half
    }
    for address, expected in copy_instruction_words.items():
        if read_word(raw, address) != expected:
            raise RuntimeError("SRAM copy/context instruction changed at 0x%08x" % address)
    if read_word(raw, 0x48001a54) != 0xb900c660:
        raise RuntimeError("saved BL31 wake-return address store changed")
    if decode_bl_target(0x4800c3fc, read_word(raw, 0x4800c3fc)) != 0x480140a0:
        raise RuntimeError("relocated SRAM stack-setup call changed")
    if decode_bl_target(0x4800c400, read_word(raw, 0x4800c400)) != 0x4800c418:
        raise RuntimeError("relocated SRAM RTC/context setup call changed")
    if decode_bl_target(0x4800c404, read_word(raw, 0x4800c404)) != 0x4800c4d0:
        raise RuntimeError("relocated SRAM powerdown helper call changed")
    if decode_bl_target(0x4800c408, read_word(raw, 0x4800c408)) != 0x4800c618:
        raise RuntimeError("relocated SRAM final standby helper call changed")
    stack_literal = struct.unpack_from("<Q", raw, SRAM_SUSPEND_STACK_SETUP_SOURCE - BASE + 0x10)[0]
    if stack_literal != SRAM_SUSPEND_STACK_TOP:
        raise RuntimeError("relocated SRAM stack-top literal changed")
    if decode_bl_target(0x4800c53c, read_word(raw, 0x4800c53c)) != 0x48013df0:
        raise RuntimeError("SRAM secondary WFI path helper changed")
    if decode_bl_target(0x4800c568, read_word(raw, 0x4800c568)) != 0x48013ee8:
        raise RuntimeError("SRAM WFI return-check helper changed")
    # The H616 reference manual names 0x03001500 CPUX_AXI_CFG_REG,
    # with CLK_SRC_SEL at bits 26:24. This image modifies only bits 25:24.
    cpux_axi_reg = (0x300 << 16) | 0x1500
    if cpux_axi_reg != 0x03001500:
        raise RuntimeError("CPUX_AXI_CFG_REG address construction changed")
    cpux_clock_words = {
        0x4800c460: 0xd282a002,  # mov x2, #0x1500
        0x4800c468: 0xf2a06002,  # movk x2, #0x300, lsl #16
        0x4800c470: 0xb9400041,  # ldr w1, [x2]
        0x4800c474: 0x12067421,  # and w1, w1, #0xfcffffff
        0x4800c478: 0xb9000041,  # str w1, [x2]
        0x4800c47c: 0x940016df,  # bl 0x48011ff8
        0x4800c680: 0xd282a002,  # mov x2, #0x1500
        0x4800c684: 0xf2a06002,  # movk x2, #0x300, lsl #16
        0x4800c690: 0xb9400041,  # ldr w1, [x2]
        0x4800c694: 0x32080421,  # orr w1, w1, #0x03000000
        0x4800c698: 0xb9000041,  # str w1, [x2]
        0x4800c69c: 0x94001657,  # bl 0x48011ff8
        0x4800c6a8: 0xd63f02c0,  # blr x22, RTC-saved resume PC
    }
    for address, expected in cpux_clock_words.items():
        if read_word(raw, address) != expected:
            raise RuntimeError("CPUX_AXI clock switch instruction changed at 0x%08x" % address)
    # Clock/context save path: CCU fields, PLL_CPUX factors and opaque AON tag.
    suspend_clock_save_words = {
        0x48011bb0: 0xf2a06005, 0x48011bc4: 0xb94000a7,
        0x48011bd4: 0x0a0100e1, 0x48011be0: 0xb9041441,
        0x48011c28: 0x528063e2, 0x48011c2c: 0x72a06002,
        0x48011c54: 0x12067463, 0x48011c5c: 0x32080063,
        0x48011c64: 0xb9000103, 0x48011c68: 0xd2804888,
        0x48011c6c: 0xf2a0e028, 0x48011c70: 0x52b4e003,
        0x48011c90: 0xb94000a1, 0x48011c94: 0x12067421,
        0x48011c98: 0x32080021, 0x48011c9c: 0xb90000a1,
        0x48011ca8: 0x0a040021, 0x48011cac: 0x120078a5,
        0x48011cb0: 0xb9000045, 0x48011cb4: 0xb9001c01,
        0x48011cbc: 0x12027800, 0x48011cc0: 0xb9000040,
        0x48011cc4: 0xb9400100, 0x48011cc8: 0x12005c00,
        0x48011ccc: 0x2a030000, 0x48011cd0: 0xb9000100,
        0x48011d88: 0xd282a205, 0x48011da8: 0x12067421,
        0x48011dac: 0x32070021, 0x48011db0: 0xb90000a1,
        0x48011dd8: 0x32010021, 0x48011ddc: 0xb9000041,
        0x48011de4: 0x121e7821, 0x48011de8: 0xb9000041,
        0x48011df0: 0x17ffffc0,
    }
    for address, expected in suspend_clock_save_words.items():
        if read_word(raw, address) != expected:
            raise RuntimeError("suspend clock-save instruction changed at 0x%08x" % address)
    if decode_bl_target(0x48011d2c, read_word(raw, 0x48011d2c)) != 0x48013620:
        raise RuntimeError("suspend GPIO configuration call A changed")
    if decode_bl_target(0x48011d6c, read_word(raw, 0x48011d6c)) != 0x48013620:
        raise RuntimeError("suspend GPIO configuration call B changed")
    # Resume path restores PLL factor bits, enables it and polls LOCK bit28.
    pll_resume_words = {
        0x48011e10: 0x9400089a,
        0x48011e48: 0x12027884,
        0x48011e54: 0x0a030083, 0x48011e58: 0x2a020062,
        0x48011e5c: 0xb9000002, 0x48011e64: 0x32010042,
        0x48011e68: 0xb9000002, 0x48011e70: 0x32030042,
        0x48011e74: 0xb9000002, 0x48011e78: 0xb9400020,
        0x48011e7c: 0x36e7ffe0, 0x48011e94: 0x52800280,
        0x48011ec0: 0x9400004e,
    }
    for address, expected in pll_resume_words.items():
        if read_word(raw, address) != expected:
            raise RuntimeError("PLL resume instruction changed at 0x%08x" % address)
    if decode_bl_target(0x48011e10, read_word(raw, 0x48011e10)) != 0x48014078:
        raise RuntimeError("PLL resume pre-delay target changed")
    # The OEM mode==2 branch selects the low-power mux encoding and edits RTC XO_CTRL.
    mode2_address_words = {
        0x48011d88: 0xd282a205,  # x5 = 0x1510
        0x48011d8c: 0xd2a0e024,  # x4 = 0x07010000
        0x48011d90: 0xf2a06005,  # x5 = 0x03001510
        0x48011d94: 0xd2800203,  # x3 = 0x10
        0x48011d98: 0xf2a0e023,  # x3 = 0x07010010
        0x48011d9c: 0xd2802c02,  # x2 = 0x160
        0x48011da0: 0xf2a0e002,  # x2 = 0x07000160
        0x48011da4: 0xb94000a1,
        0x48011da8: 0x12067421, 0x48011dac: 0x32070021,
        0x48011db0: 0xb90000a1,  # CPUX_AXI[25:24] = 10
        0x48011db4: 0xb9400081,
        0x48011db8: 0x12067421, 0x48011dbc: 0x32070021,
        0x48011dc0: 0xb9000081,  # R_PRCM+0x000[25:24] = 10
        0x48011dc4: 0xb9400061,
        0x48011dc8: 0x12067421, 0x48011dcc: 0x32070021,
        0x48011dd0: 0xb9000061,  # R_PRCM+0x010[25:24] = 10
        0x48011dd4: 0xb9400041,
        0x48011dd8: 0x32010021,  # XO_CTRL bit31 set
        0x48011ddc: 0xb9000041,
        0x48011de0: 0xb9400041,
        0x48011de4: 0x121e7821,  # XO_CTRL bit1 clear
        0x48011de8: 0xb9000041,
    }
    for address, expected in mode2_address_words.items():
        if read_word(raw, address) != expected:
            raise RuntimeError("standby mode-2 clock/XO instruction changed at 0x%08x" % address)
    if ((0x300 << 16) | 0x1510) != 0x03001510:
        raise RuntimeError("CPUX/PSI bus-clock register address decode changed")
    if ((0x701 << 16) | 0x10) != 0x07010010:
        raise RuntimeError("R_PRCM+0x010 register address decode changed")
    if ((0x700 << 16) | 0x160) != 0x07000160:
        raise RuntimeError("RTC XO_CTRL register address decode changed")
    # Cross-referenced same-family register labels: R_PRCM +0x244 PLL LDO key;
    # RTC +0x160 XO_CTRL. These names are not an OEM source-provenance claim.
    aon_key_address = (0x701 << 16) | 0x244
    if aon_key_address != 0x07010244:
        raise RuntimeError("R_PRCM PLL LDO key address decode changed")
    if read_word(raw, 0x48011c70) != 0x52b4e003:
        raise RuntimeError("PLL LDO key high-byte immediate changed")
    if read_word(raw, 0x48011c80) != 0x12067421 or read_word(raw, 0x48011c88) != 0x32080021:
        raise RuntimeError("PLL LDO key high-byte merge changed")
    if decode_bl_target(0x48011ec0, read_word(raw, 0x48011ec0)) != 0x48011ff8:
        raise RuntimeError("PLL resume settle-delay target changed")
    if decode_bl_target(0x48011ec0, read_word(raw, 0x48011ec0)) != 0x48011ff8:
        raise RuntimeError("PLL resume settle-delay target changed")
    # H616-compatible GICD_ISPENDR, IR FIFO/status and GPIO pin helpers.
    wake_source_words = {
        0x48013194: 0xd2824003, 0x4801319c: 0xf2a06043,
        0x480131a0: 0x531e7401, 0x480131a4: 0x52800022,
        0x480131a8: 0x1ac42040, 0x480131ac: 0xb8614861,
        0x480131b0: 0x0a010000, 0x480131b4: 0xd65f03c0,
        0x480131c8: 0xf2a0e089, 0x480131cc: 0xf2a0e086,
        0x480131d8: 0xb9400128, 0x480131dc: 0x12001d05,
        0x480131e0: 0xd3483901, 0x480131e4: 0xb9000125,
        0x48013200: 0xb94000c4, 0x4801320c: 0x39001024,
        0x48013598: 0x51010442, 0x480135a4: 0x5100c060,
        0x480135b4: 0x2a024000, 0x480135d8: 0x53107c01,
        0x480135e0: 0x7100283f, 0x480135f0: 0x72a06002,
        0x480135f8: 0x1a843042, 0x48013600: 0x531e7421,
        0x48013608: 0x0b020021, 0x4801360c: 0xb9400021,
        0x48013610: 0x6a01001f, 0x48013614: 0x1a9f07e0,
        0x48013654: 0x53107c26, 0x48013660: 0x72a06003,
        0x48013668: 0x710028df, 0x48013674: 0x1a883068,
        0x4801367c: 0x531e7444, 0x480136c4: 0x0a250085,
        0x480136d4: 0xb90000e5, 0x480136dc: 0xb9401007,
        0x480136f4: 0xb9000044, 0x48013714: 0xb90000c2,
        0x48013724: 0xb9000060,
    }
    for address, expected in wake_source_words.items():
        if read_word(raw, address) != expected:
            raise RuntimeError("wake-source helper instruction changed at 0x%08x" % address)
    wake_call_edges = {
        0x48013e88: 0x48013620, 0x48013ec8: 0x48013620,
        0x48013f14: 0x480131b8, 0x48013f48: 0x48013588,
        0x48013f54: 0x480135d8, 0x48013f8c: 0x48013588,
        0x48013f94: 0x480135d8, 0x48013fe4: 0x48013190,
        0x48014050: 0x480131b8,
    }
    for call_pc, expected_target in wake_call_edges.items():
        if decode_bl_target(call_pc, read_word(raw, call_pc)) != expected_target:
            raise RuntimeError("wake-source helper call target changed at 0x%08x" % call_pc)
    if decode_bl_target(0x4800c47c, read_word(raw, 0x4800c47c)) != 0x48011ff8:
        raise RuntimeError("CPUX_AXI standby transition helper changed")
    if decode_bl_target(0x4800c69c, read_word(raw, 0x4800c69c)) != 0x48011ff8:
        raise RuntimeError("CPUX_AXI resume transition helper changed")
    wait_loop_words = {
        0x48011ff8: 0x531e7400,  # lsl w0, w0, #2
        0x48011ffc: 0x1400081f,  # b 0x48014078
        0x48014078: 0xf1000400,  # subs x0, x0, #1
        0x4801407c: 0x54ffffe8,  # b.hi 0x48014078
        0x48014080: 0xd65f03c0,  # ret
    }
    for address, expected in wait_loop_words.items():
        if read_word(raw, address) != expected:
            raise RuntimeError("CPUX_AXI wait-loop instruction changed at 0x%08x" % address)
    if decode_b_target(0x48011ffc, read_word(raw, 0x48011ffc)) != 0x48014078:
        raise RuntimeError("CPUX_AXI wait wrapper target changed")
    if decode_cond_branch_target(0x4801407c, read_word(raw, 0x4801407c)) != 0x48014078:
        raise RuntimeError("CPUX_AXI wait loop branch target changed")
    if read_word(raw, 0x4800c464) != 0x52800140:
        raise RuntimeError("CPUX_AXI standby wait argument changed")
    if read_word(raw, 0x4800c688) != 0x52800140:
        raise RuntimeError("CPUX_AXI resume wait argument changed")
    if (0xfcffffff & 0x03000000) != 0:
        raise RuntimeError("CPUX_AXI clear-mask bit range changed")
    if (0x03000000 & 0x04000000) != 0:
        raise RuntimeError("CPUX_AXI transition mask unexpectedly changes bit 26")
    if (0x03000000 & 0x03000000) != 0x03000000:
        raise RuntimeError("CPUX_AXI transition mask no longer sets bits 25:24")
    wake_return_words = {
        0x48015d14: 0x1000fb77,  # adr x23, #0x48017c80
        0x48015d18: 0xd2820141,  # mov x1, #0x100a
        0x48015d1c: 0xd53e1000,  # mrs x0, SCTLR_EL3
        0x48015d20: 0xaa010000,  # orr x0, x0, x1
        0x48015d24: 0xd51e1000,  # msr SCTLR_EL3, x0
        0x48015d28: 0xd5033fdf,  # isb
        0x48015d30: 0x10019680,  # adr x0, #0x48019000
        0x48015d34: 0xd51ec000,  # msr VBAR_EL3, x0
        0x48015d38: 0xd5033fdf,  # isb
        0x48015d3c: 0xd50344ff,  # msr DAIFClr, #4
        0x48015d40: 0xd50040bf,  # msr SPSel, #0
        0x48015d44: 0xd53800a0,  # mrs x0, MPIDR_EL1
        0x48015d48: 0x97ffd930,  # restore context helper
        0x48015d68: 0x97ffcdd9,  # restore scheduler/PSCI continuation
        0x48015d6c: 0x17fffed7,  # b to BL31 handoff continuation
    }
    for address, expected in wake_return_words.items():
        if read_word(raw, address) != expected:
            raise RuntimeError("BL31 wake-return continuation changed at 0x%08x" % address)
    rtc_instruction_words = {
        0x4800c580: 0xd2803f00,  # 0x1f8
        0x4800c584: 0x52a00023,  # standby flag 0x10000
        0x4800c588: 0xf2a0e000,  # 0x07000000 high half
        0x4800c598: 0xb9000003,  # write standby flag
        0x4800c594: 0xd2803f84,  # 0x1fc
        0x4800c59c: 0xf2a0e004,  # 0x07000000 high half
        0x4800c5a8: 0xb940c426,  # saved return address from context +0xc4
        0x4800c5b8: 0xb9000086,  # write return address
    }
    for address, expected in rtc_instruction_words.items():
        if read_word(raw, address) != expected:
            raise RuntimeError("SRAM RTC standby instruction changed at 0x%08x" % address)
    if read_word(raw, 0x4800c50c) != 0xd503207f or read_word(raw, 0x4800c550) != 0xd503207f:
        raise RuntimeError("relocated SRAM WFI path changed")
    # Optional additive checksum protects selected SRAM standby contents.
    checksum_words = {
        0x48011b78: 0x29560c01,  # ldp w1,w3,[x0,#0xb0]
        0x48011b7c: 0x52800000,  # mov w0,#0
        0x48011b80: 0x0b030023,  # add w3,w1,w3
        0x48011b84: 0x2a0103e1,  # mov w1,w1 (zero-extend)
        0x48011b88: 0xeb01007f,  # cmp x3,x1
        0x48011b8c: 0x540000c9,  # b.ls return zero
        0x48011b90: 0xb8404422,  # ldr w2,[x1],#4
        0x48011b94: 0xeb01007f,  # cmp x3,x1
        0x48011b98: 0x0b020000,  # add w0,w0,w2
        0x48011b9c: 0x54ffffa8,  # b.hi word loop
        0x48011ba0: 0xd65f03c0,
        0x4800c43c: 0xb940c260,  # load checksum enable flag at context +0xc0
        0x4800c440: 0x34000080,  # cbz skip initial checksum
        0x4800c444: 0xaa1303e0,  # x0 = standby context
        0x4800c448: 0x940015cc,  # bl checksum
        0x4800c44c: 0xb900ba60,  # store checksum at context +0xb8
        0x4800c6c0: 0xaa1503e0,  # x0 = context
        0x4800c6c4: 0x9400152d,  # bl checksum after wake
        0x4800c6cc: 0xb940b822,  # load saved checksum at +0xb8
        0x4800c6d0: 0xb900bea0,  # store resumed checksum at +0xbc
        0x4800c6d4: 0xb940bc20,  # load resumed checksum
        0x4800c6d8: 0x6b00005f,  # cmp
        0x4800c6dc: 0x54fffcc0,  # b.eq continue
        0x4800c6e0: 0x52800020,  # mov w0,#1
        0x4800c6e4: 0x72b10000,  # movk w0,#0x8800,lsl #16
        0x4800c6e8: 0xb9000280,  # write fail code to RTC scratch/status
        0x4800c6ec: 0x14000000,  # self-loop; fail-stop
    }
    for address, expected in checksum_words.items():
        if read_word(raw, address) != expected:
            raise RuntimeError("standby checksum instruction changed at 0x%08x" % address)

    # The standby context's +0xb0/+0xb4/+0xc0 fields are copied from this
    # embedded descriptor. Keep payload literals separate from instruction
    # words so the test distinguishes data identity from code-flow evidence.
    checksum_config = (
        read_word(raw, SUSPEND_CHECKSUM_CONFIG_BASE_ADDR),
        read_word(raw, SUSPEND_CHECKSUM_CONFIG_LENGTH_ADDR),
        read_word(raw, SUSPEND_CHECKSUM_CONFIG_ENABLE_ADDR),
    )
    if checksum_config != (
        SUSPEND_CHECKSUM_INITIAL_BASE,
        SUSPEND_CHECKSUM_INITIAL_LENGTH,
        SUSPEND_CHECKSUM_INITIAL_ENABLE,
    ):
        raise RuntimeError("standby checksum descriptor payload values changed")
    label_offset = SUSPEND_CHECKSUM_CONFIG_LABEL_ADDR - BASE
    if raw[label_offset:label_offset + 7] != b"chipid" + bytes((0,)):
        raise RuntimeError("standby checksum descriptor adjacent label changed")
    if SUSPEND_CHECKSUM_INITIAL_LENGTH % 4:
        raise RuntimeError("standby checksum range is not word aligned")
    if read_word(raw, ARM_SVC_NAME_POINTER_ADDRESS) != ARM_SVC_NAME_ADDRESS:
        raise RuntimeError("arm_svc runtime-service name pointer changed")
    if read_word(raw, ARM_SVC_HANDLER_RECORD_ADDRESS) != ARM_SVC_HANDLER_ADDRESS:
        raise RuntimeError("arm_svc runtime-service handler pointer changed")
    arm_svc_name_offset = ARM_SVC_NAME_ADDRESS - BASE
    if raw[arm_svc_name_offset:arm_svc_name_offset + 8] != b"arm_svc" + bytes((0,)):
        raise RuntimeError("arm_svc runtime-service name string changed")

    if decode_cond_branch_target(0x4800372c, read_word(raw, 0x4800372c)) != 0x48003b8c:
        raise RuntimeError("SMC32 arm_svc checksum command dispatch changed")
    if decode_cond_branch_target(0x48003a04, read_word(raw, 0x48003a04)) != 0x48003b8c:
        raise RuntimeError("SMC64 arm_svc checksum command dispatch changed")
    if decode_bl_target(0x48003b94, read_word(raw, 0x48003b94)) != 0x480022fc:
        raise RuntimeError("arm_svc checksum descriptor writer call changed")

    # The standby entry stores a runtime scratch pointer in RTC GP_DATA1.
    if read_word(raw, 0x48001abc) != 0xb9000020:
        raise RuntimeError("RTC GP_DATA1 retention-buffer pointer store changed")
    if RETENTION_BUFFER_ADDRESS != 0x4801b400 or RETENTION_BUFFER_END != 0x4801b680:
        raise RuntimeError("retention scratch-buffer bounds changed")
    if not (RETENTION_BUFFER_ADDRESS >= BASE + len(raw) and RETENTION_BUFFER_END > RETENTION_BUFFER_ADDRESS):
        raise RuntimeError("retention scratch buffer unexpectedly overlaps monitor file payload")
    if RETENTION_WORD_COUNT != 0xa0 or RETENTION_SEGMENT_WORDS != 0x50:
        raise RuntimeError("retention snapshot word counts changed")
    if decode_bl_target(0x4800c490, read_word(raw, 0x4800c490)) != 0x48011b40:
        raise RuntimeError("pre-suspend retention snapshot callsite changed")
    if decode_bl_target(0x48011b48, read_word(raw, 0x48011b48)) != 0x48011b38:
        raise RuntimeError("pre-suspend retention wrapper call changed")
    if decode_b_target(0x48011b50, read_word(raw, 0x48011b50)) != 0x48011920:
        raise RuntimeError("pre-suspend retention wrapper tail branch changed")
    if decode_bl_target(0x48011928, read_word(raw, 0x48011928)) != 0x48011710:
        raise RuntimeError("pre-suspend retention worker call changed")
    if decode_bl_target(0x4800c658, read_word(raw, 0x4800c658)) != 0x48011b58:
        raise RuntimeError("post-resume retention restore callsite changed")
    if decode_bl_target(0x48011b60, read_word(raw, 0x48011b60)) != 0x48011b10:
        raise RuntimeError("post-resume retention wrapper call changed")
    if decode_bl_target(0x48011b20, read_word(raw, 0x48011b20)) != 0x48011938:
        raise RuntimeError("post-resume retention worker call changed")
    if decode_bl_target(0x48011974, read_word(raw, 0x48011974)) != 0x48011320:
        raise RuntimeError("post-resume retention preparation helper changed")
    if decode_bl_target(0x48011990, read_word(raw, 0x48011990)) != 0x4800c730:
        raise RuntimeError("post-resume retention selector helper changed")
    if decode_bl_target(0x48011ae0, read_word(raw, 0x48011ae0)) != 0x4800c7a8:
        raise RuntimeError("post-resume retention fallback helper changed")
    if decode_cond_branch_target(0x48011754, read_word(raw, 0x48011754)) != 0x480118d0:
        raise RuntimeError("pre-copy alternate first-segment branch changed")
    if decode_cond_branch_target(0x48011788, read_word(raw, 0x48011788)) != 0x480118f8:
        raise RuntimeError("pre-copy alternate second-segment branch changed")
    if decode_cond_branch_target(0x4801176c, read_word(raw, 0x4801176c)) != 0x480117a4:
        raise RuntimeError("pre-copy 0xa0-word loop exit changed")
    if decode_cond_branch_target(0x48011774, read_word(raw, 0x48011774)) != 0x48011740:
        raise RuntimeError("pre-copy first-segment loop changed")
    if decode_cond_branch_target(0x480117a0, read_word(raw, 0x480117a0)) != 0x48011770:
        raise RuntimeError("pre-copy second-segment loop changed")
    if decode_imm19_branch_target(0x4801197c, read_word(raw, 0x4801197c)) != 0x48011a70:
        raise RuntimeError("post-resume retention preparation skip branch changed")
    if decode_cond_branch_target(0x48011a1c, read_word(raw, 0x48011a1c)) != 0x48011a80:
        raise RuntimeError("post-copy alternate first-segment branch changed")
    if decode_cond_branch_target(0x48011a50, read_word(raw, 0x48011a50)) != 0x48011aa8:
        raise RuntimeError("post-copy alternate second-segment branch changed")
    if decode_cond_branch_target(0x48011a3c, read_word(raw, 0x48011a3c)) != 0x48011a08:
        raise RuntimeError("post-copy first-segment loop changed")
    if decode_cond_branch_target(0x48011a68, read_word(raw, 0x48011a68)) != 0x48011a38:
        raise RuntimeError("post-copy second-segment loop changed")

    if decode_bl_target(0x48001c3c, read_word(raw, 0x48001c3c)) != 0x48001a04:
        raise RuntimeError("secondary SYSTEM_SUSPEND power-entry callsite changed")
    if decode_bl_target(0x48002440, read_word(raw, 0x48002440)) != 0x48001b74:
        raise RuntimeError("suspend setup helper caller changed")
    if decode_bl_target(0x48011928, read_word(raw, 0x48011928)) != 0x48011710:
        raise RuntimeError("suspend retention wrapper target changed")
    if decode_bl_target(0x48011b48, read_word(raw, 0x48011b48)) != 0x48011b38:
        raise RuntimeError("suspend helper no-op edge changed")
    if decode_b_target(0x48011b50, read_word(raw, 0x48011b50)) != 0x48011920:
        raise RuntimeError("suspend helper tail branch changed")
    if decode_bl_target(0x48011b60, read_word(raw, 0x48011b60)) != 0x48011b10:
        raise RuntimeError("suspend helper memory operation edge changed")
    if decode_bl_target(0x48011b64, read_word(raw, 0x48011b64)) != 0x48011b30:
        raise RuntimeError("suspend helper trailing edge changed")

    if decode_bl_target(0x4800c448, read_word(raw, 0x4800c448)) != 0x48011b78:
        raise RuntimeError("standby pre-entry checksum call changed")
    if decode_bl_target(0x4800c6c4, read_word(raw, 0x4800c6c4)) != 0x48011b78:
        raise RuntimeError("wake resume checksum call changed")
    if decode_imm19_branch_target(0x4800c440, read_word(raw, 0x4800c440)) != 0x4800c450:
        raise RuntimeError("checksum disabled skip branch changed")
    if decode_cond_branch_target(0x4800c6dc, read_word(raw, 0x4800c6dc)) != 0x4800c674:
        raise RuntimeError("checksum success branch target changed")
    if decode_b_target(0x4800c6ec, read_word(raw, 0x4800c6ec)) != 0x4800c6ec:
        raise RuntimeError("checksum mismatch fail-stop loop changed")
    if decode_bl_target(0x48001b5c, read_word(raw, 0x48001b5c)) != 0x48016128:
        raise RuntimeError("SYSTEM_SUSPEND trampoline preparation helper changed")
    cache_control_words = {
        0x4801612c: 0xd53e1000,  # mrs x0, SCTLR_EL3
        0x48016130: 0x8a210000,  # bic x0, x0, #5 (clear M and C)
        0x48016134: 0xd51e1000,  # msr SCTLR_EL3, x0
        0x48016138: 0xd5033fdf,  # isb
        0x48016140: 0x17ffffda,  # enter CLIDR-based cache-level maintenance
        0x480160a8: 0xd5390029,  # mrs x9, CLIDR_EL1
        0x48015fb8: 0xb40003e3,  # cache-level loop
        0x48015fe4: 0xd51a000a,  # msr CSSELR_EL1, x10
        0x48015fec: 0xd5390001,  # mrs x1, CCSIDR_EL1
        0x48016040: 0xd508764b,  # dc isw, x11
        0x48016060: 0xd5087e4b,  # dc cisw, x11
        0x48016080: 0xd5087a4b,  # dc csw, x11
    }
    for address, expected in cache_control_words.items():
        if read_word(raw, address) != expected:
            raise RuntimeError("SCTLR/cache maintenance instruction changed at 0x%08x" % address)
    if read_word(raw, 0x48001b64) != 0xd2803700:
        raise RuntimeError("SYSTEM_SUSPEND SRAM target MOV changed")
    if read_word(raw, 0x48001b6c) != 0xf2a00040:
        raise RuntimeError("SYSTEM_SUSPEND SRAM target MOVK changed")
    if read_word(raw, 0x48001b70) != 0xd61f0000:
        raise RuntimeError("SYSTEM_SUSPEND indirect SRAM branch changed")
    sram_target = 0x1b8 | (0x2 << 16)
    if sram_target != 0x000201b8:
        raise RuntimeError("SYSTEM_SUSPEND SRAM target arithmetic changed")
    normal_powerdown_words = {
        0x48015d70: 0xd5033f9f,  # dsb sy
        0x48015d74: 0xd503207f,  # wfi
        0x48015d78: 0x17ffffff,  # b 0x48015d74
    }
    for address, expected in normal_powerdown_words.items():
        if read_word(raw, address) != expected:
            raise RuntimeError("ordinary suspend DSB/WFI loop changed at 0x%08x" % address)
    dispatch_words = {
        0x48009684: 0xf947e021,  # global ops pointer load
        0x48009688: 0xf9400021,  # slot 0 (+0x00)
        0x48009690: 0xd63f0020,  # slot 0 indirect call
        0x48008608: 0xf9400433,  # slot 1 (+0x08)
        0x480080f8: 0xf9400835,  # slot 2 (+0x10)
        0x48008118: 0xd63f02a0,
        0x48008a84: 0xf9400c26,  # slot 3 (+0x18)
        0x48008a90: 0xd63f00c0,
        0x4800854c: 0xf9401003,  # slot 4 (+0x20)
        0x48008560: 0xd63f0060,
        0x48008918: 0xf9401421,  # slot 5 (+0x28)
        0x4800893c: 0xd63f0060,
        0x48009718: 0xf947e000,  # SYSTEM_SUSPEND global ops pointer load
        0x4800971c: 0xf9402000,  # slot 8 (+0x40)
        0x48009720: 0xb5000120,  # null check
        0x48009744: 0xd63f0000,  # indirect getter call
        0x48002544: 0x52a04020,  # mov w0, #0x02010000
        0x48002548: 0xd65f03c0,
        0x48002588: 0x97fffe50,  # system_reset -> helper
        0x480025a8: 0x97fffe4c,  # system_off -> helper
        0x4800975c: 0x97fffd8f,  # getter result -> suspend transition
    }
    for address, expected in dispatch_words.items():
        actual = read_word(raw, address)
        if actual != expected:
            raise RuntimeError(
                "legacy PSCI dispatch mismatch at 0x%08x: got 0x%08x expected 0x%08x"
                % (address, actual, expected)
            )
    if decode_bl_target(0x48002588, read_word(raw, 0x48002588)) != 0x48001ec8:
        raise RuntimeError("system_reset helper target changed")
    if decode_bl_target(0x480025a8, read_word(raw, 0x480025a8)) != 0x48001ed8:
        raise RuntimeError("system_off helper target changed")
    if decode_bl_target(0x4800975c, read_word(raw, 0x4800975c)) != 0x48008d98:
        raise RuntimeError("SYSTEM_SUSPEND transition target changed")
    if decode_bl_target(0x4800255c, read_word(raw, 0x4800255c)) != 0x48002d78:
        raise RuntimeError("CPU_ON callback target changed")
    if table[3] != 0x48002878:
        raise RuntimeError("platform-ops slot 3 pointer changed")
    if table[4] != 0x48002680:
        raise RuntimeError("platform-ops slot 4 pointer changed")
    if table[5] != 0x480026c4:
        raise RuntimeError("platform-ops slot 5 pointer changed")
    if decode_bl_target(0x48002998, read_word(raw, 0x48002998)) != 0x48001790:
        raise RuntimeError("slot-3 CCU context-save call changed")
    if decode_bl_target(0x48002858, read_word(raw, 0x48002858)) != 0x480018d8:
        raise RuntimeError("slot-5 CCU context-restore call changed")
    if decode_b_target(0x48002874, read_word(raw, 0x48002874)) != table[4]:
        raise RuntimeError("slot-5 tail branch to slot-4 target changed")
    if decode_bl_target(0x48002d8c, read_word(raw, 0x48002d8c)) != 0x48002d30:
        raise RuntimeError("per-core RVBAR helper call changed")
    if decode_bl_target(0x48002d90, read_word(raw, 0x48002d90)) != 0x48002d18:
        raise RuntimeError("SoC selection helper call changed")

    missing = [value.decode() for value in STRINGS if value not in raw]
    if missing:
        raise RuntimeError("expected embedded TF-A PSCI strings missing: %s" % missing)

    # Compare the BL31 die selector against the exact pinned native SPL rule.
    spl_path = args.baseos_root.resolve() / "experiments/h700-boot-speed/spl/native-boot.c"
    spl_source = spl_path.read_text(encoding="utf-8")
    spl_selector = "ulong rvbar = (readl((void *)0x03000024) & 7) == 2 ? 0x08100040 : 0x09010040;"
    if spl_selector not in spl_source:
        raise RuntimeError("pinned BaseOS native SPL RVBAR selector changed")
    primary_bank = (0x01202008 << 3) & 0xffffffff
    alternate_bank = (0x01020008 << 3) & 0xffffffff
    if primary_bank != 0x09010040 or alternate_bank != 0x08100040:
        raise RuntimeError("decoded BL31 per-core RVBAR bank arithmetic changed")
    bl31_bank = {value: (primary_bank if value == 0 else alternate_bank) for value in range(8)}
    spl_bank = {value: (alternate_bank if value == 2 else primary_bank) for value in range(8)}
    matching_selector_values = [value for value in range(8) if bl31_bank[value] == spl_bank[value]]
    mismatching_selector_values = [value for value in range(8) if bl31_bank[value] != spl_bank[value]]
    if matching_selector_values != [0, 2] or mismatching_selector_values != [1, 3, 4, 5, 6, 7]:
        raise RuntimeError("BL31/SPL RVBAR selector parity classification changed unexpectedly")

    report = {
        "schema": "rg34xx-bl31-psci-cpu-on-static-audit-v1",
        "monitor": {"bytes": len(raw), "sha256": digest, "base": "0x48000000"},
        "platform_ops": {
            "table_address": "0x480165f8",
            "table_bytes": len(table) * 8,
            "abi_family": "legacy Allwinner TF-A plat_pm_ops_t eight-member order plus vendor get_sys_suspend_power_state extension",
            "member_order": list(OPS_MEMBER_NAMES),
            "entries": ["0x%08x" % value for value in table],
            "members": [
                {"slot": index, "byte_offset": "0x%02x" % (index * 8),
                 "name": OPS_MEMBER_NAMES[index], "address": "0x%08x" % table[index]}
                for index in range(len(table))
            ],
            "slot_0": {"name": OPS_MEMBER_NAMES[0], "address": "0x%08x" % table[0],
                       "dispatch_load_pc": "0x48009688", "indirect_call_pc": "0x48009690"},
            "slot_1": {"name": OPS_MEMBER_NAMES[1], "address": "0x%08x" % table[1],
                       "dispatch_load_pc": "0x48008608", "indirect_branch_pc": "0x48008640"},
            "slot_2": {"name": OPS_MEMBER_NAMES[2], "address": "0x%08x" % table[2],
                       "dispatch_load_pc": "0x480080f8", "indirect_call_pc": "0x48008118"},
            "slot_3": {"name": OPS_MEMBER_NAMES[3], "address": "0x%08x" % table[3],
                       "dispatch_load_pc": "0x48008a84", "indirect_call_pc": "0x48008a90"},
            "slot_4": {"name": OPS_MEMBER_NAMES[4], "address": "0x%08x" % table[4],
                       "dispatch_load_pc": "0x4800854c", "indirect_call_pc": "0x48008560"},
            "slot_5": {"name": OPS_MEMBER_NAMES[5], "address": "0x%08x" % table[5],
                       "dispatch_load_pc": "0x48008918", "indirect_call_pc": "0x4800893c"},
            "slot_6": {"name": OPS_MEMBER_NAMES[6], "address": "0x%08x" % table[6],
                       "entry_helper_call_pc": "0x480025a8", "entry_helper_target": "0x48001ed8"},
            "slot_7": {"name": OPS_MEMBER_NAMES[7], "address": "0x%08x" % table[7],
                       "reset_log_function_pc": "0x48002584",
                       "entry_helper_call_pc": "0x48002588", "entry_helper_target": "0x48001ec8"},
            "slot_8": {"name": OPS_MEMBER_NAMES[8], "address": "0x%08x" % table[8],
                       "return_pc": "0x48002544", "return_value": "0x02010000",
                       "dispatch_load_pc": "0x4800971c", "indirect_call_pc": "0x48009744"},
            "registration_function": "0x480029bc",
            "registration_callsite": "0x48009dec",
        },
        "dispatch": {
            "global_pointer_address": "0x4801bfc0",
            "global_string": "psci_plat_pm_ops",
            "member_order": list(OPS_MEMBER_NAMES),
            "member_loads": {
                "slot_0": {"offset": "0x00", "pc": "0x48009688"},
                "slot_1": {"offset": "0x08", "pc": "0x48008608"},
                "slot_2": {"offset": "0x10", "pc": "0x480080f8"},
                "slot_3": {"offset": "0x18", "pc": "0x48008a84"},
                "slot_4": {"offset": "0x20", "pc": "0x4800854c"},
                "slot_5": {"offset": "0x28", "pc": "0x48008918"},
                "slot_8": {"offset": "0x40", "pc": "0x4800971c"},
            },
            "indirect_calls": {
                "slot_0": "0x48009690",
                "slot_2": "0x48008118",
                "slot_3": "0x48008a90",
                "slot_4": "0x48008560",
                "slot_5": "0x4800893c",
                "slot_8": "0x48009744",
            },
            "slot_1_indirect_branch_pc": "0x48008640",
            "system_suspend_entry_transition_pc": "0x4800975c",
            "system_suspend_entry_transition_target": "0x48008d98",
            "psci_system_suspend_string_present": True,
            "psci_plat_pm_ops_getter_string_present": True,
        },
        "disassembly_file": "PSCI-CALLBACK-DISASSEMBLY.txt",
        "power_management_path_candidates": {
            "slot_3_function": {
                "entry": "0x%08x" % table[3],
                "semantic_member": OPS_MEMBER_NAMES[3],
                "save_context_callsite": "0x48002998",
                "save_context_target": "0x48001790",
                "ccu_candidates": ["0x03001000", "0x03001a78", "0x03001a8c"],
                "save_call_within_slot3_function": table[3] <= 0x48002998 < 0x480029b8,
            },
            "slot_5_function": {
                "entry": "0x%08x" % table[5],
                "semantic_member": OPS_MEMBER_NAMES[5],
                "restore_context_callsite": "0x48002858",
                "restore_context_target": "0x480018d8",
                "tail_branch_callsite": "0x48002874",
                "tail_branch_target": "0x%08x" % decode_b_target(0x48002874, read_word(raw, 0x48002874)),
                "next_member": OPS_MEMBER_NAMES[4],
                "restore_call_within_slot5_function": table[5] <= 0x48002858 < 0x48002878,
            },
            "slot8_system_suspend_getter": {
                "entry": "0x%08x" % table[8],
                "member": OPS_MEMBER_NAMES[8],
                "return_instruction_pc": "0x48002544",
                "return_value": "0x02010000",
                "caller_function": "0x480096f8",
                "global_ops_pointer_load_pc": "0x48009718",
                "member_pointer_load_pc": "0x4800971c",
                "indirect_call_pc": "0x48009744",
                "next_suspend_dispatch_pc": "0x4800975c",
                "next_suspend_dispatch_target": "0x48008d98",
            },
            "system_suspend_transition": {
                "entry": "0x48008d98",
                "start_affinity_level_register": "w3",
                "end_affinity_level_register": "w4",
                "system_suspend_start_affinity_level": 0,
                "system_suspend_end_affinity_level": 2,
                "callback_table_address": "0x48017d70",
                "callback_table": [
                    {"affinity_level": 0, "wrapper": "0x48008b84",
                     "platform_member": "affinst_suspend", "platform_slot": 3,
                     "platform_dispatch_call_pc": "0x48008c54"},
                    {"affinity_level": 1, "wrapper": "0x48008ab8",
                     "platform_member": "affinst_suspend", "platform_slot": 3,
                     "platform_dispatch_call_pc": "0x48008b40"},
                    {"affinity_level": 2, "wrapper": "0x48008a00",
                     "platform_member": "affinst_suspend", "platform_slot": 3,
                     "platform_dispatch_call_pc": "0x48008a90"},
                ],
                "loop": "ascending from start level to end level; skip null state records; indirect-call callback_table[current_level]; stop on nonzero callback result",
                "special_return": -4,
                "special_return_handler_call_pc": "0x48009774",
                "special_return_handler_target": "0x48001a04",
                "power_handler_disassembly": "included as system_suspend_power_entry",
                "final_helper_call_pc": "0x48001b5c",
                "final_helper_target": "0x48016128",
                "final_indirect_branch_pc": "0x48001b70",
                "final_indirect_branch_register": "x0",
                "final_indirect_branch_target": "0x00000000000201b8",
                "final_target_region": "within the sun50iw9p1 SRAM A1 candidate region [0x20000,0x28000); live contents and code identity remain unverified",
                "el3_control_before_branch": {
                    "sctlr_el3_read_pc": "0x4801612c",
                    "sctlr_el3_clear_mask": "0x5",
                    "sctlr_el3_write_pc": "0x48016134",
                    "isb_pc": "0x48016138",
                    "mmu_enable_bit_cleared": True,
                    "data_cache_enable_bit_cleared": True,
                    "cache_maintenance_entry": "0x480160a8",
                },
                "cache_maintenance": {
                    "clidr_read_pc": "0x480160a8",
                    "cache_level_loop_pc": "0x48015fb8",
                    "csselr_write_pc": "0x48015fe4",
                    "ccsidr_read_pc": "0x48015fec",
                    "dc_isw_pc": "0x48016040",
                    "dc_cisw_pc": "0x48016060",
                    "dc_csw_pc": "0x48016080",
                    "interpretation": "EL3 cache-level and set/way maintenance code; exact runtime cache-level/operation path is not dynamically observed",
                },
                "ordinary_powerdown_path": {
                    "helper": "0x48015d70",
                    "dsb_pc": "0x48015d70",
                    "wfi_pc": "0x48015d74",
                    "wfi_loop_branch_pc": "0x48015d78",
                    "sequence": "DSB SY; WFI; branch back to WFI",
                },
                "copied_sram_standby_image": {
                    "copy_regions": [
                        {"source": "[0x4800c220, 0x48015630)", "destination": "[0x20000, 0x29410)", "length": "0x9410", "condition": "always", "kind": "standby code/data image"},
                        {"source": "[0x4803cb90, 0x4803cbc0)", "destination": "[0x20080, 0x200b0)", "length": "0x30", "condition": "always", "kind": "auxiliary runtime context; source lies outside the monitor file payload"},
                        {"source": "[0x48000030, 0x480000b0)", "destination": "[0x20000, 0x20080)", "length": "0x80", "condition": "always", "kind": "BL31 header/boot-context prefix"},
                        {"source": "[0x4801b380, 0x4801b400)", "destination": "[0x20134, 0x201b4)", "length": "0x80", "condition": "conditional on saved power-mode/global state"},
                        {"source": "[0x4801b308, 0x4801b368)", "destination": "[0x200d4, 0x20134)", "length": "0x60", "condition": "conditional on saved power-mode/global state"}
                    ],
                    "memcpy_target": "0x4800aa80",
                    "memcpy_call_pcs": ["0x48001a2c", "0x48001a48", "0x48001aa8", "0x48001b10", "0x48001b30"],
                    "runtime_entry": "0x000201b8",
                    "source_entry": "0x4800c3d8",
                    "entry_source_offset": "0x1b8",
                    "scratch_source_range": "[0x48015410, 0x48015569)",
                    "scratch_runtime_range": "[0x291f0, 0x29349)",
                    "stack_setup_source": "0x480140a0",
                    "mapped_stack_setup": "0x00027e80",
                    "stack_top_literal": "0x00057ff0",
                    "saved_wakeup_return_address": "0x48015d14",
                    "saved_wakeup_return_context_offset": "0xc4",
                    "rtc_standby_flag_register": "0x070001f8",
                    "rtc_standby_flag_value": "0x00010000",
                    "rtc_soft_entry_register": "0x070001fc",
                    "rtc_soft_entry_value_source": "context + 0xc4",
                    "wfi_in_source_blob": ["0x4800c50c", "0x4800c550"],
                    "clock_save_restore": {
                        "clock_context_source": "0x48015414",
                        "saved_ccu_registers": [
                            {"address": "0x03001500", "reference_name": "CPUX_AXI_CFG_REG", "saved_mask": "0x00000303"},
                            {"address": "0x03001510", "reference_name": "PSI_AHB1_AHB2_CFG", "saved_mask": "0x0003031f"},
                            {"address": "0x0300151c", "reference_name": "AHB3_CFG", "saved_mask": "0x0003031f"},
                            {"address": "0x03001520", "reference_name": "APB1_CFG", "saved_mask": "0x00000303"},
                            {"address": "0x03001524", "reference_name": "APB2_CFG", "saved_mask": "0x00000303"}
                        ],
                        "pll_cpux": {
                            "control_register": "0x03001000",
                            "saved_factor_mask": "0x0003ff03",
                            "standby_clear_enable_bit31": True,
                            "standby_clear_lock_enable_bit29": True,
                            "resume_restore_factors": True,
                            "resume_set_enable_bit31": True,
                            "resume_set_lock_enable_bit29": True,
                            "resume_poll_lock_bit28": True
                        },
                        "r_prcm_pll_ldo_key": {
                            "address": "0x07010244",
                            "block": "R_PRCM base 0x07010000 + offset 0x244",
                            "standby_high_byte_value": "0xa7",
                            "resume_clears_high_byte": True,
                            "reference_identification": "HookedBehemoth/SenkOS sun50iw9/H616 suspend notes identify R_PRCM+0x244 as PLL LDO key 0xa7",
                            "provenance_limit": "same-family reference only; exact OEM BL31 source revision and live register response are not available"
                        },
                        "context_gpio_configuration": {
                            "pin_config_offsets": ["0xd4", "0xec"],
                            "helper": "0x48013620",
                            "calls": ["0x48011d2c", "0x48011d6c"],
                            "note": "context-provided pin configurations are statically passed to a pad configuration helper; no pad writes were performed"
                        },
                        "conditional_mode2_path": {
                            "condition": "saved mode word == 2",
                            "bus_registers": ["0x03001510", "0x07010000", "0x07010010"],
                            "bus_actions": "set bits 25:24 to 10 in all three registers",
                            "rtc_xo_control": {
                                "address": "0x07000160",
                                "block": "sun50iw9 RTC CCU",
                                "register_name": "XO_CTRL_REG",
                                "standby_actions": "set bit31; clear bit1",
                                "reference_identification": "radxa/allwinner-bsp sun50iw9 RTC CCU header names offset 0x160 XO_CTRL_REG; SenkOS suspend notes document bit31 set and bit1 clear",
                                "provenance_limit": "same-family source cross-reference, not a live RG34XX readback"
                            },
                            "interpretation": "mode-2 standby branch selects low-power bus mux fields and changes RTC XO control"
                        },
                        "resume_lock_poll_pc": "0x48011e7c",
                        "resume_post_lock_settle_call_pc": "0x48011ec0"
                    },
                    "standby_integrity_checksum": {
                        "helper": "0x48011b78",
                        "algorithm": "32-bit additive sum over 32-bit words; pointer increments by 4",
                        "range_descriptor": "context +0xb0 = start pointer; context +0xb4 = byte length",
                        "enable_flag": "context +0xc0",
                        "initial_descriptor": {
                            "payload_address": "0x4801a000",
                            "base_address": "0x40000000",
                            "length_bytes": 256,
                            "end_exclusive": "0x40000100",
                            "enabled": True,
                            "adjacent_string": "chipid",
                            "base_load_pc": "0x48001a68",
                            "base_context_store_pc": "0x48001a6c",
                            "length_load_pc": "0x48001a74",
                            "length_context_store_pc": "0x48001a78",
                            "enable_load_pc": "0x48001a5c",
                            "enable_context_store_pc": "0x48001a60",
                            "word_count": 64,
                            "payload_values_are_initial_literals": True,
                            "runtime_mutation_before_use": "not established",
                        },
                        "checked_range_bytes": 256,
                        "checked_range_word_count": 64,
                        "runtime_descriptor_writer": {
                            "service_name": "arm_svc",
                            "service_name_address": "0x48016ad9",
                            "handler_record_address": "0x48018ca8",
                            "handler_address": "0x480035d0",
                            "writer_address": "0x480022fc",
                            "writer_call_pc": "0x48003b94",
                            "smc_function_ids": ["0x80000064", "0xc0000064"],
                            "smc32_dispatch_compare_branch_pc": "0x4800372c",
                            "smc64_dispatch_compare_branch_pc": "0x48003a04",
                            "writer_argument_mapping": {
                                "smc_arg1": "enable byte at descriptor +0x8",
                                "smc_arg2": "checksum start address at descriptor +0x0",
                                "smc_arg3": "checksum length in bytes at descriptor +0x4",
                            },
                            "runtime_invocation_on_target": "not established; no EL3 SMC trace collected",
                        },
                        "initial_call_pc": "0x4800c448",
                        "saved_sum_field": "context +0xb8",
                        "resume_call_pc": "0x4800c6c4",
                        "resumed_sum_field": "context +0xbc",
                        "comparison_pc": "0x4800c6d8",
                        "success_branch_pc": "0x4800c6dc",
                        "failure_code": "0x88000001",
                        "failure_write_pc": "0x4800c6e8",
                        "failure_write_register": "RTC/system scratch at address held in x20 (base 0x07000108 in this path)",
                        "failure_path": "unconditional branch to self at 0x4800c6ec; fail-stop",
                        "range_identity_limit": "the matched payload's initial descriptor is [0x40000000,0x40000100) (256 bytes, 64 words), adjacent to the string 'chipid'. The arm_svc handler exposes writer FIDs 0x80000064 and 0xc0000064 with arguments (enable, start, length), so this descriptor can be changed before SYSTEM_SUSPEND copies it into context. Actual call occurrence, final values and live DRAM contents are unmeasured. This is not the full 0x9410-byte SRAM standby image.",
                    },
                    "dram_retention_snapshot_copy": {
                        "rtc_pointer_register": "0x07000104 (RTC GP_DATA1; same-family RTC GP_DATA register window begins at +0x100)",
                        "rtc_pointer_store_pc": "0x48001abc",
                        "buffer_address": "0x4801b400",
                        "buffer_interval": "[0x4801b400,0x4801b680)",
                        "buffer_bytes": 640,
                        "buffer_words": 160,
                        "matched_monitor_payload_end": "0x%08x" % (BASE + len(raw)),
                        "buffer_outside_matched_monitor_file_payload": True,
                        "standby_snapshot": {
                            "call_chain": [
                                "0x4800c490 -> 0x48011b40",
                                "0x48011b50 -> 0x48011920",
                                "0x48011928 -> 0x48011710"
                            ],
                            "worker": "0x48011710",
                            "selector_context_offset": "0x1c",
                            "skip_when": "context +0x1c bit0 is set",
                            "fixed_dram_segment": "[0x40000000,0x40000140)",
                            "fixed_segment_saved_to": "[0x4801b400,0x4801b540)",
                            "dynamic_bank_base": "0x40000000 + (((context[0x1c] >> 17) << 20))",
                            "selector_derivation": {
                                "helper": "0x4800c730",
                                "call_pc": "0x48011990",
                                "precomputed_override": "context +0x1c bit31 set bypasses helper",
                                "inputs": {
                                    "context_word_0x18": {
                                        "bits_4_11": "add to exponent",
                                        "bits_12_13": "add to exponent",
                                        "bits_14_15": "add to exponent",
                                        "bits_0_3": "add to exponent"
                                    },
                                    "context_word_0x1c": {
                                        "bits_12_15": "add to exponent",
                                        "bits_0_3_zero": "select exponent bias 0x12; nonzero selects 0x13"
                                    },
                                    "context_halfword_0x7a": "low three bits gate a 3/4 scaling adjustment unless context+0x1c bits31:30 encode 2",
                                    "output": "DRAM geometry/capacity selector; caller stores it in context+0x1c bits16..30"
                                },
                                "controller_exception": {
                                    "register": "0x03006200",
                                    "low16_values": ["0x5c00", "0x5d00"],
                                    "size_threshold": "0x800",
                                    "behavior": "for these two controller IDs, a selector result greater than 0x800 skips rewriting context+0x1c"
                                }
                            },
                            "dynamic_segment_bytes": 320,
                            "dynamic_segment_saved_to": "[0x4801b540,0x4801b680)",
                            "copy_direction": "DRAM -> RTC-GP_DATA1-addressed scratch buffer",
                            "normal_path_words": 160,
                            "normal_path_total_bytes": 640,
                            "alternate_layout_condition": "context +0x4 == 4",
                            "alternate_layout_branch_targets": ["0x480118d0", "0x480118f8"],
                            "alternate_layout": "strided addressing path; not normalized as a contiguous memcpy"
                        },
                        "resume_restore": {
                            "call_chain": [
                                "0x4800c658 -> 0x48011b58",
                                "0x48011b60 -> 0x48011b10",
                                "0x48011b20 -> 0x48011938"
                            ],
                            "worker": "0x48011938",
                            "rtc_pointer_load": "0x07000104",
                            "restore_gate": {
                                "helper": "0x48011320",
                                "call_pc": "0x48011974",
                                "branch_pc": "0x4801197c",
                                "skip_copy_when": "helper returns zero; CBZ branches to 0x48011a70",
                                "helper_calls": [
                                    {"pc":"0x48011330","target":"0x480111a0"},
                                    {"pc":"0x48011338","target":"0x480115c0"}
                                ],
                                "return_value": "the second helper's W0 result for context+0x4 != 4; otherwise the first 32-bit word at DRAM 0x40000000",
                                "alternate_layout_probe": "context+0x4 == 4 causes helper to load [0x40000000] and use that value as the copy-enable predicate",
                                "runtime_gate_result": "not observed",
                                "status_probe": {
                                    "helper": "0x48011360",
                                    "context_flags_word_offset": "0x6c",
                                    "tested_flag_bits": [19,20,21,22,23],
                                    "individual_probe_helpers": [
                                        {"target":"0x4800e2e0","max_attempts":5},
                                        {"target":"0x4800e500","max_attempts":5},
                                        {"target":"0x4800eba0","max_attempts":5},
                                        {"target":"0x4800e720","max_attempts":5}
                                    ],
                                    "common_setup_calls": [
                                        {"pc":"0x48011370","target":"0x4800dfe8"},
                                        {"pc":"0x48011378","target":"0x4800c9e8"}
                                    ],
                                    "success_epilogue_helper": "0x4800d500",
                                    "success_epilogue_call_pc": "0x48011480",
                                    "failure_return": "W0=0 at 0x48011520",
                                    "outer_retry": {
                                        "counter_address":"0x48015410",
                                        "maximum_attempts":10,
                                        "reenter_pc":"0x480114f4",
                                        "reenter_target":"0x48011320",
                                        "failure_when_limit_reached":"0x48011520"
                                    },
                                    "flag_meaning": "context+0x6c bits control which status probes run; exact bit-to-DRAM-subsystem mapping is not yet identified",
                                    "runtime_gate_result": "not observed"
                                },
                                "subordinate_calls": [
                                    {"pc":"0x48011640","target":"0x48010f98"},
                                    {"pc":"0x4801166c","target":"0x48011360"}
                                ],
                                "preceding_dram_clock_reinitialization": {
                                    "helper":"0x480111a0",
                                    "call_pc":"0x48011330",
                                    "mmio_registers": {
                                        "0x03001540":"CCU MBUS_CFG (+0x540); helper clears clock-gating/reset bits31/30",
                                        "0x0300180c":"CCU DRAM_BGR (+0x80c); helper clears DRAM_GATING bit0 and DRAM_RST bit16 before later re-enabling",
                                        "0x03001010":"CCU PLL_DDR0_CTRL (+0x10); helper disables PLL, composes factors from context[0], re-enables and polls LOCK bit28",
                                        "0x03001800":"CCU DRAM_CLK (+0x800); helper asserts module reset and reconfigures the DRAM clock"
                                    },
                                    "family_reference": "H616 CCU register map only; H700/sun50iw9 active bit semantics require target-specific confirmation"
                                },
                                "restore_state_staging_helper": {
                                    "helper":"0x480115c0",
                                    "calls":["0x48010f98","0x48011360"],
                                    "mmio_addresses":[
                                        "0x047fa008","0x047fa014","0x047fa020","0x047fa024","0x047fa028",
                                        "0x047fa050","0x047fa500","0x047fb000","0x047fb00c","0x047fb030",
                                        "0x047fb038","0x047fb060","0x047fb180","0x047fb1a0","0x047fb1b0",
                                        "0x047fb1c0","0x047fb250","0x047fb320","0x047fb324",
                                        "0x047fd180","0x047fe180","0x047ff180"
                                    ],
                                    "address_construction": "MOVK W/X,#0x47f,LSL #16 builds base 0x047f0000; low 16-bit MOV immediates form these register addresses",
                                    "same_family_h616_memory_map": {
                                        "msi_ctrl_window":"[0x047fa000,0x047fb000)",
                                        "dram_ctrl_window":"[0x047fb000,0x04800000)",
                                        "phy_ctrl_window":"[0x04800000,0x05000000)",
                                        "source":"Allwinner H616 User Manual v1.0, system memory map page 31; family cross-reference only"
                                    },
                                    "target_limit": "H700/sun50iw9 active register aperture and per-offset semantics are not verified by a target register readback; do not classify these as DRAM RAM buffers"
                                }
                            },
                            "copy_direction": "scratch buffer -> DRAM",
                            "fixed_dram_segment": "[0x40000000,0x40000140)",
                            "dynamic_bank_base": "0x40000000 + (((context[0x1c] >> 17) << 20))",
                            "dynamic_segment_bytes": 320,
                            "normal_path_total_bytes": 640,
                            "alternate_layout_condition": "context +0x4 == 4",
                            "alternate_layout_branch_targets": ["0x48011a80", "0x48011aa8"]
                        },
                        "relationship_to_default_checksum": "default checksum interval [0x40000000,0x40000100) lies within the first 0x140-byte snapshot segment; this overlap does not establish firmware design intent or the final SMC-overridden checksum range",
                        "runtime_limits": "RTC GP_DATA1 live value, dynamic selector, scratch-buffer ownership/content and actual pre/post-suspend execution are not measured; no SMC or MMIO was issued"
                    },
                    "wake_source_helpers": {
                        "aggregate_function": "0x48013ee8",
                        "gpio_configuration_function": "0x48013df0",
                        "gic_pending_query": {
                            "function": "0x48013190",
                            "register_base": "0x03021200",
                            "register": "GICD_ISPENDR",
                            "operation": "word_index = irq >> 5; bit_index = irq & 31; read pending bit from ISPENDR[word_index]"
                        },
                        "ir_fifo_drain": {
                            "function": "0x480131b8",
                            "status_register": "0x07040030",
                            "fifo_register": "0x07040020",
                            "status_ack": "writes low status byte back to status register",
                            "fifo_limit": "0xff entries",
                            "queue_context": "0x48016438",
                            "meaning": "drains IR receive FIFO bytes into standby context queue"
                        },
                        "gpio_pin_name_decoder": {
                            "function": "0x48013588",
                            "format": "ASCII bank letter followed by one or two decimal digits; return bank<<16 | pin; empty name -> -1"
                        },
                        "gpio_read": {
                            "function": "0x480135d8",
                            "data_register_offset": "0x10 + bank * 9 * 4",
                            "normal_pio_base": "0x0300b000 for bank < 10",
                            "r_pio_base": "0x07022000 for bank >= 10",
                            "operation": "read DAT and test requested pin bit"
                        },
                        "gpio_pad_config": {
                            "function": "0x48013620",
                            "register_families": ["PIO CFG", "PIO DAT", "PIO drive", "PIO pull"],
                            "operation": "decode bank/pin and apply context-supplied mux, pull, drive and data fields",
                            "meaning": "static helper contains writes; no target call was executed"
                        },
                        "wake_poll_edges": [
                            {"pc": "0x48013f14", "target": "0x480131b8", "kind": "IR FIFO capture"},
                            {"pc": "0x48013f48", "target": "0x48013588", "kind": "parse GPIO name"},
                            {"pc": "0x48013f54", "target": "0x480135d8", "kind": "sample GPIO DAT"},
                            {"pc": "0x48013fe4", "target": "0x48013190", "kind": "query GIC pending bit"}
                        ],
                        "runtime_limit": "static matching-binary paths only; no IR FIFO, GIC pending, GPIO data or pinmux register was probed on device"
                    },
                    "cpux_axi_clock_source_transition": {
                        "register_candidate": "0x03001500",
                        "register_name_reference": "CPUX_AXI_CFG_REG (H616 reference manual / Linux CCU description)",
                        "standby_callsite": "0x4800c47c",
                        "resume_callsite": "0x4800c69c",
                        "helper_target": "0x48011ff8",
                        "busy_wait": {
                            "wrapper": "0x48011ff8",
                            "wrapper_shift": "lsl w0, w0, #2",
                            "loop_target": "0x48014078",
                            "loop_body": "subs x0, x0, #1; b.hi 0x48014078; ret",
                            "standby_argument": 10,
                            "resume_argument": 10,
                            "effective_iterations_each_call": 40,
                            "time_unit": "unknown; no clock-frequency/calibration conversion asserted",
                        },
                        "standby_read_pc": "0x4800c470",
                        "standby_clear_pc": "0x4800c474",
                        "standby_write_pc": "0x4800c478",
                        "standby_mask": "0xfcffffff",
                        "resume_read_pc": "0x4800c690",
                        "resume_set_pc": "0x4800c694",
                        "resume_write_pc": "0x4800c698",
                        "resume_mask": "0x03000000",
                        "field_reference": "H616 manual lists CLK_SRC_SEL bits 26:24",
                        "bit26": "preserved by both standby and resume masks",
                        "interpretation_limit": "bits 25:24 are cleared on standby and set on resume; exact mux value/source cannot be inferred without live bit26 and register readback; this does not prove CPU DVFS",
                    },
                    "wakeup_return": {
                        "saved_context_offset": "0xc4",
                        "saved_return_pc": "0x48015d14",
                        "rtc_soft_entry_register": "0x070001fc",
                        "rtc_store_pc": "0x4800c5b8",
                        "restore_load_pc": "0x4800c5a8",
                        "resume_blr_pc": "0x4800c6a8",
                        "resume_blr_register": "x22",
                        "continuation": "restore SCTLR_EL3, VBAR_EL3, DAIF and stack selection, then branch back into BL31 PSCI continuation",
                    },
                    "sram_map_note": "main image spans SRAM A1 and the start of SRAM C; the mapped stack top is in SRAM C",
                    "runtime_caveat": "copy destinations and relocated entry are statically reconstructed; live SRAM_C ownership, conditional context source values, RTC behavior and successful suspend/wakeup are not measured"
                },
            },
            "interpretation": "eight legacy affinst/system power callbacks followed by a vendor get_sys_suspend_power_state extension; dispatch offsets and the exact embedded getter string identify slot 8",
        },
        "callback": {
            "address": "0x4800254c",
            "call_pc": "0x4800255c",
            "low_level_target": "0x48002d78",
            "condition": "W3 == 0 reaches low-level CPU_ON sequence; W3 != 0 skips it",
        },
        "low_level_cpu_on": {
            "address": "0x48002d78",
            "argument_use": "W0 target core index; X1 non-secure entry address",
            "rvbar_helper": "0x48002d30",
            "soc_selector": "0x48002d18",
            "rvbar_selection": {
                "selector_register": "0x03000024 (SYS_CFG VER_REG; H616 family reference)",
                "selector_mask": "0x7",
                "bl31_rule": "low3 == 0 -> 0x09010040 + core*8; low3 != 0 -> 0x08100040 + core*8",
                "native_spl_rule": "low3 == 2 -> 0x08100040; otherwise -> 0x09010040",
                "matching_low3_values": matching_selector_values,
                "mismatching_low3_values": mismatching_selector_values,
                "device_low3_value": "UNKNOWN; requires read-only target sampling",
                "native_spl_source_commit": "cbe253a43f46cf551cc9a6c79a84015bde3686cf",
            },
            "registers": {
                "C0_RST_CTRL": "0x09010000",
                "C0_CTRL_REG0": "0x09010010",
                "AON_POWERON_RESET": "0x07000440",
                "AON_CLAMP": "0x07000444",
                "DBG_REG0": "0x090100c0",
            },
            "sequence": [
                "program per-core RVBAR low and clear high word",
                "assert target core reset",
                "assert target power-on reset",
                "set target AArch64 mode",
                "apply target power",
                "release target output clamp",
                "deassert target power-on reset",
                "deassert target core reset",
                "assert target DBGPWRUPD",
            ],
        },
        "embedded_strings": [value.decode() for value in STRINGS],
        "interpretation": [
            "The matched BL31 registers a nine-entry platform-ops table at 0x480165f8 through 0x48009dec -> 0x480029bc.",
            "The first eight entries match the legacy Allwinner TF-A plat_pm_ops_t order: affinst_standby, affinst_on, affinst_off, affinst_suspend, affinst_on_finish, affinst_suspend_finish, system_off and system_reset.",
            "The ninth entry is the vendor get_sys_suspend_power_state extension: the exact member string is present, SYSTEM_SUSPEND loads table byte offset +0x40 and indirectly calls the function at 0x48002544.",
            "The getter returns encoded power state 0x02010000 and its caller passes that value to transition function 0x48008d98.",
            "Slots 0-5 are independently selected by matching table byte offsets and indirect call sites; slot 7 references Sunxi System Reset messages and calls reset helper 0x48001ec8, while slot 6 calls power-off helper 0x48001ed8.",
            "Slot 1 affinst_on reaches CPU_ON helper 0x48002d78, but initial BL31-to-BL33 core release is not established.",
            "Slot 3 affinst_suspend contains the conditional CCU/AON save helper; slot 5 affinst_suspend_finish contains the conditional restore helper and tail-branches to slot 4 affinst_on_finish.",
            "These are static binary ABI/control-flow findings. They do not prove suspend/resume, CPU_ON, system-off or system-reset execution on the tested device.",
            "CCU save/restore helpers do not establish a BL33 startup PLL transition or an exposed CPU DVFS API.",
            "The exact vendor TF-A source commit is not recovered; effective callback semantics are identified from table order, dispatcher offsets, embedded strings and function behavior.",
            "The live VER_REG selector value, secondary A53 reset/hold state and BL31-to-BL33 memory ownership remain runtime questions.",
            "No hardware MMIO, flash, boot-media write or payload execution was performed.",
        ],
    }

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n",
                        encoding="utf-8")
    disassembly_path = args.out.parent / "PSCI-CALLBACK-DISASSEMBLY.txt"
    disassembly_path.write_text(render_disassembly(raw), encoding="utf-8")
    markdown = [
        "# Matched BL31 legacy nine-member PSCI platform-ops audit", "",
        "- Monitor: 107,216 bytes; SHA-256 `%s`." % digest,
        "- Platform-ops table `0x480165f8` has nine 64-bit entries: affinst_standby, affinst_on, affinst_off, affinst_suspend, affinst_on_finish, affinst_suspend_finish, system_off, system_reset, get_sys_suspend_power_state.",
        "- Slots 0–5 are selected by dispatcher loads at table offsets +0x00, +0x08, +0x10, +0x18, +0x20 and +0x28; their load/call sites are pinned in the JSON report.",
        "- Slot 1 affinst_on reaches CPU_ON helper `0x48002d78`; slot 3 affinst_suspend saves CCU state through `0x48001790`; slot 5 affinst_suspend_finish restores via `0x480018d8` and tail-branches to slot 4 affinst_on_finish.",
        "- Slot 7 system_reset references `Sunxi System Reset` messages and calls helper `0x48001ec8`; slot 6 system_off calls helper `0x48001ed8`.",
        "- SYSTEM_SUSPEND loads slot 8 at byte offset +0x40 (PC `0x4800971c`), calls it indirectly (PC `0x48009744`), receives `0x02010000`, then enters transition `0x48008d98`.",
        "- Transition `0x48008d98` iterates affinity levels from start to end through callback map `0x48017d70`; the three wrappers `0x48008b84/0x48008ab8/0x48008a00` dispatch platform slot 3 `affinst_suspend`.",
        "- For system state `0x02010000`, slot 3's CCU-save branch returns `-4`; caller checks that sentinel and invokes power-entry target `0x48001a04` at `0x48009774`.",
        "- Ordinary suspend uses `0x48015d70`: `DSB SY` → `WFI` → branch back to WFI.",
        "- The special handler `0x48001a04` copies the 0x9410-byte standby image from `[0x4800c220,0x48015630)` to SRAM `[0x20000,0x29410)`, plus auxiliary/context blocks at SRAM offsets `0x80`, `0xd4`, and `0x134` (the latter two are conditional). Its optional integrity checksum is separate: the static default descriptor is a 256-byte range at `[0x40000000,0x40000100)`, adjacent to the string `chipid`; arm_svc SMC FIDs `0x80000064` / `0xc0000064` can override its enable/start/length before use.",
        "- The copied image entry at source `0x4800c3d8` maps exactly to branch target `0x201b8`; copied helper `0x480140a0` maps to `0x27e80` and loads SP `0x57ff0`.",
        "- The relocated entry zeros its in-image scratch range `[0x291f0,0x29349)`, records wake-return address `0x48015d14` at context offset `0xc4`, and the copied RTC path writes flag `0x10000` to `0x070001f8` plus the saved return address to `0x070001fc`.",
        "- The main copy spans the SRAM A1 range and extends into SRAM C; the SP is also in SRAM C. This is a static copy/map reconstruction, not a live SRAM ownership or wakeup measurement.",
        "- The copied code changes `0x03001500` around standby/resume: it clears bits 25:24 with `0xfcffffff` before standby and sets `0x03000000` on resume, calling helper `0x48011ff8` on both paths. The H616 reference map describes offset `0x500` as CPUX_AXI_CFG_REG and CLK_SRC_SEL bits 26:24; bit 26 is preserved here, so the actual mux source remains unknown without live readback.",
        "- Wait helper `0x48011ff8` shifts its argument left by two and branches to `0x48014078`, whose body is `subs x0, x0, #1; b.hi` back to itself; standby and resume pass 10, yielding 40 decrement iterations each. This is a count-based busy wait, not a timer-backed delay; duration is unknown.",
        "- The clock-save path snapshots masked fields from `0x03001500/1510/151c/1520/1524` into PSCI context; H616 CCU references name these CPUX_AXI, PSI_AHB1_AHB2, AHB3, APB1 and APB2 configuration registers. It also saves PLL_CPUX factor bits with mask `0x0003ff03`, clears PLL enable bit31 and lock-enable bit29, and stages high byte `0xa7` at `0x07010244` (R_PRCM+0x244). Same-family H616/sun50iw9 suspend notes identify this as the PLL LDO key; this is a cross-reference, not live readback or exact OEM-source proof.",
        "- In the saved mode==2 branch the matched binary sets bits25:24 to `10b` in `0x03001510`, `0x07010000` and `0x07010010`, and sets bit31/clears bit1 at `0x07000160`. The sun50iw9 RTC CCU header names offset +0x160 `XO_CTRL_REG`; same-family suspend notes describe this bit31/bit1 operation. This remains family-reference identification, not device register verification.",
        "- Resume restores PLL_CPUX factors, sets enable bit31 and lock-enable bit29, polls lock bit28, then restores saved CCU fields. A mode==2 branch also touches `0x07010000`, `0x07010010` and `0x07000160`; the latter's exact standby role remains unknown.",
        "- Wake-source aggregation statically calls IR receive FIFO/status drain `0x480131b8`, GPIO name/read helpers `0x48013588/0x480135d8`, GPIO pad-config helper `0x48013620`, and GIC pending helper `0x48013190`. The latter reads GICD_ISPENDR at `0x03021200`, not an enable register. IR offsets `0x07040020/0x07040030` are in the H616 IR block; GPIO read/config use PIO `0x0300b000` or R_PIO `0x07022000`.",
        "- These are matched-binary wake-source preparation/polling paths; no pending interrupt, IR FIFO, GPIO level or pin configuration was probed or written on hardware.",
        "- Integrity helper `0x48011b78` calculates a wrapping 32-bit additive sum over caller-selected 32-bit words (pointer increments by 4). The static default descriptor at `0x4801a000` is start `0x40000000`, length `0x100`, enable `1`; the adjacent embedded string is `chipid`. An arm_svc-named handler at `0x480035d0` dispatches both SMC FIDs `0x80000064` and `0xc0000064` to writer `0x480022fc`, which stores SMC arguments 1/2/3 as enable/start/length. The special power-entry `0x48001a04` reads the descriptor into context before checksum. Consequently the 256-byte interval is only the static default; the final range can be overridden through SMC. No runtime SMC invocation or final target value has been captured. This checksum remains separate from the full `0x9410`-byte SRAM standby image.",
        "- The RTC soft-entry stores saved return PC `0x48015d14`; resume code loads it from context `+0xc4` and indirect-branches through x22 at `0x4800c6a8`. The return continuation restores SCTLR_EL3, VBAR_EL3, DAIF and stack selection before returning into BL31.",
        "- The effective ABI is the legacy Allwinner `plat_pm_ops_t` order with a vendor getter extension; the exact source commit is not identified.",

        "- Slot 1 affinst_on calls CPU_ON helper `0x48002d78` at `0x4800255c` when `W3 == 0`.",
        "- The exact embedded strings `psci_plat_pm_ops->get_sys_suspend_power_state` and `psci_system_suspend` are present in the matched monitor.",
        "- Raw slot 3 entry 0x48002878 contains the conditional CCU save call at 0x48002998 -> 0x48001790. Separate raw slot 5 entry 0x480026c4 contains the conditional CCU restore call at 0x48002858 -> 0x480018d8; its epilogue at 0x48002874 branches to raw slot 4 entry 0x48002680.",
        "- Full disassembly windows for all table entries and both CCU helpers are saved in PSCI-CALLBACK-DISASSEMBLY.txt for function-boundary/CFG review.",
        "- Slot 3 and slot 5 are identified as affinst_suspend and affinst_suspend_finish by the legacy dispatcher offsets; their branches are conditional and runtime execution/cold-boot reachability remain unproven. No BL33 startup clock transition or CPU DVFS support is established.",
        "- Low-level routine writes a per-core RVBAR and contains a CPU power/reset sequence; one SoC-selected variant uses `0x09010000`, `0x09010010`, `0x07000440`, `0x07000444` and `0x090100c0`.",
        "- RVBAR selector cross-check: BL31 uses VER_REG low3==0 for base 0x09010040, otherwise 0x08100040; pinned native SPL uses low3==2 for the alternate bank. Both agree only for selector values 0 and 2; the target value must be read.",
        "",
        "- Power-entry stores the scratch-buffer pointer 0x4801b400 in RTC GP_DATA1 0x07000104. Before SRAM entry, the matched BL31 copies 160 words (0x280 bytes) from two 0x140-byte address windows into this buffer; after resume, a companion helper copies them back. The fixed address window begins at 0x40000000; the second bank is selected from context +0x1c. context +0x4 == 4 selects alternate strided-copy branches. These are address-level DRAM candidates, not approved memory ownership; the buffer is outside the matched monitor file payload and its live contents/owner are unverified.",
        "- The second DRAM window is not selected from a literal bank ID: unless context +0x1c bit31 is preset, caller 0x48011938 invokes geometry helper 0x4800c730 at 0x48011990. The helper derives a power-of-two-like selector from context DRAM-geometry fields (context words +0x18/+0x1c and halfword +0x7a), which the caller stores in context +0x1c bits16..30. Controller IDs 0x5c00/0x5d00 read from 0x03006200 add a 0x800 threshold exception. Resume is gated by 0x48011320: zero skips copy; it calls 0x480111a0 and 0x480115c0. The first disables/reconfigures/re-enables the DRAM PLL, DRAM clock, DRAM bus gate/reset and MBUS using context[0]; the second stages state through addresses formed from 0x047f0000 plus low offsets (for example 0x047fa008, 0x047fb320, 0x047fd180) and calls 0x48010f98/0x48011360. This is peripheral MMIO, not DRAM RAM: the H616 same-family memory map places 0x047fa000 in MSI_CTRL and 0x047fb000–0x047fffff in DRAM_CTRL; the mapping is a family cross-reference, not H700 target readback. For context +0x4 == 4, the gate instead reads [0x40000000] as the copy-enable predicate; the copy workers use alternate strided paths. Both gate outcomes and selector inputs remain runtime-unknown.",
        "## Interpretation", "",
        "- The effective callback ABI is the legacy Allwinner `plat_pm_ops_t` order plus the vendor `get_sys_suspend_power_state` extension; do not apply modern `plat_psci_ops_t` slot offsets.",
        "- Callback semantics are mapped from binary table entries, dispatcher byte offsets, indirect calls, embedded strings and helper behavior; the exact vendor source commit remains unknown.",
        "- This proves the matched BL31 exposes a CPU_ON-capable callback. It does **not** prove the path runs automatically at initial BL31-to-BL33 handoff.",
        "- The exact SoC variant selected by `0x03000024 & 7` was not read on-device. Do not assume the H616-named register branch is the RG34XX H700 active branch.",
        "- The effective nine-member legacy callback layout is identified from this exact binary; the exact vendor TF-A source commit/build revision remains unknown.",
        "- Initial CPU1-CPU3 reset state and the moment Linux/Linkslot requests CPU_ON remain runtime questions.",
        "- No hardware MMIO, flash, boot-media write or payload execution occurred.", "",
    ]
    (args.out.parent / "PSCI-CPU-ON.md").write_text("\n".join(markdown), encoding="utf-8")
    print("PASS: PSCI checksum default/SMC override, helper edges, PLL/XO restore and wake-source audit")
    print("  table[1] = 0x%08x -> low-level 0x48002d78" % table[1])
    print("  callback disassembly = %s" % disassembly_path)
    print("  monitor SHA-256 = %s" % digest)
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
