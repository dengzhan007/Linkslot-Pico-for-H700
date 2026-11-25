# RG34XX native build inputs recovered from the 05c39a7 CI artifact

Date: 2026-11-24

## Source and method

The BaseOS-MultiFrontend workflow run
[44644638985](https://github.com/dengzhan007/BaseOS-MultiFrontend/actions/runs/44644638985)
was built from commit `05c39a7fa761145f073982012de039d39a5e9083`.
Its A/B upgrade artifact was downloaded (artifact ID `14588761000`, ZIP size
71,851,685 bytes) and inspected locally without device access.

The artifact contains a `.bosupd` tar with `rootfs.img.gz`; the compressed rootfs
is an ext4 image. Read-only extraction of the rootfs yielded the following source
and generated files under `/usr/share/baseos/`. No files were uploaded as binaries.

## Exact build inputs recovered

| Input | Recovered path | Size | SHA-256 | Check |
| --- | --- | ---: | --- | --- |
| TOC1 source package | `boot-gzip/boot-package.bin` | 1,310,720 | `56371d984e29e74db63009ce27f829eb0a55ce9007ee5e40c52479b22418af3d` | **Exact target pin** |
| Original gzip boot image | `boot-gzip/boot.img` | 7,161,856 | `5b1e30930a9b0e7642da39ce5c699cfdb03970346c6f2757dcfb063bfc8a0b51` | Matches adjacent manifest |
| Source kernel SHA-256 | boot-gzip manifest | 17,461,256 output bytes | `39cf8c3eb9a75ff0eda51b4074cd65860c5aa5386093bcaf69339819421d84b7` | Matches BOL4 output |
| Boot0 source | `native/capsule.bin` at file offset `0x1000`, length 65,536 | 65,536 | `b6edbf39af0bf1f35a7e2fd991c8966b04d061e247c81985d2516f695464049b` | Exact RG34XX compatibility pin |
| Environment input | full image p3 first `0x20000` bytes | 131,072 | `d1a0856a8fb4c6f3b785943929a59206424a189f92efcbea48c26b31f472a396` | Matches vendor-handoff environment pin |

The extracted `boot-gzip/manifest` also records:

- original source p4 SHA-256: `bc53333953f5f6e0962705e08f626f809d6afad88fcd714e2af96416b7f1afdf`
  (matches the prepared-source metadata's preserved p4 hash);
- original Boot0 SHA-256: `b6edbf39af0bf1f35a7e2fd991c8966b04d061e247c81985d2516f695464049b`;
- the 17,461,256-byte decompressed kernel hash above.

This means the previously missing `boot-package.bin` is now recovered from the **boot-gzip input directory**, not from `native/recovery-toc.bin`.

## Do not confuse source TOC1 with recovery TOC1

The separate `native/recovery-toc.bin` is 1,310,720 bytes but has SHA-256
`e6c20e3a54c51b9336c798e3a4fa1ca6c45b28c6f70456e2db9c252aa4ba4604`.
That is the TOC1 currently deployed as the recovery copy. It is **not** the
source `boot-gzip/boot-package.bin` and does not match the native build pin.
The two files serve different roles and must not be substituted for one another.

## Generated-output cross-check

The same rootfs contains native generation
`022f0d8816c861ebbec05f0d0d7f513d6b974d9930661d20fa8ad51b7b4c0fee`,
capsule `bf886995745a0eb9b5030296f9f6163035e164b3a82183716e67ff876319289f`,
firmware `dc9dc3ae66b30244911e7054ca66136991626cabb0d59c796c4b253a126f28e9`,
and active image
`d71e5bd87b23fb409d7a837cf90d153ae117af518fe6ee330a96d2c7a5f76fc3`.
These hashes agree with the existing `manifest/native/rg34xx` output and the
local p4+40 / p4+48 extraction evidence.

## Remaining prefix limitation

The exact prepared `boot-prefix.img` is still not recovered from this Actions
artifact. The prepared manifest pins it at 222,298,112 bytes and SHA-256
`fb75bc4c835fa74ff41c0a1fdbe79b8871a960fc249716847fb7be529835c3c3`.

The full 1.5 GB A/B test image from this artifact is not a substitute: its first
222,298,112 bytes hash to
`e5bb389b7eb3df83bd88db7ffb6a1298d3b1f127fed2ce5a8d8b61fcaa670f8a`.
Its GPT uses the composed A/B layout and the p2/p4 regions have been changed
during image assembly. Do not use that slice as the prepared source prefix.

**Status:** TOC1 source package, gzip boot image, Boot0, env and existing generated
native outputs are recovered and cross-checked. Exact source-prefix byte recovery
and unmodified end-to-end generation reproduction remain open. This does not
authorize firmware flashing or change the hardware NO-GO status.
