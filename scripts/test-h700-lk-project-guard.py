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
        summary.append("PASS target-lifecycle-hooks: target_early_init/target_init are supplied by the H700 target, not the MTK generic target")

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
