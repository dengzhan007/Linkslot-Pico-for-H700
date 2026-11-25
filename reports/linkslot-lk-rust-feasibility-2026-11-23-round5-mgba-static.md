# Linkslot 1.0.36 — real mGBA AArch32 static archive and Generic VFS comparison

- Date: 2026-11-23
- Feature branch commit under test: c1ccedde479178c11f54d81437c653665e3e6437.
- PR: [#16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16), draft; not merged.
- GitHub Actions: [run 45108202490](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45108202490), conclusion **SUCCESS**.
- Artifact: linkslot-lk-rust-feasibility-45108202490, ID 14787663902, 69.7 MB.
- mGBA source: libretro/mgba @ e31759b24e7a4e3899285ff720d7b573ac328ae7, the commit pinned by Linkslot's 1.0.36 manifest.
- Architecture flags: Cortex-A53, A32 ARM state, NEON/VFPv4, softfp ABI.
- Linkslot upstream snapshot remains immutable.

## 1. Real mGBA builds

The workflow built the real mGBA Libretro target twice as static archives:
- CMake Linux target, with Linux platform sources.
- CMake Generic target, which avoids selecting the Linux-specific OS source group.

Both builds enabled LIBMGBA_ONLY, BUILD_LIBRETRO, LIBRETRO_STATIC, M_CORE_GBA and M_CORE_GB; both used Release. All Libretro entry points listed in src/platform/libretro/libretro.c were compile-time renamed from retro_* to mgba_retro_*.

| Result | Linux target | Generic target |
|---|---:|---:|
| Archive | mgba_libretro-aarch32-static.a | mgba_libretro-generic-aarch32-static.a |
| Archive size | 2,921,494 bytes | 2,878,912 bytes |
| Object members | 115 | 115 |
| Namespaced Libretro exports | 25/25 | 25/25 |
| Unprefixed global retro_* definitions | 0 | 0 |
| Unique external undefined symbols | 81 | 67 |

This establishes a real AArch32 source compilation and static archive. It does **not** establish a final LK link or execution.

## 2. What Generic removed

The Generic CMake build removes Linux/glibc-specific dependencies that the Linux archive had:
- file descriptor calls: open64, read, write, close, lseek64;
- Linux file mapping/sync calls: mmap64, msync, munmap, fsync, ftruncate64;
- glibc fortified and stack-check calls, including __memcpy_chk, __snprintf_chk, __stack_chk_fail and related variants.

It replaces the Linux VFS surface with stdio/dirent-style symbols, including fopen64, fread, fwrite, fseek, ftell, fflush, fclose, opendir, readdir64 and closedir.

This is a smaller and more plausible adaptation surface for LK, but Generic remains a CMake platform choice, **not** a true bare-metal runtime: this probe still used the arm-linux-gnueabi compiler and glibc sysroot.

## 3. Remaining Generic external references

There are 67 unique external symbol names. They fall into these groups:

| Group | Symbols / examples | H700/LK implication |
|---|---|---|
| Allocation and memory | malloc/calloc/realloc/free, memcpy/memmove/memset/memcmp | LK has similarly named APIs and an allocator wrapper, but its heap is currently uniprocessor-only; allocator use remains blocked on the SMP/secondary-core decision. |
| String and parsing | strchr/strcmp/strcpy/strdup/strlen/strncat/strncmp/strncpy/strrchr/strcasecmp, strtof_l, __ctype_b_loc, __isoc23_sscanf/strtol/strtoul, __errno_location, newlocale/freelocale | LK has basic string/ctype/strtoul functions, but no complete locale, strtof_l, sscanf/strtol or errno implementation matching glibc. |
| stdio | fopen64/fclose/fread/fwrite/fseek/ftell/fflush/fgets/vfprintf/printf/sprintf/snprintf/vsnprintf/stdout/putchar | LK stdio.h only has console putc/puts/getc; it does not provide FILE streams. Prefer VFS callback routing over a fake general-purpose FILE implementation. |
| Directory and metadata | opendir/readdir64/closedir/rewinddir/getcwd/__stat64_time64/mkdir/remove | LK reference has no POSIX dirent/stat filesystem layer. Provide a VFS directory adapter backed by H700 SD/FAT services. |
| Time and locale | __time64/__gettimeofday64/__localtime64/__localtime64_r/__mktime64/strftime/getenv | Adapt using the H700 RTC and preserve Linkslot timezone/date semantics; environment variables may become a fixed device profile/config table. |
| Math and compiler runtime | cbrtf/exp2f/fmaxf/fminf/hypotf/pow/sincosf/div/__aeabi_ldivmod | No matching libm implementation was found in the pinned LK source tree. Select an ARM-compatible math/runtime library or write/test the small required subset. |
| Process termination | abort | Map to LK panic/fatal path with controlled diagnostics; never leave host process semantics. |

The exact unique symbol list is included in mgba-generic/generic-external-undefined.txt in the workflow artifact.

## 4. Linkslot Libretro VFS gap

mGBA's pinned src/platform/libretro/libretro-vfs.c calls RETRO_ENVIRONMENT_GET_VFS_INTERFACE, requesting API v2 and falling back to v1. This allows mGBA's VFile abstraction to route individual file operations through frontend callbacks: open, close, size, seek, read, write, flush and (v2) truncate. It also emulates map/unmap using the VFS read/write functions.

However:
- Linkslot's current slot-retro::environment() supports save/system directory queries but does not define or implement GET_VFS_INTERFACE (command 45 with the experimental flag).
- mGBA's Generic src/util/vfs/vfs-dirent.c still uses POSIX DIR*/opendir/readdir/closedir/stat/mkdir/remove for directory traversal and metadata.
- Therefore a frontend file VFS alone does not remove the Generic directory backend dependency.

### Recommended H700 storage boundary

Implement Libretro VFS v2 in Linkslot and back both file and directory operations with a single H700 storage service:
- file: open/close/size/truncate/tell/seek/read/write/flush;
- directory: stat/opendir/readdir/name/is_dir/closedir/rewind;
- filesystem changes: mkdir/remove and required path metadata;
- reject path traversal, absolute path escape and invalid card-root paths.

Then adapt mGBA's directory VFS to the same callbacks (or add a small H700 VDir backend). This avoids re-creating POSIX file descriptors and FILE* inside LK.

Linkslot slot-store is not yet this storage service. Its source is deeply tied to std::fs, std::path, File, rusqlite, std::thread and Linux syncfs. The VFS backend therefore needs to be part of the broader storage/HAL separation, not a mGBA-only patch.

## 5. Save semantics that must remain unchanged

VFS must preserve Linkslot's distinction between:
- instant resume state;
- battery save RAM;
- optional cloud sync.

The new backend must not make power-off automatically write a battery save or trigger cloud sync. The project should retain the existing user-triggered battery/cloud semantics and durability guarantees when implementing file flush/atomic replacement.

## 6. Still not verified

- No final link of the mGBA archive against the actual LK headers/libraries.
- No H700 SD/FAT file or directory API implementation.
- No Libretro VFS v2 registration in Linkslot.
- No RTC/timezone adapter, locale/parser shims, math library or final EABI runtime resolution.
- No actual mGBA ROM was run, no frame/audio/save-state was exercised on H700.
- No LK scheduler/SMP fix; secondary-core state remains unknown.
- No firmware or flashable image produced; no SD/boot media writes and no payload execution.

## Next step

Design the Linkslot-owned VFS service/API independently of the UI, with a host fake backend and tests for path containment, file handle lifecycle, seek/truncate/flush, directory iteration, rename/replace durability and error mapping. Then expose that service as Libretro VFS v2 and mGBA's directory adapter. This creates one storage boundary reusable by ROM scanning, BIOS, states, battery saves and the frontend's existing web save endpoints.
