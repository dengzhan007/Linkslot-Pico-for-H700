#ifndef H700_RUNTIME_SERVICES_H
#define H700_RUNTIME_SERVICES_H

#include <stdint.h>

#include "input_service.h"
#include "time_service.h"
#include "vfs_service.h"

#define H700_RUNTIME_SERVICES_ABI_VERSION 1u

/*
 * Aggregate first-stage LK services. The descriptor and every referenced
 * context/table must remain alive for the runtime session. The caller must
 * qualify and initialize the LK heap before bootstrap: the Rust gesture
 * adapter allocates its bounded action buffer during construction.
 *
 * Startup ordering is enforced by slot-lk-runtime:
 *   validate descriptor/time/input -> install VFS -> open static core.
 */
typedef struct h700_runtime_services_v1 {
    uint32_t abi_version;
    uint32_t struct_size;
    h700_time_service_v1_t time;
    h700_input_service_v1_t input;
    h700_vfs_service_v1_t vfs;
} h700_runtime_services_v1_t;

#endif /* H700_RUNTIME_SERVICES_H */
