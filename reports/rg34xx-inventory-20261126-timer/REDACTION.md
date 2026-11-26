# Publication redaction — 2026-11-26 timer inventory

This directory is the **complete** `build/rg34xx-inventory-20261126-timer/` output of
`scripts/run-rg34xx-inventory.ps1 -OutputDirectory .\build\rg34xx-inventory-20261126-timer`
(repo `8e7f8db`), with all 41 collected files preserved. Three files were redacted before
public upload because they carried the device serial number and/or a local workstation path.

Nothing was added, removed or reinterpreted beyond the substitutions below. No MAC address
was present (`mac_addr=`, `wifi_mac=`, `bt_mac=` are empty in the boot arguments).

## Substitutions

| File | Original | Published |
|---|---|---|
| `cmdline.txt` | device serial `snum=` and `androidboot.serialno=` (2 occurrences) | both replaced by `XXXXXXXXXXXXXXXXXXX` (same length, 19 bytes) |
| `live-fdt.dtb` | same serial as a string in the `chosen/bootargs` property at byte offsets **94984** (`snum=`) and **95105** (`androidboot.serialno=`) | both occurrences replaced in place by 19 `X` bytes; file size unchanged at 143 936 B, DTB structure and all other property bytes untouched |
| `live-fdt.json` | serial twice inside `chosen_bootargs`; `file` = local absolute Windows build path; `sha256` = original DTB hash | serial masked same as above; `file` → `live-fdt.dtb`; `sha256` updated to the hash of the published (masked) DTB so the report self-verifies |
| `env-report.json` | `source` = local absolute Windows build path | `source` → `env-partition.bin` |
| `toc1-extracted/dtb-report.json` | `file` = local absolute Windows build path | `file` → `toc1-extracted\items\dtb.bin` |

The DTB remains parseable: 573 nodes, `allwinner,h616` / `arm,sun50iw9p1`, with the
interrupt-controller and timer declarations intact (5 interrupt controllers, 2 timer nodes),
and `live-fdt.json` still validates as JSON.

## Original hashes (device-captured, before redaction)

| File | SHA-256 before | SHA-256 after |
|---|---|---|
| `live-fdt.dtb` | `46c22938d60a871b50e54b13212b3e1bea82c3ec937a3e098c27fde2ac155f4d` | `620b36d84fb7e714dcb56891f50802048d83c64ca924222a4a566f340ac7242e` |
| `live-fdt.json` | `be4d7914b6bac4b5b78ec80a63eceae2ff7e83e82aa66f6a474598239ea6bc16` | see `SHA256SUMS.txt` |
| `cmdline.txt` | `99b96313111a2730b2256c10191f85375f8f7f6f1ad9fda1932966b8c4787a42` | see `SHA256SUMS.txt` |

`raw-sha256.txt` is a **verbatim** copy of the device-side hash manifest and therefore still
lists the original `live-fdt.dtb` hash `46c22938…f4d`. That mismatch is expected and is the
record of the unredacted capture; it is not a checksum failure of the published file.

## Verification

A byte-level scan of every published file found:

- 0 occurrences of the 19-byte device serial;
- 0 colon-hex MAC-shaped strings;
- 0 occurrences of the workstation user name or local build path.

The unredacted capture remains only in the local `build/` directory and was never uploaded.
`SHA256SUMS.txt` covers all published files (including this note) except itself.
