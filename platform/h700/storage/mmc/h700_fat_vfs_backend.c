#include "h700_fat_vfs_backend.h"

#define H700_FAT_VFS_PROVIDER_MAGIC 0x48375646u
#define H700_FAT_VFS_FILE_MAGIC     0x48374649u
#define H700_FAT_VFS_DIR_MAGIC      0x48374449u

static struct h700_fat_vfs *g_provider;

static const char *const g_allowed_roots[] = {
    "Games", "BIOS", "Saves", "States", "System", "linkSlot"
};

static size_t bounded_length(const char *text, size_t capacity)
{
    size_t length = 0u;
    if (text == 0)
        return capacity;
    while (length < capacity && text[length] != '\0')
        ++length;
    return length;
}

static void copy_bytes(char *destination, const char *source, size_t length)
{
    size_t i;
    for (i = 0u; i < length; ++i)
        destination[i] = source[i];
    destination[length] = '\0';
}

static int allowed_root(const char *component, size_t length)
{
    size_t i;
    for (i = 0u; i < sizeof(g_allowed_roots) / sizeof(g_allowed_roots[0]); ++i) {
        size_t root_length = bounded_length(g_allowed_roots[i], 32u);
        size_t j;
        if (root_length != length)
            continue;
        for (j = 0u; j < length; ++j)
            if (component[j] != g_allowed_roots[i][j])
                break;
        if (j == length)
            return 1;
    }
    return 0;
}

static int valid_mount_prefix(const char *prefix, char output[H700_FAT_VFS_MOUNT_CAPACITY])
{
    size_t length = bounded_length(prefix, H700_FAT_VFS_MOUNT_CAPACITY);
    size_t i;
    size_t component_start = 1u;

    if (prefix == 0 || length < 2u ||
        length >= H700_FAT_VFS_MOUNT_CAPACITY ||
        prefix[0] != '/' || prefix[length - 1u] == '/')
        return 0;

    for (i = 0u; i < length; ++i) {
        unsigned char ch = (unsigned char)prefix[i];
        if (ch < 0x20u || ch == 0x7fu || ch == '\\' || ch == ':')
            return 0;
        if (i == 0u)
            continue;
        if (ch == '/') {
            size_t component_length = i - component_start;
            if (component_length == 0u ||
                (component_length == 1u && prefix[component_start] == '.') ||
                (component_length == 2u && prefix[component_start] == '.' &&
                 prefix[component_start + 1u] == '.'))
                return 0;
            component_start = i + 1u;
        }
    }

    if (component_start >= length ||
        (length - component_start == 1u && prefix[component_start] == '.') ||
        (length - component_start == 2u && prefix[component_start] == '.' &&
         prefix[component_start + 1u] == '.'))
        return 0;

    copy_bytes(output, prefix, length);
    return 1;
}

/*
 * Resolve only absolute paths beneath the exact configured mount prefix.
 * Empty suffix means the virtual card root; all other suffixes must be under
 * one of the explicitly exposed first-level directories.
 */
static int allowed_root_name_case_insensitive(const char *component,
                                            size_t length)
{
    size_t i;
    for (i = 0u; i < sizeof(g_allowed_roots) / sizeof(g_allowed_roots[0]); ++i) {
        size_t root_length = bounded_length(g_allowed_roots[i], 32u);
        size_t j;
        if (root_length != length)
            continue;
        for (j = 0u; j < length; ++j) {
            unsigned char a = (unsigned char)component[j];
            unsigned char b = (unsigned char)g_allowed_roots[i][j];
            if (a >= 'a' && a <= 'z') a = (unsigned char)(a - 'a' + 'A');
            if (b >= 'a' && b <= 'z') b = (unsigned char)(b - 'a' + 'A');
            if (a != b)
                break;
        }
        if (j == length)
            return 1;
    }
    return 0;
}

/*
 * Resolve only absolute paths beneath the exact configured mount prefix.
 * Empty suffix means the virtual card root; all other suffixes must be under
 * one of the explicitly exposed first-level directories.
 */
static int resolve_path(const struct h700_fat_vfs *state,
                        const char *request,
                        char relative[H700_FAT_VFS_PATH_CAPACITY],
                        int allow_virtual_root)
{
    size_t request_length = bounded_length(request, H700_FAT_VFS_PATH_CAPACITY);
    size_t prefix_length = bounded_length(state->mount_prefix,
                                          H700_FAT_VFS_MOUNT_CAPACITY);
    size_t suffix_length;
    size_t component_start = 0u;
    size_t i;
    int first_component = 1;
    const char *suffix;

    if (request == 0 || request_length == 0u ||
        request_length >= H700_FAT_VFS_PATH_CAPACITY ||
        request[0] != '/' || prefix_length == 0u ||
        request_length < prefix_length)
        return -1;

    for (i = 0u; i < request_length; ++i) {
        unsigned char ch = (unsigned char)request[i];
        if (ch < 0x20u || ch == 0x7fu || ch == '\\' || ch == ':')
            return -1;
    }

    for (i = 0u; i < prefix_length; ++i)
        if (request[i] != state->mount_prefix[i])
            return -1;

    if (request_length == prefix_length) {
        relative[0] = '\0';
        return allow_virtual_root ? 0 : -1;
    }

    if (request[prefix_length] != '/')
        return -1; /* Reject e.g. /mnt/sdcard2 when the mount is /mnt/sdcard. */

    suffix = request + prefix_length + 1u;
    suffix_length = request_length - prefix_length - 1u;
    if (suffix_length == 0u || request[request_length - 1u] == '/')
        return -1;

    for (i = 0u; i <= suffix_length; ++i) {
        if (i == suffix_length || suffix[i] == '/') {
            size_t component_length = i - component_start;

            if (component_length == 0u ||
                (component_length == 1u && suffix[component_start] == '.') ||
                (component_length == 2u && suffix[component_start] == '.' &&
                 suffix[component_start + 1u] == '.'))
                return -1;

            if (first_component != 0) {
                if (!allowed_root(suffix + component_start, component_length))
                    return -1;
                first_component = 0;
            }
            component_start = i + 1u;
        }
    }

    copy_bytes(relative, suffix, suffix_length);
    return 0;
}

static struct h700_fat_vfs *provider(void)
{
    if (g_provider == 0 ||
        g_provider->magic != H700_FAT_VFS_PROVIDER_MAGIC ||
        g_provider->volume.mounted == 0u ||
        g_provider->volume.wr != 0)
        return 0;
    return g_provider;
}

static struct h700_fat_vfs_file_slot *file_slot(retro_vfs_file_handle *handle)
{
    struct h700_fat_vfs *state = provider();
    size_t i;
    if (state == 0 || handle == 0)
        return 0;
    for (i = 0u; i < H700_FAT_VFS_MAX_FILES; ++i) {
        if ((retro_vfs_file_handle *)&state->files[i] == handle &&
            state->files[i].active != 0u &&
            state->files[i].magic == H700_FAT_VFS_FILE_MAGIC)
            return &state->files[i];
    }
    return 0;
}

static struct h700_fat_vfs_dir_slot *dir_slot(retro_vfs_dir_handle *handle)
{
    struct h700_fat_vfs *state = provider();
    size_t i;
    if (state == 0 || handle == 0)
        return 0;
    for (i = 0u; i < H700_FAT_VFS_MAX_DIRS; ++i) {
        if ((retro_vfs_dir_handle *)&state->directories[i] == handle &&
            state->directories[i].active != 0u &&
            state->directories[i].magic == H700_FAT_VFS_DIR_MAGIC)
            return &state->directories[i];
    }
    return 0;
}

static const char *vfs_get_path(retro_vfs_file_handle *handle)
{
    struct h700_fat_vfs_file_slot *slot = file_slot(handle);
    return slot != 0 ? slot->path : 0;
}

static retro_vfs_file_handle *vfs_open(const char *path,
                                       unsigned mode,
                                       unsigned hints)
{
    struct h700_fat_vfs *state = provider();
    struct h700_fat_vfs_file_slot *slot = 0;
    char relative[H700_FAT_VFS_PATH_CAPACITY];
    fat_file file;
    size_t i;
    size_t path_length;

    (void)hints;
    if (state == 0 || mode != LINKSLOT_RETRO_VFS_FILE_ACCESS_READ ||
        resolve_path(state, path, relative, 0) != 0)
        return 0;

    if (fat_open(&state->volume, relative, &file) != 0)
        return 0;

    for (i = 0u; i < H700_FAT_VFS_MAX_FILES; ++i)
        if (state->files[i].active == 0u) {
            slot = &state->files[i];
            break;
        }
    if (slot == 0)
        return 0;

    path_length = bounded_length(path, H700_FAT_VFS_PATH_CAPACITY);
    if (path_length >= H700_FAT_VFS_PATH_CAPACITY)
        return 0;

    slot->magic = H700_FAT_VFS_FILE_MAGIC;
    slot->active = 1u;
    slot->position = 0u;
    slot->file = file;
    copy_bytes(slot->path, path, path_length);
    return (retro_vfs_file_handle *)slot;
}

static int vfs_close(retro_vfs_file_handle *handle)
{
    struct h700_fat_vfs_file_slot *slot = file_slot(handle);
    size_t i;
    if (slot == 0)
        return -1;
    slot->active = 0u;
    slot->magic = 0u;
    slot->position = 0u;
    slot->path[0] = '\0';
    for (i = 0u; i < sizeof(slot->file); ++i)
        ((uint8_t *)&slot->file)[i] = 0u;
    return 0;
}

static int64_t vfs_size(retro_vfs_file_handle *handle)
{
    struct h700_fat_vfs_file_slot *slot = file_slot(handle);
    return slot != 0 ? (int64_t)slot->file.size : -1;
}

static int64_t vfs_tell(retro_vfs_file_handle *handle)
{
    struct h700_fat_vfs_file_slot *slot = file_slot(handle);
    return slot != 0 ? (int64_t)slot->position : -1;
}

static int64_t vfs_seek(retro_vfs_file_handle *handle, int64_t offset, int whence)
{
    struct h700_fat_vfs_file_slot *slot = file_slot(handle);
    uint64_t base;
    uint64_t target;

    if (slot == 0)
        return -1;
    if (whence == LINKSLOT_RETRO_VFS_SEEK_POSITION_START)
        base = 0u;
    else if (whence == LINKSLOT_RETRO_VFS_SEEK_POSITION_CURRENT)
        base = slot->position;
    else if (whence == LINKSLOT_RETRO_VFS_SEEK_POSITION_END)
        base = slot->file.size;
    else
        return -1;

    if (offset >= 0) {
        if ((uint64_t)offset > 0xffffffffull - base)
            return -1;
        target = base + (uint64_t)offset;
    } else {
        uint64_t magnitude = (uint64_t)(-(offset + 1)) + 1u;
        if (magnitude > base)
            return -1;
        target = base - magnitude;
    }

    slot->position = (uint32_t)target;
    return (int64_t)slot->position;
}

static int64_t vfs_read(retro_vfs_file_handle *handle, void *buffer, uint64_t length)
{
    struct h700_fat_vfs_file_slot *slot = file_slot(handle);
    uint64_t remaining;
    uint32_t request;
    uint32_t received;

    if (slot == 0 || (buffer == 0 && length != 0u))
        return -1;
    if (length == 0u || slot->position >= slot->file.size)
        return 0;

    remaining = (uint64_t)slot->file.size - slot->position;
    if (length > remaining)
        length = remaining;
    if (length > 0xffffffffull)
        length = 0xffffffffull;
    request = (uint32_t)length;

    received = fat_read(&slot->file, slot->position, buffer, request);
    slot->position += received;
    return (int64_t)received;
}

static int64_t vfs_write(retro_vfs_file_handle *handle,
                         const void *buffer,
                         uint64_t length)
{
    (void)handle;
    (void)buffer;
    (void)length;
    return -1;
}

static int vfs_flush(retro_vfs_file_handle *handle)
{
    (void)handle;
    return -1;
}

static int vfs_remove(const char *path)
{
    (void)path;
    return -1;
}

static int vfs_rename(const char *old_path, const char *new_path)
{
    (void)old_path;
    (void)new_path;
    return -1;
}

static int64_t vfs_truncate(retro_vfs_file_handle *handle, int64_t length)
{
    (void)handle;
    (void)length;
    return -1;
}

static int vfs_stat(const char *path, int32_t *size)
{
    struct h700_fat_vfs *state = provider();
    char relative[H700_FAT_VFS_PATH_CAPACITY];
    fat_file file;
    fat_dir directory;

    if (size != 0)
        *size = 0;
    if (state == 0 || resolve_path(state, path, relative, 1) != 0)
        return 0;

    if (relative[0] == '\0')
        return LINKSLOT_RETRO_VFS_STAT_IS_VALID |
               LINKSLOT_RETRO_VFS_STAT_IS_DIRECTORY;

    if (fat_open(&state->volume, relative, &file) == 0) {
        if (size != 0)
            *size = file.size > 0x7fffffffU ? 0x7fffffff : (int32_t)file.size;
        return LINKSLOT_RETRO_VFS_STAT_IS_VALID;
    }

    if (fat_opendir(&state->volume, relative, &directory) == 0)
        return LINKSLOT_RETRO_VFS_STAT_IS_VALID |
               LINKSLOT_RETRO_VFS_STAT_IS_DIRECTORY;

    return 0;
}

static int vfs_mkdir(const char *path)
{
    (void)path;
    return -1;
}

static retro_vfs_dir_handle *vfs_opendir(const char *path, bool include_hidden)
{
    struct h700_fat_vfs *state = provider();
    struct h700_fat_vfs_dir_slot *slot = 0;
    char relative[H700_FAT_VFS_PATH_CAPACITY];
    size_t i;
    size_t path_length;

    (void)include_hidden;
    if (state == 0 || resolve_path(state, path, relative, 1) != 0)
        return 0;

    for (i = 0u; i < H700_FAT_VFS_MAX_DIRS; ++i)
        if (state->directories[i].active == 0u) {
            slot = &state->directories[i];
            break;
        }
    if (slot == 0)
        return 0;

    if (relative[0] == '\0') {
        if (fat_opendir(&state->volume, "/", &slot->directory) != 0)
            return 0;
        slot->virtual_root = 1u;
    } else {
        if (fat_opendir(&state->volume, relative, &slot->directory) != 0)
            return 0;
        slot->virtual_root = 0u;
    }

    path_length = bounded_length(path, H700_FAT_VFS_PATH_CAPACITY);
    if (path_length >= H700_FAT_VFS_PATH_CAPACITY)
        return 0;

    slot->magic = H700_FAT_VFS_DIR_MAGIC;
    slot->active = 1u;
    slot->current_valid = 0u;
    slot->root_scan_done = 0u;
    slot->root_index = 0u;
    copy_bytes(slot->path, path, path_length);
    return (retro_vfs_dir_handle *)slot;
}

static bool vfs_readdir(retro_vfs_dir_handle *handle)
{
    struct h700_fat_vfs_dir_slot *slot = dir_slot(handle);
    if (slot == 0)
        return false;

    slot->current_valid = 0u;
    while (fat_readdir(&slot->directory, &slot->current) != 0) {
        if (slot->virtual_root != 0u) {
            if (slot->current.is_dir == 0u ||
                !allowed_root_name_case_insensitive(
                    slot->current.name,
                    bounded_length(slot->current.name,
                                   sizeof(slot->current.name))))
                continue;
        }
        slot->current_valid = 1u;
        return true;
    }
    slot->root_scan_done = 1u;
    return false;
}

static const char *vfs_dirent_get_name(retro_vfs_dir_handle *handle)
{
    struct h700_fat_vfs_dir_slot *slot = dir_slot(handle);
    if (slot == 0 || slot->current_valid == 0u)
        return 0;
    return slot->current.name;
}

static bool vfs_dirent_is_dir(retro_vfs_dir_handle *handle)
{
    struct h700_fat_vfs_dir_slot *slot = dir_slot(handle);
    return slot != 0 && slot->current_valid != 0u &&
           slot->current.is_dir != 0u;
}

static int vfs_closedir(retro_vfs_dir_handle *handle)
{
    struct h700_fat_vfs_dir_slot *slot = dir_slot(handle);
    if (slot == 0)
        return -1;
    slot->active = 0u;
    slot->magic = 0u;
    slot->current_valid = 0u;
    slot->path[0] = '\0';
    return 0;
}

static const linkslot_vfs_backend g_vfs_backend = {
    .get_path = vfs_get_path,
    .open = vfs_open,
    .close = vfs_close,
    .size = vfs_size,
    .tell = vfs_tell,
    .seek = vfs_seek,
    .read = vfs_read,
    .write = vfs_write,
    .flush = vfs_flush,
    .remove = vfs_remove,
    .rename = vfs_rename,
    .truncate = vfs_truncate,
    .stat = vfs_stat,
    .mkdir = vfs_mkdir,
    .opendir = vfs_opendir,
    .readdir = vfs_readdir,
    .dirent_get_name = vfs_dirent_get_name,
    .dirent_is_dir = vfs_dirent_is_dir,
    .closedir = vfs_closedir
};

int h700_fat_vfs_mount_and_install(
    struct h700_fat_vfs *state,
    struct h700_sunxi_mmc_block_device *device,
    const struct h700_gpt_partition *partition,
    const char *mount_prefix)
{
    char validated_prefix[H700_FAT_VFS_MOUNT_CAPACITY];
    size_t i;

    if (state == 0 || device == 0 || partition == 0 ||
        g_provider != 0 ||
        !valid_mount_prefix(mount_prefix, validated_prefix))
        return -1;

    /* The state must not be reused after installation; the VFS bridge is one-shot. */
    for (i = 0u; i < sizeof(*state); ++i)
        ((uint8_t *)state)[i] = 0u;

    if (h700_sunxi_mmc_fat_mount_gpt_partition(
            device, &state->volume, partition) != 0 ||
        state->volume.wr != 0)
        return -1;

    copy_bytes(state->mount_prefix, validated_prefix,
               bounded_length(validated_prefix, sizeof(validated_prefix)));
    state->magic = H700_FAT_VFS_PROVIDER_MAGIC;
    g_provider = state;

    if (!linkslot_vfs_install(&g_vfs_backend)) {
        g_provider = 0;
        state->magic = 0u;
        return -1;
    }

    return 0;
}


enum h700_fat_vfs_bootstrap_result h700_fat_vfs_scan_partition_and_install(
    struct h700_fat_vfs *state,
    struct h700_sunxi_mmc_block_device *device,
    const struct h700_sunxi_sdc0_scan_config *scan_config,
    struct h700_gpt_partition *partitions,
    size_t partition_capacity,
    const char *partition_name,
    const char *mount_prefix,
    struct h700_sunxi_sdc0_scan_result *scan_result)
{
    const struct h700_gpt_partition *partition = 0;
    struct h700_sunxi_mmc_block_timeouts timeouts;
    struct h700_block_reader unused_reader;
    enum h700_sunxi_sdc0_scan_status scan_status;
    enum h700_gpt_result lookup_status;

    if (state == 0 || device == 0 || scan_config == 0 ||
        partitions == 0 || partition_capacity < H700_GPT_MAX_ENTRIES ||
        partition_name == 0 || partition_name[0] == '\0' ||
        scan_result == 0 || mount_prefix == 0)
        return H700_FAT_VFS_BOOTSTRAP_INVALID;

    scan_status = h700_sunxi_sdc0_scan_primary_gpt(
        scan_config, partitions, partition_capacity, scan_result);
    if (scan_status != H700_SDC0_SCAN_OK)
        return H700_FAT_VFS_BOOTSTRAP_SCAN_FAILED;

    lookup_status = h700_gpt_find_unique_name(
        partitions, scan_result->gpt.partition_count,
        partition_name, &partition);
    if (lookup_status != H700_GPT_OK || partition == 0)
        return H700_FAT_VFS_BOOTSTRAP_PARTITION_NOT_FOUND;

    timeouts.command_timeout_ms = scan_config->command_timeout_ms;
    timeouts.fifo_timeout_ms = scan_config->fifo_timeout_ms;
    timeouts.data_timeout_ms = scan_config->data_timeout_ms;
    timeouts.recovery_timeout_ms = scan_config->recovery_timeout_ms;

    if (h700_sunxi_mmc_block_device_init(
            device, &unused_reader,
            scan_config->platform->mmc_io, &scan_result->card,
            &timeouts) != 0)
        return H700_FAT_VFS_BOOTSTRAP_BLOCK_DEVICE_FAILED;

    if (h700_fat_vfs_mount_and_install(
            state, device, partition, mount_prefix) != 0)
        return H700_FAT_VFS_BOOTSTRAP_INSTALL_FAILED;

    return H700_FAT_VFS_BOOTSTRAP_OK;
}
