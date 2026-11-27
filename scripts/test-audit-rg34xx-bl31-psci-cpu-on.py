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
    assert module.EXPECTED_TABLE == [
        0x48002510, 0x4800254c, 0x4800264c, 0x48002878, 0x48002680,
        0x480026c4, 0x480025a0, 0x48002574, 0x48002544,
    ]
    assert module.OPS_MEMBER_NAMES == (
        "affinst_standby", "affinst_on", "affinst_off", "affinst_suspend",
        "affinst_on_finish", "affinst_suspend_finish", "system_off",
        "system_reset", "get_sys_suspend_power_state"
    )
    assert module.WORDS[0x48002544] == 0x52A04020
    assert module.SUSPEND_LEVEL_TABLE == 0x48017D70
    assert module.SUSPEND_LEVEL_CALLBACKS == (
        0x48008B84, 0x48008AB8, 0x48008A00
    )
    assert module.WORDS[0x48008E4C] == 0xF0000077
    assert module.WORDS[0x48008E54] == 0x9135C2F7
    assert module.WORDS[0x48008E64] == 0x0B160281
    assert module.WORDS[0x48008E68] == 0x6B0102BF
    assert module.WORDS[0x48008E70] == 0xF8767B60
    assert module.WORDS[0x48008E74] == 0xB4000100
    assert module.WORDS[0x48008E78] == 0xF861DAE4
    assert module.WORDS[0x48008E88] == 0xD63F0080
    assert module.WORDS[0x48008E94] == 0x910006D6
    assert module.WORDS[0x48008E98] == 0x17FFFFF3
    assert module.WORDS[0x48008A18] == 0x71000A9F
    assert module.WORDS[0x48008AD0] == 0x7100069F
    assert module.WORDS[0x48008A84] == 0xF9400C26
    assert module.WORDS[0x48008A90] == 0xD63F00C0
    assert module.WORDS[0x48008B34] == 0xF9400C26
    assert module.WORDS[0x48008B40] == 0xD63F00C0
    assert module.WORDS[0x48008C20] == 0xF9400C36
    assert module.WORDS[0x48008C54] == 0xD63F02C0
    assert module.WORDS[0x4800299C] == 0x12800060
    assert module.WORDS[0x48009768] == 0x3100101F
    assert module.WORDS[0x48009770] == 0x54000081
    assert module.WORDS[0x48009774] == 0x97FFE0A4
    assert module.decode_bl_target(0x48009774, 0x97FFE0A4) == 0x48001A04
    assert module.WORDS[0x48001B5C] == 0x94005173
    assert module.WORDS[0x48001B64] == 0xD2803700
    assert module.WORDS[0x48001B6C] == 0xF2A00040
    assert module.WORDS[0x48001B70] == 0xD61F0000
    assert module.decode_bl_target(0x48001B5C, 0x94005173) == 0x48016128
    assert (0x1B8 | (0x2 << 16)) == 0x201B8
    assert module.WORDS[0x48015D70] == 0xD5033F9F
    assert module.WORDS[0x48015D74] == 0xD503207F
    assert module.WORDS[0x48015D78] == 0x17FFFFFF
    assert module.SRAM_SUSPEND_COPY_SOURCE == 0x4800C220
    assert module.SRAM_SUSPEND_COPY_END == 0x48015630
    assert module.SRAM_SUSPEND_COPY_DEST == 0x20000
    assert module.SRAM_SUSPEND_ENTRY_SOURCE == 0x4800C3D8
    assert module.SRAM_SUSPEND_ENTRY == 0x201B8
    assert module.SRAM_SUSPEND_STACK_SETUP_SOURCE == 0x480140A0
    assert module.SRAM_SUSPEND_STACK_TOP == 0x57FF0
    assert module.SRAM_SUSPEND_COPY_END - module.SRAM_SUSPEND_COPY_SOURCE == 0x9410
    assert module.SRAM_SUSPEND_COPY_DEST + (
        module.SRAM_SUSPEND_ENTRY_SOURCE - module.SRAM_SUSPEND_COPY_SOURCE
    ) == module.SRAM_SUSPEND_ENTRY
    assert module.SRAM_SUSPEND_COPY_DEST + (
        module.SRAM_SUSPEND_STACK_SETUP_SOURCE - module.SRAM_SUSPEND_COPY_SOURCE
    ) == 0x27E80
    assert module.SRAM_SUSPEND_COPY_DEST + (
        module.SRAM_SUSPEND_SCRATCH_SOURCE_START - module.SRAM_SUSPEND_COPY_SOURCE
    ) == 0x291F0
    assert module.SRAM_SUSPEND_COPY_DEST + (
        module.SRAM_SUSPEND_SCRATCH_SOURCE_LAST - module.SRAM_SUSPEND_COPY_SOURCE
    ) + 1 == 0x29349
    assert module.WORDS[0x48001A08] == 0xF0000041
    assert module.WORDS[0x48001A0C] == 0x900000A2
    assert module.WORDS[0x48001A10] == 0x91088021
    assert module.WORDS[0x48001A14] == 0x9118C042
    assert module.WORDS[0x48001A1C] == 0xCB010042
    assert module.WORDS[0x48001A20] == 0xD2A00040
    assert module.WORDS[0x48001A28] == 0xD2A00053
    assert module.WORDS[0x48001A2C] == 0x94002415
    assert module.WORDS[0x48001A34] == 0xF00001C1
    assert module.WORDS[0x48001A3C] == 0x912E4021
    assert module.WORDS[0x48001A40] == 0xD2800602
    assert module.WORDS[0x48001A44] == 0xF2A00040
    assert module.WORDS[0x48001A48] == 0x9400240E
    assert module.WORDS[0x48001A90] == 0xD2800601
    assert module.WORDS[0x48001A94] == 0xD2801002
    assert module.WORDS[0x48001A9C] == 0xF2A90001
    assert module.WORDS[0x48001AA8] == 0x940023F6
    assert module.WORDS[0x48001B00] == 0xD2802680
    assert module.WORDS[0x48001B04] == 0x910E0021
    assert module.WORDS[0x48001B08] == 0xD2801002
    assert module.WORDS[0x48001B10] == 0x940023DC
    assert module.WORDS[0x48001B1C] == 0x910C2021
    assert module.WORDS[0x48001B20] == 0xD2800C02
    assert module.WORDS[0x48001B28] == 0xD2801A80
    assert module.WORDS[0x48001B30] == 0x940023D4
    assert module.WORDS[0x48001A54] == 0xB900C660
    for pc in (0x48001A2C,0x48001A48,0x48001AA8,0x48001B10,0x48001B30):
        assert module.decode_bl_target(pc,module.WORDS[pc]) == 0x4800AA80
    assert module.WORDS[0x4800AA8C] == 0x38636824
    assert module.WORDS[0x4800AA90] == 0x38236804
    assert module.WORDS[0x4800AA94] == 0x91000463
    assert module.WORDS[0x4800C3D8] == 0xB0000040
    assert module.WORDS[0x4800C3DC] == 0xB0000041
    assert module.WORDS[0x4800C3E0] == 0x91104000
    assert module.WORDS[0x4800C3E4] == 0x9115A021
    assert module.WORDS[0x4800C3E8] == 0x3800141F
    assert module.WORDS[0x4800C3EC] == 0xEB01001F
    assert module.WORDS[0x4800C3F0] == 0x54FFFFC9
    assert module.decode_bl_target(0x4800C3FC,0x94001F29) == 0x480140A0
    assert module.decode_bl_target(0x4800C400,0x94000006) == 0x4800C418
    assert module.decode_bl_target(0x4800C404,0x94000033) == 0x4800C4D0
    assert module.decode_bl_target(0x4800C408,0x94000084) == 0x4800C618
    assert module.WORDS[0x480140A0] == 0x58000081
    assert module.WORDS[0x480140A4] == 0x9100003F
    assert module.WORDS[0x480140A8] == 0xD65F03C0
    assert module.WORDS[0x4800C580] == 0xD2803F00
    assert module.WORDS[0x4800C584] == 0x52A00023
    assert module.WORDS[0x4800C588] == 0xF2A0E000
    assert module.WORDS[0x4800C598] == 0xB9000003
    assert module.WORDS[0x4800C594] == 0xD2803F84
    assert module.WORDS[0x4800C59C] == 0xF2A0E004
    assert module.WORDS[0x4800C5A8] == 0xB940C426
    assert module.WORDS[0x4800C5B8] == 0xB9000086
    assert module.WORDS[0x4800C460] == 0xD282A002
    assert module.WORDS[0x4800C468] == 0xF2A06002
    assert module.WORDS[0x4800C470] == 0xB9400041
    assert module.WORDS[0x4800C474] == 0x12067421
    assert module.WORDS[0x4800C478] == 0xB9000041
    assert module.WORDS[0x4800C47C] == 0x940016DF
    assert module.WORDS[0x4800C680] == 0xD282A002
    assert module.WORDS[0x4800C684] == 0xF2A06002
    assert module.WORDS[0x4800C690] == 0xB9400041
    assert module.WORDS[0x4800C694] == 0x32080421
    assert module.WORDS[0x4800C698] == 0xB9000041
    assert module.WORDS[0x4800C69C] == 0x94001657
    assert module.WORDS[0x4800C6A8] == 0xD63F02C0
    assert module.decode_bl_target(0x4800C47C,0x940016DF) == 0x48011FF8
    assert module.decode_bl_target(0x4800C69C,0x94001657) == 0x48011FF8
    assert ((0x300 << 16) | 0x1500) == 0x03001500
    assert (0xFCFFFFFF & 0x03000000) == 0
    assert (0x03000000 & 0x04000000) == 0
    assert (0x03000000 & 0x03000000) == 0x03000000
    assert module.WORDS[0x48015D14] == 0x1000FB77
    assert module.WORDS[0x48015D18] == 0xD2820141
    assert module.WORDS[0x48015D1C] == 0xD53E1000
    assert module.WORDS[0x48015D20] == 0xAA010000
    assert module.WORDS[0x48015D24] == 0xD51E1000
    assert module.WORDS[0x48015D28] == 0xD5033FDF
    assert module.WORDS[0x48015D30] == 0x10019680
    assert module.WORDS[0x48015D34] == 0xD51EC000
    assert module.WORDS[0x48015D38] == 0xD5033FDF
    assert module.WORDS[0x48015D3C] == 0xD50344FF
    assert module.WORDS[0x48015D40] == 0xD50040BF
    assert module.WORDS[0x48015D44] == 0xD53800A0
    assert module.WORDS[0x48015D48] == 0x97FFD930
    assert module.WORDS[0x48015D68] == 0x97FFCDD9
    assert module.WORDS[0x48015D6C] == 0x17FFFED7
    assert module.WORDS[0x4800C50C] == 0xD503207F
    assert module.WORDS[0x4800C550] == 0xD503207F
    assert module.decode_bl_target(0x4800C53C,0x94001E2D) == 0x48013DF0
    assert module.decode_bl_target(0x4800C568,0x94001E60) == 0x48013EE8
    assert module.WORDS[0x4801612C] == 0xD53E1000
    assert module.WORDS[0x48016130] == 0x8A210000
    assert module.WORDS[0x48016134] == 0xD51E1000
    assert module.WORDS[0x48016138] == 0xD5033FDF
    assert module.WORDS[0x48016140] == 0x17FFFFDA
    checksum_expected = {
        0x48011B78: 0x29560C01, 0x48011B7C: 0x52800000,
        0x48011B80: 0x0B030023, 0x48011B84: 0x2A0103E1,
        0x48011B88: 0xEB01007F, 0x48011B8C: 0x540000C9,
        0x48011B90: 0xB8404422, 0x48011B94: 0xEB01007F,
        0x48011B98: 0x0B020000, 0x48011B9C: 0x54FFFFA8,
        0x48011BA0: 0xD65F03C0, 0x4800C43C: 0xB940C260,
        0x4800C440: 0x34000080, 0x4800C444: 0xAA1303E0,
        0x4800C448: 0x940015CC, 0x4800C44C: 0xB900BA60,
        0x4800C6C0: 0xAA1503E0, 0x4800C6C4: 0x9400152D,
        0x4800C6CC: 0xB940B822, 0x4800C6D0: 0xB900BEA0,
        0x4800C6D4: 0xB940BC20, 0x4800C6D8: 0x6B00005F,
        0x4800C6DC: 0x54FFFCC0, 0x4800C6E0: 0x52800020,
        0x4800C6E4: 0x72B10000, 0x4800C6E8: 0xB9000280,
        0x4800C6EC: 0x14000000,
    }
    for address, expected in checksum_expected.items():
        assert module.WORDS[address] == expected
    assert module.decode_bl_target(0x4800C448,module.WORDS[0x4800C448]) == 0x48011B78
    assert module.decode_bl_target(0x4800C6C4,module.WORDS[0x4800C6C4]) == 0x48011B78
    assert module.decode_imm19_branch_target(0x4800C440,0x34000080) == 0x4800C450
    assert module.decode_cond_branch_target(0x4800C6DC,0x54FFFCC0) == 0x4800C674
    assert module.decode_b_target(0x4800C6EC,0x14000000) == 0x4800C6EC
    # The checksum descriptor is separate from the SRAM standby image.
    assert module.SUSPEND_CHECKSUM_CONFIG_BASE_ADDR == 0x4801A000
    assert module.SUSPEND_CHECKSUM_CONFIG_LENGTH_ADDR == 0x4801A004
    assert module.SUSPEND_CHECKSUM_CONFIG_ENABLE_ADDR == 0x4801A008
    assert module.SUSPEND_CHECKSUM_CONFIG_LABEL_ADDR == 0x4801A010
    assert module.SUSPEND_CHECKSUM_INITIAL_BASE == 0x40000000
    assert module.SUSPEND_CHECKSUM_INITIAL_LENGTH == 0x100
    assert module.SUSPEND_CHECKSUM_INITIAL_ENABLE == 1
    assert module.SUSPEND_CHECKSUM_INITIAL_END == 0x40000100
    assert module.SUSPEND_CHECKSUM_INITIAL_WORD_COUNT == 64
    checksum_config_load_store = {
        0x48001A58: 0xB00000C0, 0x48001A5C: 0x39402000,
        0x48001A60: 0xB900C260, 0x48001A64: 0xB00000C0,
        0x48001A68: 0xB9400000, 0x48001A6C: 0xB900B260,
        0x48001A70: 0xB00000C0, 0x48001A74: 0xB9400400,
        0x48001A78: 0xB900B660,
    }
    for address, expected in checksum_config_load_store.items():
        assert module.WORDS[address] == expected
    assert module.decode_bl_target(0x48001C3C,module.WORDS[0x48001C3C]) == 0x48001A04
    assert module.decode_bl_target(0x48002440,module.WORDS[0x48002440]) == 0x48001B74
    assert module.decode_bl_target(0x48011928,module.WORDS[0x48011928]) == 0x48011710
    assert module.WORDS[0x48011B30] == 0xD65F03C0
    assert module.WORDS[0x48011B38] == 0xD65F03C0
    assert module.decode_bl_target(0x48011B48,module.WORDS[0x48011B48]) == 0x48011B38
    assert module.decode_b_target(0x48011B50,module.WORDS[0x48011B50]) == 0x48011920
    assert module.decode_bl_target(0x48011B60,module.WORDS[0x48011B60]) == 0x48011B10
    assert module.decode_bl_target(0x48011B64,module.WORDS[0x48011B64]) == 0x48011B30
    suspend_clock_expected = {
        0x48011BB0: 0xF2A06005, 0x48011BC4: 0xB94000A7,
        0x48011BD4: 0x0A0100E1, 0x48011BE0: 0xB9041441,
        0x48011C28: 0x528063E2, 0x48011C2C: 0x72A06002,
        0x48011C54: 0x12067463, 0x48011C5C: 0x32080063,
        0x48011C64: 0xB9000103, 0x48011C68: 0xD2804888,
        0x48011C6C: 0xF2A0E028, 0x48011C70: 0x52B4E003,
        0x48011C90: 0xB94000A1, 0x48011C94: 0x12067421,
        0x48011C98: 0x32080021, 0x48011C9C: 0xB90000A1,
        0x48011CA8: 0x0A040021, 0x48011CAC: 0x120078A5,
        0x48011CB0: 0xB9000045, 0x48011CB4: 0xB9001C01,
        0x48011CBC: 0x12027800, 0x48011CC0: 0xB9000040,
        0x48011CC4: 0xB9400100, 0x48011CC8: 0x12005C00,
        0x48011CCC: 0x2A030000, 0x48011CD0: 0xB9000100,
        0x48011D88: 0xD282A205, 0x48011DA8: 0x12067421,
        0x48011D8C: 0xD2A0E024, 0x48011D90: 0xF2A06005,
        0x48011D94: 0xD2800203, 0x48011D98: 0xF2A0E023,
        0x48011D9C: 0xD2802C02, 0x48011DA0: 0xF2A0E002,
        0x48011DA4: 0xB94000A1, 0x48011DB4: 0xB9400081,
        0x48011DB8: 0x12067421, 0x48011DBC: 0x32070021,
        0x48011DC0: 0xB9000081, 0x48011DC4: 0xB9400061,
        0x48011DC8: 0x12067421, 0x48011DCC: 0x32070021,
        0x48011DD0: 0xB9000061, 0x48011DD4: 0xB9400041,
        0x48011DE0: 0xB9400041,
        0x48011DAC: 0x32070021, 0x48011DB0: 0xB90000A1,
        0x48011DD8: 0x32010021, 0x48011DDC: 0xB9000041,
        0x48011DE4: 0x121E7821, 0x48011DE8: 0xB9000041,
        0x48011DF0: 0x17FFFFC0,
    }
    for address, expected in suspend_clock_expected.items():
        assert module.WORDS[address] == expected
    pll_resume_expected = {
        0x48011E10: 0x9400089A, 0x48011E48: 0x12027884,
        0x48011E54: 0x0A030083, 0x48011E58: 0x2A020062,
        0x48011E5C: 0xB9000002, 0x48011E64: 0x32010042,
        0x48011E68: 0xB9000002, 0x48011E70: 0x32030042,
        0x48011E74: 0xB9000002, 0x48011E78: 0xB9400020,
        0x48011E7C: 0x36E7FFE0, 0x48011E94: 0x52800280,
        0x48011EC0: 0x9400004E,
    }
    for address, expected in pll_resume_expected.items():
        assert module.WORDS[address] == expected
    assert module.decode_bl_target(0x48011D2C,module.WORDS[0x48011D2C]) == 0x48013620
    assert module.decode_bl_target(0x48011D6C,module.WORDS[0x48011D6C]) == 0x48013620
    assert module.decode_bl_target(0x48011E10,module.WORDS[0x48011E10]) == 0x48014078
    assert module.decode_bl_target(0x48011EC0,module.WORDS[0x48011EC0]) == 0x48011FF8
    assert module.WORDS[0x48011C74] == 0xB94000C1
    assert module.WORDS[0x48011C78] == 0x29029007
    assert module.WORDS[0x48011C7C] == 0x529FE064
    assert module.WORDS[0x48011C80] == 0x12067421
    assert module.WORDS[0x48011C84] == 0x72A00064
    assert module.WORDS[0x48011C88] == 0x32080021
    assert module.WORDS[0x48011C8C] == 0xB90000C1
    assert ((0x701 << 16) | 0x244) == 0x07010244
    assert ((0x700 << 16) | 0x160) == 0x07000160
    assert (0x48011C70, module.WORDS[0x48011C70]) == (0x48011C70, 0x52B4E003)
    assert module.WORDS[0x48011DD8] == 0x32010021
    assert module.WORDS[0x48011DDC] == 0xB9000041
    assert module.WORDS[0x48011DE4] == 0x121E7821
    assert module.WORDS[0x48011DE8] == 0xB9000041
    wake_expected = {
        0x48013194: 0xD2824003, 0x4801319C: 0xF2A06043,
        0x480131A0: 0x531E7401, 0x480131A4: 0x52800022,
        0x480131A8: 0x1AC42040, 0x480131AC: 0xB8614861,
        0x480131B0: 0x0A010000, 0x480131B4: 0xD65F03C0,
        0x480131C8: 0xF2A0E089, 0x480131CC: 0xF2A0E086,
        0x480131D8: 0xB9400128, 0x480131DC: 0x12001D05,
        0x480131E0: 0xD3483901, 0x480131E4: 0xB9000125,
        0x48013200: 0xB94000C4, 0x4801320C: 0x39001024,
        0x48013598: 0x51010442, 0x480135A4: 0x5100C060,
        0x480135B4: 0x2A024000, 0x480135D8: 0x53107C01,
        0x480135E0: 0x7100283F, 0x480135F0: 0x72A06002,
        0x480135F8: 0x1A843042, 0x48013600: 0x531E7421,
        0x48013608: 0x0B020021, 0x4801360C: 0xB9400021,
        0x48013610: 0x6A01001F, 0x48013614: 0x1A9F07E0,
        0x48013654: 0x53107C26, 0x48013660: 0x72A06003,
        0x48013668: 0x710028DF, 0x48013674: 0x1A883068,
        0x4801367C: 0x531E7444, 0x480136C4: 0x0A250085,
        0x480136D4: 0xB90000E5, 0x480136DC: 0xB9401007,
        0x480136F4: 0xB9000044, 0x48013714: 0xB90000C2,
        0x48013724: 0xB9000060,
    }
    for address, expected in wake_expected.items():
        assert module.WORDS[address] == expected
    wake_call_targets = {
        0x48013E88: 0x48013620, 0x48013EC8: 0x48013620,
        0x48013F14: 0x480131B8, 0x48013F48: 0x48013588,
        0x48013F54: 0x480135D8, 0x48013F8C: 0x48013588,
        0x48013F94: 0x480135D8, 0x48013FE4: 0x48013190,
        0x48014050: 0x480131B8,
    }
    for call_pc, expected_target in wake_call_targets.items():
        assert module.decode_bl_target(call_pc,module.WORDS[call_pc]) == expected_target
    assert module.WORDS[0x48011FF8] == 0x531E7400
    assert module.WORDS[0x48011FFC] == 0x1400081F
    assert module.WORDS[0x48014078] == 0xF1000400
    assert module.WORDS[0x4801407C] == 0x54FFFFE8
    assert module.WORDS[0x48014080] == 0xD65F03C0
    assert module.decode_b_target(0x48011FFC,0x1400081F) == 0x48014078
    assert module.decode_cond_branch_target(0x4801407C,0x54FFFFE8) == 0x48014078
    assert module.WORDS[0x4800C464] == 0x52800140
    assert module.WORDS[0x4800C688] == 0x52800140
    assert module.WORDS[0x480160A8] == 0xD5390029
    assert module.WORDS[0x48015FB8] == 0xB40003E3
    assert module.WORDS[0x48015FE4] == 0xD51A000A
    assert module.WORDS[0x48015FEC] == 0xD5390001
    assert module.WORDS[0x48016040] == 0xD508764B
    assert module.WORDS[0x48016060] == 0xD5087E4B
    assert module.WORDS[0x48016080] == 0xD5087A4B
    assert module.WORDS[0x48002588] == 0x97FFFE50
    assert module.WORDS[0x480025A8] == 0x97FFFE4C
    assert module.WORDS[0x48009688] == 0xF9400021
    assert module.WORDS[0x48009690] == 0xD63F0020
    assert module.WORDS[0x480080F8] == 0xF9400835
    assert module.WORDS[0x48008A84] == 0xF9400C26
    assert module.WORDS[0x4800854C] == 0xF9401003
    assert module.WORDS[0x48008918] == 0xF9401421
    assert module.WORDS[0x4800971C] == 0xF9402000
    assert module.WORDS[0x48009744] == 0xD63F0000
    assert module.decode_bl_target(0x48002588, 0x97FFFE50) == 0x48001EC8
    assert module.decode_bl_target(0x480025A8, 0x97FFFE4C) == 0x48001ED8
    assert module.decode_bl_target(0x4800975C, 0x97FFFD8F) == 0x48008D98
    window_map = {name: (start, size) for name, start, size in module.DISASSEMBLY_WINDOWS}
    assert "ops_slot_0_raw" in window_map
    assert window_map["ops_slot_0_raw"][0] == 0x48002510
    assert window_map["psci_dispatch_cpu_on_primary"][0] <= 0x48008608
    assert window_map["psci_dispatch_cpu_on_secondary"][0] <= 0x480086a0
    assert window_map["psci_suspend_dispatch_tail"][0] <= 0x480087f4 < window_map["psci_suspend_dispatch_tail"][0] + window_map["psci_suspend_dispatch_tail"][1]
    assert window_map["psci_post_suspend_dispatch_and_helpers"][0] == 0x48008CF0
    assert window_map["psci_post_suspend_dispatch_and_helpers"][0] + window_map["psci_post_suspend_dispatch_and_helpers"][1] == 0x48009DC0
    assert window_map["psci_platform_registration"][0] <= 0x48009dec
    assert window_map["power_system_off_helper"][0] == 0x48001ec8
    assert window_map["power_cpu_off_helper"][0] == 0x48001ed8
    assert window_map["legacy_slot0_standby_dispatch"][0] == 0x48009680
    assert window_map["legacy_slot2_off_dispatch"][0] <= 0x48008118
    assert window_map["legacy_slot3_suspend_dispatch"][0] <= 0x48008a90
    assert window_map["legacy_slot4_on_finish_dispatch"][0] <= 0x48008560
    assert window_map["legacy_slot5_suspend_finish_dispatch"][0] <= 0x4800893c
    assert window_map["legacy_slot8_system_suspend_getter_dispatch"][0] <= 0x4800975c
    assert window_map["system_suspend_transition_dispatch"][0] == 0x48008D98
    assert window_map["suspend_afflvl0_wrapper"][0] == 0x48008B84
    assert window_map["suspend_afflvl1_wrapper"][0] == 0x48008AB8
    assert window_map["suspend_afflvl2_wrapper"][0] == 0x48008A00
    assert window_map["system_suspend_power_entry"][0] == 0x48001A04
    assert window_map["system_suspend_power_entry"][1] >= 0x300
    assert window_map["system_suspend_sram_memcpy"][0] == 0x4800AA80
    assert window_map["sram_resume_entry_source"][0] == 0x4800C3D8
    assert window_map["sram_resume_power_and_rtc_source"][0] == 0x4800C418
    assert window_map["sram_resume_stack_setup_source"][0] == 0x480140A0
    assert window_map["sram_resume_return_target"][0] == 0x48015D14
    assert window_map["cpux_axi_clock_transition_helper"][0] == 0x48011FF8
    assert window_map["cpux_axi_wait_target"][0] == 0x48014078
    assert window_map["cpux_axi_wait_target"][1] >= 0x100
    assert window_map["system_suspend_helper_48011b40"][0] == 0x48011B40
    assert window_map["system_suspend_helper_48011ba8"][0] == 0x48011BA8
    assert window_map["system_suspend_helper_48011df8"][0] == 0x48011DF8
    assert window_map["system_suspend_helper_48011b58"][0] == 0x48011B58
    assert window_map["system_suspend_helper_48011b78"][0] == 0x48011B78
    assert window_map["sram_resume_power_and_rtc_source"][0] == 0x4800C418
    assert window_map["system_suspend_helper_48013df0"][0] == 0x48013DF0
    assert window_map["system_suspend_helper_48013ee8"][0] == 0x48013EE8
    assert window_map["system_suspend_helper_48012670"][0] == 0x48012670
    assert window_map["system_suspend_helper_48011ba8_full"][1] >= 0x240
    assert window_map["system_suspend_helper_48011df8_full"][1] >= 0x180
    assert window_map["system_suspend_helper_48013ee8_full"][1] >= 0x180
    assert window_map["system_suspend_helper_48011b38"][0] == 0x48011B38
    assert window_map["system_suspend_helper_48011920"][0] == 0x48011920
    assert window_map["system_suspend_helper_48011b10"][0] == 0x48011B10
    assert window_map["system_suspend_helper_48011b30"][0] == 0x48011B30
    assert window_map["system_suspend_helper_48012070"][0] == 0x48012070
    assert window_map["system_suspend_helper_480126b0"][0] == 0x480126B0
    assert window_map["system_suspend_helper_48012868"][0] == 0x48012868
    assert window_map["system_suspend_helper_48013490"][0] == 0x48013490
    assert window_map["system_suspend_helper_480125d0"][0] == 0x480125D0
    assert window_map["system_suspend_helper_48013620"][0] == 0x48013620
    assert window_map["system_suspend_helper_48013738"][0] == 0x48013738
    assert window_map["system_suspend_helper_480131b8"][0] == 0x480131B8
    assert window_map["system_suspend_helper_48013588"][0] == 0x48013588
    assert window_map["system_suspend_helper_480135d8"][0] == 0x480135D8
    assert window_map["system_suspend_helper_48012510"][0] == 0x48012510
    assert window_map["system_suspend_helper_48013190"][0] == 0x48013190
    assert window_map["system_suspend_helper_48011ba8_complete"][1] >= 0x280
    assert window_map["system_suspend_helper_48011df8_complete"][1] >= 0x240
    assert window_map["system_suspend_helper_48011d80_tail"][0] == 0x48011D80
    assert window_map["system_suspend_helper_48011920_complete"][1] >= 0x220
    assert window_map["system_suspend_helper_48013df0_complete"][1] >= 0x180
    assert window_map["system_suspend_helper_480131b8_ir_capture"][0] == 0x480131B8
    assert window_map["system_suspend_helper_48013588_gpio_name"][0] == 0x48013588
    assert window_map["system_suspend_helper_480135d8_gpio_read"][0] == 0x480135D8
    assert window_map["system_suspend_helper_48013620_gpio_test"][0] == 0x48013620
    assert window_map["system_suspend_helper_480129f0"][0] == 0x480129F0
    assert window_map["system_suspend_helper_48012a08"][0] == 0x48012A08
    assert window_map["system_suspend_helper_48012aa0"][0] == 0x48012AA0
    assert window_map["system_suspend_helper_48013b98"][0] == 0x48013B98
    assert window_map["normal_suspend_powerdown_helper"][0] == 0x48015D70
    assert window_map["system_suspend_cache_teardown_and_trampoline"][0] == 0x48015FB8
    assert window_map["system_suspend_cache_teardown_and_trampoline"][1] >= 0x270
    assert window_map["system_suspend_caller_full"][0] == 0x480096F8
    assert window_map["system_suspend_setup_caller"][0] == 0x48002400
    assert window_map["system_suspend_descriptor_handoff"][0] == 0x48002284
    assert window_map["system_suspend_descriptor_consumer"][0] == 0x48003374
    assert window_map["system_suspend_retention_helper_48011710"][0] == 0x48011710
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
    print("PASS: PSCI standby integrity checksum, PLL/XO clock restore and wake-source helpers")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
