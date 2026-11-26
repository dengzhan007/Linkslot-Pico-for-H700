#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "../include/linkslot_vfs.h"

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

struct StubVfsInterfaceInfo {
    uint32_t required_interface_version;
    void *iface;
};

static uint32_t init_calls;
static uint32_t deinit_calls;
static EnvironmentFn environment_callback;
static bool vfs_request_ok;

static const char *stub_vfs_get_path(retro_vfs_file_handle *handle) {
    (void)handle;
    return NULL;
}
static retro_vfs_file_handle *stub_vfs_open(const char *path, unsigned mode, unsigned hints) {
    (void)path; (void)mode; (void)hints;
    return NULL;
}
static int stub_vfs_close(retro_vfs_file_handle *handle) {
    (void)handle;
    return -1;
}
static int64_t stub_vfs_size(retro_vfs_file_handle *handle) {
    (void)handle;
    return -1;
}
static int64_t stub_vfs_truncate(retro_vfs_file_handle *handle, int64_t length) {
    (void)handle; (void)length;
    return -1;
}
static int64_t stub_vfs_tell(retro_vfs_file_handle *handle) {
    (void)handle;
    return -1;
}
static int64_t stub_vfs_seek(retro_vfs_file_handle *handle, int64_t offset, int whence) {
    (void)handle; (void)offset; (void)whence;
    return -1;
}
static int64_t stub_vfs_read(retro_vfs_file_handle *handle, void *buffer, uint64_t length) {
    (void)handle; (void)buffer; (void)length;
    return -1;
}
static int64_t stub_vfs_write(retro_vfs_file_handle *handle, const void *buffer, uint64_t length) {
    (void)handle; (void)buffer; (void)length;
    return -1;
}
static int stub_vfs_flush(retro_vfs_file_handle *handle) {
    (void)handle;
    return -1;
}
static int stub_vfs_remove(const char *path) {
    (void)path;
    return -1;
}
static int stub_vfs_rename(const char *old_path, const char *new_path) {
    (void)old_path; (void)new_path;
    return -1;
}
static int stub_vfs_stat(const char *path, int32_t *size) {
    (void)path; (void)size;
    return 0;
}
static int stub_vfs_mkdir(const char *path) {
    (void)path;
    return -1;
}
static retro_vfs_dir_handle *stub_vfs_opendir(const char *path, bool include_hidden) {
    (void)path; (void)include_hidden;
    return NULL;
}
static bool stub_vfs_readdir(retro_vfs_dir_handle *dir) {
    (void)dir;
    return false;
}
static const char *stub_vfs_dirent_get_name(retro_vfs_dir_handle *dir) {
    (void)dir;
    return NULL;
}
static bool stub_vfs_dirent_is_dir(retro_vfs_dir_handle *dir) {
    (void)dir;
    return false;
}
static int stub_vfs_closedir(retro_vfs_dir_handle *dir) {
    (void)dir;
    return -1;
}

static const linkslot_vfs_backend STUB_VFS_BACKEND = {
    .get_path = stub_vfs_get_path,
    .open = stub_vfs_open,
    .close = stub_vfs_close,
    .size = stub_vfs_size,
    .tell = stub_vfs_tell,
    .seek = stub_vfs_seek,
    .read = stub_vfs_read,
    .write = stub_vfs_write,
    .flush = stub_vfs_flush,
    .remove = stub_vfs_remove,
    .rename = stub_vfs_rename,
    .truncate = stub_vfs_truncate,
    .stat = stub_vfs_stat,
    .mkdir = stub_vfs_mkdir,
    .opendir = stub_vfs_opendir,
    .readdir = stub_vfs_readdir,
    .dirent_get_name = stub_vfs_dirent_get_name,
    .dirent_is_dir = stub_vfs_dirent_is_dir,
    .closedir = stub_vfs_closedir,
};

bool linkslot_mgba_stub_install_vfs(void) {
    return linkslot_vfs_install(&STUB_VFS_BACKEND);
}

bool linkslot_mgba_stub_vfs_request_ok(void) {
    return vfs_request_ok;
}

void mgba_retro_cheat_reset(void) {}
void mgba_retro_cheat_set(uint32_t index, bool enabled, const char *code) {
    (void)index;
    (void)enabled;
    (void)code;
}
void mgba_retro_init(void) {
    struct StubVfsInterfaceInfo info = { 3u, NULL };
    ++init_calls;
    vfs_request_ok = environment_callback != NULL &&
        environment_callback(LINKSLOT_RETRO_ENVIRONMENT_GET_VFS_INTERFACE, &info) &&
        info.required_interface_version == 3u && info.iface != NULL;
}
void mgba_retro_deinit(void) { ++deinit_calls; }
uint32_t mgba_retro_api_version(void) { return 1; }
void mgba_retro_get_system_av_info(SystemAvInfo *info) { (void)info; }
void mgba_retro_set_environment(EnvironmentFn callback) {
    environment_callback = callback;
}
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

void linkslot_mgba_stub_reset_counters(void) {
    init_calls = 0u;
    deinit_calls = 0u;
    vfs_request_ok = false;
}
uint32_t linkslot_mgba_stub_init_calls(void) { return init_calls; }
uint32_t linkslot_mgba_stub_deinit_calls(void) { return deinit_calls; }
