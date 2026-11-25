#!/usr/bin/env python3
"""把 FBNeo 的 ClrMamePro XML DAT 转成 linkSlot 的 DAT JSON。

用法：
    python3 build/fbneo_dat.py \\
        --dat "target/fbneo-dat/<commit>/dats/FinalBurn Neo (ClrMame Pro XML, Arcade only).dat" \\
        --platform arcade \\
        --out dist/platforms/arcade.dat.json

- `--platform arcade`：排除 `sourcefile` 以 `neogeo/` 开头的条目（那 708 个属
  NeoGeo 包），其余全部保留；
- `--platform neogeo`：只保留 `neogeo/*`；
- **BIOS 与 clone 一律保留**：`is_bios` / `parent` 写进 JSON，过滤是扫描器的职责。
  在 DAT 阶段就把它们删掉，用户装了一个只含 clone 的库就再也看不到任何提示。
- 输出按 set 名升序，紧凑 JSON（这张表要装到卡上，不追求好看）。

`players` / `orientation` 不写：FBNeo 的 Arcade DAT 里**没有**这两个字段，编造它们
比留空更糟（读端缺省 1 / Horizontal，Phase 4 再按柜子布局补）。
"""

import argparse
import json
import sys
import time
import xml.etree.ElementTree as ET
from pathlib import Path

# 精确到源文件：同一驱动前缀下这几套板子的身份差别必须保留。
SYSTEM_BY_FILE = {
    "cps3/d_cps3.cpp": "CPS-3",
    "capcom/d_cps1.cpp": "CPS-1",
    "capcom/d_cps2.cpp": "CPS-2",
    "capcom/d_cps2_custom.cpp": "CPS-2",
    "capcom/d_mitchell.cpp": "Mitchell",
}

# 其余按驱动目录归组；没列到的（pre90s、pst90s、nes、megadrive、d_parent.cpp…）
# 都算通用街机。
SYSTEM_BY_PREFIX = {
    "capcom": "Capcom",
    "neogeo": "NEOGEO",
    "konami": "Konami",
    "sega": "Sega",
    "taito": "Taito",
    "irem": "Irem",
    "dataeast": "Data East",
    "cave": "CAVE",
    "toaplan": "Toaplan",
    "psikyo": "Psikyo",
    "atari": "Atari",
    "midway": "Midway",
    "galaxian": "Galaxian",
    "pgm": "PGM",
    "pgm2": "PGM2",
}

NEOGEO_PREFIX = "neogeo/"


def system_for(sourcefile: str) -> str:
    """驱动源文件 → 展示用的机板名（读端只用它做副标题）。"""
    if sourcefile in SYSTEM_BY_FILE:
        return SYSTEM_BY_FILE[sourcefile]
    prefix = sourcefile.split("/", 1)[0]
    return SYSTEM_BY_PREFIX.get(prefix, "Arcade")


def is_neogeo(sourcefile: str) -> bool:
    return sourcefile.startswith(NEOGEO_PREFIX)


def games(path: Path):
    """流式产出 (set, title, system, year, parent, is_bios, sourcefile)。

    `iterparse` + `element.clear()`：14 MB / 8747 个 game 的整棵树没必要驻留内存。
    """
    for _, element in ET.iterparse(path, events=("end",)):
        if element.tag != "game":
            continue
        set_name = (element.get("name") or "").strip()
        sourcefile = (element.get("sourcefile") or "").strip()
        description = ""
        year = ""
        for child in element:
            if child.tag == "description" and not description:
                description = (child.text or "").strip()
            elif child.tag == "year" and not year:
                year = (child.text or "").strip()
        parent = (element.get("cloneof") or "").strip()
        is_bios = (element.get("isbios") or "").strip().lower() == "yes"
        yield {
            "set": set_name,
            "title": description or set_name,
            "system": system_for(sourcefile),
            "year": year,
            "parent": parent,
            "is_bios": is_bios,
        }, sourcefile
        element.clear()


def header_version(path: Path) -> str:
    """读 `<header><version>`；只解析头部，不建整棵树。"""
    for _, element in ET.iterparse(path, events=("start", "end")):
        if element.tag == "version" and element.text:
            return element.text.strip()
        # game 一开始就说明头部结束了。
        if element.tag == "game":
            break
    return ""


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Convert a ClrMamePro DAT to linkSlot JSON")
    parser.add_argument("--dat", required=True, help="ClrMamePro XML DAT")
    parser.add_argument("--platform", required=True, choices=["arcade", "neogeo"])
    parser.add_argument("--out", required=True)
    args = parser.parse_args(argv)

    dat_path = Path(args.dat)
    if not dat_path.is_file():
        raise SystemExit(f"找不到 DAT：{dat_path}")

    started = time.monotonic()
    version = header_version(dat_path)
    sets = []
    skipped_platform = 0
    for entry, sourcefile in games(dat_path):
        if not entry["set"]:
            continue
        wants = is_neogeo(sourcefile)
        if (args.platform == "neogeo") != wants:
            skipped_platform += 1
            continue
        sets.append(entry)
    sets.sort(key=lambda entry: entry["set"])

    document = {
        "schema": 1,
        "platform": args.platform,
        "generated_from": {"dat": dat_path.name, "version": version},
        "sets": sets,
    }
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(document, ensure_ascii=False, separators=(",", ":"))
    out.write_text(payload + "\n", encoding="utf-8", newline="\n")
    elapsed_ms = (time.monotonic() - started) * 1000

    bios = sum(1 for entry in sets if entry["is_bios"])
    clones = sum(1 for entry in sets if entry["parent"])
    print(
        f"{out.name}: {len(sets)} sets ({bios} bios, {clones} clones, "
        f"{skipped_platform} filtered) {out.stat().st_size} B  {elapsed_ms:.0f} ms",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
