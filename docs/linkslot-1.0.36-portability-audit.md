# Linkslot 1.0.36 与 LK/H700 可移植性审计（初步）

> 状态：源码静态审计 + host-only 交叉编译/静态链接验证已完成第一轮；尚非 LK 集成或实机验收。本文不表示 Linkslot 已在 LK 上运行。

## 1. 当前 Linkslot 源码基线

用户提供的 linkSlot 1.0.36 源码已归档到本仓库分支：

- 分支：`upstream/linkslot`
- 快照 commit：`8590d6ff1b2caa26118051069c975106fbb14bd3`
- 快照日期：2026-11-23（输入说明为 2026-11-22 构建）
- Cargo workspace version：`1.0.36`
- `rust-toolchain.toml`：Rust `1.96`
- `Cargo.toml`：`rust-version = "1.96"`，edition 2021
- 工作区：14 个 crate，约 840 个 Git blob（838 个源码文件 + `UPSTREAM-SNAPSHOT.md` + `SHA256SUMS.txt`）
- 快照许可证声明：GPL-3.0-or-later；第三方目录/核心/字体仍须分别遵循各自许可证。

### 原始上游身份限制

输入是没有 `.git` 元数据的 ZIP 源码归档，原始 Git commit 和 tag 不可得。快照 commit `8590d6ff...` 是本项目为了保存用户提供源码创建的归档提交，不是 `luguiyi/linkslot` 上游 commit。

截至快照说明记录的 2026-11-23 GitHub API 查询，公开 `luguiyi/linkslot/main` 为 `9275419ca99846fe6738027b2ade72e1804fdc08`，其 Cargo 版本仍为 1.0.0。不得把该 commit 冒充 1.0.36 的来源，也不得声称归档快照保留了上游历史。

因此当前最可靠的身份是：
**版本号 + 快照日期 + 快照 commit + SHA256SUMS.txt**。

源码入口：
- [快照分支](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/upstream/linkslot)
- [快照说明](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/upstream/linkslot/UPSTREAM-SNAPSHOT.md)
- [Cargo workspace](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/upstream/linkslot/Cargo.toml)
- [1.0.36 Changelog](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/upstream/linkslot/CHANGELOG.md)

## 2. 一个必须先纠正的概念：Linkslot 的 device 不是 bare-metal

Linkslot 1.0.36 有 `host` / `device` Cargo feature，但它们只是桌面窗口/音频/输入后端与设备显示后端的 feature 切换，不表示 Rust `std` 被替换为 LK。

证据：

1. `crates/slot/Cargo.toml` 的 `device` 只启用 `slot-gfx/device`；核心仍依赖 `slot-store`、`slot-retro`、`slot-power`、`slot-device`、`slot-ui` 等。
2. `build/dist.sh h700` 在 `rust:1-bullseye` / `linux/arm64` 容器中构建，而不是 `aarch64-none-elf`。
3. `build/container/Dockerfile` 使用 `rust:1.96-bullseye`。
4. 构建脚本要求产物为 AArch64 Linux ELF，并把 GLIBC 版本限制在 2.31 以内。

因此：
- Linkslot 现有 H700 device binary 是 AArch64 Linux/glibc 程序；
- 不能将现有 `device` feature 等同于 `no_std`、LK 或裸机运行时；
- 不能把当前 `slot` 或 `linkSlot` 二进制直接交给 LK 执行；
- 本项目必须另行提供 LK 可链接的代码、运行时服务和平台后端，或决定哪些部分在裸机侧重写。

参考：
- [slot Cargo.toml](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/upstream/linkslot/crates/slot/Cargo.toml)
- [dist.sh](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/upstream/linkslot/build/dist.sh)
- [Dockerfile](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/upstream/linkslot/build/container/Dockerfile)

## 3. GammaOS LK 参考分支的架构边界

参考基线：
`TheGammaSqueeze/lk-mt6785`
branch `lk-gba-emu-sd-card-kpa`
commit `9bf1419a49e0af6dba8b05916df382c9c9c6d0a8`。

对该分支仓库树和构建文件的只读审查发现：

- 有 `arch/arm` 和 `arch/x86`；
- 没有 `arch/aarch64`；
- 顶层构建通过 C/C++/Assembly rules 组织，没有 Rust/Cargo 集成；
- ARM 工具链默认前缀为 `arm-linux-androideabi-`；
- `arch/arm/crt0.S`、ARM linker scripts 和 ARM object list 构成 AArch32 LK 启动/链接路径；
- `arch/arm/rules.mk` 在 `ARM_CPU=cortex-a53` 分支中目前设置 `-mcpu=cortex-a9`，需在复用前解释或修正，不能把它当作已完成 A53 最佳化。

该 LK 源码已有可供适配的基础 runtime primitive：
- heap：`heap_alloc` / `heap_realloc` / `heap_free`；
- task：`thread_create` / `thread_sleep` / `thread_yield`；
- synchronization：`mutex`、`event`。

这些服务说明 LK 有能力承载一部分 freestanding runtime，但**不等于已经有 Rust 标准库或 Rust 线程/文件系统实现**。

### 对 H700 的架构含义

当前主项目的编译 scaffold 使用 AArch64 freestanding objects；GammaOS LK 参考分支是 AArch32。两者并不自动兼容。

需要在下列路线中完成有证据的选择：

| 路线 | 说明 | 未解决的关键点 |
|---|---|---|
| AArch32 LK/runtime | 将 LK 作为 AArch32 BL33 运行，配合 A32 Core ABI / A32 cores | RG34XX 启动交接能否安全进入 AArch32 LK；A53 A32 build flags；EL/monitor/中断状态 |
| AArch64 LK/runtime | 移植 LK 的架构层、启动汇编、MMU、异常、原子操作和链接脚本到 AArch64 | 工作量明显更大；当前参考分支没有现成 AArch64 port |
| AArch64 独立 runtime | 不把现有 A32 LK 当作内核，基于已验证的 A64 handoff 建立最小 runtime | 需要自行提供调度/内存/驱动服务，不能假定 LK APIs 可直接使用 |

这只是路线比较，不是本轮作出最终选择。已有探针报告对实机自动恢复仍给出 NO-GO；在该门槛关闭前不得执行 payload。

## 4. Linkslot 1.0.36 的 crate 初步分级

这里区分“逻辑/数据可复用”和“当前 crate 能否原样在 LK 编译”：
前者可能成立。首轮 host-only 验证仅覆盖 `slot-input`、`slot-model` 的临时适配副本；不是整个 crate 的 LK 集成验收。

| Crate | 当前源码事实 | 初步处理 |
|---|---|---|
| `slot-model` | 静态机种/核心/目录模型；同时含 `Path` / `PathBuf`、`String`、`Vec` | **优先候选**。拆分纯领域表与路径/文件系统助手；验证 `core+alloc` |
| `slot-input` | 按键枚举、原始事件、手势状态机；无第三方依赖，主要用 `Vec` 和 `std::mem::take` | **优先候选**。把平台采样留给 H700 HAL，验证 gesture logic 的 `no_std+alloc` 版本 |
| `slot-i18n` | 语言表由 `include_str!` 嵌入；运行时使用 `serde_json`、`HashMap`、`OnceLock` | 可复用翻译资源/文案；运行时表解析需改为静态生成或验证 alloc/serde 支持 |
| `slot-device` | 使用环境变量、release 文件和 `OnceLock` 识别设备 | 不原样复用；由 RG34XX profile 替代 |
| `slot-power` | `Platform` trait 外观可供参考；实现使用 sysfs、Unix ioctl、`Command`、`/bin/sh`、`poweroff`、`sync` | 不原样复用；保留行为契约，重写 PMIC/电源 HAL |
| `slot-gfx` | GL 绘图/滤镜/着色器；fbdev backend 打开 `/dev/fb0`、ioctl、`libloading` EGL/GLES | 可研究 shader/渲染数学与 Surface 边界；H700 显示后端需替换 |
| `slot-ui` | 依赖 `slot-gfx`、`slot-store`、`slot-model`，另使用 fontdue/png/resvg/usvg | 视觉资源和 UI 状态逻辑可审；当前 crate 不是独立的裸机 UI |
| `slot-retro` | `RetroCore` trait + Libretro FFI；加载器通过 `libloading::Library` 加载动态库，依赖文件路径和 std | 可复用核心能力模型/部分 ABI 定义；LK 不应假设存在 Linux `.so` 动态加载器 |
| `slot-store` | SQLite/rusqlite bundled、std 文件系统、原子写与 Linux `syncfs` | 数据模型/目录语义可保留；需要重新设计 FAT 文件后端/持久化事务；SQLite 仅在 VFS 和耐久性测试后决定 |
| `slot-cheevos` | rcheevos C 运行时由 build.rs/cc 构建；Rust binding 使用 FFI、thread-local 与 Vec | C 引擎可作为候选；需要 LK allocator、线程/回调和目标构建集成验证 |
| `slot-mods` | MOD 元数据/权限/存储模型，依赖 serde、sha2、ring 等 | 可复用格式与权限语义；加密实现、持久化和 Lua Host 环境需单独适配 |
| `slot` | 前端主程序，组合图形、输入、存储、核心和运行时模块 | 不整体导入；只按 crate 审查可抽取逻辑 |
| `linkslot` | 独立 CLI，WebDAV/HTTP/TLS、文件和 Lua MOD 功能，含 ureq/rustls/mlua | 不作为 LK 前端依赖；保留协议、冲突仲裁和文件语义，重做 transport/runtime |
| `slot-sfxcut` | 独立工具 crate | 不进入掌机 runtime |

### 最小候选复用集合

第一轮只对以下模块做技术试编译，不要同时接入 UI、文件系统、网络或模拟器：

1. `slot-input` 手势状态机；
2. `slot-model` 中平台/核心/布局的静态数据部分；
3. Linkslot 的纯协议常量与少量数据结构（先从 `slot-model` 选择，不把整个 workspace 加进来）。

`slot-i18n` 与 `slot-cheevos` 可作为第二批候选。其余模块先保留语义/资源级复用，不承诺 crate 原样复用。

## 5. 已完成的 host-only 技术验证（2026-11-23）

验证由 GitHub Actions 云端 Runner 执行，不依赖用户本地电脑性能。测试脚本位于本分支 `scripts/audit-linkslot-nostd.py`，工作流为 `.github/workflows/linkslot-lk-rust-feasibility.yml`。

### 第二轮完成结果

- Workflow run：[45103281417](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45103281417)，commit `8c0b9c62accd5ee904cd9a8fb69733b0eac25484`，结论 success。
- Rust/Cargo：`1.96.0`；目标：`armv7a-none-eabi`、`aarch64-unknown-none`。
- 现有 H700 AArch64 C11/C++17/ASM compile scaffold：通过，三个 object 均核对为 AArch64。
- 未修改的 Linkslot 1.0.36 `slot-input`、`slot-model` host tests：**58 项通过**（gesture 47、layout 3、registry 8）。
- `slot-input` 隔离 `no_std + alloc` 副本：AArch32、AArch64 `cargo check` 通过；原版 gesture integration tests 在该 `no_std` 副本上 **47/47 通过**。
- `slot-model` 隔离 `no_std + alloc` 副本：AArch32、AArch64 `cargo check` 通过；纯机种/核心/ROM 扩展名回归测试 **4/4 通过**。该副本刻意暂时移除 `PathBuf` 根路径 resolver 和 `Path` 扩展名 helper。
- Rust `staticlib` + freestanding C shim + allocator ABI 薄桩：AArch32、AArch64 最终 ELF 均链接成功；架构字段正确；`nm -u` 无未解析外部符号；map 已生成。
- 另将 pinned GammaOS LK `include/lib/heap.h` 放入 AArch32 C 编译器进行真实头文件编译；Rust `GlobalAlloc` 使用相同的 `heap_alloc(size_t, unsigned int)` / `heap_free(void *)` 原型，并与匹配签名的测试桩静态链接成功。最终 ELF32/ARM 定义了 `heap_alloc`、`heap_free`、Rust probe 符号，无未解析外部符号。

### LK heap API 的真实契约与限制

固定参考：
`TheGammaSqueeze/lk-mt6785` branch `lk-gba-emu-sd-card-kpa` commit `9bf1419a49e0af6dba8b05916df382c9c9c6d0a8`。

头文件声明：

```c
void *heap_alloc(size_t size, unsigned int alignment);
void *heap_realloc(void *ptr, size_t size);
void heap_free(void *ptr);
```

静态源码审查还确认：

- `heap_alloc` 接受 alignment=0 或 2 的幂；非零且非 2 的幂会返回 NULL。
- 请求的正对齐小于 16 时，heap 实现会提高到 16。
- `heap_free` 只需要指针，因此可对应 Rust `GlobalAlloc::dealloc`。
- heap free-list 修改位于 LK `enter_critical_section()` / `exit_critical_section()` 保护区内。
- 但 LK `critical_section_count` 是全局变量；AArch32 `arch_disable_ints` 只设置本核 CPSR 的 IRQ mask 位。这证明本核中断屏蔽，不证明多核互斥。

**重要结论：当前 adapter 不批准用于多核并发分配。** 只有在证明 LK 生命周期内 H700 其他核心始终被可靠停住，或为 heap 增加并验证跨核锁/SMP-safe allocator 后，才可把它用作 H700 多线程的全局 allocator。

本次 LK heap probe 使用固定 LK 头文件，但实现的 `heap_alloc/free` 是测试桩，**没有编译或链接真实 `lib/heap/heap.c`，也没有运行堆代码**。这仅是 header/signature/ABI 兼容证据。

### AArch32 EHABI 说明

Rust 1.96 的 AArch32 `alloc` 静态库含有 `__aeabi_unwind_cpp_pr0` 符号引用。已使用 `readelf -Wr` 检查全部引用，确认均为 `.ARM.exidx` 上的 `R_ARM_NONE` 元数据 relocation，不是可执行代码调用或 `PREL31` relocation。仅在本次 `panic=abort`、无异常展开的 ABI 探测中，通过 linker `--defsym=__aeabi_unwind_cpp_pr0=0` 解析该元数据标记。

这不是 EHABI unwinder 实现；未来启用 C++/Rust 异常展开或需要栈展开，必须接入真实且经过验证的 unwinder。

### 验证边界

- 两个 Linkslot crate 都是在临时隔离副本中测试；`upstream/linkslot` 快照没有修改。
- 没有把完整 workspace 改成 `no_std`；UI、存储、网络、图形、Libretro loader 尚未接入。
- LK allocator 薄桩不代表真实 LK heap 行为，尤其没有确认并发、释放合并、oom 行为和 heap 初始化时序。
- 没有验证 LK 启动、栈、EL、MMU、异常/中断、调度或 H700 硬件驱动。
- 没有生成可刷写镜像、写 SD 或执行 payload。

工作流和产物：
- [第二轮 GitHub Actions 成功运行](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45103281417)
- [PR #16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16)


## 6. 下一步必须做的技术验证

1. **启动架构决策**：评估把 AArch32 LK 作为厂商 BL31 的 AArch32 BL33（现有现场分析的 BL33 状态为 AArch32 SVC，SPSR `0x1d3`）的路线，与移植 AArch64 LK/建立 AArch64 runtime 的路线；不能只因 Rust 双目标都能编译就决定。
2. **H700 多核状态审计**：查清 BL31 → BL33 时每个 Cortex-A53 核心的启动/停驻状态，以及 LK 生命周期内 secondary core 是否可能运行。若多核活动，不能复用当前依赖本核 IRQ 屏蔽的 heap 临界区作为全局 allocator 锁。
3. **LK heap 实现验证**：只有在 pinned LK heap 的平台依赖和编译配置可控后，尝试单独编译真实 `lib/heap/heap.c`；再考虑跨核 heap lock、对齐、free/coalescing、OOM 和初始化时序测试。
4. **Linkslot 模块继续拆分**：保留 `slot-input` 的 47 项上游行为测试；进一步从 `slot-model` 分离路径/文件 I/O 助手，确保纯数据模型的 no_std 测试与上游语义保持一致。
5. **第二批复用评估**：再评估 `slot-i18n` 静态翻译表和 `slot-cheevos` C 引擎；之后才开始 UI、存储、图形和 Libretro runtime 的分层设计。
6. 继续单独关闭 H700 启动交接/内存 ownership 的 NO-GO 门槛；未关闭前不得执行设备 payload。

仍不生成可刷写镜像、不写 SD、不修改启动区域、不执行 payload。
