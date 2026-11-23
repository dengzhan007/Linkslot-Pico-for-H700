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

验证由 GitHub Actions 云端 Runner 执行，不依赖用户本地电脑性能。测试脚本位于本分支 `scripts/audit-linkslot-nostd.py`，工作流为 `.github/workflows/linkslot-lk-rust-feasibility.yml`。本次成功运行：

- Workflow run：`45102606482`（commit `3cc4bd9917ec77c8730cd42e6d41ea7186bd1f40`）。
- Rust/Cargo：`1.96.0`；目标：`armv7a-none-eabi`、`aarch64-unknown-none`。
- 现有 H700 AArch64 C11/C++17/ASM compile scaffold：通过，三个 object 均核对为 AArch64。
- `slot-input` 临时副本：`no_std + alloc`，两个目标 `cargo check` 均通过。
- `slot-model` 临时副本：保留纯静态数据与领域类型，暂时移除 `PathBuf` 根路径 resolver 和 `Path` 扩展名 helper；两个目标 `cargo check` 均通过。
- Rust `staticlib` + C freestanding shim + allocator ABI 薄桩：AArch32、AArch64 两个最终 ELF 均链接成功；ELF header 架构正确；`nm -u` 无未解析符号；两份 linker map 已生成。

### AArch32 EHABI 说明

Rust 1.96 的 AArch32 `alloc` 静态库含有 `__aeabi_unwind_cpp_pr0` 符号引用。已使用 `readelf -Wr` 检查全部引用，确认均为 `.ARM.exidx` 上的 `R_ARM_NONE` 元数据 relocation，不是可执行代码调用或 `PREL31` relocation。仅在本次 `panic=abort`、无异常展开的 ABI 探测中，通过 linker `--defsym=__aeabi_unwind_cpp_pr0=0` 解析该元数据标记。**这不是 EHABI unwinder 实现**；若未来启用 C++/Rust 异常展开或需要栈展开，必须接入真实且经过验证的 unwinder，不能复用这个零地址占位。

### 验证边界

- 编译的是两个 Linkslot crate 的隔离临时副本，不是直接把 upstream workspace 全量改成 `no_std`。
- allocator 只验证通用 C ABI 形状，使用测试 arena；尚未连接 GammaOS LK 的真实 `heap_alloc` / `heap_free`。
- C stub 仅验证函数调用、静态链接、ELF 目标架构和符号闭合；尚未验证 LK 启动、栈、内存布局、异常/中断、调度或设备 ABI。
- 没有编译完整 Linkslot UI、Libretro 核心、文件系统、显示/输入/音频 HAL；没有生成可刷写镜像、写 SD 或执行 payload。

工作流与产物：
- [GitHub Actions 成功运行](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45102606482)
- [PR #16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16)

## 6. 下一步必须做的技术验证

1. 确认 H700 选用 AArch32 LK 还是 AArch64 runtime；本轮证明两个 Rust 目标都能做 freestanding staticlib，不足以单独决定启动架构。
2. 在 host-only 环境中对照 GammaOS LK 的真实 API/头文件，写一个专用适配层，并链接实际 LK heap symbols；核实 allocator 对齐、释放、失败路径与线程安全策略。
3. 扩展 ABI 测试，覆盖多参数结构体、对齐、回调、枚举布局、panic abort 与栈需求；检查最终 ELF 的 `readelf -A/-h/-S`、map 和调用约定。
4. 继续切分 `slot-model` 路径/I/O 依赖，增加单元测试；验证 `slot-input` gesture 行为与上游测试一致。
5. 在不修改 `upstream/linkslot` 快照的前提下，逐步评估 `slot-i18n`、`slot-cheevos`，之后才评估 UI、存储、图形和 Libretro runtime。
6. 继续单独关闭 H700 启动交接/内存 ownership 的 NO-GO 门槛；未关闭前不得执行设备 payload。

仍不生成可刷写镜像、不写 SD、不修改启动区域、不执行 payload。

## 6. 当前结论

- linkSlot 1.0.36 已成为当前 UI/功能审查基线，但原始上游 Git commit 不可得。
- Linkslot 的 `device` feature 仍是 Linux AArch64，不是 LK/bare-metal。
- GammaOS LK 参考分支目前是 AArch32 LK，不能与当前 AArch64 probe/scaffold 混为同一目标。
- `slot-input` 与 `slot-model` 的纯逻辑已在 AArch32、AArch64 两目标通过隔离 `no_std+alloc` 编译；通用 Rust/C ABI 静态链接探针亦已通过。
- AArch32 探针用 `ABS 0` 仅解析 R_ARM_NONE unwind 元数据 marker；这不提供 unwinder，启用异常展开前仍须接入真实运行时。
- 当前仍没有可运行 Linkslot Pico 的 firmware、H700 HAL、实际 LK allocator 绑定或已批准 payload 地址。
