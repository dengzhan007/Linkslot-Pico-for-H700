#!/usr/bin/env python3
"""Host-only tests for the RG34XX H700 LK project and memory fail-closed gate."""
from __future__ import annotations

import argparse
import shutil
import subprocess
import tempfile
from pathlib import Path

LK_PIN = "9bf1419a49e0af6dba8b05916df382c9c9c6d0a8"
SCAFFOLD = (
    Path("project/rg34xx_h700.mk"),
    Path("target/rg34xx_h700/rules.mk"),
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
            command.append("build-rg34xx_h700/system-onesegment.ld")
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
