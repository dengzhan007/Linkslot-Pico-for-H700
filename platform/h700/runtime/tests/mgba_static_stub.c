#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

typedef struct {
    const char *path;
    const void *data;
    size_t size;
    const char *meta;
} GameInfo;

typedef struct {
    uint32_t base_width;
    uint32_t base_height;
    uint32_t max_width;
    uint32_t max_height;
    float aspect_ratio;
} GameGeometry;

typedef struct {
    double fps;
    double sample_rate;
} SystemTiming;

typedef struct {
    GameGeometry geometry;
    SystemTiming timing;
} SystemAvInfo;

typedef bool (*EnvironmentFn)(uint32_t, void *);
typedef void (*VideoRefreshFn)(const void *, uint32_t, uint32_t, size_t);
typedef void (*AudioSampleFn)(int16_t, int16_t);
typedef size_t (*AudioBatchFn)(const int16_t *, size_t);
typedef void (*InputPollFn)(void);
typedef int16_t (*InputStateFn)(uint32_t, uint32_t, uint32_t, uint32_t);

static uint32_t init_calls;
static uint32_t deinit_calls;

void mgba_retro_cheat_reset(void) {}
void mgba_retro_cheat_set(uint32_t index, bool enabled, const char *code) {
    (void)index;
    (void)enabled;
    (void)code;
}
void mgba_retro_init(void) { ++init_calls; }
void mgba_retro_deinit(void) { ++deinit_calls; }
uint32_t mgba_retro_api_version(void) { return 1; }
void mgba_retro_get_system_av_info(SystemAvInfo *info) { (void)info; }
void mgba_retro_set_environment(EnvironmentFn callback) { (void)callback; }
void mgba_retro_set_video_refresh(VideoRefreshFn callback) { (void)callback; }
void mgba_retro_set_audio_sample(AudioSampleFn callback) { (void)callback; }
void mgba_retro_set_audio_sample_batch(AudioBatchFn callback) { (void)callback; }
void mgba_retro_set_input_poll(InputPollFn callback) { (void)callback; }
void mgba_retro_set_input_state(InputStateFn callback) { (void)callback; }
void mgba_retro_set_controller_port_device(uint32_t port, uint32_t device) {
    (void)port;
    (void)device;
}
bool mgba_retro_load_game(const GameInfo *info) {
    (void)info;
    return false;
}
void mgba_retro_unload_game(void) {}
void mgba_retro_run(void) {}
size_t mgba_retro_serialize_size(void) { return 0; }
bool mgba_retro_serialize(void *data, size_t size) {
    (void)data;
    (void)size;
    return false;
}
bool mgba_retro_unserialize(const void *data, size_t size) {
    (void)data;
    (void)size;
    return false;
}
void *mgba_retro_get_memory_data(uint32_t id) {
    (void)id;
    return NULL;
}
size_t mgba_retro_get_memory_size(uint32_t id) {
    (void)id;
    return 0;
}

uint32_t linkslot_mgba_stub_init_calls(void) { return init_calls; }
uint32_t linkslot_mgba_stub_deinit_calls(void) { return deinit_calls; }
