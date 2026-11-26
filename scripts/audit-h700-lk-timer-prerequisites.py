#!/usr/bin/env python3
"""Audit the pinned GammaOS LK early-platform and scheduler-timer contract.

This is a source-level guard only. It does not implement an H700 timer/GIC
provider, qualify the BL31 -> BL33 handoff, or certify a bootable LK image.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

LK_PIN = "9bf1419a49e0af6dba8b05916df382c9c9c6d0a8"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(f"FAIL: {message}")
    print(f"PASS: {message}")


def read_text(root: Path, relative: str) -> str:
    path = root / relative
    if not path.is_file():
        raise SystemExit(f"FAIL: missing pinned source file: {path}")
    return path.read_text(encoding="utf-8", errors="strict")


def ordered(text: str, tokens: list[str], label: str) -> None:
    positions = []
    for token in tokens:
        position = text.find(token)
        if position < 0:
            raise SystemExit(f"FAIL: {label}: missing {token!r}")
        positions.append(position)
    if positions != sorted(positions) or len(set(positions)) != len(positions):
        raise SystemExit(f"FAIL: {label}: unexpected ordering for {tokens!r}")
    print(f"PASS: {label}")


def function_body(source: str, signature: str, label: str) -> str:
    start = source.find(signature)
    if start < 0:
        raise SystemExit(f"FAIL: {label}: function signature not found: {signature}")
    brace = source.find("{", start)
    if brace < 0:
        raise SystemExit(f"FAIL: {label}: function body not found")
    depth = 0
    for index in range(brace, len(source)):
        char = source[index]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return source[brace + 1:index]
    raise SystemExit(f"FAIL: {label}: unterminated function body")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lk-source", required=True, type=Path)
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=Path(__file__).resolve().parents[1],
    )
    args = parser.parse_args()

    lk_root = args.lk_source.resolve()
    repo_root = args.repo_root.resolve()
    if not (lk_root / ".git").exists():
        raise SystemExit(f"FAIL: {lk_root} is not a Git checkout")

    try:
        actual_pin = subprocess.run(
            ["git", "-C", str(lk_root), "rev-parse", "HEAD"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
    except (OSError, subprocess.CalledProcessError) as exc:
        raise SystemExit(f"FAIL: cannot identify LK checkout: {exc}") from exc
    require(actual_pin == LK_PIN, f"GammaOS LK source is pinned at {LK_PIN}")

    main_c = read_text(lk_root, "kernel/main.c")
    ordered(
        main_c,
        [
            "platform_early_init();",
            "target_early_init();",
            "heap_init();",
            "thread_init();",
            "dpc_init();",
            "timer_init();",
        ],
        "platform_early_init precedes target, heap, thread, DPC and kernel timer startup",
    )

    bootstrap = function_body(
        main_c, "static int bootstrap2(void *arg)", "bootstrap2"
    )
    ordered(
        bootstrap,
        ["platform_init();", "target_init();", "apps_init();"],
        "platform_init and target_init occur after timer_init in bootstrap2",
    )

    timer_c = read_text(lk_root, "kernel/timer.c")
    timer_init = function_body(timer_c, "void timer_init(void)", "timer_init")
    require(
        "platform_set_periodic_timer(timer_tick, NULL, 10)" in timer_init,
        "ordinary LK build registers a 10 ms scheduler tick",
    )
    require(
        "platform_set_periodic_timer(timer_tick, NULL, 1)" in timer_init,
        "specific Konkr configuration registers a 1 ms scheduler tick",
    )

    timer_header = read_text(lk_root, "include/platform/timer.h")
    platform_header = read_text(lk_root, "include/platform.h")
    for symbol in (
        "platform_set_periodic_timer",
        "mdelay",
        "udelay",
        "platform_tick_rate",
    ):
        require(symbol in timer_header, f"LK platform timer API declares {symbol}")
    for symbol in ("current_time(void)", "current_time_hires(void)"):
        require(symbol in platform_header, f"LK platform time API declares {symbol}")

    generic_init = read_text(lk_root, "platform/init.c")
    weak_early_init = function_body(
        generic_init,
        "__WEAK void platform_early_init(void)",
        "generic weak platform_early_init",
    )
    require(
        not weak_early_init.strip(),
        "generic platform_early_init fallback is empty",
    )
    platform_rules = read_text(lk_root, "platform/rules.mk")
    require(
        "$(LOCAL_DIR)/init.o" in platform_rules,
        "generic platform lifecycle object is part of the platform rules",
    )

    mtk_platform = read_text(lk_root, "platform/mt6785/platform.c")
    mtk_early_init = function_body(
        mtk_platform,
        "void platform_early_init(void)",
        "MT6785 platform_early_init",
    )
    ordered(
        mtk_early_init,
        ["platform_init_interrupts();", "platform_early_init_timer();"],
        "MT6785 early init prepares interrupts before its early timer",
    )

    mtk_timer = read_text(lk_root, "platform/mt6785/timer.c")
    mtk_early_timer = function_body(
        mtk_timer,
        "void platform_early_init_timer(void)",
        "MT6785 platform_early_init_timer",
    )
    require(
        "APXGPT_BASE" in mtk_early_timer and "0x31" in mtk_early_timer,
        "MT6785 early delay timebase is its GPT4 free-running counter",
    )
    mtk_periodic = function_body(
        mtk_timer,
        "status_t platform_set_periodic_timer(",
        "MT6785 platform_set_periodic_timer",
    )
    require(
        "GPT5_COMPARE_REG" in mtk_periodic
        and "mt_irq_unmask(MT_GPT_IRQ_ID)" in mtk_periodic,
        "MT6785 scheduler tick uses GPT5 and the MTK interrupt path",
    )

    gic_v2 = read_text(lk_root, "dev/gic/mt_gic_v2.c")
    for token in (
        "platform/mt_reg_base.h",
        "platform/mt_irq.h",
        "mt_irq_get_target()",
        "GIC_DIST_BASE",
        "GIC_CPU_BASE",
    ):
        require(token in gic_v2, f"MTK GICv2 source depends on {token}")
    gic_rules = read_text(lk_root, "dev/gic/rules.mk")
    require(
        "MTK_GIC_VERSION" in gic_rules,
        "pinned GIC driver selection is controlled by MTK_GIC_VERSION",
    )

    overlay = read_text(repo_root, "scripts/apply-h700-lk-overlay.py")
    require(
        "$(LOCAL_DIR)/gic" in overlay,
        "H700 overlay explicitly excludes the MTK dev/gic module",
    )

    print(
        "BLOCKER: this audit verifies the upstream startup contract only. "
        "H700 still needs a board-qualified early platform/GIC/time provider "
        "before its LK scheduler timer can be considered implemented."
    )
    print(
        "SAFETY: no hardware registers are read or written by this script; "
        "it does not infer a H700 PPI, GIC base, or BL31 -> BL33 access state."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
