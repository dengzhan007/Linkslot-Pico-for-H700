#!/usr/bin/env python3
"""Prove static Libretro symbol tables link and round-trip through mock cores."""
from __future__ import annotations
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import sys

TARGETS = {
    "armv7a-none-eabi": ("armv7a-none-eabi", "armelf", "ARM"),
    "aarch64-unknown-none": ("aarch64-none-elf", "aarch64elf", "AArch64"),
}

def run(args, log=None):
    proc = subprocess.run(args, text=True, stdout=subprocess.PIPE,
                          stderr=subprocess.STDOUT)
    if log:
        log.parent.mkdir(parents=True, exist_ok=True)
        log.write_text(proc.stdout, encoding="utf-8")
    if proc.returncode:
        print(proc.stdout[-12000:], file=sys.stderr)
        raise RuntimeError("command failed: " + " ".join(map(str, args)))
    return proc.stdout

def require_tools():
    for tool in ("rustc", "clang", "ld.lld", "llvm-nm", "readelf", "cc"):
        if not shutil.which(tool):
            raise RuntimeError("required tool missing: " + tool)

def rust_source():
    return r'''#![no_std]
use core::ffi::{c_char, c_uint, c_void};
use core::panic::PanicInfo;

#[repr(C)]
pub struct GameInfo {
    pub path: *const c_char,
    pub data: *const c_void,
    pub size: usize,
    pub meta: *const c_char,
}
#[repr(C)]
#[derive(Default)]
pub struct GameGeometry {
    pub base_width: c_uint,
    pub base_height: c_uint,
    pub max_width: c_uint,
    pub max_height: c_uint,
    pub aspect_ratio: f32,
}
#[repr(C)]
#[derive(Default)]
pub struct SystemTiming { pub fps: f64, pub sample_rate: f64 }
#[repr(C)]
#[derive(Default)]
pub struct SystemAvInfo { pub geometry: GameGeometry, pub timing: SystemTiming }

pub type EnvironmentFn = unsafe extern "C" fn(c_uint, *mut c_void) -> bool;
pub type VideoRefreshFn = unsafe extern "C" fn(*const c_void, c_uint, c_uint, usize);
pub type AudioSampleFn = unsafe extern "C" fn(i16, i16);
pub type AudioBatchFn = unsafe extern "C" fn(*const i16, usize) -> usize;
pub type InputPollFn = unsafe extern "C" fn();
pub type InputStateFn = unsafe extern "C" fn(c_uint, c_uint, c_uint, c_uint) -> i16;

#[repr(C)]
pub struct Api {
    pub cheat_reset: Option<unsafe extern "C" fn()>,
    pub cheat_set: Option<unsafe extern "C" fn(c_uint, bool, *const c_char)>,
    pub init: unsafe extern "C" fn(),
    pub deinit: unsafe extern "C" fn(),
    pub api_version: unsafe extern "C" fn() -> c_uint,
    pub get_system_av_info: unsafe extern "C" fn(*mut SystemAvInfo),
    pub set_environment: unsafe extern "C" fn(EnvironmentFn),
    pub set_video_refresh: unsafe extern "C" fn(VideoRefreshFn),
    pub set_audio_sample: unsafe extern "C" fn(AudioSampleFn),
    pub set_audio_sample_batch: unsafe extern "C" fn(AudioBatchFn),
    pub set_input_poll: unsafe extern "C" fn(InputPollFn),
    pub set_input_state: unsafe extern "C" fn(InputStateFn),
    pub set_controller_port_device: unsafe extern "C" fn(c_uint, c_uint),
    pub load_game: unsafe extern "C" fn(*const GameInfo) -> bool,
    pub unload_game: unsafe extern "C" fn(),
    pub run: unsafe extern "C" fn(),
    pub serialize_size: unsafe extern "C" fn() -> usize,
    pub serialize: unsafe extern "C" fn(*mut c_void, usize) -> bool,
    pub unserialize: unsafe extern "C" fn(*const c_void, usize) -> bool,
    pub get_memory_data: unsafe extern "C" fn(c_uint) -> *mut c_void,
    pub get_memory_size: unsafe extern "C" fn(c_uint) -> usize,
}

unsafe extern "C" {
    #[link_name = "mgba_retro_cheat_reset"] fn mgba_cheat_reset();
    #[link_name = "mgba_retro_cheat_set"] fn mgba_cheat_set(index: c_uint, enabled: bool, code: *const c_char);
    #[link_name = "mgba_retro_init"] fn mgba_init();
    #[link_name = "mgba_retro_deinit"] fn mgba_deinit();
    #[link_name = "mgba_retro_api_version"] fn mgba_api_version() -> c_uint;
    #[link_name = "mgba_retro_get_system_av_info"] fn mgba_get_system_av_info(info: *mut SystemAvInfo);
    #[link_name = "mgba_retro_set_environment"] fn mgba_set_environment(cb: EnvironmentFn);
    #[link_name = "mgba_retro_set_video_refresh"] fn mgba_set_video_refresh(cb: VideoRefreshFn);
    #[link_name = "mgba_retro_set_audio_sample"] fn mgba_set_audio_sample(cb: AudioSampleFn);
    #[link_name = "mgba_retro_set_audio_sample_batch"] fn mgba_set_audio_sample_batch(cb: AudioBatchFn);
    #[link_name = "mgba_retro_set_input_poll"] fn mgba_set_input_poll(cb: InputPollFn);
    #[link_name = "mgba_retro_set_input_state"] fn mgba_set_input_state(cb: InputStateFn);
    #[link_name = "mgba_retro_set_controller_port_device"] fn mgba_set_controller_port_device(port: c_uint, device: c_uint);
    #[link_name = "mgba_retro_load_game"] fn mgba_load_game(info: *const GameInfo) -> bool;
    #[link_name = "mgba_retro_unload_game"] fn mgba_unload_game();
    #[link_name = "mgba_retro_run"] fn mgba_run();
    #[link_name = "mgba_retro_serialize_size"] fn mgba_serialize_size() -> usize;
    #[link_name = "mgba_retro_serialize"] fn mgba_serialize(data: *mut c_void, size: usize) -> bool;
    #[link_name = "mgba_retro_unserialize"] fn mgba_unserialize(data: *const c_void, size: usize) -> bool;
    #[link_name = "mgba_retro_get_memory_data"] fn mgba_get_memory_data(id: c_uint) -> *mut c_void;
    #[link_name = "mgba_retro_get_memory_size"] fn mgba_get_memory_size(id: c_uint) -> usize;

    #[link_name = "gpsp_retro_cheat_reset"] fn gpsp_cheat_reset();
    #[link_name = "gpsp_retro_cheat_set"] fn gpsp_cheat_set(index: c_uint, enabled: bool, code: *const c_char);
    #[link_name = "gpsp_retro_init"] fn gpsp_init();
    #[link_name = "gpsp_retro_deinit"] fn gpsp_deinit();
    #[link_name = "gpsp_retro_api_version"] fn gpsp_api_version() -> c_uint;
    #[link_name = "gpsp_retro_get_system_av_info"] fn gpsp_get_system_av_info(info: *mut SystemAvInfo);
    #[link_name = "gpsp_retro_set_environment"] fn gpsp_set_environment(cb: EnvironmentFn);
    #[link_name = "gpsp_retro_set_video_refresh"] fn gpsp_set_video_refresh(cb: VideoRefreshFn);
    #[link_name = "gpsp_retro_set_audio_sample"] fn gpsp_set_audio_sample(cb: AudioSampleFn);
    #[link_name = "gpsp_retro_set_audio_sample_batch"] fn gpsp_set_audio_sample_batch(cb: AudioBatchFn);
    #[link_name = "gpsp_retro_set_input_poll"] fn gpsp_set_input_poll(cb: InputPollFn);
    #[link_name = "gpsp_retro_set_input_state"] fn gpsp_set_input_state(cb: InputStateFn);
    #[link_name = "gpsp_retro_set_controller_port_device"] fn gpsp_set_controller_port_device(port: c_uint, device: c_uint);
    #[link_name = "gpsp_retro_load_game"] fn gpsp_load_game(info: *const GameInfo) -> bool;
    #[link_name = "gpsp_retro_unload_game"] fn gpsp_unload_game();
    #[link_name = "gpsp_retro_run"] fn gpsp_run();
    #[link_name = "gpsp_retro_serialize_size"] fn gpsp_serialize_size() -> usize;
    #[link_name = "gpsp_retro_serialize"] fn gpsp_serialize(data: *mut c_void, size: usize) -> bool;
    #[link_name = "gpsp_retro_unserialize"] fn gpsp_unserialize(data: *const c_void, size: usize) -> bool;
    #[link_name = "gpsp_retro_get_memory_data"] fn gpsp_get_memory_data(id: c_uint) -> *mut c_void;
    #[link_name = "gpsp_retro_get_memory_size"] fn gpsp_get_memory_size(id: c_uint) -> usize;
}

static MGBA_API: Api = Api {
    cheat_reset: Some(mgba_cheat_reset), cheat_set: Some(mgba_cheat_set),
    init: mgba_init, deinit: mgba_deinit, api_version: mgba_api_version,
    get_system_av_info: mgba_get_system_av_info, set_environment: mgba_set_environment,
    set_video_refresh: mgba_set_video_refresh, set_audio_sample: mgba_set_audio_sample,
    set_audio_sample_batch: mgba_set_audio_sample_batch, set_input_poll: mgba_set_input_poll,
    set_input_state: mgba_set_input_state, set_controller_port_device: mgba_set_controller_port_device,
    load_game: mgba_load_game, unload_game: mgba_unload_game, run: mgba_run,
    serialize_size: mgba_serialize_size, serialize: mgba_serialize, unserialize: mgba_unserialize,
    get_memory_data: mgba_get_memory_data, get_memory_size: mgba_get_memory_size,
};
static GPSP_API: Api = Api {
    cheat_reset: Some(gpsp_cheat_reset), cheat_set: Some(gpsp_cheat_set),
    init: gpsp_init, deinit: gpsp_deinit, api_version: gpsp_api_version,
    get_system_av_info: gpsp_get_system_av_info, set_environment: gpsp_set_environment,
    set_video_refresh: gpsp_set_video_refresh, set_audio_sample: gpsp_set_audio_sample,
    set_audio_sample_batch: gpsp_set_audio_sample_batch, set_input_poll: gpsp_set_input_poll,
    set_input_state: gpsp_set_input_state, set_controller_port_device: gpsp_set_controller_port_device,
    load_game: gpsp_load_game, unload_game: gpsp_unload_game, run: gpsp_run,
    serialize_size: gpsp_serialize_size, serialize: gpsp_serialize, unserialize: gpsp_unserialize,
    get_memory_data: gpsp_get_memory_data, get_memory_size: gpsp_get_memory_size,
};

#[no_mangle]
pub extern "C" fn linkslot_static_core_get(core_id: c_uint) -> *const Api {
    match core_id {
        0 => &MGBA_API,
        1 => &GPSP_API,
        _ => core::ptr::null(),
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! { loop { core::hint::spin_loop(); } }
'''

def mock_core(prefix: str, seed: int, fps: int):
    return f'''#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include "static_abi_c.h"
#define PFX(name) {prefix}_##name
static EnvironmentFn env_cb;
static VideoRefreshFn video_cb;
static AudioSampleFn audio_cb;
static AudioBatchFn audio_batch_cb;
static InputPollFn input_poll_cb;
static InputStateFn input_state_cb;
static uint32_t state = {seed}u;
static uint8_t memory_data[16] = {{{seed}, 2, 3, 4}};
static int initialized, loaded, deinitialized, port_device;
static int cheat_count;
void PFX(retro_cheat_reset)(void) {{ cheat_count = 0; }}
void PFX(retro_cheat_set)(uint32_t index, bool enabled, const char *code) {{
    if (enabled && code && index < 16) cheat_count++;
}}
void PFX(retro_init)(void) {{
    initialized = 1;
    bool can_dupe = false;
    if (!env_cb || !env_cb(3u, &can_dupe) || !can_dupe) initialized = 0;
}}
void PFX(retro_deinit)(void) {{ deinitialized = 1; }}
uint32_t PFX(retro_api_version)(void) {{ return 1u; }}
void PFX(retro_get_system_av_info)(SystemAvInfo *info) {{
    info->geometry.base_width = 240; info->geometry.base_height = 160;
    info->geometry.max_width = 240; info->geometry.max_height = 160;
    info->geometry.aspect_ratio = 1.5f;
    info->timing.fps = {fps}.0; info->timing.sample_rate = 48000.0;
}}
void PFX(retro_set_environment)(EnvironmentFn cb) {{ env_cb = cb; }}
void PFX(retro_set_video_refresh)(VideoRefreshFn cb) {{ video_cb = cb; }}
void PFX(retro_set_audio_sample)(AudioSampleFn cb) {{ audio_cb = cb; }}
void PFX(retro_set_audio_sample_batch)(AudioBatchFn cb) {{ audio_batch_cb = cb; }}
void PFX(retro_set_input_poll)(InputPollFn cb) {{ input_poll_cb = cb; }}
void PFX(retro_set_input_state)(InputStateFn cb) {{ input_state_cb = cb; }}
void PFX(retro_set_controller_port_device)(uint32_t port, uint32_t device) {{ port_device = (int)(port + device); }}
bool PFX(retro_load_game)(const GameInfo *info) {{
    loaded = initialized && info && info->data && info->size == 4;
    return loaded;
}}
void PFX(retro_unload_game)(void) {{ loaded = 0; }}
void PFX(retro_run)(void) {{
    uint32_t frame[2] = {{{seed}, {seed + 1}}};
    int16_t samples[2] = {{{seed}, -{seed}}};
    if (!loaded) return;
    if (input_poll_cb) input_poll_cb();
    if (input_state_cb) state += (uint16_t)input_state_cb(0, 1, 0, 8);
    if (video_cb) video_cb(frame, 2, 1, sizeof(frame));
    if (audio_cb) audio_cb(samples[0], samples[1]);
    if (audio_batch_cb) state += (uint32_t)audio_batch_cb(samples, 1);
    state++;
}}
size_t PFX(retro_serialize_size)(void) {{ return sizeof(state); }}
bool PFX(retro_serialize)(void *dst, size_t size) {{
    if (!dst || size != sizeof(state)) return false;
    *(uint32_t *)dst = state; return true;
}}
bool PFX(retro_unserialize)(const void *src, size_t size) {{
    if (!src || size != sizeof(state)) return false;
    state = *(const uint32_t *)src; return true;
}}
void *PFX(retro_get_memory_data)(uint32_t id) {{ return id == 0 ? memory_data : 0; }}
size_t PFX(retro_get_memory_size)(uint32_t id) {{ return id == 0 ? sizeof(memory_data) : 0; }}
int PFX(test_initialized)(void) {{ return initialized; }}
int PFX(test_deinitialized)(void) {{ return deinitialized; }}
int PFX(test_port_device)(void) {{ return port_device; }}
int PFX(test_cheat_count)(void) {{ return cheat_count; }}
'''

def c_header():
    return r'''#ifndef STATIC_ABI_C_H
#define STATIC_ABI_C_H
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
} Api;
const Api *linkslot_static_core_get(uint32_t core_id);
int static_abi_run(void);
#endif
'''

def harness():
    return r'''#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include "static_abi_c.h"
extern int mgba_test_initialized(void), mgba_test_deinitialized(void), mgba_test_port_device(void), mgba_test_cheat_count(void);
extern int gpsp_test_initialized(void), gpsp_test_deinitialized(void), gpsp_test_port_device(void), gpsp_test_cheat_count(void);
static int video_calls, audio_calls, batch_calls, poll_calls;
static uint32_t video_first;
static int16_t audio_l, audio_r;
static uint32_t input_value;
static bool env_ok(uint32_t cmd, void *data) {
    if (cmd != 3 || !data) return false;
    *(bool *)data = true; return true;
}
static void video_cb(const void *data, uint32_t w, uint32_t h, size_t pitch) {
    if (data && w == 2 && h == 1 && pitch == 8) {
        video_first = *(const uint32_t *)data; video_calls++;
    }
}
static void audio_cb(int16_t l, int16_t r) { audio_l = l; audio_r = r; audio_calls++; }
static size_t batch_cb(const int16_t *data, size_t frames) {
    if (data && frames == 1) batch_calls++;
    return frames;
}
static void input_poll(void) { poll_calls++; }
static int16_t input_state(uint32_t port, uint32_t device, uint32_t index, uint32_t id) {
    if (port == 0 && device == 1 && index == 0 && id == 8) return (int16_t)input_value;
    return 0;
}
static int exercise(uint32_t id, uint32_t expected_seed, uint32_t expected_fps) {
    const Api *api = linkslot_static_core_get(id);
    const uint8_t rom[4] = {1,2,3,4};
    GameInfo game = {0, rom, sizeof(rom), 0};
    SystemAvInfo av = {0};
    uint32_t state, restored;
    if (!api || api->api_version() != 1) return 10;
    api->set_environment(env_ok);
    api->set_video_refresh(video_cb);
    api->set_audio_sample(audio_cb);
    api->set_audio_sample_batch(batch_cb);
    api->set_input_poll(input_poll);
    api->set_input_state(input_state);
    api->init();
    if (id == 0 ? !mgba_test_initialized() : !gpsp_test_initialized()) return 11;
    api->get_system_av_info(&av);
    if (av.geometry.base_width != 240 || av.geometry.base_height != 160 ||
        av.timing.fps != (double)expected_fps || av.timing.sample_rate != 48000.0) return 12;
    api->set_controller_port_device(0, 1);
    if (id == 0 ? mgba_test_port_device() != 1 : gpsp_test_port_device() != 1) return 13;
    if (!api->load_game(&game)) return 14;
    api->cheat_reset();
    api->cheat_set(0, true, "012345");
    if (id == 0 ? mgba_test_cheat_count() != 1 : gpsp_test_cheat_count() != 1) return 15;
    video_calls = audio_calls = batch_calls = poll_calls = 0;
    input_value = 1;
    api->run();
    if (video_calls != 1 || video_first != expected_seed ||
        audio_calls != 1 || audio_l != (int16_t)expected_seed || audio_r != -(int16_t)expected_seed ||
        batch_calls != 1 || poll_calls != 1) return 16;
    if (api->serialize_size() != sizeof(uint32_t)) return 17;
    if (!api->serialize(&state, sizeof(state))) return 18;
    if (!api->get_memory_data(0) || api->get_memory_size(0) != 16) return 19;
    if (*(const uint8_t *)api->get_memory_data(0) != (uint8_t)expected_seed) return 20;
    restored = state ^ 0x55aa55aau;
    if (!api->unserialize(&restored, sizeof(restored))) return 21;
    if (!api->serialize(&state, sizeof(state)) || state != restored) return 22;
    if (api->get_memory_data(99) || api->get_memory_size(99) != 0) return 23;
    api->unload_game();
    api->deinit();
    if (id == 0 ? !mgba_test_deinitialized() : !gpsp_test_deinitialized()) return 24;
    return 0;
}
int static_abi_run(void) {
    int a = exercise(0, 31, 60);
    if (a) return a;
    int b = exercise(1, 47, 50);
    if (b) return b;
    if (linkslot_static_core_get(99) != 0) return 25;
    return 0;
}
#ifdef STATIC_ABI_CROSS
volatile int static_abi_cross_result;
void _start(void) {
    static_abi_cross_result = static_abi_run();
    for (;;) { __asm__ volatile("wfi"); }
}
#else
int main(void) { return static_abi_run(); }
#endif
'''

def compile_rust(out: Path, target: str) -> Path:
    source = out / "static-core-api.rs"
    source.write_text(rust_source(), encoding="utf-8")
    archive = out / ("libstatic_core_api-" + target + ".a")
    run(["rustc", "--edition=2021", "--crate-type=staticlib", "--target", target,
         "-C", "panic=abort", "-C", "force-unwind-tables=no", "-C", "opt-level=2",
         str(source), "-o", str(archive)], out / ("rust-" + target + ".log"))
    return archive

def compile_c(out: Path, compiler: str, target_arg=None, cpu=None, cross=False):
    (out / "static_abi_c.h").write_text(c_header(), encoding="utf-8")
    harness_path = out / ("abi_harness_cross.c" if cross else "abi_harness_host.c")
    harness_path.write_text(harness(), encoding="utf-8")
    objects = []
    for prefix, seed, fps in (("mgba", 31, 60), ("gpsp", 47, 50)):
        source = out / (prefix + "_mock_core.c")
        source.write_text(mock_core(prefix, seed, fps), encoding="utf-8")
        obj = out / (prefix + ("-cross.o" if cross else "-host.o"))
        cmd = [compiler]
        if target_arg:
            cmd.append(target_arg)
        if cpu:
            cmd += ["-mcpu=" + cpu]
            if target_arg and "armv7a" in target_arg:
                cmd.append("-marm")
        cmd += ["-std=c11", "-ffreestanding", "-fno-builtin", "-fno-stack-protector",
                "-fno-unwind-tables", "-fno-asynchronous-unwind-tables",
                "-Wall", "-Wextra", "-Werror", "-I", str(out), "-c", str(source), "-o", str(obj)]
        run(cmd, out / (obj.name + ".log"))
        objects.append(obj)
    harness_obj = out / ("harness-cross.o" if cross else "harness-host.o")
    cmd = [compiler]
    if target_arg:
        cmd.append(target_arg)
    if cpu:
        cmd += ["-mcpu=" + cpu]
        if target_arg and "armv7a" in target_arg:
            cmd.append("-marm")
    cmd += ["-std=c11", "-ffreestanding", "-fno-builtin", "-fno-stack-protector",
            "-fno-unwind-tables", "-fno-asynchronous-unwind-tables",
            "-Wall", "-Wextra", "-Werror", "-I", str(out)]
    if cross:
        cmd.append("-DSTATIC_ABI_CROSS=1")
    cmd += ["-c", str(harness_path), "-o", str(harness_obj)]
    run(cmd, out / (harness_obj.name + ".log"))
    objects.append(harness_obj)
    return objects

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    require_tools()
    report = [
        "# Static Libretro core ABI feasibility\n\n",
        "- Two mock cores expose separately prefixed retro_* symbols (mgba and gpsp).\n",
        "- Rust provides a static API registry; no dlopen, dlsym or libloading is used.\n",
        "- The C harness exercises environment, video, audio, input, load/run, save-state, memory and cheat callbacks.\n\n"
    ]
    for target, (clang_target, emulation, machine) in TARGETS.items():
        archive = compile_rust(out, target)
        objects = compile_c(out, "clang", "--target=" + clang_target, "cortex-a53", True)
        elf = out / ("static-core-" + target + ".elf")
        mapfile = out / ("static-core-" + target + ".map")
        run(["ld.lld", "-m", emulation, "-static", "-e", "_start", "--no-undefined",
             "-Map=" + str(mapfile), "-o", str(elf), *map(str, objects), str(archive)],
            out / (target + "-link.log"))
        header = run(["readelf", "-h", str(elf)])
        undefined = run(["llvm-nm", "-u", str(elf)])
        if machine not in header or undefined.strip():
            raise RuntimeError(target + ": wrong ELF machine or unresolved symbols: " + undefined)
        report.append("- PASS: " + target + " Rust static API + two namespaced C mock cores; ELF machine " + machine + "; no undefined symbols; map generated.\n")

    host_archive = compile_rust(out, "x86_64-unknown-linux-gnu")
    host_objects = compile_c(out, "cc")
    host_exe = out / "static-core-host-test"
    run(["cc", "-o", str(host_exe), *map(str, host_objects), str(host_archive),
         "-ldl", "-lpthread", "-lm", "-lrt", "-lutil"], out / "host-link.log")
    run([str(host_exe)], out / "host-run.log")
    report.append("- PASS: x86_64 host harness ran both mock cores and verified callback, save-state and memory round-trips (exit 0).\n")
    report.append("- Boundary: these are mock cores. The test proves static namespacing/table plumbing only, not mGBA/gpSP build integration or emulator correctness.\n")
    (out / "REPORT.md").write_text("".join(report), encoding="utf-8")
    print((out / "REPORT.md").read_text(encoding="utf-8"))

if __name__ == "__main__":
    main()
