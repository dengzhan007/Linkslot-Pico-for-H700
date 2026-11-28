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
    assert current["classification"] == "LINUX_ACTIVE_ANON_MAPPED_PAGE"
    assert current["gate"] == "BLOCKED_LIVE_LINUX_PAGE_REQUIRES_EXACT_PRESERVE_RESTORE"
    for name in ("uptodate", "lru", "active", "mmap", "anon", "swapbacked"):
        assert current["kpf_flags"][name]
    for name in ("buddy", "compound_head", "compound_tail", "huge", "reserved", "hwpoison"):
        assert not current["kpf_flags"][name]
    assert current["H700_MEMORY_LAYOUT_APPROVED"] is False

    mapped = module.decode_report(fixture(flags="0008000000000000", count="0100000000000000"))
    assert mapped["classification"] == "MAPPED_PAGE_TYPE_NOT_CONFIRMED"

    race = module.decode_report(fixture(flags="0008000000000000", count="0000000000000000"))
    assert race["classification"] == "INCONSISTENT_OR_RACY_KPAGE_METADATA"

    unmapped = module.decode_report(fixture(flags="0000000000000000", count="0000000000000000"))
    assert unmapped["classification"] == "NO_MAPPING_OBSERVED_AT_SAMPLE"
    assert unmapped["gate"] == "BLOCKED_OWNERSHIP_NOT_PROVEN"

    dual = fixture() + """[kpageflags_pfn_60000]
status=READ_OK
raw_8_bytes_hex=0000000000000000
[kpagecount_pfn_60000]
status=READ_OK
raw_8_bytes_hex=0000000000000000
[kernel_memory_iomem]
status=READ_OK
00000000-7fffffff : System RAM
[kernel_memory_zoneinfo]
status=READ_OK
Node 0, zone Normal
[kernel_memory_buddyinfo]
status=READ_OK
Node 0, zone Normal 1 2 3
[kernel_memory_pagetypeinfo]
status=READ_OK
Page block order
[kernel_memory_meminfo]
status=READ_OK
MemTotal: 2048000 kB
"""
    both = module.decode_report(dual)
    secondary = both["secondary_page_0x60000000"]
    assert secondary["pfn"] == "0x60000"
    assert secondary["classification"] == "NO_MAPPING_OR_RESERVED_FLAG_OBSERVED"
    assert secondary["gate"] == "BLOCKED_OWNERSHIP_NOT_PROVEN"
    assert secondary["ownership_proven"] is False
    assert secondary["free_scratch_eligible"] is False
    assert both["kernel_memory_context"]["iomem"]["status"] == "READ_OK"
    assert both["kernel_memory_context"]["meminfo"]["present"] is True

    absent = module.decode_report(fixture())
    assert absent["secondary_page_0x60000000"]["present"] is False
    assert absent["secondary_page_0x60000000"]["free_scratch_eligible"] is False

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
