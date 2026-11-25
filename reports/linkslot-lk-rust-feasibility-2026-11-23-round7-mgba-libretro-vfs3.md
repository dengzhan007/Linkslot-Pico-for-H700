# Linkslot 1.0.36 — mGBA Libretro VFS v3 file + directory backend (round 7)

- Date: 2026-11-23
- Feature branch commit: `c5bd5ea66a3f91f6cf15370a9b52d332411c43f5`.
- PR: [#16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16), remains draft; not merged.
- GitHub Actions: [run 45113739310](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45113739310), conclusion **SUCCESS**.
- Artifact: `linkslot-lk-rust-feasibility-45113739310`, ID `14789909499`, about 80.5 MB.
- mGBA source: `libretro/mgba` @ `e31759b24e7a4e3899285ff720d7b573ac328ae7`, same source revision pinned by Linkslot 1.0.36.
- Build target: AArch32 Cortex-A53, ARM state, NEON/VFPv4, softfp ABI, CMake Generic.

## 1. Result: real mGBA A32 archive through Libretro VFS v3

The CI-only source patch (no upstream source or immutable Linkslot snapshot changes) built the real mGBA Libretro core into a static AArch32 archive:

| Metric | Result |
|---|---:|
| Archive | `mgba_libretro-vfs3-aarch32-static.a` |
| Size | 2,870,400 bytes |
| Object members | 114 |
| Namespaced Libretro entry points | 25/25 |
| Unprefixed global `retro_*` definitions | 0 |
| True external unresolved symbols | 51 |
| POSIX file/dir/config path fallback symbols | 0 |

The 51 count is computed as unique symbols from `nm -u archive` minus global definitions within the same archive. The raw per-object undefined list is retained separately; it also contains ordinary cross-object references resolved by other archive members.

## 2. mGBA source adaptations applied in the temporary CI checkout

1. `libretroVFSInit()` requests Libretro VFS v3 first, with v2/v1 fallback retained for compatibility. A getter exposes the negotiated interface to the VDir adapter.
2. The mGBA `VFileOpen` backend is forced through frontend Libretro VFS; it cannot silently fall back to generic `FILE*` I/O in this build.
3. Added `libretro-vfs-dir.c`, implementing mGBA `VDirOpen` and `VDirCreate` through VFS v3:
   - `opendir/readdir/dirent_get_name/dirent_is_dir/closedir`;
   - nested file and directory opens;
   - file deletion and directory creation/stat;
   - directory rewind implemented as close + reopen of the retained path because Libretro VFS v3 has no rewind callback.
4. CMake excludes the old `vfs-file.c` and POSIX `vfs-dirent.c` objects, and includes the new Libretro VFS directory backend.
5. The unused INIH `ini_parse_file(FILE*)` / `ini_parse(filename)` convenience functions are disabled for the required-VFS build. mGBA's used configuration path remains the stream-based `ini_parse_stream` and `ConfigurationReadVFile/ConfigurationWriteVFile`.
6. mGBA's default desktop configuration directory is replaced with a VFS-relative `.` root. The `portable.ini` current-working-directory probe is disabled for this build. Configuration data remains addressed as a VFS file; there is no host HOME/XDG directory or `mkdir` dependency.

The final archive member list contains `libretro-vfs-dir.c.obj` and `libretro-vfs.c.obj`; it does not contain `vfs-file.c.obj` or `vfs-dirent.c.obj`. The build script now asserts the presence of `VDirOpen`, `VDirCreate`, and `libretroVFSGetInterface`, and fails if any old VFS object or guarded POSIX/stdio symbols reappear.

## 3. Remaining 51 external symbols

These are the actual platform/runtime dependencies still unresolved by the mGBA archive:

| Area | Symbols | H700/LK work required |
|---|---|---|
| Heap allocation | `calloc`, `free`, `malloc`, `realloc` | Bind to a verified LK allocator only after the single-core/SMP and heap initialization gates are closed. |
| Memory/string | `memcmp`, `memcpy`, `memmove`, `memset`, `strcasecmp`, `strchr`, `strcmp`, `strcpy`, `strdup`, `strlen`, `strncat`, `strncmp`, `strncpy`, `strrchr`, `strstr` | Most have analogues in LK libc; verify signatures, overlap semantics, bounds and ABI. |
| Locale, parsing, errno | `__ctype_b_loc`, `__errno_location`, `__isoc23_sscanf`, `__isoc23_strtol`, `__isoc23_strtoul`, `freelocale`, `newlocale`, `strtof_l` | Select a fixed C locale or implement narrow tested shims; do not blindly alias glibc locale structures. |
| Time / RTC | `__gettimeofday64`, `__localtime64`, `__localtime64_r`, `__mktime64`, `__time64`, `strftime` | Map to an H700 RTC source and define timezone/UTC semantics. |
| Math / EABI | `__aeabi_ldivmod`, `cbrtf`, `div`, `exp2f`, `fmaxf`, `fminf`, `hypotf`, `pow`, `sincosf` | Choose a compatible ARM libm/compiler-runtime set or implement/test the required subset. |
| Console / formatting | `printf`, `putchar`, `snprintf`, `sprintf`, `stdout`, `vfprintf`, `vsnprintf` | Route to a bounded LK/Linkslot log/console interface; avoid host `FILE*` semantics. |
| Runtime / algorithms | `abort`, `qsort` | Map fatal errors to LK diagnostics/panic and provide/test sorting behavior. |

The full exact set is in `vfs3-external-undefined.txt` in the CI artifact.

## 4. What is now removed from this variant

The archive has **no true external references** to:
- POSIX directory calls: `opendir`, `readdir`, `closedir`, `rewinddir`, `stat`, `mkdir`, `remove`;
- stdio file operations: `fopen/fopen64`, `fclose`, `fread`, `fwrite`, `fseek`, `ftell`, `fflush`;
- Linux file-descriptor/mmap I/O: `open/open64/read/write/close/lseek64/mmap64/msync/munmap/fsync/ftruncate64`;
- desktop configuration discovery: `getcwd`, `getenv`.

This materially narrows the storage integration: actual file and directory I/O can be supplied by a frontend-owned Libretro VFS v3 backend instead of implementing POSIX `FILE*` and `DIR*` inside LK.

## 5. Still not verified

- The Rust no_std VFS v3 fake backend is not a real Linkslot storage implementation.
- Linkslot's current `slot-retro::environment()` has not yet been wired to return the VFS v3 table.
- No Allwinner H700 SDC0/SD block driver or FAT/exFAT filesystem is implemented. BaseOS early-MMC experiments use pre-initialized U-Boot block APIs and must not be mistaken for an independent LK driver.
- The mGBA Generic/VFS3 archive still uses `arm-linux-gnueabi` GCC and a glibc sysroot. This is not yet an `arm-none-eabi`/newlib build or a final LK link.
- No real mGBA ROM has run; no audio/video/save-state performance or correctness was tested.
- No H700 firmware, flashable image, SD write, boot-sector change, or payload execution was performed.
- LK A32 cold-entry, memory ownership and secondary-core status remain NO-GO.

## 6. Next build gate

Attempt the same patched mGBA VFS v3 source with `arm-none-eabi` GCC/newlib to see whether the code compiles independently of glibc headers. Then map the remaining 51 external symbols against the actual pinned LK runtime and decide which require a minimal libm/locale/time adapter. This still precedes any real LK final link and does not waive the boot handoff/RAM-ownership gate.
