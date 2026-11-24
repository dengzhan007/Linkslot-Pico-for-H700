# Linkslot slot-retro Libretro VFS v3 integration

## Why this seam is needed

The supplied Linkslot 1.0.36 snapshot has an environment callback in crates/slot-retro/src/libretro.rs, but that callback does not handle RETRO_ENVIRONMENT_GET_VFS_INTERFACE (command 45 | 0x10000). The H700 mGBA build routes file and directory operations through Libretro VFS v3. Without a frontend VFS response, the core cannot use the intended H700 storage path.

This port adds a small bridge module and routes the real slot-retro environment callback to it. The patch is applied only to the temporary CI checkout of the immutable upstream/linkslot archive. The source snapshot branch remains unchanged.

## Runtime contract

The H700 platform provides a static linkslot_vfs_backend table and calls linkslot_vfs_install(&h700_vfs_backend) once during platform initialization and before retro_init(). The backend object and callback state must remain valid until process exit. A second installation is rejected.

When a core requests VFS v1, v2 or v3, the environment callback returns the frontend-owned static VFS table and reports version 3. Requests above version 3 are rejected. The bridge requires every VFS v1-v3 callback, including get_path, to be present before publishing the backend.

The C ABI declaration is in platform/h700/runtime/include/linkslot_vfs.h. The Rust implementation is in platform/h700/runtime/linkslot_vfs.rs.

## Ownership and safety rules

The bridge owns only the stable Libretro function table and forwards calls to the platform backend. It does not own file or directory handles.

The H700 backend must:
- Keep a separate, validated handle for every open file and directory.
- Resolve paths against explicit frontend-configured roots for game content, system/BIOS, saves, states and cache. Linkslot's `slot-retro` passes ROM paths and system/save directories as paths; do not assume every VFS request is relative to a process working directory.
- Accept a native absolute path only when component-wise resolution proves that it is inside one of those configured roots; translate it to a card-relative path before calling FAT/exFAT. Reject absolute paths outside allowed roots, unknown URI schemes or drive prefixes, parent traversal, empty/repeated components that could escape the root, and invalid separators.
- Resolve relative paths only against an explicit configured base (never an implicit process CWD). Do not silently reinterpret an unknown URI such as `http://` or `content://` as a local filename.
- Preserve the exact path string supplied to `open` for the lifetime of the returned file handle; `get_path` must return that same path string and must not return a temporary buffer. Keep directory-entry names valid for the lifetime required by the Libretro API.
- For rename/replace, require both paths to resolve to the same mounted volume and enforce the storage layer's documented flush/barrier ordering. A successful `flush` must represent the durability guarantee advertised by the provider, not merely an empty software cache.
- Map read/write/seek/truncate/stat errors to Libretro return conventions.
- Implement flush with the required durable-storage semantics; save-state replacement must not report success before its required flush/rename sequence completes.
- Provide synchronization if any callback can run concurrently. Atomic publication of the backend pointer does not make the backend or LK heap SMP-safe.
- Keep ROM, BIOS, battery save and save-state paths inside their intended frontend mount roots. Do not conflate battery RAM with save-state files.

This bridge deliberately does not normalize paths itself because mount roots and path policies belong to the storage provider. No arbitrary path should be passed directly to FAT/exFAT.

## Validation

scripts/test-linkslot-slot-retro-vfs.py applies the bridge to a clean 1.0.36 checkout and runs cargo test -p slot-retro --lib. The regression calls the patched production environment callback; it checks missing-backend behavior, one-shot installation, null requests, rejection of a VFS v4 request, VFS v3 negotiation, file callbacks, directory enumeration and handle closure.

The test backend is in-memory. It is not a real card filesystem and provides no evidence of power-loss safety or persistent writes.

## Not yet complete

- No H700 SD block driver or FAT/exFAT filesystem provider.
- No verified ROM/save/system mount roots or profile mapping.
- No persistent file-handle implementation, FAT flush ordering or crash-safe rename.
- No live mGBA + Linkslot run; no ROM was run on the device.
- slot-retro itself still uses std and libloading in the supplied snapshot; this patch does not make that crate no_std or load a statically linked AArch32 core into LK.
- H700 reserved-memory ownership and BL31/BL33 handoff remain NO-GO. No payload execution or SD write is authorized by this work.

The next implementation gate is a host-testable H700 storage backend with path containment, multi-handle lifetime, write/flush/rename semantics and error mapping. Only after that should it be connected to verified SD/FAT hardware services.
