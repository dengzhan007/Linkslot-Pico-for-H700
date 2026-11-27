# RG34XX native generation / capsule provenance

**Status:** host-side exact reproduction **PASS**; device/hardware qualification **NOT PERFORMED**.
**Target:** RG34XX non-SP, Allwinner H700, 1 GiB LPDDR4.
**Builder source pin:** BaseOS-MultiFrontend `cbe253a43f46cf551cc9a6c79a84015bde3686cf`.

## 1. Reproduction workflow

The dedicated BaseOS workflow
[RG34XX native generation reproduction #45301188593](https://github.com/dengzhan007/BaseOS-MultiFrontend/actions/runs/45301188593)
completed successfully on 2026-11-24 (workflow head
`448d7f4f332cbfc4b316ad53aa361dbe53145ed7`). It checked out the builder
source at `cbe253a43f46cf551cc9a6c79a84015bde3686cf` and separately checked
out the expected deployed capsule from commit
`339e1cd5f84a6178c496257f835beb3388b8d9f9`,
path `manifest/native/rg34xx/capsule.bin`.

The workflow artifact is
[RG34XX-SPL-Capsule-Reproduction-448d7f4f](https://github.com/dengzhan007/BaseOS-MultiFrontend/actions/runs/45301188593/artifacts/14877904442)
(SHA-256
`5561d625aecc872954bc920b00d752992ccb5be314ed4fb8921fd2e149f6824a`).
It contains native generation manifest/policy/comparison, native generated
header, raw-capsule SPL manifest/build logs, and capsule verification/
comparison reports. It intentionally does not publish raw prepared stock
media or installable outputs as a release.

## 2. Exact prepared inputs restored and checked

The workflow invokes the pinned repository's `fetch-prepared.sh rg34xx`.
The committed prepared-bundle pin is:

- Bundle: `baseos-prepared-20260914.tar.zst`
- Release URL:
  `https://github.com/pvaibhav/BaseOS/releases/download/prepared-20260914/baseos-prepared-20260914.tar.zst`
- Bundle SHA-256:
  `86a0186b5ac8ab907dc1d5c24220c9fcd94f38dd9dc3fddb7fb5cf59d4c7719f`
- Prepared model manifest:
  `manifest/prepared/rg34xx.json`, Git blob
  `8fadc1784bf78460edb9039115411debdfc738e7`.

The prepared manifest pins the entire `boot-prefix.img` to 222,298,112
bytes and SHA-256
`fb75bc4c835fa74ff41c0a1fdbe79b8871a960fc249716847fb7be529835c3c3`.
The reproduction workflow verifies this full file, then derives and hashes
the precise inventory inputs used by `native/build.py`:

| Input | Size | SHA-256 | Result |
|---|---:|---|---|
| `inventory/prefix.bin` (first 73,728 sectors) | 37,748,736 B | `725f8a1552b90659da9dee5e71ef02a3d914ef7f8b5e433584b85829df65b650` | PASS |
| `inventory/boot0.bin` (prepared stock Boot0) | 65,536 B | `b6edbf39af0bf1f35a7e2fd991c8966b04d061e247c81985d2516f695464049b` | PASS |
| `inventory/env.bin` | 131,072 B | `d1a0856a8fb4c6f3b785943929a59206424a189f92efcbea48c26b31f472a396` | PASS |
| TOC1 source package `boot-gzip/boot-package.bin` | 1,310,720 B | `56371d984e29e74db63009ce27f829eb0a55ce9007ee5e40c52479b22418af3d` | PASS |
| original `boot-gzip/boot.img` | 7,161,856 B | `5b1e30930a9b0e7642da39ce5c699cfdb03970346c6f2757dcfb063bfc8a0b51` | PASS |
| source kernel | 17,461,256 B | `39cf8c3eb9a75ff0eda51b4074cd65860c5aa5386093bcaf69339819421d84b7` | PASS |
| U-Boot v2026.10 source archive | archive | `f943a88389c8fd70a6b0d061a88267465e143c7ca6ea2b75837ab20f5b60c739` | PASS |

The prepared-source package/Boot0 inputs are different layers from the
captured recovery TOC and 128 KiB capsule:

- `56371d98…` is the TOC1 package read by the native builder as source input.
- `e6c20e3a…` is the separately generated/deployed recovery TOC.
- `b6edbf39…` is the 64 KiB prepared stock Boot0 embedded inside the capsule.
- `bf886995…` is the full 128 KiB raw capsule, which is also the target's
  captured Boot0-primary image.

These hashes are not expected to be equal.

## 3. Exact native generation and image comparison

The workflow's `build/native-reproduction/comparison.json` reports all four
expected output comparisons as true:

| Output | Size | SHA-256 | Result |
|---|---:|---|---|
| Generation | 32 B | `022f0d8816c861ebbec05f0d0d7f513d6b974d9930661d20fa8ad51b7b4c0fee` | PASS |
| A32 loader | 32,768 B | `48b2a837ac5d8daad8f524c976f2b3911d26a838f8b68fa695e3283412dd8590` | PASS |
| Native firmware | 287,232 B | `dc9dc3ae66b30244911e7054ca66136991626cabb0d59c796c4b253a126f28e9` | PASS |
| Active BOL4 image | 8,556,544 B | `d71e5bd87b23fb409d7a837cf90d153ae117af518fe6ee330a96d2c7a5f76fc3` | PASS |

The raw-capsule build then produces:

- AArch64 SPL: 49,152 B, SHA-256
  `cdfc6d480a257c5ccc3f5826c4e6cbcf499163c57119390e8bebeba6964a8570`.
- Generation cookie: `0xf7bed1c8`.
- Capsule: 131,072 B, SHA-256
  `bf886995745a0eb9b5030296f9f6163035e164b3a82183716e67ff876319289f`.

The authoritative capsule comparison file reports:
`capsule_byte_exact_match=true`, `different_byte_count=0`, 36 raw-capsule
verification cases passed, and 2 entry verification cases passed. The
expected bytes are the capsule committed under
`manifest/native/rg34xx/capsule.bin` at commit
`339e1cd5f84a6178c496257f835beb3388b8d9f9`.

**Correction to the earlier active-loader provenance note:** its 2026-11-22
16-combination search correctly described the evidence available then, but
the conclusion that exact generation could not be reproduced is superseded
by this dedicated #45301188593 workflow. The missing prefix and stock Boot0
were subsequently restored from the pinned prepared bundle and verified;
the same run then regenerated the expected outputs and byte-exact capsule.

## 4. Link to the target-device evidence

The 2026-11-27 read-only RG34XX inventory reports the 128 KiB Boot0 primary
SHA-256 as `bf886995745a0eb9b5030296f9f6163035e164b3a82183716e67ff876289f`.
This equals the rebuilt/reference capsule hash above. The capsule's embedded
64 KiB stock Boot0 hash `b6edbf39…` equals the prepared source Boot0 pin.
The target's live `baseos,native-boot` generation marker equals the
reproduced generation. The active p4+40 MiB firmware and p4+48 MiB image
hashes also match the reproduced outputs.

Together, these are strong host-level byte-identity links between the
published prepared inputs, native build, committed deployment capsule, and
target-device inventory/live marker.

## 5. What this closes — and what it does not

**Closed:** exact build/source provenance for the RG34XX native generation
and capsule can be reproduced from a pinned builder source, a checksum-pinned
prepared bundle and the original CI upgrade artifact. The previous “missing
prefix/Boot0” build-provenance blocker is no longer OPEN.

**Not closed:** the reproduction workflow does not boot hardware. Its
manifests explicitly state `hardware_qualified=false` and
`durable_install_eligible=false`. It does not prove BL33 UART access,
physical test-pad wiring, clock/reset/pinmux/firewall state, safe DRAM
ownership, CPU1–CPU3 hold-off, cold boot, recovery, or a diagnostic payload
run. Do not write or install anything based on this host reproduction.

M1-S1 remains OPEN pending exact board/revision physical signal qualification
and BL31→BL33 entry-state evidence. G2 and G3 remain OPEN.
