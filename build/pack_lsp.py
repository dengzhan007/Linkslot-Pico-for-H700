#!/usr/bin/env python3
"""把扩展包元数据和文件打成 `.lsp`（ZIP 容器）。

用法：
    python3 build/pack_lsp.py \\
        --meta packs/supafaust/pack.json \\
        --payload System/linkSlot/cores/supafaust/supafaust_libretro.so=vendor/supafaust_libretro.so \\
        --out dist/packs/linkslot-pack-supafaust-v1.0.0.lsp

- `--payload` 可以重复，格式是「卡内相对路径=本地文件」；
- `pack.json` 里的 `files` 由脚本按实际文件计算（大小 + sha256）；
- ZIP 内层就是卡内目标路径，安装器只接受白名单目录；
- 时间戳跟随 `SOURCE_DATE_EPOCH`，同样的输入产生同样的包。
"""

import argparse
import datetime
import json
import os
import sys
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EPOCH = int(os.environ.get("SOURCE_DATE_EPOCH", "0") or "0")
CORE_PREFIX = "System/linkSlot/cores/"


def zip_time() -> tuple:
    if EPOCH <= 0:
        return (1980, 1, 1, 0, 0, 0)
    moment = datetime.datetime.fromtimestamp(EPOCH, datetime.timezone.utc)
    return (max(moment.year, 1980), moment.month, moment.day, moment.hour, moment.minute, moment.second)


def sha256(data: bytes) -> str:
    import hashlib

    return hashlib.sha256(data).hexdigest()


def load_payloads(specs: list[str]) -> dict[str, bytes]:
    payloads: dict[str, bytes] = {}
    for spec in specs:
        if "=" not in spec:
            raise SystemExit(f"--payload 需要 卡内路径=本地文件：{spec}")
        card_path, local_path = spec.split("=", 1)
        card_path = card_path.strip().strip("/")
        if card_path in payloads:
            raise SystemExit(f"重复的目标路径：{card_path}")
        source = Path(local_path)
        if not source.is_file():
            raise SystemExit(f"找不到文件：{source}")
        payloads[card_path] = source.read_bytes()
    if not payloads:
        raise SystemExit("至少需要一个 --payload")
    return payloads


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Build a .lsp extension pack")
    parser.add_argument("--meta", required=True, help="pack.json 模板")
    parser.add_argument("--payload", action="append", default=[], help="卡内路径=本地文件")
    parser.add_argument("--out", required=True)
    args = parser.parse_args(argv)

    meta = json.loads(Path(args.meta).read_text(encoding="utf-8"))
    if meta.get("schema") != 1:
        raise SystemExit("pack.json schema 必须是 1")
    kind = meta.get("kind")
    if kind not in ("core", "achievements"):
        raise SystemExit(f"不支持的扩展包类型：{kind}")
    pack_id = str(meta.get("id") or "")
    if not pack_id:
        raise SystemExit("pack.json 缺少 id")

    payloads = load_payloads(args.payload)
    files = []
    for card_path, data in sorted(payloads.items()):
        if kind == "core":
            expected_prefix = f"{CORE_PREFIX}{pack_id}/"
            if not card_path.startswith(expected_prefix):
                raise SystemExit(f"核心包路径必须在 {expected_prefix} 下：{card_path}")
            if not card_path.endswith(f"{pack_id}_libretro.so"):
                raise SystemExit(f"核心文件名必须是 {pack_id}_libretro.so：{card_path}")
        else:
            if not card_path.startswith("linkSlot/Achievements/"):
                raise SystemExit(
                    f"成就包路径必须在 linkSlot/Achievements/ 下：{card_path}"
                )
            if not card_path.endswith((".json", ".png")):
                raise SystemExit(f"成就包只接受 .json/.png：{card_path}")
        files.append({"path": card_path, "size": len(data), "sha256": sha256(data)})
    meta["files"] = files

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    manifest = (json.dumps(meta, ensure_ascii=False, indent=2) + "\n").encode("utf-8")
    stamp = zip_time()
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as archive:
        info = zipfile.ZipInfo("pack.json", date_time=stamp)
        info.compress_type = zipfile.ZIP_DEFLATED
        archive.writestr(info, manifest)
        for card_path, data in sorted(payloads.items()):
            info = zipfile.ZipInfo(card_path, date_time=stamp)
            info.compress_type = zipfile.ZIP_DEFLATED
            archive.writestr(info, data)
    digest = sha256(out.read_bytes())
    print(f"{out.name}  {out.stat().st_size} B  {digest}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
