#!/usr/bin/env python3
"""Audit the pinned BaseOS SPL/capsule side effects relevant to M1 one-shot.

This is a host-only source contract audit. It does not build, run, flash, or
access a device. It deliberately distinguishes the ordinary, raw-capsule, and
native-handoff code paths; a source grep is not a substitute for final ELF
instruction-level auditing of any future M1 implementation.
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

BASEOS_PIN = "cbe253a43f46cf551cc9a6c79a84015bde3686cf"

EARLY_MMC = Path("experiments/h700-boot-speed/spl/early-mmc.c")
SPL_BUILDER = Path("experiments/h700-boot-speed/spl/build-early-mmc.py")
NATIVE_BOOT = Path("experiments/h700-boot-speed/spl/native-boot.c")
CAPSULE = Path("experiments/h700-boot-speed/native/capsule/capsule.c")
CAPSULE_BUILDER = Path("experiments/h700-boot-speed/native/capsule/build.py")


def git_head(root: Path) -> str:
    return subprocess.run(
        ["git", "-C", str(root), "rev-parse", "HEAD"],
        check=True, capture_output=True, text=True,
    ).stdout.strip()


def source(root: Path, path: Path) -> str:
    full = root / path
    if not full.is_file():
        raise RuntimeError(f"pinned BaseOS source is missing: {path}")
    return full.read_text(encoding="utf-8")


def require(text: str, token: str, label: str) -> None:
    if token not in text:
        raise RuntimeError(f"pinned BaseOS source contract changed: missing {label}: {token}")


def reject(text: str, token: str, label: str) -> None:
    if token in text:
        raise RuntimeError(f"unexpected {label} in source section: {token}")


def audit(root: Path) -> dict:
    head = git_head(root)
    if head != BASEOS_PIN:
        raise RuntimeError(f"BaseOS source must be pinned at {BASEOS_PIN}; found {head}")

    early = source(root, EARLY_MMC)
    builder = source(root, SPL_BUILDER)
    native = source(root, NATIVE_BOOT)
    capsule = source(root, CAPSULE)
    capsule_builder = source(root, CAPSULE_BUILDER)

    if "#if defined(BASEOS_RAW_CAPSULE)" not in early or "#else" not in early:
        raise RuntimeError("early-mmc.c no longer has the audited raw/ordinary split")
    raw = early.split("#if defined(BASEOS_RAW_CAPSULE)", 1)[1].split("#else", 1)[0]
    ordinary = early.split("#else", 1)[1].split("#endif /* BASEOS_RAW_CAPSULE */", 1)[0]

    for token, label in (
        ("#define RESULT_SECTOR 0x56000", "ordinary result-sector definition"),
        ("#define SOURCE_SECTOR (RESULT_SECTOR + 1)", "ordinary stock-source sector"),
        ("#define MIRROR_SECTOR 0x56200", "ordinary Boot0 mirror sector"),
        ("blk_dwrite(card, RESULT_SECTOR", "ordinary result-sector write"),
        ("blk_dwrite(card, destination", "ordinary Boot0 mirror write"),
        ("writel(1, (void *)0x030090b4)", "ordinary watchdog configuration write"),
        ("writel(0x31, (void *)0x030090b8)", "ordinary watchdog mode write"),
        ("writel(0x14af, (void *)0x030090b0)", "ordinary watchdog control write"),
    ):
        require(ordinary, token, label)

    for token, label in (
        ("baseos_raw_adopt();", "raw SPL adoption call"),
        ("0x07000138ul", "raw GP14 read address"),
        ("0x0700013cul", "raw GP15 read address"),
        ("0x030090b4ul", "raw watchdog CFG read address"),
        ("0x030090b8ul", "raw watchdog MODE read address"),
        ("mmc_initialize(NULL)", "raw MMC initialization"),
    ):
        require(raw, token, label)
    reject(raw, "blk_dwrite(", "raw SPL block-device write")
    reject(raw, "attempt_commit(", "raw SPL persistent attempt commit")

    for token, label in (
        ('"--disable SPL_MMC_WRITE"', "raw SPL MMC-write disable"),
        ('"--raw-capsule"', "raw-capsule builder option"),
    ):
        require(builder, token, label)

    for token, label in (
        ("#define GP14", "outer GP14 address"),
        ("#define GP15", "outer GP15 address"),
        ("#define RETENTION", "outer retention address"),
        ("#define WDT_CTRL", "outer watchdog control address"),
        ("#define WDT_CFG", "outer watchdog config address"),
        ("#define WDT_MODE", "outer watchdog mode address"),
        ("attempt_commit(&io, CAPSULE_COOKIE)", "outer RTC attempt-pair commit"),
        ("arm_watchdog()", "outer watchdog arm"),
        ("read32(RETENTION)", "outer retention read"),
    ):
        require(capsule, token, label)

    for token, label in (
        ("pmic_bus_read(0xf0", "native PMIC scratch read"),
        ("pmic_bus_write(0xf0, 0)", "conditional native PMIC scratch write"),
        ("writel(readl(reg) | BIT(31), reg)", "native CCU PLL gate writes"),
        ("0x0300b124", "native PI15/PIO access"),
        ("0x0300b130", "native PI14/PIO access"),
        ("0x0300b090", "native PE6/PIO access"),
        ("writel(0x14af, (void *)0x030090b0)", "native watchdog reload"),
        ("msr rmr_el3", "native EL3 reset handoff"),
    ):
        require(native, token, label)

    # The current pinned tree has no separate no-state M1 mode. Fail if a future
    # edit introduces one without updating this audit and its contract.
    m1_mode_sources = {
        str(EARLY_MMC): early,
        str(SPL_BUILDER): builder,
        str(NATIVE_BOOT): native,
        str(CAPSULE): capsule,
        str(CAPSULE_BUILDER): capsule_builder,
    }
    unexpected = [path for path, body in m1_mode_sources.items() if "BASEOS_M1_ONESHOT" in body]
    if unexpected:
        raise RuntimeError(
            "BASEOS_M1_ONESHOT appeared in the pinned baseline; review and update "
            "this audit before treating it as a supported implementation: " + ", ".join(unexpected)
        )

    return {
        "audit": "M1 BaseOS one-shot baseline side-effect audit",
        "baseos_commit": head,
        "host_only": True,
        "device_io_performed": False,
        "hardware_qualified": False,
        "durable_install_eligible": False,
        "ordinary_spl": {
            "block_device_writes": True,
            "result_sector_lba": "0x56000",
            "stock_boot0_source_lba": "0x56001",
            "default_boot0_mirror_lba": "0x56200",
            "watchdog_programming": True,
        },
        "raw_capsule_spl": {
            "block_device_writes_in_early_mmc": False,
            "requires_outer_rtc_gp14_gp15": True,
            "requires_outer_watchdog_cfg_mode": True,
            "watchdog_reads": True,
            "persistent_attempt_commit_in_spl": False,
            "builder_disables_spl_mmc_write": True,
        },
        "raw_capsule_outer": {
            "rtc_attempt_pair_commit": True,
            "watchdog_arm": True,
            "retention_read": True,
        },
        "native_handoff": {
            "pmic_access": True,
            "conditional_pmic_write": True,
            "ccu_pll_gate_writes": True,
            "display_pio_writes": True,
            "watchdog_reload": True,
            "el3_reset_handoff": True,
        },
        "m1_one_shot_mode_present": False,
        "disposition": "Do not reuse ordinary or raw-capsule v2 unchanged for M1. A separate outer/SPL/native-handoff implementation is required.",
        "limits": [
            "Source contract audit only; not an instruction-level audit of a newly linked M1 ELF.",
            "Does not prove a given conditional branch executes on the target device.",
            "Does not approve MMIO, DRAM, RTC, watchdog, PMIC, SD writes, or hardware execution.",
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseos-root", type=Path, required=True,
                        help="clean BaseOS-MultiFrontend checkout at the pinned commit")
    args = parser.parse_args()
    try:
        result = audit(args.baseos_root.resolve())
    except (OSError, RuntimeError, subprocess.CalledProcessError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1
    print(json.dumps(result, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
