# RG34XX Boot0 capture size compatibility

Date: 2026-11-22

## Summary

The read-only RG34XX/H700 inventory pipeline now supports Boot0 images whose
`eGON.BT0` header declares either 0x10000 (64 KiB) or 0x20000 (128 KiB) bytes.
The collector always captures 128 KiB and the inspector crops to the size
declared by the header before validating the checksum.

No firmware write is performed. The device-side collector only reads the whole
disk block device (`/dev/mmcblk0`); its output is written under `/tmp` and then
pulled into `build/` on the host.

## Symptom

Running the Windows wrapper:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\run-rg34xx-inventory.ps1
```

completed the device-side collection and the `adb pull`, then failed with:

```
error: Boot0 header declares 0x20000 bytes, expected 0x10000
Primary Boot0 inspection failed.
```

## Root cause

* `scripts/collect-rg34xx-readonly-inventory.sh` read each Boot0 copy with
  `dd bs=512 count=128` (64 KiB).
* `tools/inspect_h700_boot0.py` hardcoded `BOOT0_BYTES = 0x10000` and rejected
  both a different captured length and a different header-declared size.
* The live device used for verification declares 0x20000 bytes at the header's
  image-size field (offset `0x10`). The `eGON.BT0` signature itself was valid.

This was a tooling assumption, not a corrupted Boot0 image.

## Changes

| File | Change |
| --- | --- |
| `scripts/collect-rg34xx-readonly-inventory.sh` | Boot0 copies at LBA 16 and 512 are now captured with `count=256` (128 KiB). TOC1 capture is unchanged. |
| `tools/inspect_h700_boot0.py` | Accepts a capture of at least 64 KiB, validates the header-declared size as 512-byte-aligned and within the captured length, crops to that size, and computes the checksum over the declared image. Reports `captured_bytes` alongside `bytes`. |
| `tools/test_inspect_h700_boot0.py` | Adds a 128 KiB image case and a case where a 64 KiB image is captured with a 64 KiB tail. |
| `tools/test_inventory_script_safety.py` | Updates the expected sector counts to 256. |

The extra tail bytes in a 128 KiB capture are ignored whenever the header
declares 64 KiB, so older cards remain compatible.

## Verification

All 24 unit tests pass:

```text
python -m unittest discover -s tools -p 'test_*.py'
```

A full read-only inventory run succeeded against device `2c00107411818711bd2`
on `/dev/mmcblk0` at `2026-11-22 18:17 UTC`, output:

`build/rg34xx-inventory-20261122-181713`

Results:

* Boot0 primary: checksum valid, SHA-256
  `bf886995745a0eb9b5030296f9f6163035e164b3a82183716e67ff876319289f`
* Boot0 secondary: checksum valid, SHA-256
  `4701a85e4db7e8339b91b92f691a83cc954ec63a09369087c2e41d80eafa9f5d`
* The primary parameter block at `0x38` reports LPDDR4 / 672 MHz on both
  copies. The alternate blocks at `0x9f4` and `0xa74` differ between copies,
  consistent with documented training/header mutation; no DRAM map conclusion
  is drawn from raw words alone.
* TOC1: checksum valid, SHA-256
  `e6c20e3a54c51b9336c798e3a4fa1ca6c45b28c6f70456e2db9c252aa4ba4604`,
  four items (`u-boot`, `monitor`, `dtbo`, `dtb`) extracted to
  `toc1-extracted/`.

## Operational notes

* The device-side collector prints `No write was issued to /dev/mmcblk0.`
* `adb` and Python 3 must be available in `PATH`. On the verification host,
  `adb` came from a local platform-tools install and Python 3 came from the
  WorkBuddy bundled interpreter, because the WindowsApps `python.exe` alias is
  a non-functional stub.
* The body hash in the Boot0 report excludes the mutable first 512-byte
  sector; it is not a replacement for the full-image checksum.
