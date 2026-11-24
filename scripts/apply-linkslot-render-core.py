#!/usr/bin/env python3
"""Apply the H700 software-render contract to an isolated Linkslot 1.0.36 tree.

The immutable upstream snapshot is never edited in the repository. CI applies
this patch to a temporary checkout and later exports the resulting source diff.
"""
from __future__ import annotations

import argparse
import hashlib
import shutil
import subprocess
from pathlib import Path


EXPECTED_SOURCE = "8590d6ff1b2caa26118051069c975106fbb14bd3"


def run_git(source: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(source), *args],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    return result.stdout.strip()


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected one match, found {count}")
    return text.replace(old, new, 1)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, required=True,
                        help="temporary checkout of upstream/linkslot")
    parser.add_argument("--crate", type=Path, required=True,
                        help="repository's integration/linkslot/h700-render-core")
    args = parser.parse_args()

    source = args.source.resolve()
    crate = args.crate.resolve()
    if run_git(source, "rev-parse", "HEAD") != EXPECTED_SOURCE:
        raise SystemExit("refusing to patch an unexpected Linkslot source revision")

    root_manifest = source / "Cargo.toml"
    if 'version = "1.0.36"' not in root_manifest.read_text(encoding="utf-8"):
        raise SystemExit("source tree is not the pinned Linkslot 1.0.36 snapshot")

    if not (crate / "Cargo.toml").is_file() or not (crate / "src/lib.rs").is_file():
        raise SystemExit("render-core crate is incomplete")

    target_crate = source / "crates/h700-render-core"
    if target_crate.exists():
        raise SystemExit(f"refusing to overwrite existing {target_crate}")
    shutil.copytree(crate, target_crate)

    gfx_manifest = source / "crates/slot-gfx/Cargo.toml"
    manifest_text = gfx_manifest.read_text(encoding="utf-8")
    manifest_text = replace_once(
        manifest_text,
        'test-support = []',
        'test-support = ["h700-render-core/test-support"]',
        "slot-gfx test-support feature",
    )
    manifest_text = replace_once(
        manifest_text,
        "[dependencies]\n",
        '[dependencies]\nh700-render-core = { path = "../h700-render-core" }\n',
        "slot-gfx render-core dependency",
    )
    gfx_manifest.write_text(manifest_text, encoding="utf-8")

    draw_file = source / "crates/slot-gfx/src/draw.rs"
    draw_text = draw_file.read_text(encoding="utf-8")
    start_marker = '#[derive(Copy, Clone, PartialEq, Eq, Debug)]\npub struct TexId(usize);'
    end_marker = 'pub struct Sprites {'
    start = draw_text.find(start_marker)
    end = draw_text.find(end_marker, start)
    if start < 0 or end < 0:
        raise SystemExit("slot-gfx Draw/TexId declarations do not match the audited baseline")
    if "pub enum Draw {" not in draw_text[start:end]:
        raise SystemExit("Draw enum is missing from the audited declaration block")
    draw_text = (
        draw_text[:start]
        + "pub use h700_render_core::{Draw, TexId};\n\n"
        + draw_text[end:]
    )

    for field, expected in (("id.0", 3), ("tex.0", 2)):
        actual = draw_text.count(field)
        if actual != expected:
            raise SystemExit(f"expected {expected} occurrences of {field}, found {actual}")
        draw_text = draw_text.replace(field, field[:-2] + ".index()")

    draw_file.write_text(draw_text, encoding="utf-8")

    if "pub struct TexId" in draw_text or "pub enum Draw" in draw_text:
        raise SystemExit("local duplicate Draw/TexId types remain in slot-gfx")
    if "pub use h700_render_core::{Draw, TexId};" not in draw_text:
        raise SystemExit("slot-gfx did not re-export the shared render contract")

    run_git(source, "diff", "--check")
    changed = run_git(source, "status", "--short")
    print("Applied shared H700 render contract to temporary Linkslot 1.0.36 tree.")
    print(f"Base: {EXPECTED_SOURCE}")
    print("Changed files:")
    print(changed)
    for path in (target_crate / "Cargo.toml", target_crate / "src/lib.rs",
                 gfx_manifest, draw_file):
        print(f"SHA256 {sha256(path)}  {path.relative_to(source)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
