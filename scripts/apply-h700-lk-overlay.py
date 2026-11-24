#!/usr/bin/env python3
"""Generate a non-invasive H700 BL33 source overlay for the pinned GammaOS LK."""
from __future__ import annotations

import argparse
import difflib
import hashlib
import json
import re
import subprocess
from pathlib import Path

LK_PIN = "9bf1419a49e0af6dba8b05916df382c9c9c6d0a8"
FILES = (
    Path("makefile"),
    Path("dev/rules.mk"),
    Path("target/rules.mk"),
    Path("arch/arm/crt0.S"),
    Path("arch/arm/arch.c"),
    Path("arch/arm/rules.mk"),
)


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def read_pinned_source(source: Path) -> tuple[str, dict[Path, str]]:
    revision = subprocess.check_output(
        ["git", "-C", str(source), "rev-parse", "HEAD"], text=True
    ).strip()
    if revision != LK_PIN:
        raise RuntimeError(f"GammaOS LK pin mismatch: expected {LK_PIN}, got {revision}")
    texts: dict[Path, str] = {}
    for rel in FILES:
        path = source / rel
        if not path.is_file():
            raise RuntimeError(f"required pinned LK source is missing: {path}")
        texts[rel] = path.read_text(encoding="utf-8")
    return revision, texts


def patch_makefile(original: str) -> str:
    project_include = "include project/$(PROJECT).mk"
    if original.count(project_include) != 1:
        raise RuntimeError("expected one project make include")
    early_toolchain = (
        project_include + "\n"
        "ifeq ($(H700_BUILD),yes)\n"
        "TOOLCHAIN_PREFIX ?= arm-none-eabi-\n"
        "endif"
    )
    original = original.replace(project_include, early_toolchain, 1)

    old_pal = "include platform/pal/rules.mk"
    if original.count(old_pal) != 1:
        raise RuntimeError("expected one platform/pal make include")
    new_pal = (
        "ifneq ($(H700_BUILD),yes)\n"
        "# The MediaTek PAL uses MTK GPT timing and a placeholder mutex implementation.\n"
        + old_pal + "\n"
        "endif"
    )
    patched = original.replace(old_pal, new_pal, 1)

    old = "include platform/common/rules.mk"
    if patched.count(old) != 1:
        raise RuntimeError("expected one platform/common make include")
    new = (
        "ifneq ($(H700_BUILD),yes)\n"
        "# The MediaTek common platform stack is not an H700 HAL.\n"
        + old + "\n"
        "endif"
    )
    patched = patched.replace(old, new, 1)

    old_tools = "\n".join((
        "# default to no ccache",
        "CCACHE ?=",
        "TOOLCHAIN_PREFIX =gcc/bin/arm-linux-androideabi-",
        "CC := $(CCACHE) $(TOOLCHAIN_PREFIX)gcc",
        "#LD := $(TOOLCHAIN_PREFIX)ld",
        "LD := $(TOOLCHAIN_PREFIX)ld.bfd",
        "OBJDUMP := $(TOOLCHAIN_PREFIX)objdump",
        "OBJCOPY := $(TOOLCHAIN_PREFIX)objcopy",
        "CPPFILT := $(TOOLCHAIN_PREFIX)c++filt",
        "SIZE := $(TOOLCHAIN_PREFIX)size",
        "NM := $(TOOLCHAIN_PREFIX)nm",
    ))
    if patched.count(old_tools) != 1:
        raise RuntimeError("unexpected pinned LK toolchain selection block")
    new_tools = "\n".join((
        "# default to no ccache",
        "CCACHE ?=",
        "ifeq ($(H700_BUILD),yes)",
        "TOOLCHAIN_PREFIX ?= arm-none-eabi-",
        "LD := $(TOOLCHAIN_PREFIX)ld",
        "else",
        "TOOLCHAIN_PREFIX =gcc/bin/arm-linux-androideabi-",
        "LD := $(TOOLCHAIN_PREFIX)ld.bfd",
        "endif",
        "CC := $(CCACHE) $(TOOLCHAIN_PREFIX)gcc",
        "OBJDUMP := $(TOOLCHAIN_PREFIX)objdump",
        "OBJCOPY := $(TOOLCHAIN_PREFIX)objcopy",
        "CPPFILT := $(TOOLCHAIN_PREFIX)c++filt",
        "SIZE := $(TOOLCHAIN_PREFIX)size",
        "NM := $(TOOLCHAIN_PREFIX)nm",
    ))
    return patched.replace(old_tools, new_tools, 1)


def patch_dev_rules(original: str) -> str:
    old_modules = "\n".join((
        "MODULES += \\",
        "\t$(LOCAL_DIR)/video \\",
        "\t$(LOCAL_DIR)/lcm \\",
        "\t$(LOCAL_DIR)/gic \\",
        "\t$(LOCAL_DIR)/md_com",
    ))
    if original.count(old_modules) != 1:
        raise RuntimeError("unexpected dev/rules.mk MTK module list")
    new_modules = "ifneq ($(H700_BUILD),yes)\n" + old_modules + "\nendif"
    patched = original.replace(old_modules, new_modules, 1)

    old_logo = "include $(LOCAL_DIR)/logo/rules.mk"
    if patched.count(old_logo) != 1:
        raise RuntimeError("expected one dev logo rules include")
    new_logo = "ifneq ($(H700_BUILD),yes)\n" + old_logo + "\nendif"
    return patched.replace(old_logo, new_logo, 1)


def patch_target_rules(original: str) -> str:
    old = "\n".join((
        "OBJS += \\",
        "\t$(LOCAL_DIR)/init.o \\",
        "\t$(LOCAL_DIR)/target_display.o",
    ))
    if original.count(old) != 1:
        raise RuntimeError("unexpected generic target object list")
    new = (
        "ifneq ($(H700_BUILD),yes)\n"
        "OBJS += $(LOCAL_DIR)/init.o\n"
        "endif\n"
        "OBJS += $(LOCAL_DIR)/target_display.o"
    )
    return original.replace(old, new, 1)


def patch_crt0(original: str) -> str:
    start_marker = ".globl _start\n_start:"
    reset_marker = "reset:\n"
    start = original.find(start_marker)
    reset = original.find(reset_marker, start)
    if start < 0 or reset < 0:
        raise RuntimeError("unexpected crt0.S shape; refusing to guess the startup layout")

    # The first branch is the vendor-loader entry. Bytes +4..+0x63f are
    # reserved for the opaque vendor header; vectors live at +0x1000.
    prefix = """ .global _start
_start:
    b lk_bl33_entry
    .space 0xffc, 0

    .balign 32
.global __lk_vectors_start
__lk_vectors_start:
    b reset
    b arm_undefined
    b arm_syscall
    b arm_prefetch_abort
    b arm_data_abort
    b arm_reserved
    b arm_irq
    b arm_fiq

.global BOOT_ARGUMENT_LOCATION
BOOT_ARGUMENT_LOCATION:
    .word 0

.global lk_bl33_entry
lk_bl33_entry:
    ldr r0, =__lk_vectors_start
    mcr p15, 0, r0, c12, c0, 0
    isb
"""
    prefix = prefix.lstrip()
    patched = original[:start] + prefix + original[reset:]

    relocation = "sub\t\tr0, r0, #(.Laddr - _start)"
    if patched.count(relocation) != 1:
        raise RuntimeError("unexpected crt0 self-relocation instruction; refusing to patch")
    patched = patched.replace(
        relocation,
        "ldr\t\tr1, =(.Laddr - _start)\n\tsub\t\tr0, r0, r1",
        1,
    )
    return patched


def patch_arch_c(original: str) -> str:
    old = "set_vector_base(MEMBASE);"
    if original.count(old) != 1:
        raise RuntimeError("expected exactly one arch_early_init VBAR assignment")
    new = (
        "extern char __lk_vectors_start[];\n"
        "\tset_vector_base((addr_t)__lk_vectors_start);"
    )
    return original.replace(old, new, 1)


def patch_arm_rules(original: str) -> str:
    # Do not change other CPU branches. Only correct the pinned cortex-a53
    # branch, which currently selects cortex-a9 code generation.
    pattern = re.compile(
        r"(^ifeq \(\$\(ARM_CPU\),cortex-a53\)\n.*?^CFLAGS \+= )"
        r"-mcpu=cortex-a9(\n.*?^HANDLED_CORE := true)",
        re.MULTILINE | re.DOTALL,
    )
    patched, count = pattern.subn(r"\g<1>-mcpu=cortex-a53 -mfpu=neon-vfpv4 -mfloat-abi=softfp\g<2>", original)
    if count != 1:
        raise RuntimeError(
            f"expected one cortex-a53 branch with -mcpu=cortex-a9, found {count}"
        )
    return patched


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--lk-source", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()

    source = args.lk_source.resolve()
    out = args.out.resolve()
    revision, originals = read_pinned_source(source)
    patched = {
        Path("makefile"): patch_makefile(originals[Path("makefile")]),
        Path("dev/rules.mk"): patch_dev_rules(originals[Path("dev/rules.mk")]),
        Path("target/rules.mk"): patch_target_rules(originals[Path("target/rules.mk")]),
        Path("arch/arm/crt0.S"): patch_crt0(originals[Path("arch/arm/crt0.S")]),
        Path("arch/arm/arch.c"): patch_arch_c(originals[Path("arch/arm/arch.c")]),
        Path("arch/arm/rules.mk"): patch_arm_rules(originals[Path("arch/arm/rules.mk")]),
    }

    overlay_root = out / "files"
    overlay_root.mkdir(parents=True, exist_ok=True)
    diff_parts: list[str] = []
    records = []
    for rel in FILES:
        payload = patched[rel].encode("utf-8")
        dest = overlay_root / rel
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_bytes(payload)

        diff_parts.append(f"diff --git a/{rel.as_posix()} b/{rel.as_posix()}\n")
        diff_parts.extend(
            difflib.unified_diff(
                originals[rel].splitlines(keepends=True),
                patched[rel].splitlines(keepends=True),
                fromfile=f"a/{rel.as_posix()}",
                tofile=f"b/{rel.as_posix()}",
                lineterm="\n",
            )
        )
        records.append(
            {
                "path": rel.as_posix(),
                "original_sha256": sha256(originals[rel].encode("utf-8")),
                "overlay_sha256": sha256(payload),
                "changed": originals[rel] != patched[rel],
            }
        )

    patch_path = out / "h700-bl33-overlay.patch"
    patch_path.write_text("".join(diff_parts), encoding="utf-8")
    check = subprocess.run(
        ["git", "-C", str(source), "apply", "--check", str(patch_path)],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    if check.returncode:
        raise RuntimeError("generated overlay patch failed git apply --check:\n" + check.stdout)
    manifest = {
        "schema": "gammaos-lk-h700-overlay-v1",
        "source_repository": "TheGammaSqueeze/lk-mt6785",
        "source_commit": revision,
        "status": "host-generated-source-overlay-only",
        "source_tree_modified": False,
        "git_apply_check": "pass",
        "files": records,
        "patch_sha256": sha256(patch_path.read_bytes()),
        "explicitly_not_changed": [
            "ACTLR.SMP / Cortex-A53 coherency setup",
            "secondary-core parking or PSCI",
            "platform MMU mappings or memory reservations",
            "MEMBASE, MEMSIZE, heap or stack addresses",
            "H700 project/target/platform rules or a physical memory map",
            "H700 MMIO, clocks, GPIO, PMIC, UART or display drivers",
            "newlib syscall providers and LK heap initialization",
        ],
    }
    (out / "manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    readme = """# GammaOS LK H700 BL33 source overlay

This directory is generated from the exact pinned GammaOS LK source commit in
manifest.json. It is an overlay for source review and a future H700-specific
build; it is not a firmware image.

## Changes

- arch/arm/crt0.S: branch from image offset +0 into a BL33 wrapper, reserve
  the opaque vendor header window [+4,+0x640), put the A32 vector table at
  +0x1000, preserve the r4 boot-argument scratch word, and enter the original
  reset path after setting VBAR.
- arch/arm/arch.c: keep arch_early_init() VBAR pointed at
  __lk_vectors_start instead of resetting it to MEMBASE.
- arch/arm/rules.mk: correct the Cortex-A53 branch compiler target from
  -mcpu=cortex-a9 to -mcpu=cortex-a53 with NEON/VFPv4 and softfp ABI flags.
- makefile: use the arm-none-eabi toolchain by default for H700 while preserving the MTK Android EABI prefix for all non-H700 projects.
- makefile: skip the MediaTek platform/pal objects for H700; its timer is tied to MTK GPT and its PAL mutex is a no-op placeholder.
- makefile: do not include the MediaTek platform/common stack when H700_BUILD=yes.
- dev/rules.mk: do not pull in MediaTek video, LCM, GIC, modem communication or logo modules for H700.
- target/rules.mk: do not compile the MediaTek-specific target/init.c for H700; keep only the generic weak display stubs.

The H700 project must set H700_BUILD=yes before the top-level makefile loads
these rules. It may override TOOLCHAIN_PREFIX, and must add its own
target/platform modules explicitly.

h700-bl33-overlay.patch can be applied to a fresh checkout of the pinned
source. files/ contains the resulting replacement files. The generator never
modifies --lk-source.

## Required work not included

This overlay does not configure A53 coherency, park secondary CPUs, make LK's
scheduler or heap SMP-safe, choose H700 RAM/heap/stack addresses, implement
the H700 project/target/platform, or supply any H700 device drivers. In
particular, no ACTLR.SMP write is guessed. The H700 project must separately
select and verify its ARM/Thumb interworking and Thumb-2 policy.

Do not flash, write to an SD card, or execute this source overlay. It has not
been built as a complete LK image or booted on RG34XX.
"""
    (out / "README.md").write_text(readme, encoding="utf-8")
    print(
        f"Generated H700 BL33 overlay from {revision}: "
        f"{len(records)} files, patch SHA-256 {manifest['patch_sha256']}"
    )


if __name__ == "__main__":
    main()
