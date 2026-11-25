# Linkslot 1.0.36 — mGBA AArch32 arm-none-eabi/newlib VFS v3 archive (round 8)

- Date: 2026-11-23
- Feature branch commit: cbb9208152dc3c85188ad374b7e085843b991565.
- PR: https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16 (draft; not merged).
- GitHub Actions: https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45114286962 — conclusion SUCCESS.
- Artifact: linkslot-lk-rust-feasibility-45114286962, ID 14791290676, about 82.7 MB.
- mGBA source: libretro/mgba @ e31759b24e7a4e3899285ff720d7b573ac328ae7.
- Existing CI-only VFS v3 file/directory patches reused from round 7.

## 1. Bare-metal compiler result

The same mGBA Generic VFS v3 source was built using Debian arm-none-eabi GCC and newlib, rather than the previous arm-linux-gnueabi GCC/glibc sysroot.

| Metric | Result |
|---|---:|
| Archive | mgba_libretro-vfs3-arm-none-eabi.a |
| Size | 2,856,556 bytes |
| Object members | 94 |
| Extracted object header | ELF32 / ARM / EABI5 |
| Libretro exports | 25/25 namespaced mgba_retro_* |
| Old unprefixed retro_* globals | 0 |
| VFS backend symbols | VDirOpen, VDirCreate, libretroVFSGetInterface present |
| True unresolved external symbols | 52 |
| POSIX/stdio file-system fallback references | 0 |

The archive object list excludes vfs-file.c.obj and vfs-dirent.c.obj, and includes the custom libretro-vfs-dir.c.obj backend.

## 2. Remaining external symbol set

The 52 symbols are computed as unique nm -u names minus global definitions in the same archive. They are not evidence that all 52 are missing from newlib; the next step is a whole-archive link against the installed newlib/libm/libgcc/libnosys libraries to identify which are already satisfied.

~~~text
__aeabi_ldivmod
__ctype_b_loc
__errno
_ctype_
abort
abs
calloc
cbrtf
cosf
div
exp2f
fmaxf
fminf
free
freelocale
gettimeofday
hypotf
localtime
localtime_r
malloc
memcmp
memcpy
memmove
memset
mktime
newlocale
pow
printf
qsort
realloc
sinf
snprintf
sprintf
sscanf
strcasecmp
strcat
strchr
strcmp
strcpy
strdup
strftime
strlen
strncat
strncmp
strncpy
strrchr
strstr
strtof_l
strtol
strtoul
time
vprintf
vsnprintf
~~~

## 3. What this proves

- The mGBA core source can compile as AArch32 ARM ELF objects using an arm-none-eabi compiler and newlib headers/sysroot.
- The VFS v3 file and directory backend remains buildable without glibc.
- mGBA's Libretro API symbols remain namespaced and the old POSIX/stdio storage fallback is still absent.

## 4. What it does not prove

- This is a static archive, not a final LK link. No startup/runtime or application ELF was produced.
- Availability of newlib does not imply compatibility with GammaOS LK's libc, heap, syscall boundary, or H700 runtime.
- No actual Linkslot VFS v3 implementation or H700 SD/FAT backend is connected.
- No ROM, video/audio callback, battery save or save-state was run on H700.
- No firmware or flashable image was produced; no SD/boot media was written and no payload was executed.

## 5. Next step

Perform a whole-archive static link against the installed newlib, libm, libgcc and libnosys libraries, using only a test _start stub. Record which of the 52 symbols are satisfied by the generic bare-metal toolchain and which remain platform adapters. Then compare the residual set with the pinned LK libc and add only the minimal H700 RTC/log/VFS/math/EABI compatibility services. This is still a host link diagnostic, not a device handoff test.
