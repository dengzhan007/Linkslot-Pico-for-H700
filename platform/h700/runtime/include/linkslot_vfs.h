#ifndef LINKSLOT_H700_LIBRETRO_VFS_H
#define LINKSLOT_H700_LIBRETRO_VFS_H
#include <stdbool.h>
#include <stdint.h>

typedef struct retro_vfs_file_handle retro_vfs_file_handle;
typedef struct retro_vfs_dir_handle retro_vfs_dir_handle;

typedef const char *(*linkslot_vfs_get_path_fn)(retro_vfs_file_handle *);
typedef retro_vfs_file_handle *(*linkslot_vfs_open_fn)(const char *, unsigned, unsigned);
typedef int (*linkslot_vfs_close_fn)(retro_vfs_file_handle *);
typedef int64_t (*linkslot_vfs_size_fn)(retro_vfs_file_handle *);
typedef int64_t (*linkslot_vfs_truncate_fn)(retro_vfs_file_handle *, int64_t);
typedef int64_t (*linkslot_vfs_tell_fn)(retro_vfs_file_handle *);
typedef int64_t (*linkslot_vfs_seek_fn)(retro_vfs_file_handle *, int64_t, int);
typedef int64_t (*linkslot_vfs_read_fn)(retro_vfs_file_handle *, void *, uint64_t);
typedef int64_t (*linkslot_vfs_write_fn)(retro_vfs_file_handle *, const void *, uint64_t);
typedef int (*linkslot_vfs_flush_fn)(retro_vfs_file_handle *);
typedef int (*linkslot_vfs_path_fn)(const char *);
typedef int (*linkslot_vfs_rename_fn)(const char *, const char *);
typedef int (*linkslot_vfs_stat_fn)(const char *, int32_t *);
typedef int (*linkslot_vfs_mkdir_fn)(const char *);
typedef retro_vfs_dir_handle *(*linkslot_vfs_opendir_fn)(const char *, bool);
typedef bool (*linkslot_vfs_readdir_fn)(retro_vfs_dir_handle *);
typedef const char *(*linkslot_vfs_dirent_name_fn)(retro_vfs_dir_handle *);
typedef bool (*linkslot_vfs_dirent_is_dir_fn)(retro_vfs_dir_handle *);
typedef int (*linkslot_vfs_closedir_fn)(retro_vfs_dir_handle *);

typedef struct linkslot_vfs_backend {
    linkslot_vfs_get_path_fn get_path;
    linkslot_vfs_open_fn open;
    linkslot_vfs_close_fn close;
    linkslot_vfs_size_fn size;
    linkslot_vfs_tell_fn tell;
    linkslot_vfs_seek_fn seek;
    linkslot_vfs_read_fn read;
    linkslot_vfs_write_fn write;
    linkslot_vfs_flush_fn flush;
    linkslot_vfs_path_fn remove;
    linkslot_vfs_rename_fn rename;
    linkslot_vfs_truncate_fn truncate;
    linkslot_vfs_stat_fn stat;
    linkslot_vfs_mkdir_fn mkdir;
    linkslot_vfs_opendir_fn opendir;
    linkslot_vfs_readdir_fn readdir;
    linkslot_vfs_dirent_name_fn dirent_get_name;
    linkslot_vfs_dirent_is_dir_fn dirent_is_dir;
    linkslot_vfs_closedir_fn closedir;
} linkslot_vfs_backend;

/* Install once before retro_init(); backend and callback state need process lifetime. */
bool linkslot_vfs_install(const linkslot_vfs_backend *backend);

#define LINKSLOT_RETRO_ENVIRONMENT_GET_VFS_INTERFACE (45u | 0x10000u)
#define LINKSLOT_RETRO_VFS_FILE_ACCESS_READ 1u
#define LINKSLOT_RETRO_VFS_FILE_ACCESS_WRITE 2u
#define LINKSLOT_RETRO_VFS_FILE_ACCESS_READ_WRITE \
    (LINKSLOT_RETRO_VFS_FILE_ACCESS_READ | LINKSLOT_RETRO_VFS_FILE_ACCESS_WRITE)
#define LINKSLOT_RETRO_VFS_FILE_ACCESS_UPDATE_EXISTING 4u
#define LINKSLOT_RETRO_VFS_SEEK_POSITION_START 0
#define LINKSLOT_RETRO_VFS_SEEK_POSITION_CURRENT 1
#define LINKSLOT_RETRO_VFS_SEEK_POSITION_END 2
#define LINKSLOT_RETRO_VFS_STAT_IS_VALID 1
#define LINKSLOT_RETRO_VFS_STAT_IS_DIRECTORY 2
#define LINKSLOT_RETRO_VFS_STAT_IS_CHARACTER_SPECIAL 4

#endif
