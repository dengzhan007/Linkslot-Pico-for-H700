#ifndef H700_VFS_SERVICE_H
#define H700_VFS_SERVICE_H

#include <stdbool.h>
#include <stdint.h>

#define H700_VFS_SERVICE_ABI_VERSION 1u

/*
 * Install or confirm the process-lifetime Libretro VFS backend.
 *
 * The callback must be idempotent: return true if the intended backend is
 * already installed, and false only when no usable backend is available.
 * Backend tables, callback code and backend context must remain valid for the
 * lifetime of all static Libretro cores. The backend is not uninstalled when
 * an individual core is deinitialized.
 */
typedef bool (*h700_vfs_install_fn)(void *context);

typedef struct h700_vfs_service_v1 {
    uint32_t abi_version;
    uint32_t struct_size;
    void *context;
    h700_vfs_install_fn install;
} h700_vfs_service_v1_t;

#endif /* H700_VFS_SERVICE_H */
