#!/usr/bin/env python3
"""Cross-compile the public H700 input-service C ABI layout for A32 and A64."""
from __future__ import annotations

import argparse
import subprocess
from pathlib import Path

PROBE = r"""
#include <stddef.h>
#include <stdint.h>
#include "input_service.h"
#include "runtime_services.h"

_Static_assert(sizeof(h700_input_event_t) == 4u, "input event ABI size");
_Static_assert(sizeof(h700_time_service_v1_t) == 8u + 2u * sizeof(void *),
    "time service table size");
_Static_assert(sizeof(h700_vfs_service_v1_t) == 8u + 2u * sizeof(void *),
    "VFS service table size");
_Static_assert(offsetof(h700_vfs_service_v1_t, install) == 8u + sizeof(void *),
    "VFS install callback offset");
_Static_assert(sizeof(h700_runtime_services_v1_t) ==
    8u + sizeof(h700_time_service_v1_t) +
    sizeof(h700_input_service_v1_t) + sizeof(h700_vfs_service_v1_t),
    "aggregate runtime service table size");
_Static_assert(offsetof(h700_runtime_services_v1_t, time) == 8u,
    "aggregate time service offset");
_Static_assert(offsetof(h700_runtime_services_v1_t, input) ==
    8u + sizeof(h700_time_service_v1_t), "aggregate input service offset");
_Static_assert(offsetof(h700_runtime_services_v1_t, vfs) ==
    8u + sizeof(h700_time_service_v1_t) + sizeof(h700_input_service_v1_t),
    "aggregate VFS service offset");
_Static_assert(offsetof(h700_runtime_services_v1_t, abi_version) == 0u,
    "aggregate ABI version offset");
_Static_assert(offsetof(h700_runtime_services_v1_t, struct_size) == 4u,
    "aggregate struct size offset");
_Static_assert(offsetof(h700_input_service_v1_t, abi_version) == 0u, "ABI version offset");
_Static_assert(offsetof(h700_input_service_v1_t, struct_size) == 4u, "struct size offset");
_Static_assert(offsetof(h700_input_service_v1_t, context) == 8u, "context offset");
_Static_assert(
    offsetof(h700_input_service_v1_t, drain) == 8u + sizeof(void *),
    "drain callback offset"
);
_Static_assert(
    sizeof(h700_input_service_v1_t) == 8u + 2u * sizeof(void *),
    "service table size"
);

static size_t fake_drain(void *context, h700_input_event_t *out, size_t capacity) {
    (void)context;
    if (capacity == 0u) {
        return 0u;
    }
    out[0].button = H700_BTN_A;
    out[0].state = H700_INPUT_PRESSED;
    out[0].reserved = 0u;
    return 1u;
}

static bool fake_vfs_install(void *context) {
    return context != NULL;
}

size_t h700_input_abi_probe(const h700_input_service_v1_t *service) {
    h700_input_event_t events[H700_INPUT_MAX_EVENTS];
    if (service == NULL || service->abi_version != H700_INPUT_ABI_VERSION) {
        return 0u;
    }
    if (service->drain == NULL) {
        return 0u;
    }
    return service->drain(service->context, events, H700_INPUT_MAX_EVENTS)
        + fake_drain(NULL, events, H700_INPUT_MAX_EVENTS);
}

bool h700_runtime_services_abi_probe(const h700_runtime_services_v1_t *services) {
    if (services == NULL ||
        services->abi_version != H700_RUNTIME_SERVICES_ABI_VERSION ||
        services->struct_size < sizeof(*services) ||
        services->time.abi_version != H700_TIME_ABI_VERSION ||
        services->time.monotonic_ms == NULL ||
        services->input.abi_version != H700_INPUT_ABI_VERSION ||
        services->input.drain == NULL ||
        services->vfs.abi_version != H700_VFS_SERVICE_ABI_VERSION) {
        return false;
    }
    return services->vfs.install == fake_vfs_install &&
        services->vfs.install(services->vfs.context);
}
"""

TARGETS = (
    ("armv7a-none-eabi", "cortex-a53", "h700-input-abi-a32.o"),
    ("aarch64-none-elf", "cortex-a53", "h700-input-abi-a64.o"),
)


def run(command: list[str]) -> None:
    print("+", " ".join(command), flush=True)
    subprocess.run(command, check=True)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--clang", default="clang")
    parser.add_argument("--out", type=Path, default=Path("build/h700-input-abi"))
    args = parser.parse_args()

    root = Path(__file__).resolve().parents[1]
    header_dir = root / "platform/h700/runtime"
    args.out.mkdir(parents=True, exist_ok=True)
    source = args.out / "input-service-abi-probe.c"
    source.write_text(PROBE, encoding="utf-8")

    for triple, cpu, output in TARGETS:
        run([
            args.clang,
            f"--target={triple}",
            f"-mcpu={cpu}",
            "-std=c11",
            "-ffreestanding",
            "-Wall",
            "-Wextra",
            "-Werror",
            f"-I{header_dir}",
            "-c",
            str(source),
            "-o",
            str(args.out / output),
        ])
        run([
            args.clang,
            f"--target={triple}",
            f"-mcpu={cpu}",
            "-std=c11",
            "-ffreestanding",
            "-Wall",
            "-Wextra",
            "-Werror",
            f"-I{header_dir}",
            "-c",
            str(header_dir / "input_queue.c"),
            "-o",
            str(args.out / (output.replace("abi", "queue"))),
        ])

    print("H700 input service ABI and queue compile passed for AArch32 and AArch64.")


if __name__ == "__main__":
    main()
