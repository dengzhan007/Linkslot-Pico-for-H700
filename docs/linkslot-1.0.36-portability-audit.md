# Linkslot 1.0.36 与 LK/H700 可移植性审计（初步）

> 状态：源码静态审计，尚非编译/实机验收。本文用于确定下一阶段技术验证顺序，不表示 Linkslot 已在 LK 上运行。

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
前者可能成立，后者目前均未通过 LK 编译验证。

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

## 5. 下一步必须做的技术验证

在创建 `third_party/linkslot` subtree、修改上游源码或实现 HAL 之前，完成一个隔离的 host-only feasibility spike：

1. 固定并记录 LK 源码 commit 与目标 ABI（AArch32 或 AArch64），不得把 A32 archive 链接进 A64 ELF。
2. 用 freestanding Rust crate 导出一个很小的 `extern "C"` API；实现 `panic=abort`，不依赖 Rust `std`。
3. 分别检查 `core`、`alloc` 的目标支持和所需工具链；若要 `alloc`，用薄 `GlobalAlloc` wrapper 调用经审查的 LK heap API，不伪装成完整 allocator。
4. 用 LK 侧 C stub 调用 Rust 静态库，验证参数、返回值、结构体布局、栈/调用约定和 panic 行为；只做主机交叉编译与链接检查，不进入 RG34XX。
5. 在临时副本中把 `slot-input` 与 `slot-model` 纯逻辑切为 `no_std+alloc` 试编译；不得直接修改 `upstream/linkslot` 快照。
6. 生成 map、符号表、ELF header、undefined symbol 清单，确保没有意外引入 `std`、libc/Linux syscall、pthread、动态链接器或 unwinder。
7. 对 AArch32 LK 路线和 AArch64 freestanding 路线分别出具结果；若任一工具链/链接假设不成立，明确记录失败原因。

输出：
- `docs/linkslot-1.0.36-portability-audit.md`（本文）
- `build/audit-work/linkslot-1.0.36/` 下的临时构建对象、map 和检查结果
- 下一阶段架构选择建议；不生成可刷写镜像、不写 SD、不修改启动区域、不执行 payload。

## 6. 当前结论

- linkSlot 1.0.36 已成为当前 UI/功能审查基线，但原始上游 Git commit 不可得。
- Linkslot 的 `device` feature 仍是 Linux AArch64，不是 LK/bare-metal。
- GammaOS LK 参考分支目前是 AArch32 LK，不能与当前 AArch64 probe/scaffold 混为同一目标。
- `slot-input` 与 `slot-model` 的纯逻辑是目前最值得验证的 Rust 复用入口。
- 当前没有可运行 Linkslot Pico 的 firmware、HAL 或已批准 payload 地址。
