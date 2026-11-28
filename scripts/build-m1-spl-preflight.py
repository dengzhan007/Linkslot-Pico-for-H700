#!/usr/bin/env python3
"""Build a fail-closed, non-installable H700 M1 preflight-only U-Boot SPL.

The build validates source integration and the final linked SRAM/BSS layout.
It never runs the image, accesses a device, writes media, or creates a capsule.
"""
from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
UBOOT_COMMIT = "5508406582f6f7120dce0c4b3059819a41788db2"
BSS_START = 0x30000
BSS_LIMIT = 0x4000
SPL_STACK = 0x52a00


def run(command: list[str], cwd: Path | None = None) -> str:
    result = subprocess.run(command, cwd=cwd, stdout=subprocess.PIPE,
                            stderr=subprocess.STDOUT, text=True)
    if result.returncode:
        raise RuntimeError(f"command failed ({result.returncode}): {' '.join(command)}\n{result.stdout[-12000:]}")
    return result.stdout


def require(condition: bool, message: str) -> None:
    if not condition:
        raise RuntimeError(message)


def symbol_table(nm_text: str) -> dict[str, tuple[int, int | None, str]]:
    symbols: dict[str, tuple[int, int | None, str]] = {}
    for line in nm_text.splitlines():
        fields = line.split()
        if len(fields) == 3:
            address, kind, name = fields
            size = None
        elif len(fields) >= 4:
            address, size_text, kind, name = fields[:4]
            try:
                size = int(size_text, 16)
            except ValueError:
                continue
        else:
            continue
        try:
            value = int(address, 16)
        except ValueError:
            continue
        symbols[name] = (value, size, kind)
    return symbols


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--uboot-root", type=Path, required=True)
    ap.add_argument("--output", type=Path, required=True)
    ap.add_argument("--cross-compile", default="aarch64-linux-gnu-")
    ap.add_argument("--jobs", type=int, default=2)
    args = ap.parse_args()

    uboot = args.uboot_root.resolve()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=True)
    head = run(["git", "-C", str(uboot), "rev-parse", "HEAD"]).strip()
    require(head == UBOOT_COMMIT, f"U-Boot pin mismatch: expected {UBOOT_COMMIT}, got {head}")

    mach = uboot / "arch/arm/mach-sunxi"
    repo_files = {
        "baseos_m1_preflight.c": ROOT / "platform/h700/boot/baseos_m1_preflight.c",
        "m1_firmware_readonly.c": ROOT / "platform/h700/boot/m1_firmware_readonly.c",
        "m1_firmware_readonly.h": ROOT / "platform/h700/boot/m1_firmware_readonly.h",
        "m1_firmware_spl_adapter.c": ROOT / "platform/h700/boot/m1_firmware_spl_adapter.c",
        "m1_gpt_spl_adapter.c": ROOT / "platform/h700/boot/m1_gpt_spl_adapter.c",
        "m1_gpt_spl_adapter.h": ROOT / "platform/h700/boot/m1_gpt_spl_adapter.h",
        "h700_gpt_readonly.c": ROOT / "platform/h700/storage/gpt/h700_gpt_readonly.c",
        "h700_gpt_readonly.h": ROOT / "platform/h700/storage/gpt/h700_gpt_readonly.h",
    }
    for name, source in repo_files.items():
        require(source.is_file(), f"missing M1 source: {source}")
        shutil.copy2(source, mach / name)

    makefile = mach / "Makefile"
    make_text = makefile.read_text()
    marker = "obj-$(CONFIG_DRAM_SUN50I_H616)\t+= dram_timings/\n"
    require(marker in make_text, "unexpected sunxi SPL Makefile layout")
    objects = (
        "obj-y\t+= baseos_m1_preflight.o\n"
        "obj-y\t+= m1_firmware_readonly.o m1_firmware_spl_adapter.o\n"
        "obj-y\t+= m1_gpt_spl_adapter.o h700_gpt_readonly.o\n"
    )
    require("baseos_m1_preflight.o" not in make_text, "M1 overlay already applied")
    makefile.write_text(make_text.replace(marker, marker + objects))

    board_file = mach / "board.c"
    board_text = board_file.read_text()
    start = board_text.index("void board_init_f(ulong dummy)")
    end_marker = "\n}\n#endif /* CONFIG_XPL_BUILD */"
    end = board_text.index(end_marker, start) + 2
    replacement = """void board_init_f(ulong dummy)
{
    extern char __bss_start[], __bss_end[];
    extern void baseos_early_mmc(void);

    /* Clear the separate SRAM-C BSS before spl_early_init and the M1 hook. */
    memset(__bss_start, 0, __bss_end - __bss_start);
    sunxi_sram_init();
    tzpc_init();
    timer_init();
    clock_init();
    spl_early_init();

    /* Non-returning preflight harness; no GPIO console, I2C, PMIC or DRAM path. */
    baseos_early_mmc();
}
"""
    board_file.write_text(board_text[:start] + replacement.rstrip() + board_text[end:])

    cross = args.cross_compile
    run(["make", f"O={out}", f"CROSS_COMPILE={cross}",
         "anbernic_rg35xx_h700_defconfig"], cwd=uboot)
    config = out / ".config"
    scripts_config = str(uboot / "scripts/config")
    # The stock board defconfig is a full-featured vendor SPL. This isolated
    # preflight only needs DM-backed read-only MMC + the FDT needed to bind the
    # controller. Drop unrelated command/FIT/PMIC paths so their full-U-Boot
    # objects cannot leak into this SPL link.
    run([scripts_config, "--file", str(config),
         "--set-val", "SPL_BSS_START_ADDR", hex(BSS_START),
         "--set-val", "SPL_BSS_MAX_SIZE", hex(BSS_LIMIT),
         "--enable", "SPL_DM",
         "--enable", "DM_MMC",
         "--enable", "SPL_DM_MMC",
         "--enable", "BLK",
         "--enable", "SPL_BLK",
         "--enable", "SPL_OF_CONTROL",
         "--enable", "SPL_OF_REAL",
         "--disable", "SPL_MMC_WRITE",
         "--disable", "SPL_EFI_PARTITION",
         "--disable", "EFI_PARTITION",
         "--enable", "SPL_LIBCOMMON_SUPPORT",
         "--enable", "SPL_ENV_SUPPORT",
         "--enable", "SPL_CYCLIC",
         "--disable", "SPL_ENV_SUPPORT",
         "--enable", "SPL_SYS_MALLOC",
         "--set-val", "SPL_SYS_MALLOC_SIZE", "0x10000",
         "--disable", "SPL_SYS_MALLOC_SIMPLE",
         "--disable", "SPL_USE_TINY_PRINTF",
         "--disable", "SPL_CMDLINE_EDITING",
         "--disable", "CMDLINE",
         "--disable", "SPL_FIT_SIGNATURE",
         "--disable", "SPL_FIT_IMAGE_TINY",
         "--disable", "SPL_FIT_PRINT",
         "--disable", "FIT_SIGNATURE",
         "--disable", "FIT",
         "--disable", "SPL_FIT",
         "--disable", "SPL_LOAD_FIT",
         "--disable", "SPL_SERIAL",
         "--disable", "SPL_I2C",
         "--disable", "SPL_POWER",
         "--disable", "SPL_SUNXI_LED_STATUS",
         "--disable", "SPL_DM_I2C",
         "--disable", "SPL_DM_PMIC",
         "--disable", "SPL_DM_REGULATOR",
         "--disable", "SPL_DM_SERIAL",
         "--disable", "SPL_DM_USB",
         "--disable", "R_I2C_ENABLE",
         "--disable", "REGULATOR_AXP",
         "--disable", "AXP717_POWER",
         "--disable", "CMD_BOOTD",
         "--disable", "VERSION_VARIABLE",
         ], cwd=uboot)
    # ARMv8 Kconfig force-selects SPL_FIT even for this read-only preflight.
    # The harness neither loads nor parses FIT images, so remove that one
    # architecture-wide selector in the disposable source checkout only.
    armv8_kconfig = uboot / "arch/arm/cpu/armv8/Kconfig"
    armv8_text = armv8_kconfig.read_text()
    fit_selector = "\tselect SPL_FIT\n"
    require(armv8_text.count(fit_selector) == 1,
            "unexpected ARMv8 SPL_FIT selector count")
    armv8_kconfig.write_text(armv8_text.replace(fit_selector, "", 1))

    # SPL_PRINTF is a hidden Kconfig symbol with no prompt/default; scripts/config
    # cannot enable it directly. SPL_SERIAL normally selects it, but serial is
    # deliberately disabled for this preflight. Select it from the already
    # required SPL_LIBCOMMON_SUPPORT parent in this disposable checkout only.
    spl_kconfig = uboot / "common/spl/Kconfig"
    spl_kconfig_text = spl_kconfig.read_text()
    spl_common_block = "config SPL_LIBCOMMON_SUPPORT\\n\\tbool \\\"Support common libraries\\\"\\n"
    require(spl_kconfig_text.count(spl_common_block) == 1,
            "unexpected SPL_LIBCOMMON_SUPPORT Kconfig definition")
    spl_kconfig.write_text(spl_kconfig_text.replace(
        spl_common_block,
        spl_common_block + "\\tselect SPL_PRINTF\\n", 1))

    # ARCH_SUNXI force-selects the tiny allocator/printf profile for SPL.
    # This preflight needs dlmalloc realloc and full snprintf for DM/FDT helpers.
    arm_kconfig = uboot / "arch/arm/Kconfig"
    arm_text = arm_kconfig.read_text()
    arch_start = arm_text.index("config ARCH_SUNXI\n")
    arch_end = arm_text.find("\nconfig ", arch_start + 1)
    require(arch_end > arch_start, "cannot isolate ARCH_SUNXI Kconfig block")
    arch_block = arm_text[arch_start:arch_end]
    for selector in ("\tselect SPL_SYS_MALLOC_SIMPLE if SPL\n",
                     "\tselect SPL_USE_TINY_PRINTF if SPL\n"):
        require(arch_block.count(selector) == 1,
                f"unexpected ARCH_SUNXI SPL allocator/printf selector: {selector.strip()}")
        arch_block = arch_block.replace(selector, "", 1)
    arm_kconfig.write_text(arm_text[:arch_start] + arch_block + arm_text[arch_end:])

    # The H616 SoC-family Kconfig also selects FIT and SPL_LOAD_FIT. The
    # M1 preflight reads a fixed raw p4+40MiB stream and does not load FIT.
    sunxi_kconfig = uboot / "arch/arm/mach-sunxi/Kconfig"
    sunxi_text = sunxi_kconfig.read_text()
    family_start = sunxi_text.index("config SUN50I_GEN_H6\n")
    family_end = sunxi_text.find("\nconfig ", family_start + 1)
    require(family_end > family_start, "cannot isolate SUN50I_GEN_H6 Kconfig block")
    family_block = sunxi_text[family_start:family_end]
    for selector in ("\tselect FIT\n", "\tselect SPL_LOAD_FIT if SPL\n"):
        require(family_block.count(selector) == 1,
                f"unexpected H616 FIT selector: {selector.strip()}")
        family_block = family_block.replace(selector, "", 1)
    sunxi_kconfig.write_text(sunxi_text[:family_start] + family_block + sunxi_text[family_end:])

    # This isolated M1 hook is a fixed read-only MMC/GPT/FDT path, not a
    # command-capable SPL. Upstream common/Makefile unconditionally adds
    # stdio.o, cli.o, command.o and s_record.o even in XPL builds; those
    # objects pull serial/I2C/console input providers that this profile
    # deliberately excludes. Remove only these objects in the disposable
    # U-Boot checkout used by this preflight build.
    common_makefile = uboot / "common/Makefile"
    common_text = common_makefile.read_text()
    for object_name in ("stdio.o", "cli.o", "command.o", "s_record.o"):
        object_line = f"obj-y += {object_name}\n"
        require(common_text.count(object_line) == 1,
                f"unexpected common/Makefile object rule: {object_name}")
        common_text = common_text.replace(
            object_line,
            "ifndef CONFIG_XPL_BUILD\n" + object_line + "endif\n", 1)
    common_makefile.write_text(common_text)

    # Keep general-purpose helpers in U-Boot proper, but omit image-fdt from
    # the isolated XPL link: this path never relocates a FIT/image and does not
    # need LMB. The FDT support functions used by DM remain linked.
    boot_makefile = uboot / "boot/Makefile"
    boot_text = boot_makefile.read_text()
    image_fdt_rule = "obj-$(CONFIG_$(PHASE_)OF_LIBFDT) += image-fdt.o\n"
    require(boot_text.count(image_fdt_rule) == 1,
            "unexpected U-Boot image-fdt object rule")
    boot_text = boot_text.replace(
        image_fdt_rule,
        "ifneq ($(CONFIG_XPL_BUILD),y)\n" + image_fdt_rule + "endif\n",
        1,
    )
    boot_makefile.write_text(boot_text)

    # display_options is a full-U-Boot banner/hexdump helper and pulls in
    # version_string/sprintf; the M1 SPL path has no console/banner output.
    lib_makefile = uboot / "lib/Makefile"
    lib_text = lib_makefile.read_text()
    display_rule = "obj-y += display_options.o\n"
    require(lib_text.count(display_rule) == 1,
            "unexpected U-Boot display_options object rule")
    lib_text = lib_text.replace(
        display_rule,
        "ifneq ($(CONFIG_XPL_BUILD),y)\n" + display_rule + "endif\n",
        1,
    )
    lib_makefile.write_text(lib_text)

    # OF_REAL's string helpers use these section bounds; the upstream SPL
    # linker script does not currently publish them.
    spl_lds = uboot / "arch/arm/cpu/armv8/u-boot-spl.lds"
    lds_text = spl_lds.read_text()
    rodata_rule = "\t.rodata : {\n\t\t. = ALIGN(8);\n"
    require(lds_text.count(rodata_rule) == 1,
            "unexpected AArch64 SPL rodata section")
    lds_text = lds_text.replace(
        rodata_rule,
        "\t.rodata : {\n\t\t__start_rodata = .;\n\t\t. = ALIGN(8);\n",
        1,
    )
    rodata_end = "\t\t*(SORT_BY_ALIGNMENT(SORT_BY_NAME(.rodata*)))\n\t} >.sram"
    require(lds_text.count(rodata_end) == 1,
            "unexpected AArch64 SPL rodata end")
    lds_text = lds_text.replace(
        rodata_end,
        "\t\t*(SORT_BY_ALIGNMENT(SORT_BY_NAME(.rodata*)))\n\t\t__end_rodata = .;\n\t} >.sram",
        1,
    )
    spl_lds.write_text(lds_text)

    run(["make", f"O={out}", f"CROSS_COMPILE={cross}", "olddefconfig"], cwd=uboot)
    config_text = config.read_text()
    # Preserve the resolved Kconfig result in the artifact even when a
    # fail-closed contract assertion stops the build.
    (out / "m1-spl-preflight-resolved.config").write_text(config_text)
    relevant_config = "\n".join(
        line for line in config_text.splitlines()
        if any(key in line for key in ("SPL_FIT", "SPL_LOAD_FIT", "FIT_SIGNATURE",
                                       "SPL_OF_CONTROL", "SPL_OF_REAL", "SPL_DM",
                                       "SPL_DM_MMC", "SPL_BLK", "SPL_LIBCOMMON"))
    )
    for token in (
        "CONFIG_ARM64=y",
        "CONFIG_MACH_SUN50I_H616=y",
        "CONFIG_SPL_SEPARATE_BSS=y",
        "CONFIG_SPL_BSS_START_ADDR=0x30000",
        "CONFIG_SPL_BSS_MAX_SIZE=0x4000",
        "CONFIG_SPL_DM=y",
        "CONFIG_DM_MMC=y",
        "CONFIG_SPL_DM_MMC=y",
        "CONFIG_BLK=y",
        "CONFIG_SPL_BLK=y",
        "CONFIG_SPL_OF_CONTROL=y",
        "CONFIG_SPL_OF_REAL=y",
        "# CONFIG_SPL_MMC_WRITE is not set",
        "# CONFIG_SPL_EFI_PARTITION is not set",
        "# CONFIG_SPL_I2C is not set",
        "CONFIG_SPL_LIBCOMMON_SUPPORT=y",
        "# CONFIG_SPL_ENV_SUPPORT is not set",
        "CONFIG_SPL_PRINTF=y",
        "CONFIG_SPL_SPRINTF=y",
        "CONFIG_SPL_CYCLIC=y",
        "# CONFIG_SPL_SYS_MALLOC_SIMPLE is not set",
        "# CONFIG_SPL_USE_TINY_PRINTF is not set",
        "CONFIG_SPL_SYS_MALLOC=y",
        "CONFIG_SPL_SYS_MALLOC_SIZE=0x10000",
        "CONFIG_SPL_SYS_MALLOC_F=y",
        "CONFIG_SPL_SYS_MALLOC_F_LEN=0x2000",
        "# CONFIG_SPL_POWER is not set",
        "# CONFIG_SPL_DM_I2C is not set",
        "# CONFIG_SPL_DM_PMIC is not set",
        "# CONFIG_SPL_DM_SERIAL is not set",
        "# CONFIG_SPL_DM_USB is not set",
        "# CONFIG_CMD_BOOTD is not set",
    ):
        require(token in config_text,
                f"resolved SPL config missing contract: {token}\n{relevant_config}")

    require("CONFIG_SPL_FIT=y" not in config_text and
            "CONFIG_SPL_LOAD_FIT=y" not in config_text,
            "resolved M1 preflight profile unexpectedly enables FIT loading")

    require("CONFIG_SPL_FIT=y" not in config_text and
            "CONFIG_SPL_LOAD_FIT=y" not in config_text,
            "resolved M1 preflight profile unexpectedly enables FIT loading")
    require("CONFIG_OF_REAL=y" in config_text,
            "SPL_OF_CONTROL must retain its OF_CONTROL-selected OF_REAL dependency")

    require("CONFIG_SPL_SYS_MALLOC=y" in config_text and
            "CONFIG_SPL_SYS_MALLOC_SIZE=0x10000" in config_text,
            "M1 preflight requires a bounded 64KiB SRAM-C dlmalloc pool")
    map_file = out / "spl/u-boot-spl.map"
    map_arg = f"-Map={map_file}"
    build_log = run([
        "make", f"O={out}", f"CROSS_COMPILE={cross}",
        f"LDFLAGS_u-boot-spl={map_arg}", f"-j{args.jobs}",
        "spl/u-boot-spl.bin",
    ], cwd=uboot)
    elf = out / "spl/u-boot-spl"
    raw = out / "spl/u-boot-spl.bin"
    require(elf.is_file() and raw.is_file(), "SPL ELF/raw output missing")
    require(map_file.is_file(), "SPL linker map missing; refusing to infer layout from source alone")

    nm = run([f"{cross}nm", "-S", "-n", str(elf)])
    symbols = symbol_table(nm)
    for name in ("__bss_start", "__bss_end", "m1_package_scratch",
                 "m1_boot_partitions", "baseos_early_mmc",
                 "h700_m1_spl_preflight_gpt"):
        require(name in symbols, f"linked SPL symbol missing: {name}")

    bss_start, _, _ = symbols["__bss_start"]
    bss_end, _, _ = symbols["__bss_end"]
    scratch_addr, scratch_size, _ = symbols["m1_package_scratch"]
    parts_addr, parts_size, _ = symbols["m1_boot_partitions"]
    require(bss_start == BSS_START, f"linked BSS start changed: {bss_start:#x}")
    require(bss_start <= scratch_addr and scratch_addr + (scratch_size or 0) <= bss_end,
            "4KiB scratch is outside linked BSS")
    require(scratch_size == 4096, f"scratch symbol size mismatch: {scratch_size}")
    require(scratch_addr % 64 == 0, "scratch symbol is not 64-byte aligned")
    require(parts_size is not None and bss_start <= parts_addr < parts_addr + parts_size <= bss_end,
            "bounded GPT table is outside linked BSS")
    require(bss_end <= BSS_START + BSS_LIMIT,
            f"linked BSS exceeds 16KiB ceiling: {bss_end - bss_start:#x}")
    require(bss_end < SPL_STACK, "linked BSS overlaps the configured initial SPL stack")

    all_symbols = "\n".join(symbols.keys())
    forbidden = (
        "blk_dwrite", "blk_write", "blk_erase", "mmc_write", "mmc_erase",
        "i2c_write", "i2c_read", "pmic_bus_write", "sunxi_board_init",
        "preloader_console_init", "gpio_init",
    )
    present = [name for name in forbidden if name in all_symbols]
    require(not present, f"forbidden/unexpected linked symbols present: {present}")

    nm_path = out / "m1-spl-preflight.nm.txt"
    nm_path.write_text(nm)
    disassembly = run([f"{cross}objdump", "-d", str(elf)])
    (out / "m1-spl-preflight.disasm.txt").write_text(disassembly)
    shutil.copy2(elf, out / "m1-spl-preflight.elf")
    shutil.copy2(raw, out / "m1-spl-preflight.bin")
    shutil.copy2(map_file, out / "m1-spl-preflight.map")
    (out / "m1-spl-build.log").write_text(build_log)

    manifest = {
        "kind": "M1 preflight-only H700 SPL link audit",
        "uboot_tag": "v2026.10",
        "uboot_commit": head,
        "source_variant": "isolated board_init_f overlay; no sunxi_board_init continuation",
        "elf_bytes": elf.stat().st_size,
        "raw_bytes": raw.stat().st_size,
        "bss": {"start": hex(bss_start), "end_exclusive": hex(bss_end),
                "used_bytes": bss_end - bss_start, "limit_bytes": BSS_LIMIT},
        "m1_package_scratch": {"address": hex(scratch_addr), "bytes": scratch_size,
                               "alignment": 64, "inside_bss": True},
        "m1_gpt_partitions": {"address": hex(parts_addr), "bytes": parts_size,
                              "entry_slots": 8, "inside_bss": True},
        "spl_stack_pointer": hex(SPL_STACK),
        "bss_end_to_stack_gap_bytes": SPL_STACK - bss_end,
        "sram_c": {"start": "0x28000", "end_exclusive": "0x58000"},
        "spl_malloc_pool_to_stack_gap_bytes": SPL_STACK - (BSS_START + BSS_LIMIT + 0x10000),
        "linked_map": str(map_file.name),
        "forbidden_symbols_absent": forbidden,
        "test_fingerprint": "0x00000000 (deliberate fail-closed sentinel)",
        "spl_allocator_profile": {
            "allocator": "U-Boot SPL dlmalloc",
            "simple_malloc_disabled": "# CONFIG_SPL_SYS_MALLOC_SIMPLE is not set" in config_text,
            "early_malloc_f_enabled": "CONFIG_SPL_SYS_MALLOC_F=y" in config_text,
            "early_malloc_f_bytes": "0x2000",
            "tiny_printf_disabled": "# CONFIG_SPL_USE_TINY_PRINTF is not set" in config_text,
            "heap_start": hex(BSS_START + BSS_LIMIT),
            "heap_bytes": 0x10000,
            "heap_end_exclusive": hex(BSS_START + BSS_LIMIT + 0x10000),
            "heap_high_water_measured": False,
        },
        "firmware_payload_read_at_runtime": False,
        "device_io_performed": False,
        "media_write_performed": False,
        "firmware_flash_performed": False,
        "payload_executed": False,
        "hardware_qualified": False,
        "installable": False,
        "candidate_eligible": False,
        "limitations": [
            "This is a compile/link-only preflight harness, not a complete M1 boot path.",
            "The isolated profile disables ARCH_SUNXI's simple-malloc/tiny-printf selectors and uses a bounded 64KiB SPL dlmalloc pool immediately after the reserved 16KiB BSS window in SRAM C; SPL_CMDLINE is not a symbol in this pinned tree.",
            "The 64KiB heap range is statically separated from the configured initial stack, but allocator high-water usage is not measured; this is not a complete runtime memory-usage proof.",
            "The zero FNV sentinel makes the package planner reject before firmware payload reads.",
            "The harness still contains pre-MMC SRAM/TZPC/timer/clock setup and the MMC controller read path; no device execution is performed.",
            "Absence of named forbidden functions is not a complete MMIO/SMC instruction audit; inspect the map and disassembly before any hardware authorization.",
            "No DRAM training, PMIC/TWI, display GPIO, vendor monitor handoff, BL33 launch, suspend or wake is performed."
        ]
    }
    manifest_path = out / "m1-spl-preflight-manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n")
    print(json.dumps(manifest, indent=2))
    print("PASS actual U-Boot SPL link contains the aligned 4KiB M1 scratch and bounded GPT table")
    print("PASS BSS size is within the configured 16KiB SRAM-C window")
    print("PASS write/PMIC/TWI/vendor-handoff symbols are absent from this preflight-only ELF")
    print("LIMIT build/link evidence only; non-installable, hardware-unvalidated, candidate_eligible=false")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, subprocess.SubprocessError, ValueError) as error:
        raise SystemExit(f"FAIL: {error}")
