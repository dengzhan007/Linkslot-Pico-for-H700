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
    assert window_map["normal_suspend_powerdown_helper"][0] == 0x48015D70
    assert window_map["system_suspend_cache_teardown_and_trampoline"][0] == 0x48015FB8
    assert window_map["system_suspend_cache_teardown_and_trampoline"][1] >= 0x270
    assert window_map["system_suspend_caller_full"][0] == 0x480096F8
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
    print("PASS: PSCI standby staging, CPUX_AXI bitfield, wait loop and RTC wake-return")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
