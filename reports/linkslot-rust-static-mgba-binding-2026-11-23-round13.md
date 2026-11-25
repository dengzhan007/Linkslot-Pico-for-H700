# Linkslot Rust static mGBA binding — round 13

Date: 2026-11-23  
Validated workflow: [45128096825](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45128096825)  
Validated feature commit: de655d3 (full SHA recorded in the Actions run)  
Immutable Linkslot source snapshot: upstream/linkslot @ 8590d6ff1b2caa26118051069c975106fbb14bd3

## Result

**PASS — the 21 namespaced mGBA symbols are now mapped into the real Rust slot-retro::Api struct and exercised through LibretroCore::open_static().**

The integration is applied in a temporary CI checkout of the supplied Linkslot 1.0.36 snapshot. The immutable upstream/linkslot branch is not modified.

## Implementation

Added platform/h700/runtime/mgba_static.rs. The module:
- Declares every required namespaced mGBA Libretro symbol with Rust C-ABI signatures and explicit link_name attributes.
- Builds Linkslot's actual Api struct in the exact field order, including optional cheat reset/set, initialization/deinitialization, version query, AV metadata, environment/video/audio/input callbacks, controller selection, game load/unload/run, serialization and memory access.
- Is compiled only with the static-mgba Cargo feature.
- Provides process-lifetime function pointers into the statically linked mGBA archive; it does not create or load a shared object.

Added platform/h700/runtime/tests/mgba_static_stub.c for host testing. It exports the same 21 names and compatible signatures, plus init/deinit counters. The test patches a temporary Linkslot checkout to register the Rust module and runs the real LibretroCore::open_static() path against those C stubs.

## Validation

Main feasibility workflow 45128096825 completed successfully.

The dedicated static mGBA binding test passed with 63 tests and zero failures. The new regression:
1. Obtains the Rust API table through mgba_static_api().
2. Checks that the linked API version is the Libretro version expected by Linkslot.
3. Verifies both optional cheat entry points are populated.
4. Opens the table through LibretroCore::open_static().
5. Checks system and save directory registration.
6. Verifies the C init counter changes from 0 to 1 and deinit remains 0 while open.
7. Drops the core and verifies deinit changes to 1.

This is stronger than the previous C-only table probe: the Rust module now provides the actual Api struct consumed by Linkslot. The C stub is still a test double, not the real mGBA implementation. Real mGBA symbol existence and the AArch32 whole-archive/newlib final link are independently covered by the preceding probe against the actual archive.

The same workflow re-ran the earlier checks, including VFS v3 production callback routing, dynamic-default and static-only slot-retro tests, Generic and arm-none-eabi/newlib mGBA archive builds, the actual mGBA VDir fake-provider host test, H700 newlib adapter validation, the real 21-symbol mGBA archive link, and the synthetic LK AArch32 layout audit.

## Not yet complete

- The slot application core-selection layer has not yet been changed to prefer the built-in static mGBA table. That is the next integration step.
- The Rust static binding has been exercised against same-name C stubs, not an actual running mGBA core or ROM.
- The actual archive symbol resolution is a link proof; it does not demonstrate runtime startup, frame/audio/input behavior or save persistence.
- slot-retro and the broader Linkslot workspace still depend on std and host OS facilities.
- No H700 SD/block/FAT32/exFAT backend, path resolver, durable flush/rename, or persistent save/state service exists.
- No complete LK application, actual LK heap/UART/RTC binding, proven single-core allocator invariant, H700 memory-ownership map or BL33 handoff has been validated.
- Boot/handoff remains **NO-GO**. No firmware write, SD/boot-area write, payload execution or RG34XX device test was performed.

## Next step

Integrate a slot-level static-mgba feature and select the built-in Rust API by default for Core::Mgba when SLOT_CORE is not explicitly overridden. Preserve existing dynamic .so search for ordinary builds and explicit overrides. Add tests proving both selection and override precedence. Only after that should the full slot application and remaining platform/runtime dependencies be assessed.

## References

- Draft PR #16: https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16
- Feature branch: https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/porting/linkslot-1.0.36-feasibility
- Round 12: https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/linkslot-mgba-static-api-link-2026-11-23-round12.md
