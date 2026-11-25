# Linkslot 1.0.36 — Libretro VFS v3 service ABI feasibility (round 6)

- Date: 2026-11-23
- Feature branch commit under test: `3d6b8ba74248db66ee1a63d0e5b3271da1c2911e`.
- PR: [#16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16), draft; not merged.
- GitHub Actions: [run 45109949583](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45109949583), conclusion **SUCCESS**.
- Artifact: `linkslot-lk-rust-feasibility-45109949583`, ID `14789192113`, 82,003,215 bytes.

## What was tested

The host-only script `scripts/audit-libretro-vfs-v3-abi.py` builds a Rust `no_std` registry with a C ABI function table in the order required by Libretro VFS v3:
- v1 file operations: get_path/open/close/size/tell/seek/read/write/flush/remove/rename;
- v2: truncate;
- v3: stat/mkdir/opendir/readdir/dirent_get_name/dirent_is_dir/closedir.

Its environment callback accepts `RETRO_ENVIRONMENT_GET_VFS_INTERFACE` (45 plus the experimental flag), negotiates version 3 and rejects null data or a request for an unsupported newer version. The registry installs one backend table (currently a singleton).

## Results

| Test | Result |
|---|---:|
| AArch32 Rust no_std + C mock backend static link | PASS, ELF32/ARM |
| AArch64 Rust no_std + C mock backend static link | PASS, ELF64/AArch64 |
| Undefined symbol check for each ELF | PASS, none |
| x86_64 host callback harness | PASS, exit 0 |

The in-memory fake backend and host C harness exercise:
- VFS version negotiation, null data and unsupported version refusal;
- open mode validation and path escape refusal;
- file handle get_path/open/size/tell/seek/read/write/truncate/flush/close;
- stat, mkdir, rename and remove;
- directory open/readdir/name/is_dir/closedir, including hidden-entry filtering in the fake backend.

The host harness actually calls the returned VFS function table rather than merely checking its struct size.

## Important limits

- The backend is an in-memory test fake. It is not a FAT filesystem, SD/MMC block driver or persistent storage layer.
- The Rust registry is a singleton. It does not yet provide a per-instance context, per-core handle table, multi-thread synchronization or reentrant backend ownership.
- Path validation in production must canonicalize and constrain every operation beneath the designated SD-card root, reject traversal/absolute paths, and map filesystem errors consistently.
- The test proves ABI layout and function-table call flow only; it does not connect the interface to the existing Linkslot `slot-retro` crate or to mGBA.
- mGBA's current Generic `vfs-dirent.c` still uses POSIX dirent/stat/mkdir/remove and has not yet been adapted to Libretro VFS v3. VFS v3 has no rewind-directory function; a directory adapter can implement rewind by closing and reopening its stored path.
- Durability semantics (flush, atomic replacement, power-loss behavior) are unimplemented. The existing separation between instant resume state, battery save RAM and cloud sync must remain intact.

## Next integration work

1. Patch a temporary pinned mGBA Generic source copy so its `VDir` implementation uses Libretro VFS v3's opendir/readdir/name/is_dir/closedir callbacks; implement rewind as close+reopen.
2. Link the patched Generic mGBA archive again and confirm POSIX dirent symbols are gone.
3. Integrate the same VFS interface into Linkslot `slot-retro::environment()`.
4. Design the actual H700 SD/FAT backend and its handle table, path-security, locking and durability contracts. The current pinned MTK LK reference FS is not an H700 SD/FAT solution.
5. Only then perform a final mGBA + VFS link against the H700/LK platform layer.

## Overall validation boundary

The full workflow run also passed previous Rust no_std crate candidates, static Libretro core ABI, real mGBA Linux/Generic AArch32 archive builds, A32 BL33 layout and existing H700 compile probes. No firmware was generated, no SD/boot media was written and no payload was executed.
