#!/usr/bin/env python3
"""Host tests for the RG34XX Linux v4.9.170 KPF report decoder."""
from __future__ import annotations
import importlib.util
from pathlib import Path

root = Path(__file__).resolve().parents[1]
path = root / "scripts/audit-rg34xx-dram-pfn-metadata.py"
spec = importlib.util.spec_from_file_location("dram_pfn_metadata", path)
if spec is None or spec.loader is None:
    raise RuntimeError("cannot load PFN decoder")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


def fixture(flags="6858000000000000", count="0100000000000000", kernel="4.9.170"):
    return f"""kernel_release={kernel}
target_pfn=0x40000 decimal=262144
uid=0
page_size_bytes=4096
[kpageflags]
status=READ_OK
raw_8_bytes_hex={flags}
[kpagecount]
status=READ_OK
raw_8_bytes_hex={count}
"""


def main():
    current = module.decode_report(fixture())
    assert current["raw"]["kpageflags_u64_little_endian"] == "0x5868"
    assert current["interpretation"]["page_mapcount"] == 1
    assert current["classification"] == "MAPPED_PAGE_TYPE_NOT_CONFIRMED"
    assert current["gate"] == "BLOCKED_LINUX_PAGE_MAPPING_PRESENT"
    for name in ("uptodate", "dirty", "active", "mmap", "swapcache", "swapbacked"):
        assert current["kpf_flags"][name]
    for name in ("lru", "anon", "buddy", "compound_head", "compound_tail", "huge", "reserved", "hwpoison"):
        assert not current["kpf_flags"][name]
    assert current["H700_MEMORY_LAYOUT_APPROVED"] is False

    mapped = module.decode_report(fixture(flags="0008000000000000", count="0100000000000000"))
    assert mapped["classification"] == "MAPPED_PAGE_TYPE_NOT_CONFIRMED"

    race = module.decode_report(fixture(flags="0008000000000000", count="0000000000000000"))
    assert race["classification"] == "INCONSISTENT_OR_RACY_KPAGE_METADATA"

    unmapped = module.decode_report(fixture(flags="0000000000000000", count="0000000000000000"))
    assert unmapped["classification"] == "NO_MAPPING_OBSERVED_AT_SAMPLE"
    assert unmapped["gate"] == "BLOCKED_OWNERSHIP_NOT_PROVEN"

    for bad in (fixture(kernel="4.9.171"), fixture(flags="685800")):
        try:
            module.decode_report(bad)
        except ValueError:
            pass
        else:
            raise AssertionError("invalid report accepted")

    print("PASS: v4.9.170 KPF decode, mapcount semantics, and fail-closed outcomes")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
