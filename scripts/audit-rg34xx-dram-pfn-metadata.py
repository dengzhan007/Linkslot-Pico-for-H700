#!/usr/bin/env python3
"""Decode an RG34XX PFN sample using the Linux stable v4.9.170 KPF ABI."""
from __future__ import annotations
import argparse, json, re, sys
from pathlib import Path

EXPECTED_KERNEL = "4.9.170"
EXPECTED_PFN = 0x40000
KPF = {
    "LOCKED": 0, "ERROR": 1, "REFERENCED": 2, "UPTODATE": 3,
    "DIRTY": 4, "LRU": 5, "ACTIVE": 6, "SLAB": 7,
    "WRITEBACK": 8, "RECLAIM": 9, "BUDDY": 10,
    "MMAP": 11, "ANON": 12, "SWAPCACHE": 13, "SWAPBACKED": 14,
    "COMPOUND_HEAD": 15, "COMPOUND_TAIL": 16, "HUGE": 17,
    "UNEVICTABLE": 18, "HWPOISON": 19, "NOPAGE": 20, "KSM": 21,
    "THP": 22, "BALLOON": 23, "ZERO_PAGE": 24, "IDLE": 25,
    "RESERVED": 32, "MLOCKED": 33, "MAPPEDTODISK": 34,
    "PRIVATE": 35, "PRIVATE_2": 36, "OWNER_PRIVATE": 37,
    "ARCH": 38, "UNCACHED": 39, "SOFTDIRTY": 40,
}
SOURCES = {
    "uapi": "https://github.com/gregkh/linux/blob/v4.9.170/include/uapi/linux/kernel-page-flags.h",
    "proc_page": "https://github.com/gregkh/linux/blob/v4.9.170/fs/proc/page.c",
    "internal": "https://github.com/gregkh/linux/blob/v4.9.170/include/linux/kernel-page-flags.h",
}


def scalar(text, pattern, label):
    match = re.search(pattern, text, re.M)
    if not match:
        raise ValueError("missing " + label)
    return match.group(1)


def section_hex(text, section):
    match = re.search(r"(?ms)^\[" + re.escape(section) + r"\]\s*(.*?)(?=^\[|\Z)", text)
    if not match:
        raise ValueError("missing [" + section + "]")
    body = match.group(1)
    status = re.search(r"(?m)^status=(\S+)", body)
    raw = re.search(r"(?m)^raw_8_bytes_hex=([0-9a-fA-F]{16})\s*$", body)
    if not status or status.group(1) != "READ_OK" or not raw:
        raise ValueError("[" + section + "] incomplete or not READ_OK")
    return raw.group(1).lower()


def decode_report(text):
    kernel = scalar(text, r"^kernel_release=(\S+)$", "kernel_release")
    uid = int(scalar(text, r"^uid=(\d+)$", "uid"))
    page_size = int(scalar(text, r"^page_size_bytes=(\d+)$", "page_size_bytes"))
    match = re.search(r"(?m)^target_pfn=0x([0-9a-fA-F]+)\s+decimal=(\d+)$", text)
    if not match:
        raise ValueError("missing target_pfn")
    pfn, decimal = int(match.group(1), 16), int(match.group(2), 10)
    if pfn != decimal:
        raise ValueError("PFN hex/decimal mismatch")
    if kernel != EXPECTED_KERNEL:
        raise ValueError("unsupported kernel: " + kernel)
    if uid != 0 or page_size != 4096 or pfn != EXPECTED_PFN:
        raise ValueError("unexpected UID, page size, or PFN")

    flags_hex = section_hex(text, "kpageflags")
    count_hex = section_hex(text, "kpagecount")
    flags = int.from_bytes(bytes.fromhex(flags_hex), "little")
    mapcount = int.from_bytes(bytes.fromhex(count_hex), "little")
    present = {name: bool(flags & (1 << bit)) for name, bit in KPF.items()}

    if present["MMAP"] and present["ANON"] and mapcount > 0:
        classification = "LINUX_ACTIVE_ANON_MAPPED_PAGE" if present["ACTIVE"] and present["LRU"] else "LINUX_ANON_MAPPED_PAGE"
        gate = "BLOCKED_LIVE_LINUX_PAGE_REQUIRES_EXACT_PRESERVE_RESTORE"
        conclusion = "Anonymous mapped page; page_mapcount is " + str(mapcount) + "."
    elif present["MMAP"] != (mapcount > 0):
        classification = "INCONSISTENT_OR_RACY_KPAGE_METADATA"
        gate = "BLOCKED_INCONSISTENT_PAGE_METADATA"
        conclusion = "KPF_MMAP and page_mapcount disagree; investigate race/vendor semantics."
    elif mapcount > 0:
        classification = "MAPPED_PAGE_TYPE_NOT_CONFIRMED"
        gate = "BLOCKED_LINUX_PAGE_MAPPING_PRESENT"
        conclusion = "Nonzero page_mapcount; do not treat as free scratch."
    else:
        classification = "NO_MAPPING_OBSERVED_AT_SAMPLE"
        gate = "BLOCKED_OWNERSHIP_NOT_PROVEN"
        conclusion = "No mapping observed in this sample; ownership is still unproven."

    return {
        "schema": "rg34xx-dram-pfn-kpage-metadata-v1",
        "kernel_release": kernel,
        "pfn": hex(pfn),
        "page": {
            "start": "0x40000000", "end_exclusive": "0x40001000",
            "bl31_source_window": ["0x40000000", "0x40000140"],
            "source_window_inside_page": True,
        },
        "raw": {
            "kpageflags_bytes_hex": flags_hex,
            "kpageflags_u64_little_endian": hex(flags),
            "kpagecount_bytes_hex": count_hex,
            "kpagecount_page_mapcount": mapcount,
        },
        "kpf_flags": {k.lower(): v for k, v in present.items()},
        "set_flags": [k for k, v in present.items() if v],
        "interpretation": {
            "page_mapped": present["MMAP"],
            "anonymous_page": present["ANON"],
            "active_lru": present["ACTIVE"] and present["LRU"],
            "on_buddy_free_list": present["BUDDY"],
            "linux_pg_reserved": present["RESERVED"],
            "page_mapcount": mapcount,
            "page_owner_available": False,
            "process_identity": "NOT_IDENTIFIED",
            "single_sample_only": True,
            "conclusion": conclusion,
        },
        "classification": classification,
        "gate": gate,
        "H700_MEMORY_LAYOUT_APPROVED": False,
        "hardware_applied": 0,
        "source_code_references": SOURCES,
        "source_match_limit": "KPF definitions and proc implementation checked against upstream stable v4.9.170; exact vendor build-ID source tree not recovered.",
        "safety_note": "The page must not be treated as free scratch. This sample does not prove the BL31 snapshot/restore corrupts it; safe use requires proof of exact save-before-training, restore-before-Linux-resume, and CPU/DMA/cache quiescence.",
    }


def render(report, input_name):
    flags = report["kpf_flags"]
    lines = [
        "# RG34XX DRAM retention PFN metadata decode", "",
        f"- Input: `{input_name}`",
        f"- Kernel: `{report['kernel_release']}`",
        f"- PFN: `{report['pfn']}`",
        f"- Page: `{report['page']['start']}`–`{report['page']['end_exclusive']}`",
        f"- Raw kpageflags: `{report['raw']['kpageflags_bytes_hex']}` = `{report['raw']['kpageflags_u64_little_endian']}`",
        f"- Raw kpagecount: `{report['raw']['kpagecount_bytes_hex']}` = `{report['raw']['kpagecount_page_mapcount']}`",
        f"- Classification: **{report['classification']}**",
        f"- Gate: **{report['gate']}**", "",
        "| KPF | Bit | Set |", "|---|---:|---:|",
    ]
    for name, bit in KPF.items():
        if name in ("UPTODATE", "LRU", "ACTIVE", "MMAP", "ANON", "SWAPBACKED", "BUDDY", "COMPOUND_HEAD", "COMPOUND_TAIL", "HUGE", "RESERVED", "HWPOISON"):
            lines.append(f"| `KPF_{name}` | {bit} | {int(flags[name.lower()])} |")
    lines += [
        "", f"- page_mapcount: **{report['interpretation']['page_mapcount']}** (not a reference count or process ID)",
        "- page_owner: NOT AVAILABLE", "- process identity: NOT IDENTIFIED",
        "- `H700_MEMORY_LAYOUT_APPROVED`: `false`", "",
        "> " + report["safety_note"], "",
        "## Linux v4.9.170 source references", "",
        "- UAPI KPF definitions: " + SOURCES["uapi"],
        "- page flags/count implementation: " + SOURCES["proc_page"],
        "- internal KPF_RESERVED mapping: " + SOURCES["internal"], "",
        "> " + report["source_match_limit"], "",
    ]
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--out-dir", required=True, type=Path)
    args = parser.parse_args()
    try:
        report = decode_report(args.input.read_text(encoding="utf-8"))
        args.out_dir.mkdir(parents=True, exist_ok=True)
        (args.out_dir / "DRAM-PFN-METADATA.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        (args.out_dir / "DRAM-PFN-METADATA.md").write_text(render(report, str(args.input)), encoding="utf-8")
        print(json.dumps({"classification": report["classification"], "gate": report["gate"], "kpageflags": report["raw"]["kpageflags_u64_little_endian"], "page_mapcount": report["interpretation"]["page_mapcount"]}, indent=2))
        return 0
    except (OSError, UnicodeError, ValueError) as exc:
        print("DRAM PFN metadata audit failed closed: " + str(exc), file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
