# Publication redaction — 2026-11-27 UART inventory

This directory is the **complete** `build/rg34xx-inventory-20261127-185937/` output of
`scripts/run-rg34xx-inventory.ps1` (repo `e87de1e`), with all 42 collected files preserved.
Five files were redacted before public upload because they carried the device serial number
and/or a local workstation path. Nothing was added, removed or reinterpreted beyond the
substitutions below.

No MAC address was present: `mac_addr=`, `wifi_mac=` and `bt_mac=` are empty in the boot
arguments. `/sys/class/sunxi_info/sys_info` and `/dev/sunxi_soc_info` were deliberately not
read, so no chipid is present either.

## Substitutions

| File | Original | Published |
|---|---|---|
| `cmdline.txt` | device serial in `snum=` and `androidboot.serialno=` (2 occurrences) | both replaced by `XXXXXXXXXXXXXXXXXXX` (same length, 19 bytes); file stays 700 B |
| `live-fdt.dtb` | same serial as a string inside `chosen/bootargs` at byte offsets **94984** (`snum=`) and **95105** (`androidboot.serialno=`) | both replaced in place by 19 `X` bytes; size unchanged at **143 936 B**, DTB structure and all other property bytes untouched |
| `live-fdt.json` | serial twice inside `chosen_bootargs`; `file` = local absolute Windows build path | serial masked as above; `file` → `live-fdt.dtb` |
| `env-report.json` | `source` = local absolute Windows build path | `source` → `env-partition.bin` |
| `toc1-extracted/dtb-report.json` | `file` = local absolute Windows build path | `file` → `toc1-extracted\items\dtb.bin` |

The JSON files were edited at text level and keep their original key order, indentation and
value formatting; only the substituted strings and the resulting byte counts differ.

## Hashes (device-captured before redaction → published)

| File | SHA-256 before | SHA-256 after |
|---|---|---|
| `cmdline.txt` | `99b96313111a2730b2256c10191f85375f8f7f6f1ad9fda1932966b8c4787a42` | `f597cff02c42289ff72efe4d0437de223420a7bf6bf0853e4a1e8bbe4b084061` |
| `live-fdt.dtb` | `b761cbcca4087951b3d57fc40b5e84d100c0a51dad6b6b98ab8cf74527a45a89` | `ad62516767719a197c7e2fed1c7ef6df91b0d3d040742a9a0045feddc338593e` |
| `live-fdt.json` | `39f9ad63039ecbcba32c0189fbad48de246b969f5ae736c18584260eda3a45e9` | `80798a43dece202ffe5f6ed1965c529fda547dd29a5a1f407a3506b5a2c73f76` |
| `env-report.json` | `8ca257aa09aa1a45922bb25bd2f671a386c75cc91e5785eb1e3616de78410d69` | `a815ef317c0663e25826e800133e40c18181c9f8bcf7f5ba362ad15accf81837` |
| `toc1-extracted/dtb-report.json` | `d547884bca5d7fd83bf4bcac1f9b4db1285cc091e27784cd4254cf9626c9f8a7` | `c8078588a981c31aca7153e8531e03ffe5a0906a990b7efadf5c6808bf08c414` |

## Two files intentionally keep the pre-redaction DTB hash

- `raw-sha256.txt` is a **verbatim** copy of the device-side manifest, so it still lists
  `live-fdt.dtb` as `b761cbcc…45a89`.
- `memory-map-report.json` records that same hash in its `fdt_sha256` field, because it was
  generated from the captured DTB before masking.

Both mismatches are expected and are the record of the unredacted capture; they are not
checksum failures of the published `live-fdt.dtb`, which is listed correctly in
`SHA256SUMS.txt`. The unredacted DTB itself was not uploaded and no copy was retained.

## Verification

After redaction, of the 42 published files:

- 0 occurrences of the 19-byte device serial (byte-level scan, including binaries);
- 0 colon-hex MAC-shaped strings;
- 0 occurrences of the workstation user name, host name or local build path.

Structural checks on the redacted files:

- `live-fdt.dtb` still parses to **573 nodes**, `sun50iw9`, 5 interrupt controllers,
  2 timer nodes, and its `chosen_bootargs` shows the masked `snum=` / `androidboot.serialno=`;
- all 8 published JSON files parse as valid UTF-8 JSON.

`SHA256SUMS.txt` covers every published file (including this note) except itself.
`hardware_applied = 0`.
