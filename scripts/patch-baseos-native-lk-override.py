#!/usr/bin/env python3
"""Patch the pinned BaseOS native builder to accept a fixed-size LK BL33 payload.

This only edits a local BaseOS source checkout. It never builds, flashes, or
writes a device. The replacement must fit the currently deployed 32 KiB loader
extent so the firmware extent table and every following offset stay unchanged.
"""
from __future__ import annotations

import argparse
import ast
import inspect
import shutil
import struct
import subprocess
import sys
from pathlib import Path

BASEOS_PIN = "cbe253a43f46cf551cc9a6c79a84015bde3686cf"
NATIVE_BUILD_REL = Path("experiments/h700-boot-speed/native/build.py")
PATCH_MARKER = "# LINKslot-H700-LK-BL33-OVERRIDE"
BACKUP_SUFFIX = ".linkslot-h700-original"
LOADER_EXTENT_BYTES = 0x8000
IMAGE_BASE = 0x4A000000
HEADER_END = 0x640
VECTOR_OFFSET = 0x1000


def _a32_branch_target(raw: bytes, offset: int, pc: int) -> int:
    word = struct.unpack_from("<I", raw, offset)[0]
    if (word & 0x0F000000) != 0x0A000000 or (word & 0x01000000):
        raise ValueError(f"expected A32 B (not BL) at {pc:#010x}, got {word:#010x}")
    immediate = word & 0x00FFFFFF
    if immediate & 0x00800000:
        immediate -= 1 << 24
    return pc + 8 + (immediate << 2)


def _validate_h700_lk_raw(raw: bytes) -> None:
    if len(raw) < VECTOR_OFFSET + 8 * 4:
        raise ValueError("raw LK image is too small for the fixed A32 vector table")
    if len(raw) > LOADER_EXTENT_BYTES:
        raise ValueError(
            f"raw LK image exceeds the active 32 KiB native loader extent ({LOADER_EXTENT_BYTES:#x})"
        )
    if any(raw[4:HEADER_END]):
        raise ValueError("raw LK image must leave vendor header bytes [+4,+0x640) zero")
    entry = _a32_branch_target(raw, 0, IMAGE_BASE)
    image_end = IMAGE_BASE + len(raw)
    if not IMAGE_BASE + HEADER_END <= entry < image_end:
        raise ValueError(f"BL33 entry branch is outside the LK code image: {entry:#x}")
    for index in range(8):
        offset = VECTOR_OFFSET + index * 4
        pc = IMAGE_BASE + offset
        target = _a32_branch_target(raw, offset, pc)
        if not IMAGE_BASE + HEADER_END <= target < image_end:
            raise ValueError(f"A32 vector {index} branches outside the LK code image: {target:#x}")


def patch_source(source: str) -> str:
    """Apply a narrow, idempotent patch to the exact pinned native/build.py."""
    if PATCH_MARKER in source:
        ast.parse(source)
        required = (
            "BASEOS_LK_BL33_RAW",
            "_validate_h700_lk_raw",
            "lk_bl33_override",
            "LOADER_EXTENT_BYTES",
        )
        missing = [item for item in required if item not in source]
        if missing:
            raise RuntimeError("partial or damaged previous patch; missing: " + ", ".join(missing))
        return source

    def replace_once(old: str, new: str, label: str) -> None:
        nonlocal source
        count = source.count(old)
        if count != 1:
            raise RuntimeError(f"pinned BaseOS native/build.py anchor {label!r}: expected once, found {count}")
        source = source.replace(old, new, 1)

    fnv_block = (
        "def fnv(data):\n"
        "    result = 2166136261\n"
        "    for byte in data: result = (result ^ byte) * 16777619 & 0xffffffff\n"
        "    return result\n"
        "\n"
        "\n"
    )
    helper_source = (
        "IMAGE_BASE = 0x4a000000\\n"
        "HEADER_END = 0x640\\n"
        "VECTOR_OFFSET = 0x1000\\n"
        "LOADER_EXTENT_BYTES = 0x8000\\n\\n"
        + inspect.getsource(_a32_branch_target)
        + "\\n\\n"
        + inspect.getsource(_validate_h700_lk_raw)
        + "\\n\\n"
    )
    replace_once(fnv_block, fnv_block + helper_source, "FNV helper")

    input_anchor = '    source_boot0 = (inventory / "boot0.bin").read_bytes()\n'
    input_patch = (
        f"    {PATCH_MARKER}\n"
        '    lk_bl33_override_path = None\n'
        '    lk_bl33_override_data = None\n'
        '    override_value = os.environ.get("BASEOS_LK_BL33_RAW")\n'
        '    if override_value:\n'
        '        candidate = Path(override_value)\n'
        '        if not candidate.is_absolute():\n'
        '            candidate = ROOT / candidate\n'
        '        lk_bl33_override_path = candidate.resolve()\n'
        '        try:\n'
        '            lk_bl33_override_path.relative_to(ROOT.resolve())\n'
        '        except ValueError as error:\n'
        '            raise ValueError("BASEOS_LK_BL33_RAW must be inside the BaseOS repository") from error\n'
        '        if not lk_bl33_override_path.is_file():\n'
        '            raise FileNotFoundError(lk_bl33_override_path)\n'
        '        lk_bl33_override_data = lk_bl33_override_path.read_bytes()\n'
        '        _validate_h700_lk_raw(lk_bl33_override_data)\n'
        '        # Include the binary in both the generation digest and persistent source hashes.\n'
        '        inputs.append(lk_bl33_override_path)\n'
        '\n'
        + input_anchor
    )
    replace_once(input_anchor, input_patch, "generated-input list")

    blob_anchor = '    blob = bytearray((output / "loader.bin").read_bytes())\n'
    blob_patch = (
        blob_anchor
        + '    if lk_bl33_override_data is not None:\n'
        '        # The replacement is a raw, header-free LK image; the existing\n'
        '        # verified native header is copied below from the paired TOC item.\n'
        '        blob = bytearray(lk_bl33_override_data)\n'
    )
    replace_once(blob_anchor, blob_patch, "loader blob selection")

    padding_anchor = (
        "    blob += bytes(-len(blob) % 512)\n"
        "    assert len(blob) <= 0xe0000\n"
    )
    padding_patch = (
        "    blob += bytes(-len(blob) % 512)\n"
        "    if lk_bl33_override_data is not None:\n"
        "        if len(blob) > 0x8000:\n"
        "            raise ValueError('header plus LK payload exceeds the active 32 KiB loader extent')\n"
        "        # Keep the loader extent exactly 32 KiB: all firmware offsets and\n"
        "        # the native SPL extent table therefore remain unchanged.\n"
        "        blob.extend(bytes(0x8000 - len(blob)))\n"
        "        if len(blob) != 0x8000:\n"
        "            raise AssertionError('LK replacement did not preserve the 32 KiB loader extent')\n"
        "    assert len(blob) <= 0xe0000\n"
    )
    replace_once(padding_anchor, padding_patch, "fixed loader extent padding")

    report_anchor = (
        "                  durable_install_eligible=False)\n"
        "    if persistent or raw_capsule:\n"
    )
    report_patch = (
        "                  durable_install_eligible=False)\n"
        "    if lk_bl33_override_path is not None:\n"
        "        report['lk_bl33_override'] = {\n"
        "            'path': str(lk_bl33_override_path.relative_to(ROOT)),\n"
        "            'raw_bytes': len(lk_bl33_override_data),\n"
        "            'raw_sha256': sha(lk_bl33_override_data),\n"
        "            'loader_extent_bytes': len(blob),\n"
        "            'loader_extent_preserved': len(blob) == 0x8000,\n"
        "            'device_execution_verified': False,\n"
        "        }\n"
        "    if persistent or raw_capsule:\n"
    )
    replace_once(report_anchor, report_patch, "build manifest")

    if PATCH_MARKER not in source:
        raise AssertionError("patch marker missing after application")
    ast.parse(source)
    return source


def git_output(root: Path, *args: str) -> str:
    return subprocess.run(
        ["git", "-C", str(root), *args],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseos-root", type=Path, required=True,
                        help="clean BaseOS-MultiFrontend checkout at the pinned commit")
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--apply", action="store_true", help="patch native/build.py in place")
    mode.add_argument("--check", action="store_true", help="verify an already patched build.py")
    mode.add_argument("--restore", action="store_true", help="restore the exact pre-patch build.py backup")
    args = parser.parse_args()

    root = args.baseos_root.resolve()
    source_path = root / NATIVE_BUILD_REL
    backup_path = source_path.with_name(source_path.name + BACKUP_SUFFIX)
    if not source_path.is_file():
        parser.error(f"not a BaseOS checkout: missing {source_path}")
    try:
        head = git_output(root, "rev-parse", "HEAD")
    except (OSError, subprocess.CalledProcessError) as error:
        parser.error(f"cannot verify BaseOS git checkout: {error}")
    if head != BASEOS_PIN:
        parser.error(f"BaseOS source must be pinned at {BASEOS_PIN}; found {head}")

    source = source_path.read_text(encoding="utf-8")
    if args.check:
        if PATCH_MARKER not in source:
            parser.error("native/build.py is not patched")
        ast.parse(source)
        if "_validate_h700_lk_raw" not in source or "BASEOS_LK_BL33_RAW" not in source:
            parser.error("native/build.py patch is incomplete")
        print(f"PASS: BaseOS native builder patch is present at {head}")
        return 0

    if args.restore:
        if not backup_path.is_file():
            parser.error(f"missing original-source backup: {backup_path}")
        original = backup_path.read_text(encoding="utf-8")
        if source != patch_source(original):
            parser.error("native/build.py changed after patching; refusing to overwrite later edits")
        source_path.write_text(original, encoding="utf-8", newline="\n")
        backup_path.unlink()
        print(f"Restored: {source_path}")
        return 0

    dirty = git_output(root, "status", "--porcelain", "--", NATIVE_BUILD_REL.as_posix())
    if dirty:
        parser.error("native/build.py is already modified; use a clean pinned BaseOS checkout")
    if backup_path.exists():
        parser.error(f"refusing to overwrite existing backup: {backup_path}")
    updated = patch_source(source)
    shutil.copy2(source_path, backup_path)
    source_path.write_text(updated, encoding="utf-8", newline="\n")
    print(f"Patched: {source_path}")
    print(f"Backup:  {backup_path}")
    print(f"Pinned BaseOS commit: {head}")
    print("The patch preserves the active 32 KiB loader extent and does not build or flash anything.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
