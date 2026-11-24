#!/usr/bin/env python3
"""Apply the persistent H700 no_std Linkslot business crates to a temporary source tree."""
from __future__ import annotations

import argparse
import shutil
import subprocess
from pathlib import Path

PINNED_SNAPSHOT = "8590d6ff1b2caa26118051069c975106fbb14bd3"
CRATES = ("slot-input", "slot-model")


def run_git(source: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(source), *args],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    return result.stdout.strip()


def apply(source: Path, crate_root: Path) -> None:
    source = source.resolve()
    crate_root = crate_root.resolve()
    if not (source / ".git").exists():
        raise RuntimeError(f"not a Git checkout: {source}")
    actual = run_git(source, "rev-parse", "HEAD")
    if actual != PINNED_SNAPSHOT:
        raise RuntimeError(
            f"Linkslot snapshot pin mismatch: expected {PINNED_SNAPSHOT}, got {actual}"
        )
    workspace = (source / "Cargo.toml").read_text(encoding="utf-8")
    if 'version = "1.0.36"' not in workspace:
        raise RuntimeError("source workspace is not Linkslot 1.0.36")

    for name in CRATES:
        source_crate = crate_root / name
        manifest = source_crate / "Cargo.toml"
        src = source_crate / "src"
        if not manifest.is_file() or not src.is_dir():
            raise RuntimeError(f"incomplete persistent crate: {source_crate}")
        dest = source / "crates" / name
        if not dest.is_dir():
            raise RuntimeError(f"upstream crate directory missing: {dest}")
        old_src = dest / "src"
        if old_src.exists():
            shutil.rmtree(old_src)
        shutil.copytree(src, old_src)
        shutil.copy2(manifest, dest / "Cargo.toml")

    # The immutable checkout is changed only inside the CI workspace. Fail
    # rather than silently broadening the patch to any other upstream files.
    changed = run_git(source, "status", "--short", "--", "crates/slot-input", "crates/slot-model")
    if not changed:
        raise RuntimeError("business-crate overlay produced no source changes")
    run_git(source, "diff", "--check", "--", "crates/slot-input", "crates/slot-model")
    print("Applied H700 no_std business crates to the isolated Linkslot 1.0.36 checkout:")
    print(changed)
    print("The repository's upstream/linkslot branch is not modified by this host-only tool.")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", required=True, type=Path)
    parser.add_argument("--crate-root", required=True, type=Path)
    args = parser.parse_args()
    apply(args.source, args.crate_root)


if __name__ == "__main__":
    main()
