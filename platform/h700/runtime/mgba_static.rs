//! H700 static binding for the namespaced mGBA Libretro core.
//!
//! Compile with the static-mgba Cargo feature and link the matching
//! namespaced mGBA archive into the final executable. Host tests provide a C
//! stub archive with the same exported symbol names.

use core::ffi::{c_char, c_uint, c_void};

use crate::ffi::{
    Api, AudioBatchFn, AudioSampleFn, EnvironmentFn, GameInfo, InputPollFn,
    InputStateFn, SystemAvInfo, VideoRefreshFn,
};

unsafe extern "C" {
    #[link_name = "mgba_retro_cheat_reset"]
    fn core_cheat_reset();
    #[link_name = "mgba_retro_cheat_set"]
    fn core_cheat_set(index: c_uint, enabled: bool, code: *const c_char);
    #[link_name = "mgba_retro_init"]
    fn core_init();
    #[link_name = "mgba_retro_deinit"]
    fn core_deinit();
    #[link_name = "mgba_retro_api_version"]
    fn core_api_version() -> c_uint;
    #[link_name = "mgba_retro_get_system_av_info"]
    fn core_get_system_av_info(info: *mut SystemAvInfo);
    #[link_name = "mgba_retro_set_environment"]
    fn core_set_environment(callback: EnvironmentFn);
    #[link_name = "mgba_retro_set_video_refresh"]
    fn core_set_video_refresh(callback: VideoRefreshFn);
    #[link_name = "mgba_retro_set_audio_sample"]
    fn core_set_audio_sample(callback: AudioSampleFn);
    #[link_name = "mgba_retro_set_audio_sample_batch"]
    fn core_set_audio_sample_batch(callback: AudioBatchFn);
    #[link_name = "mgba_retro_set_input_poll"]
    fn core_set_input_poll(callback: InputPollFn);
    #[link_name = "mgba_retro_set_input_state"]
    fn core_set_input_state(callback: InputStateFn);
    #[link_name = "mgba_retro_set_controller_port_device"]
    fn core_set_controller_port_device(port: c_uint, device: c_uint);
    #[link_name = "mgba_retro_load_game"]
    fn core_load_game(info: *const GameInfo) -> bool;
    #[link_name = "mgba_retro_unload_game"]
    fn core_unload_game();
    #[link_name = "mgba_retro_run"]
    fn core_run();
    #[link_name = "mgba_retro_serialize_size"]
    fn core_serialize_size() -> usize;
    #[link_name = "mgba_retro_serialize"]
    fn core_serialize(data: *mut c_void, size: usize) -> bool;
    #[link_name = "mgba_retro_unserialize"]
    fn core_unserialize(data: *const c_void, size: usize) -> bool;
    #[link_name = "mgba_retro_get_memory_data"]
    fn core_get_memory_data(id: c_uint) -> *mut c_void;
    #[link_name = "mgba_retro_get_memory_size"]
    fn core_get_memory_size(id: c_uint) -> usize;
}

/// Return the real namespaced mGBA entry points in Linkslot's API shape.
///
/// All function pointers refer to code resident in the statically linked
/// mGBA archive and remain valid for the lifetime of the process.
pub static API: Api = Api {
        cheat_reset: Some(core_cheat_reset),
        cheat_set: Some(core_cheat_set),
        init: core_init,
        deinit: core_deinit,
        api_version: core_api_version,
        get_system_av_info: core_get_system_av_info,
        set_environment: core_set_environment,
        set_video_refresh: core_set_video_refresh,
        set_audio_sample: core_set_audio_sample,
        set_audio_sample_batch: core_set_audio_sample_batch,
        set_input_poll: core_set_input_poll,
        set_input_state: core_set_input_state,
        set_controller_port_device: core_set_controller_port_device,
        load_game: core_load_game,
        unload_game: core_unload_game,
        run: core_run,
        serialize_size: core_serialize_size,
        serialize: core_serialize,
        unserialize: core_unserialize,
        get_memory_data: core_get_memory_data,
        get_memory_size: core_get_memory_size,
};

/// Return a copy of the resident function table for Linkslot's static core API.
pub fn api() -> Api {
    API
}
