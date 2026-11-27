# RG34XX fixed DRAM-source PFN metadata probe — 2026-11-27

Read-only probe of the **page metadata** for the physical page containing the stock BL31
DRAM source window (`[0x40000000, 0x40000140)`), using
`scripts/collect-rg34xx-dram-pfn-readonly.sh` from the `porting/linkslot-1.0.45-feasibility`
branch at `3f61baa`.

```
adb push scripts/collect-rg34xx-dram-pfn-readonly.sh /data/local/tmp/rg34xx-dram-pfn-readonly.sh
adb shell id -u                                    # 0
adb shell sh /data/local/tmp/rg34xx-dram-pfn-readonly.sh > dram-retention-pfn.txt
```

Root check returned `0`, as required; no privilege escalation and no boot-argument change
was attempted. The script exited 0 and reported `probe_status=COMPLETE_READ_ONLY_METADATA_ONLY`.

## Result

| Item | Value |
|---|---|
| kernel | `4.9.170` |
| page size | 4096 B |
| target page / PFN | `[0x40000000, 0x40001000)` / `0x40000` (262144) |
| `kpageflags` entry offset | 2 097 152 B (`262144 × 8`) |
| `/proc/kpageflags` | `READ_OK`, raw 8 bytes `6858000000000000` |
| `/proc/kpagecount` | `READ_OK`, raw 8 bytes `0100000000000000` |
| `page_owner` | `NOT_PRESENT` / not readable |
| config | `CONFIG_PROC_PAGE_MONITOR=y`, `CONFIG_DEBUG_VM=y` |
| safety note emitted by script | `H700_MEMORY_LAYOUT_APPROVED remains unset; no ownership approval implied` |

The script deliberately **defers decoding** of the raw flag word until the flag definitions
of the matching 4.9.170 kernel are checked. That is preserved verbatim here: no
reinterpretation, no DRAM page read, no SMC/MMIO call and no suspend/resume operation.

## Files

| File | Content |
|---|---|
| `dram-retention-pfn.txt` | Verbatim device output (1053 bytes, 30 lines). |
| `SHA256SUMS.txt` | SHA-256 of the file above, excluding itself. |

No device serial, MAC address, host user name or host path is present in the output
(byte-level scan: all zero hits). `hardware_applied = 0`.
