# Linkslot 1.0.45 H700 移植基线更新

**日期：** 2026-11-24  
**工作分支：** `porting/linkslot-1.0.45-feasibility`  
**状态：** 已完成源码基线审查、三核心状态机和 overlay patcher 的第一轮升级；完整 CI 构建与设备验证尚未完成。

## 1. 新的上游基线

Linkslot 源码快照现存放于本仓库 `upstream/linkslot` 分支根目录。它是归档分支，不是一个保留了 GitCode 完整历史的 git mirror。

| 快照位置 | 上游分支 | 源仓库 commit | Linkslot 版本 | 文件数 |
|---|---|---|---:|---:|
| `upstream/linkslot` 根目录 | `main` | `25664ecd2a712279dfa840fbc1ea896ebc0ef0ba` | 1.0.45 | 852 |
| `upstream/linkslot/DEV` | `DEV` | `3cf0b04a1d31344bc05972cbc829fc2b69ff84c7` | 1.0.42 | 852 |
| `upstream/linkslot/test` | `test` | `fbe9fcab8ad26da8cf767d4ad820579682ca10dd` | 1.0.45 | 852 |

归档分支的 GitHub HEAD 为 `3b06d96adadd0375f071555ecb3cc20c3a5b53db`。这是本项目的归档提交，不是上游 GitCode commit。overlay patcher 应固定归档 HEAD，同时在报告中保留原始来源 commit，不能混为一谈。

main 与 test 快照的 852 个源码文件逐 blob 一致；DEV 是更早的 1.0.42，与 1.0.45 相比有 26 个文件内容不同。DEV/test 是快照目录，不是 git branch 历史，不能对这些目录执行 merge/rebase。

## 2. 裸机架构和语言分工

项目目标明确为：**以 C 为底层、Rust 为前端业务层、C/C++ 为模拟器核心的裸机系统**。

- **C / ARM 汇编：** GammaOS LK、BL33 启动入口、异常/中断、H700 BSP/HAL、内存/时钟/显示/输入/音频/SD/电源等硬件服务。
- **C ABI：** 在 LK 与 Rust、模拟器核心之间提供窄小、版本化、固定宽度、明确所有权的服务接口。
- **Rust `no_std`：** Linkslot 前端业务状态、事件循环、游戏库、菜单/按键逻辑、配置语义和 UI/渲染编排。不得把 Linux `std`、线程、进程、动态 .so loader 或完整 POSIX 兼容层带入 LK。
- **C/C++ 静态模拟器核心：** mGBA、gpSP 等通过 Libretro/Core ABI 接入；核心不得直接访问板级 MMIO，也不拥有前端 UI。
- **Python / Shell：** 仅用于开发机上的源码审计、patch、构建、测试和打包，不进入设备运行路径。

第一条可启动垂直切片仍按单活动核心、单核协作式调度设计。BL31→BL33 的入口、内存所有权和显示缓冲区归属未通过审计前，不分配猜测的物理地址，不执行 MMIO 写入，不生成可刷写固件。

## 3. 1.0.45 对原移植层的影响

### 3.1 可复用部分

- 上游 `slot-input` 与 `slot-model` 在 DEV 1.0.42 和 main/test 1.0.45 之间的 blob 没有变化；但本项目持久化的 H700 `slot-model` 是更早期的 no_std 抽取版，`core.rs` 当时缺少 `ARMGBA` 元数据。已将纯数据 `core.rs` 与 1.0.45 上游同步，保留 `lib.rs`、`machine.rs`、`layout.rs` 中为 H700 增加的 `no_std`、`alloc` 和 `host-compat` 适配。新增测试检查核心 ID 唯一性、armGBA 元数据和颜色校正配置。
- 1.0.45 的 `frontend.rs` 中 `FacesLang` 区块标记仍唯一匹配，既有 `slot-frontend-state` 提取可复用。
- 1.0.45 的 `app.rs` 中 FolderHold 字段、初始化、方法和转换代码锚点仍唯一匹配，既有 `slot-hold-state` 提取可复用。
- 渲染器 overlay 的主要源码锚点仍匹配 1.0.45：Frontend 中 `&mut Compositor` 有 41 处，CartFaces 有 2 处；Draw/TexId 声明、纹理 ID 构造以及 slot-gfx feature/模块插入锚点均符合预期。patcher 已更新到 1.0.45 pin，但仍需由 CI 执行实际 patch 和完整 Cargo check。

### 3.2 必须升级的核心选择器

1.0.36 的抽取实现只支持单/双核心芯片 hop；1.0.45 已有 GBA 三核心布局和 UI。三核心逻辑语义如下：

- 槽 1 固定为 mGBA。
- 槽 2 放当前运行的替代核心。
- 托盘放另一个备用核心（gpSP / armGBA）。
- 左右键移动高亮位置；A 选择槽 1/槽 2 时直接写入该核心；A 选择托盘时交换托盘与槽 2，并将交换后的槽 2 设为运行核心；B 取消不写入。
- 交换过程使用 180ms 动画节拍，然后关闭选择页面。

持久化的 `slot-core-picker` 已升级为通用 `CorePicker<C>`，新增 `ThreeLayout<C>`、`open_three`、`layout`、`active`、`selected_pos`、`is_three` 和 `swap_progress`。它只管理三座位布局、选择、运行核心和动画状态，不依赖 Linkslot 的 `Core` 枚举、`slot-store` 或 UI。

Linkslot 特定枚举的映射留在临时 overlay 的 `App` 适配层：该层检查 mGBA/gpSP/armGBA 是否全部可用，再构造 `ThreeLayout<Core>`。缺少必要核心或布局非法时回退到传统选择器。单核心只展示、双核心 hop/掉头连续性仍保留。

新增回归测试覆盖三核心布局、高亮与运行核心分离、槽位直接选择、托盘/槽 2 交换、初始托盘规范化、非法布局回退，以及原有单/双核心路径。测试结果需以本分支 CI 运行记录为准，尚未把源代码提交等同于测试通过。

### 3.3 patcher 和构建入口

以下 patcher 已切换到归档快照 commit `3b06d96adadd0375f071555ecb3cc20c3a5b53db` 和 workspace version 1.0.45：

- `scripts/apply-linkslot-h700-frontend-core.py`
- `scripts/apply-linkslot-render-core.py`
- `scripts/test-linkslot-slot-retro-vfs.py`、`scripts/test-linkslot-static-core-api.py`、`scripts/test-linkslot-mgba-static-rust-binding.py` 已更新到 1.0.45；静态 API 测试适配 1.0.45 新增的 `LibretroCore::six_button` 字段，并默认初始化为 `false`。

`.github/workflows/h700-linkslot-runtime.yml` 已更新为 checkout 1.0.45，并为 `porting/linkslot-1.0.45-feasibility` 加入 push path trigger。该 workflow 原本存在于移植分支而非 main；在 workflow 进入默认分支之前，不能假定 GitHub Actions 会因本分支 push 自动执行。CI 运行状态应通过对应 commit 的 Actions 记录单独确认。

### 3.4 第一轮 CI 发现的问题与修复

- 综合 workflow 首次运行 [45369954559](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45369954559) 在“内置静态 mGBA 前端选择”步骤失败。静态 mGBA 的初始化/释放生命周期测试和显式 `SLOT_CORE` 覆盖优先级测试均通过；失败的是同一测试筛选器同时执行的上游核心颜色配置测试。
- 失败原因是持久化 H700 `slot-model/src/core.rs` 没有 1.0.45 的 `ARMGBA` 条目，而上游 `slot-store::Core` 已包含 `Core::Armgba`，调用 `Core::spec()` 时因此触发缺失表项 panic。
- 已同步完整上游 1.0.45 `core.rs`，把 H700 `slot-model` 包版本设为 1.0.45，并增加核心注册表与 armGBA 颜色配置测试。没有删除 armGBA、放宽 `Core::spec()` 或跳过测试。
- 修复后完整综合 workflow [45370886678](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45370886678) 已成功，69/69 步全部通过。失败原因被定位并修复；没有删除 armGBA、放宽 `Core::spec()` 或跳过测试。

## 4. 1.0.43–1.0.45 新上游功能的保留策略

- **1.0.43：** 卡带/存档传输模式、`.sav`（兼容 `.srm`）、`resume.state`、`resume.png` 的传输和覆盖前备份。不是第一个可启动 GB/GBC/GBA 垂直切片的阻塞条件，但不得删去或改变上游语义；应在本地存档和 VFS 持久化阶段后移植并测试。
- **1.0.44：** SFC X/Y 按键映射、组合键释放后的快进/倒带状态修复，以及 `controls.txt` 配置。SFC 不属于首期 GB/GBC/GBA 范围，后续支持 SFC 时需对照 1.0.45 原实现。
- **1.0.45：** MD 默认六键手柄映射。MD 不属于首期范围，后续支持 MD 时需保留 X/Y/Z/MODE 输入能力。

armGBA README 描述为 AArch64 recompiler；当前 LK/BL33 主线为 AArch32，不能据此推定它可直接链接或运行。首期优先验证 mGBA AArch32 静态核心和 gpSP 的 A32 可行性；armGBA 单独进行 ISA、工具链和内存模型审查。

## 5. 当前验证边界和下一步

1.0.45 的验证基线现已通过：软件渲染器 workflow [45370068413](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45370068413)、runtime slices workflow [45370886590](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45370886590)，以及完整综合 workflow [45370886678](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45370886678) 全部成功。综合 workflow 共 69 步，覆盖 GammaOS LK H700 BL33 源 overlay CRT0/exception-vector 编译、1.0.45 Renderer/Frontend/no_std overlay、A32/A64 检查、C input ABI、VFS v3、mGBA AArch32 namespaced static archive、newlib 构建/链接诊断、H700 SD→GPT→FAT→Libretro VFS host 路径，以及 BL33 header/vector layout probe。完整产物（约 381 MB，含 map、ELF、报告和日志）已上传：[GitHub Actions artifact 14908469502](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45370886678/artifacts/14908469502)。

### 5.1 已修复的 BL33 overlay 项与剩余阻塞

完整 workflow [45370886678](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45370886678) 的确切通过范围是 host/cross-build feasibility，不是完整 LK 内核构建或实机启动。需要区分上游原始代码审计和 H700 overlay 的实际修补：

- **Cortex-A53 编译参数：已在 overlay 中修正。** pinned upstream `arch/arm/rules.mk` 的 Cortex-A53 分支原本仍使用 `-mcpu=cortex-a9`。生成器 `apply-h700-lk-overlay.py` 已将其改为 `-mcpu=cortex-a53 -mfpu=neon-vfpv4 -mfloat-abi=softfp`；`test-h700-lk-overlay.py` 对生成后的规则做了明确断言。当前 audit 报告已调整为分别说明 upstream baseline 和 overlay 修复，不再把上游原始问题误记为 overlay 当前状态。
- **VBAR 重新定位：已在 overlay 中修正。** BaseOS header overlay 覆盖原始 `crt0.S` 的 `+4..+0x1f` 向量分支和 `+0x20` boot-argument word。生成器将异常向量移到 `+0x1000`，入口设置 VBAR，并把 `arch.c::arch_early_init()` 后续的 `set_vector_base(MEMBASE)` 改为 `set_vector_base((addr_t)__lk_vectors_start)`；`test-h700-lk-overlay.py` 检查此修改。prototype 布局为入口分支目标 `0x4a001024`、向量表 `0x4a001000`、boot-argument scratch `0x4a001020`。
- **A53 一致性/多核启动状态仍未闭合：** 设备 live inventory 确认四核 Cortex-A53；vendor monitor 二进制中有 PSCI CPU_ON/CPU_OFF/CPU_SUSPEND 路径名称，但没有运行时调用或 BL31→BL33 次级核状态证据。LK `crt0.S` 的 ACTLR.SMP 分支目前只有 Cortex-A7，且 scheduler 使用全局 current_thread/run queue/critical-section 状态和本地 IRQ mask，是单核模型。必须保持 single-core fail-closed，不调用 PSCI/Allwinner CPU_ON，不调度二级核；证据汇总见 [RG34XX A53 handoff and memory boundary](rg34xx-a53-handoff-and-memory-boundary.md)。
- **RAM 上界不是已批准的内存分配：** RG34XX live FDT 与 `/proc/iomem` 已确认 `[0x40000000,0x80000000)` 这一 1 GiB 运行时窗口；Linux memblock 记录了 kernel、initrd、FDT、BL31 `[0x48000000,0x49000000)`、64 MiB CMA `[0x77c00000,0x7bc00000)`、framebuffer `[0x7bf24800,0x7c076000)` 和 owner 未明的 16 MiB 保留区 `[0x7ec00000,0x7fc00000)` 等区间。现有证据仍是 Linux 阶段的占用快照，不是 BL31→BL33 时的完整生命周期内存地图。当前 BaseOS BL33 位于 `0x4a000000`、使用 1 MiB arena，SP=`0x4a0f0000`，页表位于 `0x4a0f8000`；这些是现有 loader 的使用事实，不是新 LK 可复用的内存许可。完整区间和证据边界见 [RG34XX A53 handoff and memory boundary](rg34xx-a53-handoff-and-memory-boundary.md)。
- **newlib/syscall 运行服务尚未闭合：** mGBA AArch32 archive 已通过 whole-archive 链接诊断；其中 `libnosys` 配置使用 dummy syscall stubs，`_start` 是 link-test stub。H700 newlib adapter 已有 heap/console/RTC 回调 ABI 和 host/A32 adapter tests，但真实 LK heap 初始化、UART/console、RTC、每线程 reentrancy 与 VFS 挂载尚未形成完整 runtime 服务。不能把 archive 链接成功等同于模拟器可启动。

BL33 layout probe 的 `_end=0x4a0033f8` 和 5100-byte prototype image 只反映 tiny `kmain`/exception stubs 的 host layout，不是完整 LK kernel，也不是对整个 H700 RAM 的授权，更不是可刷写 payload。artifact 中 synthetic header/ELF/bin 不得写入 SD 或设备。

### 5.2 下一步

1. 对 H700 overlay 的 A53 flags 与 relocated VBAR 回归保持 CI 保护；完整 LK build 时再核实实际编译命令和架构 ABI。
2. 取证并明确 BL31→BL33 时其它 A53 核心的状态；若不能证明二级核心已停驻，则保持单核 fail-closed，不启用 scheduler/IRQ 并发，不宣称支持 SMP。
3. 完成 DRAM/reserved-memory/heap/stack/framebuffer ownership 审计。
4. 将已存在的 newlib adapter ABI 接到经过验证的 LK heap、console、RTC 和真实文件服务；当前 `libnosys` link probe 只是可链接性诊断。
5. 在完整 LK/H700 BSP 可启动且服务 ABI 稳定后，才把已验证的 mGBA archive 接入 Linkslot runtime，继续 ROM→视频→音频→输入→存档的 playable vertical slice。

**当前仍没有可刷写固件，也没有声称 1.0.45 已在 RG34XX 上启动。**
