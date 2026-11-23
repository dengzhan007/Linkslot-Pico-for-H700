#!/usr/bin/env python3
"""Bind the real namespaced mGBA exports into a Linkslot-shaped static API table."""
from __future__ import annotations
import argparse
import re
import shutil
import subprocess
import sys
from pathlib import Path

API_C = r'''
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
typedef struct { const char *path; const void *data; size_t size; const char *meta; } GameInfo;
typedef struct { uint32_t base_width, base_height, max_width, max_height; float aspect_ratio; } GameGeometry;
typedef struct { double fps, sample_rate; } SystemTiming;
typedef struct { GameGeometry geometry; SystemTiming timing; } SystemAvInfo;
typedef bool (*EnvironmentFn)(uint32_t, void *);
typedef void (*VideoRefreshFn)(const void *, uint32_t, uint32_t, size_t);
typedef void (*AudioSampleFn)(int16_t, int16_t);
typedef size_t (*AudioBatchFn)(const int16_t *, size_t);
typedef void (*InputPollFn)(void);
typedef int16_t (*InputStateFn)(uint32_t, uint32_t, uint32_t, uint32_t);

/* Mirrors crates/slot-retro/src/ffi.rs::Api field order and FFI signatures. */
typedef struct {
    void (*cheat_reset)(void);
    void (*cheat_set)(uint32_t, bool, const char *);
    void (*init)(void);
    void (*deinit)(void);
    uint32_t (*api_version)(void);
    void (*get_system_av_info)(SystemAvInfo *);
    void (*set_environment)(EnvironmentFn);
    void (*set_video_refresh)(VideoRefreshFn);
    void (*set_audio_sample)(AudioSampleFn);
    void (*set_audio_sample_batch)(AudioBatchFn);
    void (*set_input_poll)(InputPollFn);
    void (*set_input_state)(InputStateFn);
    void (*set_controller_port_device)(uint32_t, uint32_t);
    bool (*load_game)(const GameInfo *);
    void (*unload_game)(void);
    void (*run)(void);
    size_t (*serialize_size)(void);
    bool (*serialize)(void *, size_t);
    bool (*unserialize)(const void *, size_t);
    void *(*get_memory_data)(uint32_t);
    size_t (*get_memory_size)(uint32_t);
} LinkslotApi;

extern void mgba_retro_cheat_reset(void);
extern void mgba_retro_cheat_set(uint32_t, bool, const char *);
extern void mgba_retro_init(void);
extern void mgba_retro_deinit(void);
extern uint32_t mgba_retro_api_version(void);
extern void mgba_retro_get_system_av_info(SystemAvInfo *);
extern void mgba_retro_set_environment(EnvironmentFn);
extern void mgba_retro_set_video_refresh(VideoRefreshFn);
extern void mgba_retro_set_audio_sample(AudioSampleFn);
extern void mgba_retro_set_audio_sample_batch(AudioBatchFn);
extern void mgba_retro_set_input_poll(InputPollFn);
extern void mgba_retro_set_input_state(InputStateFn);
extern void mgba_retro_set_controller_port_device(uint32_t, uint32_t);
extern bool mgba_retro_load_game(const GameInfo *);
extern void mgba_retro_unload_game(void);
extern void mgba_retro_run(void);
extern size_t mgba_retro_serialize_size(void);
extern bool mgba_retro_serialize(void *, size_t);
extern bool mgba_retro_unserialize(const void *, size_t);
extern void *mgba_retro_get_memory_data(uint32_t);
extern size_t mgba_retro_get_memory_size(uint32_t);

static const LinkslotApi MGBA_API = {
    .cheat_reset = mgba_retro_cheat_reset,
    .cheat_set = mgba_retro_cheat_set,
    .init = mgba_retro_init,
    .deinit = mgba_retro_deinit,
    .api_version = mgba_retro_api_version,
    .get_system_av_info = mgba_retro_get_system_av_info,
    .set_environment = mgba_retro_set_environment,
    .set_video_refresh = mgba_retro_set_video_refresh,
    .set_audio_sample = mgba_retro_set_audio_sample,
    .set_audio_sample_batch = mgba_retro_set_audio_sample_batch,
    .set_input_poll = mgba_retro_set_input_poll,
    .set_input_state = mgba_retro_set_input_state,
    .set_controller_port_device = mgba_retro_set_controller_port_device,
    .load_game = mgba_retro_load_game,
    .unload_game = mgba_retro_unload_game,
    .run = mgba_retro_run,
    .serialize_size = mgba_retro_serialize_size,
    .serialize = mgba_retro_serialize,
    .unserialize = mgba_retro_unserialize,
    .get_memory_data = mgba_retro_get_memory_data,
    .get_memory_size = mgba_retro_get_memory_size,
};
const LinkslotApi *linkslot_static_mgba_api(void) { return &MGBA_API; }
'''

def run(args: list[str], log: Path) -> str:
    proc = subprocess.run(args, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    log.parent.mkdir(parents=True, exist_ok=True)
    log.write_text(proc.stdout, encoding="utf-8")
    if proc.returncode:
        print(proc.stdout[-12000:], file=sys.stderr)
        raise RuntimeError("command failed: " + " ".join(args))
    return proc.stdout


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--archive", type=Path, required=True)
    ap.add_argument("--adapter-object", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    args = ap.parse_args()
    archive, adapter, out = args.archive.resolve(), args.adapter_object.resolve(), args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    if not archive.is_file() or not adapter.is_file():
        raise RuntimeError("mGBA archive or H700 newlib adapter object is missing")
    for tool in ("arm-none-eabi-gcc", "arm-none-eabi-ld", "arm-none-eabi-nm", "arm-none-eabi-readelf"):
        if not shutil.which(tool):
            raise RuntimeError("required tool missing: " + tool)

    source, obj = out / "mgba-static-api.c", out / "mgba-static-api.o"
    combined = out / "mgba-static-api-and-newlib-adapter.o"
    source.write_text(API_C, encoding="utf-8")
    flags = [
        "-mcpu=cortex-a53", "-marm", "-mfpu=neon-vfpv4", "-mfloat-abi=softfp",
        "-ffreestanding", "-fno-builtin", "-fno-pic", "-fno-pie",
        "-fno-stack-protector", "-ffunction-sections", "-fdata-sections",
        "-Wall", "-Wextra", "-Werror",
    ]
    run(["arm-none-eabi-gcc", *flags, "-c", str(source), "-o", str(obj)], out / "compile.log")

    undefined = subprocess.check_output(["arm-none-eabi-nm", "-u", str(obj)], text=True)
    expected = {
        "mgba_retro_cheat_reset", "mgba_retro_cheat_set", "mgba_retro_init",
        "mgba_retro_deinit", "mgba_retro_api_version", "mgba_retro_get_system_av_info",
        "mgba_retro_set_environment", "mgba_retro_set_video_refresh",
        "mgba_retro_set_audio_sample", "mgba_retro_set_audio_sample_batch",
        "mgba_retro_set_input_poll", "mgba_retro_set_input_state",
        "mgba_retro_set_controller_port_device", "mgba_retro_load_game",
        "mgba_retro_unload_game", "mgba_retro_run", "mgba_retro_serialize_size",
        "mgba_retro_serialize", "mgba_retro_unserialize",
        "mgba_retro_get_memory_data", "mgba_retro_get_memory_size",
    }
    seen = set(re.findall(r"\b(mgba_retro_[A-Za-z0-9_]+)\b", undefined))
    if seen != expected:
        raise RuntimeError(f"unexpected mGBA reference set; missing={sorted(expected-seen)}, extra={sorted(seen-expected)}")

    header = subprocess.check_output(["arm-none-eabi-readelf", "-h", str(obj)], text=True)
    if "ELF32" not in header or "ARM" not in header:
        raise RuntimeError("static API object is not ELF32/ARM")
    run(["arm-none-eabi-ld", "-r", str(obj), str(adapter), "-o", str(combined)],
        out / "combine-api-and-adapter.log")
    combined_undefined = subprocess.check_output(["arm-none-eabi-nm", "-u", str(combined)], text=True)
    if "mgba_retro_api_version" not in combined_undefined:
        raise RuntimeError("combined object lost expected mGBA API references")

    report = (
        "# Linkslot-shaped static mGBA API table\n\n"
        "- Compiles a freestanding AArch32 C representation of the slot-retro::Api field order and callback signatures.\n"
        "- Binds all mandatory Libretro entry points plus optional cheat reset/set to namespaced mgba_retro_* symbols.\n"
        "- Strict C compilation with -Wall -Wextra -Werror passes; all 21 expected symbol references are present.\n"
        "- Target object is validated as ELF32/ARM.\n"
        "- Combines the API-table object with the H700 newlib adapter using arm-none-eabi-ld -r.\n"
        "- The combined object can be passed to the following whole-archive mGBA/newlib final-link diagnostic.\n\n"
        "Boundary: this is a C ABI/symbol binding probe shaped like slot-retro::Api. It is not the Rust Api object and does not call LibretroCore::open_static(). The latter currently has a separate mock-API host lifecycle test. No LK or device execution occurs.\n"
    )
    (out / "REPORT.md").write_text(report, encoding="utf-8")
    print(report)


if __name__ == "__main__":
    main()
