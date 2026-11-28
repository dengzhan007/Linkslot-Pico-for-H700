#!/usr/bin/env python3
"""Audit the pinned H700 SPL SRAM/BSS geometry before reserving M1 streaming scratch.

This is a source-level layout proof only. It does not build SPL, inspect a link
map, or authorize a device-side SRAM access.
"""
from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path

UBOOT_COMMIT = "5508406582f6f7120dce0c4b3059819a41788db2"
BASEOS_COMMIT = "cbe253a43f46cf551cc9a6c79a84015bde3686cf"
BASEOS_ARCHIVE_SHA256 = "f943a88389c8fd70a6b0d061a88267465e143c7ca6ea2b75837ab20f5b60c739"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit("FAIL " + message)


def read(root: Path, path: str) -> str:
    file = root / path
    require(file.is_file(), f"missing source file: {path}")
    return file.read_text()


def git_head(root: Path) -> str:
    return subprocess.run(
        ["git", "-C", str(root), "rev-parse", "HEAD"],
        check=True, capture_output=True, text=True,
    ).stdout.strip()


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--uboot-root", type=Path, required=True)
    ap.add_argument("--baseos-root", type=Path, required=True)
    ap.add_argument("--output", type=Path)
    args = ap.parse_args()
    uboot = args.uboot_root.resolve()
    baseos = args.baseos_root.resolve()

    uboot_head = git_head(uboot)
    baseos_head = git_head(baseos)
    require(uboot_head == UBOOT_COMMIT,
            f"U-Boot source pin mismatch: expected {UBOOT_COMMIT}, got {uboot_head}")
    require(baseos_head == BASEOS_COMMIT,
            f"BaseOS source pin mismatch: expected {BASEOS_COMMIT}, got {baseos_head}")

    defconfig = read(uboot, "configs/anbernic_rg35xx_h700_defconfig")
    arm_kconfig = read(uboot, "arch/arm/Kconfig")
    sunxi_kconfig = read(uboot, "arch/arm/mach-sunxi/Kconfig")
    spl_kconfig = read(uboot, "common/spl/Kconfig")
    disk_kconfig = read(uboot, "disk/Kconfig")
    linker = read(uboot, "arch/arm/cpu/armv8/u-boot-spl.lds")
    sunxi_header = read(uboot, "include/configs/sunxi-common.h")
    h616_dtsi = read(uboot, "dts/upstream/src/arm64/allwinner/sun50i-h616.dtsi")
    builder = read(baseos, "experiments/h700-boot-speed/spl/build-early-mmc.py")
    board = read(uboot, "arch/arm/mach-sunxi/board.c")
    m1_adapter = read(Path(__file__).resolve().parents[1], "platform/h700/boot/m1_gpt_spl_adapter.c")
    spl_builder = read(Path(__file__).resolve().parents[1], "scripts/build-m1-spl-preflight.py")

    require('CONFIG_DEFAULT_DEVICE_TREE="allwinner/sun50i-h700-anbernic-rg35xx-2024"' in defconfig,
            "defconfig is not the RG35XX H700 board")
    require("config MACH_SUN50I_H616" in sunxi_kconfig and
            "select ARM64" in sunxi_kconfig[sunxi_kconfig.index("config MACH_SUN50I_H616"):][:500] and
            "select SUN50I_GEN_H6" in sunxi_kconfig[sunxi_kconfig.index("config MACH_SUN50I_H616"):][:500],
            "H616 must select ARM64 and SUN50I_GEN_H6")
    require("imply SPL_SEPARATE_BSS" in arm_kconfig,
            "ARM64 no longer implies SPL_SEPARATE_BSS")
    require("default y if ARCH_SUNXI" in spl_kconfig,
            "sunxi SPL no longer defaults to a dedicated BSS linker section")
    require("MEMORY { .sdram : ORIGIN = CONFIG_SPL_BSS_START_ADDR," in linker and
            "LENGTH = CONFIG_SPL_BSS_MAX_SIZE" in linker and
            "} >.sdram" in linker,
            "AArch64 SPL linker script no longer places BSS in the separate .sdram region")
    require("CFG_SYS_INIT_RAM_ADDR\tCONFIG_SUNXI_SRAM_ADDRESS" in sunxi_header and
            "CFG_SYS_INIT_RAM_SIZE\t0x8000" in sunxi_header,
            "sunxi initial SRAM window contract changed")
    require("default 0x20000 if SUN50I_GEN_H6" in sunxi_kconfig,
            "H616 SRAM base default changed")
    require("default 0x52a00 if MACH_SUN50I_H616" in spl_kconfig,
            "H616 initial SPL stack default changed")
    spl_efi = disk_kconfig[disk_kconfig.index("config SPL_EFI_PARTITION"):][:700]
    require("default n if ARCH_SUNXI" in spl_efi and
            "default y if EFI_PARTITION" in spl_efi,
            "SPL GPT support defaults changed for sunxi")
    require("CONFIG_EFI_PARTITION=y" not in defconfig and
            "CONFIG_SPL_EFI_PARTITION=y" not in defconfig,
            "RG35XX H700 defconfig unexpectedly enables built-in SPL GPT")
    require("sram_c: sram@28000" in h616_dtsi and
            "reg = <0x00028000 0x30000>" in h616_dtsi,
            "H616 SRAM C range changed")

    require('PIN = "' + BASEOS_ARCHIVE_SHA256 + '"' in builder,
            "BaseOS U-Boot source archive checksum changed")
    require('archive = PARENT / "u-boot-v2026.10.tar.gz"' in builder,
            "BaseOS no longer builds its pinned U-Boot v2026.10 archive")
    require('--set-val SPL_BSS_START_ADDR 0x30000 --set-val SPL_BSS_MAX_SIZE 0x4000' in builder,
            "BaseOS SPL BSS placement/limit changed")
    require('memset(__bss_start, 0, __bss_end - __bss_start);' in builder,
            "BaseOS no longer clears SPL BSS before early hook")
    require('body = body.replace("\\tspl_init();", "\\tspl_early_init();")' in builder and
            'body = body.replace("\\tsunxi_board_init();", "\\tbaseos_early_mmc();\\n\\tsunxi_board_init();")' in builder,
            "BaseOS early-MMC hook ordering changed")
    require("void board_init_f(ulong dummy)" in board,
            "expected U-Boot SPL board_init_f entry missing")
    require("static uint8_t m1_package_scratch[H700_M1_STREAM_CHUNK_BYTES]" in m1_adapter and
            "__attribute__((aligned(64)))" in m1_adapter,
            "M1 adapter no longer declares its aligned static streaming scratch")
    require("static struct h700_gpt_partition m1_boot_partitions[H700_M1_GPT_ENTRY_SLOTS]" in m1_adapter and
            "#define H700_M1_GPT_ENTRY_SLOTS 8u" in m1_adapter,
            "M1 adapter no longer bounds its GPT entry table to eight slots")
    require("h700_m1_spl_preflight(card, p4_first_lba, p4_block_count" in m1_adapter,
            "M1 GPT adapter no longer chains partition discovery to firmware preflight")
    for option in ("SPL_DM", "DM_MMC", "SPL_DM_MMC", "BLK",
                   "SPL_BLK", "SPL_OF_CONTROL", "SPL_OF_REAL"):
        require(f'"--enable", "{option}"' in spl_builder,
                f"M1 preflight builder no longer enables {option}")
    for option in ("SPL_FIT_SIGNATURE", "SPL_FIT_IMAGE_TINY", "SPL_FIT_PRINT", "FIT_SIGNATURE", "FIT", "SPL_FIT", "SPL_LOAD_FIT",
                   "SPL_I2C", "SPL_POWER", "SPL_SUNXI_LED_STATUS", "SPL_DM_I2C", "SPL_DM_PMIC", "SPL_DM_REGULATOR", "SPL_DM_SERIAL", "SPL_DM_USB",
                   "CMD_BOOTD", "VERSION_VARIABLE", "SPL_CMDLINE_EDITING", "CMDLINE"):
        require(f'"--disable", "{option}"' in spl_builder,
                f"M1 preflight builder no longer disables unrelated {option}")
    require("fit_selector =" in spl_builder and
            "armv8_text.count(fit_selector) == 1" in spl_builder and
            "armv8_kconfig.write_text(armv8_text.replace(fit_selector" in spl_builder,
            "M1 preflight builder no longer removes the unused ARMv8 SPL_FIT selector")
    require('"m1-spl-preflight-resolved.config"' in spl_builder and
            "relevant_config =" in spl_builder,
            "M1 preflight builder no longer preserves resolved Kconfig diagnostics")
    require('"CONFIG_OF_REAL=y" in config_text' in spl_builder,
            "M1 preflight builder incorrectly drops the OF_CONTROL-selected OF_REAL dependency")
    require('sunxi_kconfig = uboot / "arch/arm/mach-sunxi/Kconfig"' in spl_builder and
            'config SUN50I_GEN_H6' in spl_builder and
            'select SPL_LOAD_FIT if SPL' in spl_builder and
            'sunxi_kconfig.write_text' in spl_builder,
            "M1 preflight builder no longer removes H616 FIT selectors")
    require('arm_kconfig = uboot / "arch/arm/Kconfig"' in spl_builder and
            'config ARCH_SUNXI' in spl_builder and
            'select SPL_SYS_MALLOC_SIMPLE if SPL' in spl_builder and
            'select SPL_USE_TINY_PRINTF if SPL' in spl_builder and
            'arm_kconfig.write_text' in spl_builder,
            "M1 preflight builder no longer releases ARCH_SUNXI tiny allocator/printf selectors")
    for option in ("SPL_LIBCOMMON_SUPPORT", "SPL_ENV_SUPPORT", "SPL_CYCLIC"):
        require(f'"--enable", "{option}"' in spl_builder,
                f"M1 preflight builder no longer enables required SPL provider {option}")
    require('image_fdt_rule =' in spl_builder and
            'boot_makefile.write_text(boot_text)' in spl_builder and
            'display_rule =' in spl_builder and
            'lib_makefile.write_text(lib_text)' in spl_builder and
            '__start_rodata = .' in spl_builder and '__end_rodata = .' in spl_builder,
            "M1 preflight builder no longer isolates SPL image/banner helpers and rodata bounds")

    # Geometry derived from pinned Kconfig, linker script and H616 SRAM node.
    sram_start, sram_bytes = 0x28000, 0x30000
    sram_end = sram_start + sram_bytes
    bss_start, bss_limit = 0x30000, 0x4000
    bss_end = bss_start + bss_limit
    spl_stack = 0x52a00
    init_ram_start, init_ram_bytes = 0x20000, 0x8000
    require(sram_start <= bss_start < bss_end <= sram_end,
            "configured BSS window is outside H616 SRAM C")
    require(init_ram_start + init_ram_bytes == sram_start,
            "initial SRAM window no longer meets SRAM C base")
    require(bss_end < spl_stack < sram_end,
            "BSS limit and initial SPL stack are no longer disjoint")
    gap = spl_stack - bss_end

    report = {
        "format": "H700 SPL SRAM/BSS source-layout audit",
        "uboot_tag": "v2026.10",
        "uboot_commit": uboot_head,
        "baseos_commit": baseos_head,
        "baseos_u_boot_archive_sha256": BASEOS_ARCHIVE_SHA256,
        "h616_sram_c": {"start": hex(sram_start), "bytes": hex(sram_bytes), "end_exclusive": hex(sram_end)},
        "sunxi_initial_ram": {"start": hex(init_ram_start), "bytes": hex(init_ram_bytes), "end_exclusive": hex(init_ram_start + init_ram_bytes)},
        "spl_bss_configured_window": {"start": hex(bss_start), "max_bytes": hex(bss_limit), "end_exclusive": hex(bss_end)},
        "spl_initial_stack_pointer": hex(spl_stack),
        "bss_end_to_stack_pointer_gap_bytes": gap,
        "separate_bss": "ARM64 implies SPL_SEPARATE_BSS; sunxi linker places .bss in .sdram",
        "bss_zeroed_before_early_mmc": True,
        "early_mmc_hook_order": "spl_early_init -> baseos_early_mmc -> sunxi_board_init",
        "uboot_spl_gpt_support_default": False,
        "m1_gpt_strategy": "use the project read-only GPT parser; do not assume part_get_info_by_name is backed by SPL_EFI_PARTITION",
        "m1_scratch_4k_fits_configured_bss_window": True,
        "preflight_build_profile": "DM-backed read-only MMC + SPL_OF_CONTROL; unrelated common/FIT/PMIC/TWI/display paths disabled",
        "dynamic_buffer_helpers": "membuf.o and abuf.o excluded from isolated SPL link",
        "adapter_scratch_symbol_declared": True,
        "adapter_gpt_entry_slots": 8,
        "actual_bss_consumption_known": False,
        "linked_map_verified": False,
        "scratch_symbol_integrated": False,
        "buffer_reservation_approved": False,
        "device_io_performed": False,
        "hardware_qualified": False,
        "candidate_eligible": False,
        "limitations": [
            "The BaseOS archive SHA-256 and upstream U-Boot tag commit are separate pins; this audit does not prove the archive bytes are identical to the Git tag tree.",
            "The 16 KiB BSS limit is a linker-enforced ceiling, not a measurement of current BSS use or remaining free bytes.",
            "The adapter declares a named 4 KiB aligned scratch symbol and an eight-slot GPT table; the isolated builder now requests DM MMC + SPL_OF_CONTROL and removes unrelated command/FIT/PMIC/TWI/display paths, but the current linked SPL attempt has not yet passed.",
            "The isolated build profile is not the stock BaseOS SPL configuration and is not an installable image.",
            "A final M1-enabled SPL link map and ELF are still required to measure BSS symbols, prove the 4 KiB scratch reservation fits, and inspect transitive call sites.",
            "The initial stack pointer is source-configured; stack high-water usage is not measured here.",
            "U-Boot SPL EFI/GPT partition support defaults off for ARCH_SUNXI and is not enabled by the RG35XX H700 defconfig; M1 needs its own bounded GPT adapter or a separately audited config change.",
            "No SRAM access, MMC read, DRAM access, hardware register access, media write, or payload execution occurs."
        ]
    }
    encoded = json.dumps(report, indent=2) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(encoded)
    print(encoded, end="")
    print("PASS pinned H616 SRAM C / separate SPL BSS geometry")
    print("PASS configured 16 KiB BSS ceiling and 0x52a00 SPL stack are non-overlapping")
    print("LIMIT source-only: no linked map, actual BSS headroom, or scratch-symbol proof")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
