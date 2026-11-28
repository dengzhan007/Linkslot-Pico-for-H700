#!/usr/bin/env python3
"""Build a fail-closed, non-installable H700 M1 preflight-only U-Boot SPL.

The build validates source integration and the final linked SRAM/BSS layout.
It never runs the image, accesses a device, writes media, or creates a capsule.
"""
from __future__ import annotations

import argparse
import json
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
    run([str(uboot / "scripts/config"), "--file", str(config),
         "--set-val", "SPL_BSS_START_ADDR", hex(BSS_START),
         "--set-val", "SPL_BSS_MAX_SIZE", hex(BSS_LIMIT),
         "--disable", "SPL_MMC_WRITE",
         "--disable", "SPL_EFI_PARTITION",
         "--disable", "EFI_PARTITION"], cwd=uboot)
    run(["make", f"O={out}", f"CROSS_COMPILE={cross}", "olddefconfig"], cwd=uboot)
    config_text = config.read_text()
    for token in (
        "CONFIG_ARM64=y",
        "CONFIG_MACH_SUN50I_H616=y",
        "CONFIG_SPL_SEPARATE_BSS=y",
        "CONFIG_SPL_BSS_START_ADDR=0x30000",
        "CONFIG_SPL_BSS_MAX_SIZE=0x4000",
        "# CONFIG_SPL_MMC_WRITE is not set",
        "# CONFIG_SPL_EFI_PARTITION is not set",
    ):
        require(token in config_text, f"resolved SPL config missing contract: {token}")

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
        "linked_map": str(map_file.name),
        "forbidden_symbols_absent": forbidden,
        "test_fingerprint": "0x00000000 (deliberate fail-closed sentinel)",
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
