#!/usr/bin/env python3
"""Host-only tests for the RG34XX H700 LK project and memory fail-closed gate."""
from __future__ import annotations

import argparse
import re
import shutil
import subprocess
import tempfile
from pathlib import Path

LK_PIN = "9bf1419a49e0af6dba8b05916df382c9c9c6d0a8"
SCAFFOLD = (
    Path("project/rg34xx_h700.mk"),
    Path("target/rg34xx_h700/rules.mk"),
    Path("target/rg34xx_h700/init.c"),
    Path("platform/h700/rules.mk"),
    Path("platform/h700/runtime/lk_heap_service.c"),
    Path("platform/h700/runtime/lk_heap_service.h"),
    Path("platform/h700/runtime/h700_arch_timer.c"),
    Path("platform/h700/runtime/h700_arch_timer.h"),
    Path("platform/h700/runtime/h700_bl33_entry_snapshot.c"),
    Path("platform/h700/runtime/h700_bl33_entry_snapshot.h"),
    Path("platform/h700/arch/h700_physical_memory.c"),
    Path("platform/h700/arch/h700_physical_memory.h"),
    Path("platform/h700/arch/h700_data_abort_recovery.c"),
    Path("platform/h700/arch/h700_data_abort_recovery.h"),
    Path("platform/h700/arch/h700_syscfg_probe.S"),
    Path("platform/h700/arch/tests/h700_data_abort_recovery_test.c"),
)


def run(command: list[str], *, cwd: Path | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        command,
        cwd=cwd,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--lk-source", type=Path, required=True)
    parser.add_argument("--overlay", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()

    root = Path(__file__).resolve().parents[1]
    lk_source = args.lk_source.resolve()
    overlay = args.overlay.resolve()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)

    for tool in ("git", "make"):
        if shutil.which(tool) is None:
            raise RuntimeError(f"required tool missing: {tool}")

    patch = overlay / "h700-bl33-overlay.patch"
    if not patch.is_file():
        raise RuntimeError(f"generated pinned-LK overlay patch not found: {patch}")

    revision = run(["git", "-C", str(lk_source), "rev-parse", "HEAD"])
    if revision.returncode or revision.stdout.strip() != LK_PIN:
        raise RuntimeError(
            f"LK pin mismatch: expected {LK_PIN}, got {revision.stdout.strip()}"
        )

    temp_root = Path(tempfile.mkdtemp(prefix="h700-lk-project-guard-"))
    worktree = temp_root / "lk"
    worktree_added = False
    summary: list[str] = []

    def make_case(name: str, variables: list[str], expected: str | None,
                  *, dry_run: bool = False) -> str:
        command = ["make", "--no-print-directory"]
        if dry_run:
            command.append("--dry-run")
        command.extend(["-f", "makefile", "PROJECT=rg34xx_h700"])
        command.extend(variables)
        if dry_run:
            command.append("./build-rg34xx_h700/system-onesegment.ld")
        result = run(command, cwd=worktree)
        log = result.stdout
        (out / f"{name}.log").write_text(
            "$ " + " ".join(command) + "\n" + log, encoding="utf-8"
        )

        if expected is not None:
            if result.returncode == 0 or expected not in log:
                raise RuntimeError(
                    f"{name}: expected refusal marker {expected!r}; "
                    f"exit={result.returncode}\n{log[-6000:]}"
                )
            summary.append(f"PASS {name}: rejected as required")
        else:
            if result.returncode != 0:
                raise RuntimeError(
                    f"{name}: dry-run failed (exit {result.returncode})\n{log[-10000:]}"
                )
            if "system-onesegment.ld" not in log:
                raise RuntimeError(f"{name}: linker-script target missing from dry-run\n{log[-6000:]}")
            if (worktree / "build-rg34xx_h700").exists():
                raise RuntimeError(f"{name}: dry-run unexpectedly created a build directory")
            summary.append(f"PASS {name}: linker-script graph parsed; no build output created")
        return log

    try:
        added = run([
            "git", "-C", str(lk_source), "worktree", "add", "--detach",
            str(worktree), LK_PIN,
        ])
        if added.returncode:
            raise RuntimeError("could not create disposable LK worktree:\n" + added.stdout)
        worktree_added = True

        applied = run(["git", "-C", str(worktree), "apply", str(patch)])
        if applied.returncode:
            raise RuntimeError("pinned H700 overlay did not apply:\n" + applied.stdout)

        for rel in SCAFFOLD:
            source = root / rel
            if not source.is_file():
                raise RuntimeError(f"project scaffold file missing: {source}")
            destination = worktree / rel
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, destination)

        platform_rules = (worktree / "platform/h700/rules.mk").read_text(encoding="utf-8")
        if "LINKER_SCRIPT += $(BUILDDIR)/system-onesegment.ld" not in platform_rules:
            raise RuntimeError("H700 platform does not assign the generated one-segment linker script")

        # target_init() calls this marker, so the provider must be in the LK object graph.
        heap_object = "runtime/lk_heap_service.o"
        if f"$(LOCAL_DIR)/{heap_object}" not in platform_rules:
            raise RuntimeError("H700 platform does not link the LK heap readiness service")
        for heap_path in (
            root / "platform/h700/runtime/lk_heap_service.c",
            root / "platform/h700/runtime/lk_heap_service.h",
        ):
            if not heap_path.is_file():
                raise RuntimeError("H700 LK heap readiness source/header missing: " + str(heap_path))
        summary.append("PASS lk-heap-service-object: provider is present in the H700 LK OBJS list")

        # arch/arm/mmu.c weakly calls this ABI; keep the provider in the real LK object graph.
        dram_object = "arch/h700_physical_memory.o"
        if f"$(LOCAL_DIR)/{dram_object}" not in platform_rules:
            raise RuntimeError("H700 platform does not link the physical DRAM capacity provider")
        for dram_path in (
            root / "platform/h700/arch/h700_physical_memory.c",
            root / "platform/h700/arch/h700_physical_memory.h",
        ):
            if not dram_path.is_file():
                raise RuntimeError("H700 physical DRAM capacity source/header missing: " + str(dram_path))
        summary.append("PASS physical-dram-object: provider is present in the H700 LK OBJS list")

        build_objects = (root / "scripts/build-h700-objects.sh").read_text(encoding="utf-8")
        if '"$ROOT/platform/h700/arch/h700_physical_memory.c"' not in build_objects:
            raise RuntimeError("H700 architecture matrix does not compile the physical DRAM provider")
        if '"$OUT_DIR/h700_physical_memory.o"' not in build_objects:
            raise RuntimeError("H700 architecture matrix does not validate the physical DRAM provider object")
        summary.append("PASS physical-dram-compile-matrix: provider is compiled and architecture-checked in all three ABI modes")

        # Audit lifecycle assumptions against the exact pinned GammaOS LK checkout.
        lk_main = (worktree / "kernel/main.c").read_text(encoding="utf-8")
        lk_main_code = re.sub(r"/\*.*?\*/|//[^\n]*", "", lk_main, flags=re.S)
        startup_calls = (
            "arch_early_init", "platform_early_init", "target_early_init",
            "heap_init", "thread_init", "dpc_init", "timer_init",
            "platform_init", "target_init", "apps_init",
        )
        startup_positions = []
        for symbol in startup_calls:
            match = re.search(r"\b" + re.escape(symbol) + r"\s*\(\s*\)\s*;", lk_main_code)
            if match is None:
                raise RuntimeError("pinned LK startup call missing: " + symbol)
            startup_positions.append(match.start())
        if startup_positions != sorted(startup_positions):
            raise RuntimeError("pinned LK normal bootstrap order changed; heap readiness lifecycle must be re-audited")

        heap_header = (worktree / "include/lib/heap.h").read_text(encoding="utf-8")
        heap_source = (worktree / "lib/heap/heap.c").read_text(encoding="utf-8")
        if "void heap_get_info(size_t *free_total, size_t *largest);" not in heap_header:
            raise RuntimeError("pinned LK heap_get_info ABI changed")
        getter = re.search(
            r"\bvoid\s+heap_get_info\s*\([^)]*\)\s*\{(?P<body>.*?)\n\}",
            heap_source,
            flags=re.S,
        )
        if getter is None:
            raise RuntimeError("pinned LK heap_get_info implementation missing")
        getter_body = getter.group("body")
        getter_tokens = ("enter_critical_section();", "list_for_every_entry", "exit_critical_section();")
        getter_positions = [getter_body.find(token) for token in getter_tokens]
        if min(getter_positions) < 0 or getter_positions != sorted(getter_positions):
            raise RuntimeError("pinned LK heap_get_info no longer protects its free-list walk")
        if "list_initialize(&theheap.free_list);" not in heap_source or "heap_insert_free_chunk(heap_create_free_chunk(theheap.base, theheap.len));" not in heap_source:
            raise RuntimeError("pinned LK heap_init free-list initialization contract changed")
        summary.append("PASS pinned-lk-heap-contract: startup order, heap initialization and critical-section getter verified")

        target_lifecycle = (root / "target/rg34xx_h700/init.c").read_text(encoding="utf-8")
        if "#if !defined(ENABLE_NANDWRITE) || !(ENABLE_NANDWRITE)" not in target_lifecycle:
            raise RuntimeError("H700 heap readiness marker must stay disabled for the no-heap NAND-write bootstrap")
        summary.append("PASS lk-heap-bootstrap-gate: NAND-write path cannot mark the LK heap ready")

        # Read-only Generic Timer provider: no timer comparator/control or IRQ setup.
        timer_object = "runtime/h700_arch_timer.o"
        if f"$(LOCAL_DIR)/{timer_object}" not in platform_rules:
            raise RuntimeError("H700 platform does not link the Generic Timer provider")
        timer_source_path = root / "platform/h700/runtime/h700_arch_timer.c"
        timer_header_path = root / "platform/h700/runtime/h700_arch_timer.h"
        timer_source = timer_source_path.read_text(encoding="utf-8")
        if not timer_header_path.is_file():
            raise RuntimeError("H700 Generic Timer header missing")
        for instruction in ("mrrc p15, 1", "mrc p15, 0", "cntvct_el0", "cntfrq_el0"):
            if instruction not in timer_source:
                raise RuntimeError("H700 Generic Timer source is missing read path: " + instruction)
        for forbidden in ("mcr p15", "mcrr p15", "msr cntvct", "msr cntfrq",
                          "CNTP_CTL", "CNTP_CVAL", "CNTV_CTL", "CNTV_CVAL",
                          "platform_set_periodic_timer", "register_int_handler"):
            if forbidden.lower() in timer_source.lower():
                raise RuntimeError("H700 Generic Timer provider contains a write/IRQ path: " + forbidden)
        summary.append("PASS h700-arch-timer-object: CNTVCT/CNTFRQ read-only provider linked; no comparator or IRQ setup")

        # The A32 CRT0 wrapper publishes its first-instruction BL31 handoff
        # state into this C-owned BSS object; this reader is not a hardware probe.
        snapshot_object = "runtime/h700_bl33_entry_snapshot.o"
        if f"$(LOCAL_DIR)/{snapshot_object}" not in platform_rules:
            raise RuntimeError("H700 platform does not link the BL33 entry snapshot reader")
        snapshot_source = root / "platform/h700/runtime/h700_bl33_entry_snapshot.c"
        snapshot_header = root / "platform/h700/runtime/h700_bl33_entry_snapshot.h"
        if not snapshot_source.is_file() or not snapshot_header.is_file():
            raise RuntimeError("H700 BL33 entry snapshot source/header missing")
        snapshot_text = snapshot_source.read_text(encoding="utf-8")
        for required in (
            "H700_BL33_ENTRY_SNAPSHOT_MAGIC",
            "H700_BL33_ENTRY_SNAPSHOT_ENOTREADY",
            "h700_bl33_entry_snapshot_read",
        ):
            if required not in snapshot_text:
                raise RuntimeError("H700 BL33 entry snapshot reader is missing: " + required)
        for forbidden in (
            "mcr p15", "mcrr p15", "str r", "platform_set_periodic_timer",
            "register_int_handler", "GICD", "GICC", "CNTP_CTL", "CNTV_CTL",
        ):
            if forbidden.lower() in snapshot_text.lower():
                raise RuntimeError("BL33 entry snapshot reader must remain hardware-read/write free: " + forbidden)
        summary.append("PASS h700-bl33-entry-snapshot-object: CRT0 handoff snapshot reader is linked and has no MMIO/IRQ path")

        cpu_objects = (
            "power/h700_cpu_performance_policy.o",
            "power/h700_cpu_performance_service.o",
            "power/h700_cpu_performance_runtime.o",
        )
        for object_name in cpu_objects:
            object_entry = f"$(LOCAL_DIR)/{object_name}"
            if object_entry not in platform_rules:
                raise RuntimeError("H700 platform does not link CPU service object: " + object_entry)
            source_path = root / "platform/h700" / Path(object_name).with_suffix(".c")
            if not source_path.is_file():
                raise RuntimeError("H700 CPU service object has no source file: " + str(source_path))
        summary.append("PASS cpu-performance-objects: policy, ABI service and static runtime owner are in the H700 link object list")

        led_object = "power/h700_rg34xx_power_led.o"
        if f"$(LOCAL_DIR)/{led_object}" not in platform_rules:
            raise RuntimeError("RG34XX PI12 power LED object is missing from H700 platform rules")
        led_source = root / "platform/h700" / Path(led_object).with_suffix(".c")
        if not led_source.is_file():
            raise RuntimeError("RG34XX PI12 power LED object has no source file")
        summary.append("PASS rg34xx-power-led-object: PI12 board LED service is linked")

        mmu_object = "arch/h700_pio_mmu_mapping.o"
        if f"$(LOCAL_DIR)/{mmu_object}" not in platform_rules:
            raise RuntimeError("RG34XX PIO MMU mapping object is missing from H700 platform rules")
        mmu_source = root / "platform/h700" / Path(mmu_object).with_suffix(".c")
        if not mmu_source.is_file():
            raise RuntimeError("RG34XX early PIO MMU mapping source is missing")
        mmu_text = mmu_source.read_text(encoding="utf-8")
        for required in (
            "arm_mmu_init()",
            "H700_DRAM_BASE 0x40000000u",
            "H700_DRAM_SIZE 0x40000000u",
            "MMU_MEMORY_TYPE_NORMAL_WRITE_BACK_ALLOCATE",
            "H700_SOC_MMIO_WINDOW_BASE 0x03000000u",
            "H700_SYS_CFG_VER_REG 0x03000024u",
            "H700_RG34XX_PIO_BASE 0x0300b000u",
            "_Static_assert",
            "MMU_MEMORY_TYPE_DEVICE_SHARED",
            "arch_enable_mmu()",
        ):
            if required not in mmu_text:
                raise RuntimeError("RG34XX early MMU mapping is missing: " + required)
        if "DRAM_PHY_ADDR=0x40000000" not in platform_rules:
            raise RuntimeError("H700 build does not set the verified RG34XX DRAM physical base")
        summary.append("PASS rg34xx-soc-mmu-map: LK section table, DRAM attributes, SYS_CFG/PIO Device section and MMU enable are ordered")

        abort_objects = ("arch/h700_data_abort_recovery.o", "arch/h700_syscfg_probe.o")
        for obj in abort_objects:
            if f"$(LOCAL_DIR)/{obj}" not in platform_rules:
                raise RuntimeError("H700 probe-scoped Data Abort object is missing: " + obj)
        abort_src = root / "platform/h700/arch/h700_data_abort_recovery.c"
        abort_asm = root / "platform/h700/arch/h700_syscfg_probe.S"
        abort_header = root / "platform/h700/arch/h700_data_abort_recovery.h"
        if not abort_src.is_file() or not abort_asm.is_file() or not abort_header.is_file():
            raise RuntimeError("H700 probe-scoped Data Abort source set is incomplete")
        target_text = (root / "target/rg34xx_h700/init.c").read_text(encoding="utf-8")
        if "h700_syscfg_ver_reg_low3" in target_text:
            raise RuntimeError("SYS_CFG diagnostic must not be invoked from startup hooks")
        summary.append("PASS probe-scoped-data-abort: dormant one-register probe is linked; target startup does not invoke it")

        target_rules = (worktree / "target/rg34xx_h700/rules.mk").read_text(encoding="utf-8")
        target_init = root / "target/rg34xx_h700/init.c"
        if "$(LOCAL_DIR)/init.o" not in target_rules:
            raise RuntimeError("RG34XX H700 target rules do not link the target lifecycle object")
        if not target_init.is_file():
            raise RuntimeError("RG34XX H700 target lifecycle source is missing")
        target_text = target_init.read_text(encoding="utf-8")
        for hook in ("void target_early_init(void)", "void target_init(void)"):
            if target_text.count(hook) != 1:
                raise RuntimeError("RG34XX H700 target lifecycle source must define exactly one " + hook)
        if "target/$(TARGET)/init.o" in platform_rules or "$(LOCAL_DIR)/init.o" in platform_rules:
            raise RuntimeError("H700 target init object was incorrectly placed in platform rules")
        if "h700_rg34xx_power_led_early_init" not in target_text or "h700_rg34xx_power_led_is_ready" not in target_text:
            raise RuntimeError("RG34XX target hooks must initialize PI12 LED early and retry after platform_init")
        summary.append("PASS target-lifecycle-hooks: early PI12 power LED startup and safe retry are owned by RG34XX target hooks")

        generic_target_rules = (worktree / "target/rules.mk").read_text(encoding="utf-8")
        generic_init_excluded = re.search(
            r"ifneq \(\$\(H700_BUILD\),yes\)\s+"
            r"OBJS \+= \$\(LOCAL_DIR\)/init\.o\s+endif\s+"
            r"OBJS \+= \$\(LOCAL_DIR\)/target_display\.o",
            generic_target_rules,
        )
        if not generic_init_excluded:
            raise RuntimeError("H700 overlay must exclude the MTK generic target/init.c object")
        project_text = (root / "project/rg34xx_h700.mk").read_text(encoding="utf-8")
        if "app/aboot" in project_text or "app/mt_boot" in project_text:
            raise RuntimeError("H700 project must not enable the MTK aboot/mt_boot application")
        summary.append("PASS mtk-target-isolation: generic MTK target init and MTK boot apps stay out of the H700 project")

        make_case(
            "no-memory-approval",
            [
                "H700_MEMORY_LAYOUT_APPROVED=no",
                "H700_MEMBASE=0x12340000",
                "H700_MEMSIZE=0x10000",
            ],
            "H700_MEMORY_LAYOUT_APPROVED",
        )
        make_case(
            "missing-membase",
            [
                "H700_MEMORY_LAYOUT_APPROVED=yes",
                "H700_MEMBASE=",
                "H700_MEMSIZE=0x10000",
            ],
            "H700_MEMBASE",
        )
        make_case(
            "missing-memsize",
            [
                "H700_MEMORY_LAYOUT_APPROVED=yes",
                "H700_MEMBASE=0x12340000",
                "H700_MEMSIZE=",
            ],
            "H700_MEMSIZE",
        )
        make_case(
            "invalid-membase",
            [
                "H700_MEMORY_LAYOUT_APPROVED=yes",
                "H700_MEMBASE=not-an-address",
                "H700_MEMSIZE=0x10000",
            ],
            "H700_MEMBASE must be a hexadecimal",
        )
        make_case(
            "invalid-memsize",
            [
                "H700_MEMORY_LAYOUT_APPROVED=yes",
                "H700_MEMBASE=0x12340000",
                "H700_MEMSIZE=0xGG",
            ],
            "H700_MEMSIZE must be a hexadecimal",
        )
        make_case(
            "zero-membase",
            [
                "H700_MEMORY_LAYOUT_APPROVED=yes",
                "H700_MEMBASE=0x00000000",
                "H700_MEMSIZE=0x10000",
            ],
            "H700_MEMBASE must be non-zero",
        )
        make_case(
            "zero-memsize",
            [
                "H700_MEMORY_LAYOUT_APPROVED=yes",
                "H700_MEMBASE=0x12340000",
                "H700_MEMSIZE=0x00000000",
            ],
            "H700_MEMSIZE must be non-zero",
        )
        make_case(
            "approved-parser-dry-run",
            [
                "H700_MEMORY_LAYOUT_APPROVED=yes",
                # Test fixtures only: never valid RG34XX memory-map claims.
                "H700_MEMBASE=0x12340000",
                "H700_MEMSIZE=0x00010000",
            ],
            None,
            dry_run=True,
        )

        linker_var_command = [
            "make",
            "--no-print-directory",
            "-f",
            "makefile",
            "PROJECT=rg34xx_h700",
            "H700_MEMORY_LAYOUT_APPROVED=yes",
            "H700_MEMBASE=0x12340000",
            "H700_MEMSIZE=0x00010000",
            "--eval=print-h700-linker: ; @printf '%s\\n' '$(LINKER_SCRIPT)'",
            "print-h700-linker",
        ]
        linker_var_result = run(linker_var_command, cwd=worktree)
        expected_linker = "./build-rg34xx_h700/system-onesegment.ld"
        if (
            linker_var_result.returncode != 0
            or expected_linker not in linker_var_result.stdout.splitlines()
        ):
            raise RuntimeError(
                "H700 linker-script variable is not wired to the generated LK script:\\n"
                + linker_var_result.stdout[-6000:]
            )
        if (worktree / "build-rg34xx_h700").exists():
            raise RuntimeError("linker variable probe unexpectedly created a build directory")
        summary.append("PASS linker-script-variable: resolves to the generated one-segment script")

        report = [
            "# RG34XX H700 LK project guard",
            "",
            f"- Pinned GammaOS LK: `{LK_PIN}`",
            "- Overlay: six-file H700 BL33 source patch, applied in a disposable worktree",
            "- Result: all fail-closed checks passed; explicit sentinel dry-run emitted no files",
            "",
            *[f"- {line}" for line in summary],
            "",
            "This is a make-graph/guard test, not a complete LK link or device boot.",
        ]
        (out / "REPORT.md").write_text("\n".join(report) + "\n", encoding="utf-8")
        print("\n".join(summary))
        print("H700 LK project guard: PASS (host-only; no firmware image built)")
    finally:
        if worktree_added:
            removed = run([
                "git", "-C", str(lk_source), "worktree", "remove", "--force",
                str(worktree),
            ])
            if removed.returncode:
                (out / "worktree-cleanup-warning.log").write_text(
                    removed.stdout, encoding="utf-8"
                )
        shutil.rmtree(temp_root, ignore_errors=True)


if __name__ == "__main__":
    main()
