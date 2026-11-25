# 开发路线与阶段验收

**续接入口：** 每次继续移植前先核对[项目当前状态与交接记录](PROJECT-STATUS.md)，再按本路线图推进；状态文档记录当前分支、HEAD、CI 和下一步任务。

本项目采用可验证的垂直切片推进。每一阶段都必须产出可重复构建的测试对象和明确的验收记录；不以“编译通过”代替设备验证。

## M0：项目基线

- [x] 建立独立仓库。
- [x] 确认产品方向：GammaOS Pico 裸机架构参考 + Linkslot 唯一前端。
- [x] 确认首期只做 GB/GBC/GBA。
- [x] 固化关机、手动电池存档、手动云同步、家庭 LAN Web 和调试规则。
- [x] 确认目标设备无 eMMC，只设计 SD 卡启动，不做旧系统自动回退。
- [x] 建立 RG34XX / Allwinner H700 Cortex-A53 的 AArch64 freestanding C、C++ 与汇编编译检查和 CI。
- [x] 获取并归档用户提供的 Linkslot 1.0.36 历史源码快照（历史审计记录；已由 1.0.45 作为当前正式基线取代）。当时原始 ZIP 不含 `.git`，不能据 ZIP 推定上游 commit/tag。
- [x] 完成 Linkslot 1.0.36 第一轮逐 crate 可移植性审计与 host-only Rust `no_std`/`alloc` 构建（历史结果）：`slot-input`、`slot-model`、`slot-i18n` 候选通过 AArch32/AArch64 编译与行为测试；当前验收以 1.0.45 源码和新分支 CI 为准。
- [x] 确认目标 SoC 是 Allwinner H700，不是 Rockchip RK3566；AArch64 编译 CPU target 已修正为 Cortex-A53。
- [x] 锁定第一台实机为 Anbernic RG34XX（非 SP），建立 Allwinner H700 / Cortex-A53 硬件基线。

## 当前工作：GammaOS LK → H700 BL33 → H700 HAL → Linkslot 1.0.45

### 当前源码基线升级：Linkslot 1.0.45（进行中）

- [x] 归档并比较 `upstream/linkslot` 根目录 main、`DEV/`、`test/` 三份源码：正式基线为 1.0.45；DEV 为 1.0.42；main 与 test 的 852 个源码文件逐 blob 一致。原始来源 commit、归档 commit 与目录快照的关系见 [1.0.45 移植基线更新](linkslot-1.0.45-portability-update.md)。
- [x] 保留旧 `porting/linkslot-1.0.36-feasibility` 作为历史验证分支，另建 `porting/linkslot-1.0.45-feasibility`，不改写旧分支或上游快照。
- [x] 将 `slot-core-picker` 扩展为通用三座位 `ThreeLayout<C>` 状态机：保留单/双核心路径，覆盖运行核心与高亮位置分离、托盘/槽 2 交换、非法布局回退，并补充对应单测。
- [x] 更新 Frontend/App overlay 和 Renderer overlay 的 source pin/workspace version 到归档 1.0.45；App 层负责将 Linkslot Core 枚举映射到通用布局，避免 no_std crate 依赖 `slot-store`。
- [x] 将 runtime workflow 的 Linkslot checkout 更新到 1.0.45，并在新实验分支添加 push path trigger。
- [x] 完整综合 workflow [45370886678](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45370886678) 通过（69/69 步）：GammaOS LK H700 BL33 CRT0/vector overlay compile、A32/A64、10 个 no_std 业务 crate、完整 Linkslot 1.0.45 Frontend + Renderer overlay、C Input ABI/queue、VFS v3、真实 mGBA AArch32 namespaced static archive/newlib 构建与链接诊断、SD→GPT→FAT→Libretro VFS host path、BL33 header/vector layout probe 全部成功。artifact [14908469502](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45370886678/artifacts/14908469502) 已上传。
- [ ] 按 1.0.45 对照完整 App/UI/核心 adapter，审查 transfer/save、remap、session、mGBA ABI 变化；仅 GB/GBC/GBA 首期功能可延后，但不能丢失上游行为语义。

### A2（历史验证）：Linkslot 1.0.36 裸机可移植性与静态核心接入（host-only 阶段）

- [x] 固定 Linkslot 1.0.36 只读 ZIP 快照及 SHA256 清单，不伪造原始上游 commit。
- [x] `slot-input`、`slot-model`、`slot-i18n` 的候选副本通过 AArch32/AArch64 `no_std + alloc` 编译和对应行为测试；这些测试使用隔离临时副本，`upstream/linkslot` 快照保持只读。
- [x] `slot-retro` 静态 Libretro API 入口通过 host regression；静态模式不要求 `libloading`，保留动态模式用于比较。
- [x] 建立 Rust Libretro VFS v3 服务 ABI、虚拟文件/目录操作和 host fake-backend 测试；AArch32/AArch64 静态链接探针通过。
- [x] 将真实 mGBA 固定源码构建为 AArch32 namespaced Libretro 静态 archive；VFS v3 文件与目录后端构建通过，25/25 导出符号已前缀化。
- [x] 将真实 mGBA VDir adapter 接入 fake Libretro VFS v3 backend，覆盖目录遍历、rewind、嵌套目录和路径越界拒绝。
- [x] 使用 arm-none-eabi GCC/newlib 构建 mGBA VFS v3 archive，并完成 whole-archive/newlib/libm/libgcc 链接诊断；诊断阶段曾用 `libnosys` 对照，但 dummy syscall 不作为正式 runtime。
- [x] 新增 H700 newlib syscall adapter；host 行为回归、AArch32 EABI5 编译，以及与固定 GammaOS LK `heap_alloc/heap_free` 头文件 ABI 的编译核对通过。真实 heap 初始化、console、RTC 和每线程 newlib reentrancy 服务仍未实现。
- [x] 在隔离的 Linkslot 1.0.36 候选副本中，完成 namespaced mGBA 导出到 Rust 静态 API 的绑定和前端核心选择回归；未修改上游快照。
- [x] 将 Linkslot `slot-input` 与纯 `slot-model` 逻辑持久化到 `integration/linkslot/h700-frontend-core`，保留上游 blob provenance；`slot-model` 的 Linux 路径辅助函数放在可选 `host-compat` feature 下，设备构建使用 `--no-default-features`。
- [x] 在隔离 Linkslot 1.0.36 源树应用持久化业务 crate overlay，保留原始测试目录；上游 `slot-input` / `slot-model` 测试通过，并以原有 Linux host compatibility feature 编译完整 `slot` Frontend。
- [x] 从 `Frontend` 抽取通用 `FacesLang<L>` 构建代次状态机为 `slot-frontend-state`（`no_std`）：保留 build coalescing、generation/stale-result 处理、abandon/retry 与 synchronous completion；6 项独立测试通过，并通过临时源 overlay 注入回原版 `Frontend`。
- [x] 从 `App` 抽取泛型 `CorePicker<C>` 状态机为 `slot-core-picker`（`no_std`）：保留核心选择、单核心只展示、chip hop/中途反向、open/close easing、边界拒绝与最终选择写回；6 项独立测试通过，`App` 类型通过 overlay 替换为 `CorePicker<Core>`。
- [x] 从 `slot::link_state` 抽取 `slot-link-state`：保留 `SLK1` 双人状态打包/拆分、BIOS ID 比较，使用借用切片完成校验避免 BIOS 比较时克隆整份 GBA 状态；增加畸形长度与尾随数据拒绝测试。
- [x] 从 `slot::link_kind` 抽取 `slot-link-kind`：保留 43 项无线游戏代码表、Pokémon clean retail 例外、ROM hack 的 cable fallback、Libretro serial option 选择和 link capability 判定；`slot::link_kind` 保持 public re-export。
- [x] 从 `slot::video_mode` 抽取 `slot-video-mode`：`VideoMode` 解析与 GB/GBC padded-frame source crop geometry 进入 no_std crate，INI 文件读取/写回留在 host wrapper；原 module API 通过 re-export 及薄文件服务包装保持兼容。
- [x] 从 `slot::rom_id` 抽取 `slot-rom-id`：SHA-256、hex 编码/解码和 `RomHasher` 增量接口进入 `no_std` crate；文件/reader 读取留在 `slot::rom_id` host/VFS wrapper，保留原 public API 与原测试。
- [x] 从 `slot::game_search` 抽取 `slot-game-search`：拼音候选索引、排序/分页和跨平台命中算法进入 `no_std` crate；通过 `PinyinProvider` 注入拼音字典，并由 host wrapper 保留 `Cart`/`Platform` API 与原有搜索测试。
- [x] 从 `App` 抽取 `TimedHold` 计时状态到 `slot-hold-state`：提示延迟、按住进度、达到阈值只触发一次、提前释放淡出、受限状态取消均为独立 `no_std` 逻辑；7 项测试通过。App 仍持有“是否允许长按”和完成后进入/退出文件夹的产品策略。
- [x] 专项 workflow [45362859979](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45362859979) 验证十个持久化 no_std crate 的 host tests/A32/A64 checks、上游输入/模型回归、Renderer + 十个业务 crate overlay、完整 Linkslot `slot` Frontend 编译、双路径手势输出一致性与复用缓冲区、文件夹 hold-state 注入，以及 C input queue 与 ABI A32/A64 检查全部成功。
- [ ] 将更多 Frontend/App 的纯状态、游戏库、菜单、配置和本地存档策略逐步提取成 `no_std` Rust 模块；不能把当前 Linux `device_app` / 完整 `App` 直接链接为 LK 应用。
- [ ] 在真实 H700 runtime 中验证 mGBA 启动 ROM、视频帧、音频、输入、即时状态和电池存档；host archive/link 结果不能替代运行测试。

### A3：Linkslot LK runtime 与显示渲染边界（1.0.36 历史记录 + 1.0.45 当前复验）

### A3.1 Linkslot 1.0.45 综合 host/cross-build 验收

- [x] 最新 [1.0.45 完整 feasibility workflow](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45428664653)：**71/71 steps 成功**，新增实际执行 CPU Performance QuickRow App unit tests（2 passed / 0 failed）；Rust static mGBA final link 和 BL33 layout probe 也通过。完整日志/构建报告/ELF/map 已作为 [artifact 14937338188](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45428664653/artifacts/14937338188) 上传。前一个完整 70-step run #45425514685 和 #45413653400 均保留为历史基线。
- [x] 真实 GammaOS LK H700 BL33 source overlay CRT0 与 exception-vector 交叉编译/链接通过；RG34XX vendor header packaging 和 fail-closed memory-layout gate 通过。
- [x] Linkslot 1.0.45 cumulative overlay 通过：Renderer `SoftwareCompositor`、完整 `slot` Frontend、10 个 no_std 业务 crate、A32/A64 freestanding checks、Input Service ABI 与 C queue 均通过。
- [x] Linkslot 1.0.45 `slot-retro` VFS v3 host integration、static/dynamic Libretro API、21 个 namespaced mGBA Rust bindings、前端 built-in static mGBA 选择与生命周期测试全部通过。
- [x] H700 host storage vertical path 通过：sdc0/CMD17 read-only、GPT、FAT32 目录/文件读取、Libretro VFS v3；FAT16/FAT32/exFAT bounds 与真实镜像 host regression 通过。
- [x] 固定版本 mGBA 的 AArch32 namespaced static archive、Generic backend、VFS v3 file/directory backend、VDir fake-backend regression、arm-none-eabi/newlib archive 与 whole-archive link diagnostic 通过。
- [x] 新增并通过 [Rust Linkslot static mGBA / H700 newlib AArch32 link probe](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45413653400)：用实际 Linkslot 1.0.45 `slot-retro::Api` 类型与 `platform/h700/runtime/mgba_static.rs` 编成 no_std A32 staticlib，确认全部 21 个 namespaced mGBA exports，再与真实 mGBA VFS v3 archive、H700 newlib adapter、`libc/libm/libgcc` 链接；最终 ELF32/ARM 无未解析符号。只验证 ABI/符号闭合，不执行 API、不代表完整 LK 或设备运行。artifact [14929808289](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45413653400/artifacts/14929808289)。
- [x] BL33 header/vector host layout prototype 通过，但该 prototype 只覆盖 tiny kmain/exception stubs，不是 full LK kernel 或 boot image。
- [x] H700 overlay 已将 upstream Cortex-A53 `rules.mk` 的 `-mcpu=cortex-a9` 改为 `-mcpu=cortex-a53 -mfpu=neon-vfpv4 -mfloat-abi=softfp`；`test-h700-lk-overlay.py` 对生成的 overlay 做精确断言。完整 LK project 构建仍待后续。
- [x] H700 overlay 已将 `arch_early_init()` 的 VBAR 目标改为 relocated `__lk_vectors_start`；`test-h700-lk-overlay.py` 断言旧 `MEMBASE` 赋值不存在。CRT0 vector table 放在 `0x4a001000`，boot argument scratch 在 `0x4a001020`。
- [ ] 明确 BL31→BL33 时其它 Cortex-A53 核心是否停驻。当前 GammaOS LK scheduler/critical-section 是全局单核实现，`ACTLR.SMP` 路径只有 Cortex-A7；在证明其它核停驻之前，保持单核 fail-closed，不开放 SMP/多核执行。
- [ ] 完成 RG34XX 全部 DRAM/reserved-memory/heap/stack/framebuffer ownership 取证；`0x4a0e0000` 是 BaseOS loader BSS 上界参考，不是已授权 heap 或通用 RAM map。
- [ ] 把 H700 newlib adapter ABI 接入真实 LK heap、console、RTC、文件/VFS 与 reentrancy 生命周期；新 Rust+mGBA 最终 ELF 已把 adapter object 纳入链接，但没有安装或调用这些服务，`libnosys` 对照 probe 仍只是诊断，不能据此认为运行环境已闭合。
- [x] 修订 `scripts/audit-lk-bl33-layout.py` 的报告语义：明确区分 upstream 原始 `cortex-a9`/VBAR 行为与 H700 overlay 已做的两项修复，避免审计结果误读为修补未生效；修订后的审计输出等待新一次 workflow 验证。
- [x] 对照 Linkslot 1.0.36 原始 `device_app`、`Frontend`、`App`、`slot-gfx`、`slot-ui`、输入、音频与电源实现，确认 `device` feature 仍是 Linux/std 后端，不是 LK/no_std 运行模式。
- [x] 确认现有 `Draw::{Rect,Tex,Turned,Game,Shot}` 已提供可复用的 CPU 绘制命令边界；并在临时 Linkslot 源码补丁中抽出对象安全 `Renderer` trait。
- [x] 制定 LK 软件 framebuffer 的分层方案：先实现 RGBA 软件 2D 基线，再逐步补屏幕滤镜；不依赖 LK 内的 Linux fbdev/EGL/GLES/Mali userspace。
- [x] 记录 Rust 运行时边界和 R0–R6 可验收垂直切片，见 [Linkslot H700 LK runtime architecture](linkslot-h700-runtime-architecture.md)。
- [x] 在 `integration/linkslot/h700-render-core` 建立独立 `no_std + alloc` 软件 rasterizer，覆盖 Rect/Tex/Turned/Game/Shot、裁剪、alpha、最近邻/双线性采样、RGBA/BGRA byte order、Game source-rect crop 和 RGB gain；13 项 host regression test 通过。该 crate 不修改 `upstream/linkslot`，也不映射物理 framebuffer。
- [x] 新增隔离源补丁脚本：将 `h700-render-core` 注入临时 Linkslot 工作树，让 `slot-gfx` 与 renderer 共享唯一的 `Draw/TexId` 类型；上游快照保持只读。
- [x] 让 GL `Compositor` 实现对象安全 `Renderer` trait，并将真实 `Frontend` 与 `CartFaces` 的绘制/纹理接口改为 `&mut dyn Renderer`。
- [x] 实现并编译 `slot-gfx::SoftwareCompositor`：调用者提供 framebuffer slice，支持基础菜单/游戏命令、texture upload/draw/delete、BGRA/RGBA、游戏画面缩放/裁切、power/brightness、RGB gain 和 bezel 纹理接入点；未实现的 shader/filter/高级 frame effects 必须通过 unsupported 状态显式报告。
- [x] 完成 SoftwareCompositor framebuffer host integration regression，并在同一隔离 Linkslot 源树中编译完整 `slot` Frontend + `h700-software` feature。
- [x] 最新完整 feasibility workflow [45346015251](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45346015251) 通过：13 项 rasterizer 测试、SoftwareCompositor framebuffer integration、完整 Frontend Renderer 注入编译、A32 BL33/头区/memory fail-closed 检查、H700 SD→GPT→FAT→VFS host 路径、真实 mGBA A32/newlib archive 构建和静态 API 绑定均成功。
- [ ] 对 SoftwareCompositor 做与上游 GL backend 的 golden-image/逐命令语义对照，尤其核实 Draw 顺序、begin/end frame、brightness/power、Bezel 合成与纹理采样；当前通过的是基础 framebuffer 集成测试，不是完整视觉一致性。
- [ ] 将 framebuffer descriptor/scanout ownership、stride/像素格式、cache clean/invalidate、double-buffer 与 panel handoff 审计关闭后，再连接 H700 显示 HAL。
- [x] 新增 RG34XX 纯 C 显示刷新策略、UI preference / GAME 120-Hz opt-in 与 frame-period helper；该模块只请求策略，不连接 Frontend、不等待 VBlank、不写硬件。live FDT timing/interface 已实机重解析为 `rg34xx_v1` / RGB666 / 720×480 / 26-MHz DCLK property / HT×VT 820×536 / `lcd_type=old`；TCON/PLL 只读回读为 `0xf000000b` / `0x88001703`。按 288-MHz 直接馈入 divider 11 推算当前 DCLK 26.1818 MHz、scanout 59.569 Hz，与 IRQ proxy 约 59.6 Hz 一致；只闭合当前 60-Hz 基线，不验证 120 Hz。pvaibhav 120-Hz 候选 pair 为 divider `0xf000000a` + PLL raw `0x88002b03`，但其 gate 要求 `fog_fj035fhd05_v1`，不匹配 live `rg34xx_v1`；ROCKNIX firmware 另有 52-MHz / 813×533 120-Hz reference，但 totals 也不等于 live 820×536。外部 `rgxx_boot_package_mods` 的 54-MHz / HT840 / VT536 与 54-MHz / HT1679 profiles 继续作为参考候选，不作最终板型 profile。无持久化、无真实 TCON/CCU/PLL writes，`hardware_applied=0`。已有 policy/period arithmetic/host tests 与三 ISA freestanding compile 分别由 #45569849692 / #45572079415 验证；inventory collector host/cross compile 与 Windows safety 由 #45573555763 / #45573555742 验证。
- [x] 完成 RG34XX non-SP 的 TCON0 DCLK divider / PLL **只读**实机回读（2026-11-25 19:59 +08）：`0x06511044=0xf000000b`（divider 11）、`0x03001040=0x88001703`；与 pvaibhav/slot 的 60-Hz register pair 完全相同。结合 `tcon_lcd=288 MHz`，若该 clock 直接进入 divider，则 DCLK 推算 26.1818 MHz、820×536 下 59.569 Hz，与 IRQ proxy 约 59.6 Hz 吻合。**只确认当前 60-Hz 状态；没有写寄存器，也没有验证 120 Hz。** 详见 [readback report](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/pvaibhav-slot-runtime/tcon-readback)。
- [ ] 对与 Build ID 匹配的 Linux 4.9.170 #19 kernel 继续离线恢复/反汇编，追出 `rg34xx_v1 + lcd_type=old` 的 panel init、PLL 位域、时钟父链与最终 DCLK；把 pvaibhav/slot 的 120-Hz pair（divider 10 / PLL raw `0x88002b03`）仅作为候选对照，不能直接写入。
- [ ] 分别实现/验证 UI present FPS 与 armGBA committed-frame FPS 的只读测量；LCD IRQ 只能作为 scanout/vblank proxy，不得冒充任一 FPS。GAME 120 Hz 默认仍关闭，所有硬件 writer 保持关闭。

- [x] 把 Linkslot `slot-input` 与 `slot-model` 纯逻辑整理为独立 `no_std + alloc` Rust 业务 crate；两个 crate 的 13 项本地单测通过，`--no-default-features` 下完成 `armv7a-none-eabi` 与 `aarch64-unknown-none` 编译，上游原有输入/模型回归测试也通过。
- [x] 将真实 C/Rust 输入边界接入 `slot-input`：`input_service.h` 定义版本化 H700 Input Service v1，Rust `H700InputSource::drain_into()` 将最多 32 条逻辑按键边沿写入调用者缓冲区；`H700GestureInput` 继续复用 Linkslot 原生 Gestures，覆盖菜单长按和电源长按动作。
- [x] 为 `Gestures` 增加兼容的 `feed_into()` / `tick_into()` / `drop_ff_latch_into()` caller-owned 输出接口；`H700GestureInput` 初始化时预分配 72 个 Action，32 项输入 batch 与计时事件处理不创建临时 Vec。双路径对照测试覆盖菜单/电源/音量组合/快进/倒带/Select，且验证动作缓冲区容量不增长。
- [x] 新增 C 侧 64 项有界 FIFO `h700_input_queue`，验证部分 drain、FIFO、非法按键/状态拒绝、overflow drop-oldest 和 dropped-event 计数；队列当前明确为单执行上下文，不宣称 ISR/multi-producer safe。
- [x] H700 input C ABI 与 queue 分别通过 AArch32/AArch64 freestanding 编译；C 队列 host regression 通过。
- [x] 专项 workflow [45350316021](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45350316021) 全部通过：no_std crate 单测、A32/A64 cross-check、上游 `slot-input`/`slot-model` 行为测试、renderer + 业务 crate overlay 应用、完整 Linkslot `slot` Frontend 编译、C input queue host test 与 C ABI A32/A64 编译。
- [ ] 将按键扫描/ADC/GPIO 读取和 event queue producer 接到经过现场取证的 H700 按键硬件驱动；在明确 IRQ/critical-section 模型前，队列不得从 ISR 或多个 producer 调用。
- [ ] 从 `slot::App` 与 `Frontend` 继续抽取无 I/O 的菜单状态、游戏库和配置决策，建立 Rust 单核事件循环；实际显示扫描输出和输入寄存器访问仍由 C HAL 管理。
- [ ] 复现 GammaOS Pico 的快速启动目标，但以 RG34XX 实测为准；分别测量 power-on→BL33 entry、panel first-light、menu-ready、ROM-ready、first-game-frame 和 power-off resume。参考仓库 README 在 MediaTek KPA 上报告约 2.5 s 到 GBA boot logo，该数值不能直接当作 H700 KPI。
- [ ] 保留低延迟帧路径：H700 高分辨率 monotonic counter + panel TE/VBlank/扫描节拍或验证过的 audio clock；在每个 committed frame 读取输入；PCM DMA 使用有界 ring；逐项测量 input-to-core latency、frame period/jitter、missed frames 和 audio underruns。
- [ ] 将 run-ahead / Preemptive Frames 作为可选能力，验证静态核心 save/restore 的确定性、预测帧静音、dynarec/cache invalidation 与音频/显示截止时间；只暴露设备实测可持续的 0–3 帧深度。
- [ ] 实现独立 H700 monotonic timer backend 和 RTC wall-clock backend。现有 `time_service.h` 只提供 monotonic milliseconds；`_gettimeofday()` 仅委托未实现的 `h700_platform_rtc_gettimeofday`，缺失时返回 `ENOSYS`。RTC 需支持读/设 UTC、时区、掉电/电池状态和 cartridge RTC 时间，禁止用 monotonic tick 伪造日历时间。
- [ ] 对外宣称 GammaOS Pico 等效前，完成冷启动、屏幕刷新率/抖动、输入延迟分布、audio underrun、run-ahead 状态一致性、RTC 断电保持与时钟漂移的实机报告。当前只完成时间 ABI、单核 cooperative event-loop host tests，不代表这些特性已在 H700 实现。

### B. RG34XX / Allwinner H700 启动链与 BL33

- [x] 完成 RG34XX 非 SP 当前活动链与 fallback 的只读取证：stock Boot0 → BaseOS AArch64 native SPL → vendor BL31 → AArch32 SVC BL33 loader → p4+48 MiB BOL4 Android v2 → AArch64 Linux。TOC1 U-Boot 与 p4+0 gzip 镜像保留为 stock fallback，不是本次活动启动路径。
- [x] 从本地 64 MiB p4 dump 提取活动 Android v2/BOL4 输入；21 项交叉检查全部通过，包括 BOL4 解压与 kernel.Image 逐字节相同、AArch64 `ARMd` magic、Android v2 组件边界、空 newc initrd 与 DTB 长度；8 条 SHA-256 清单本地校验通过。报告标记 host-only / not-installable / not-hardware-validated，二进制未上传。
- [x] 从现场 memblock/FDT 交叉确认 monitor 保留区、CMA 候选、framebuffer、initrd、FDT 和未知高地址保留区；未知区域继续排除，不把 `memory - reserved` 当作可分配 RAM。
- [x] 固定 GammaOS LK 参考源码 `TheGammaSqueeze/lk-mt6785@9bf1419a49e0af6dba8b05916df382c9c9c6d0a8`，核对 AArch32 CRT0、异常向量、A53 flags、VBAR、多核 scheduler 和 heap 实现。
- [x] AArch32 LK-as-BL33 入口/头区/向量布局 host-only 探针通过：入口分支位于 image +0，保留 +4..+0x63f 厂商头区，向量候选位于 +0x1000；它使用合成头和 stub，不是完整 LK build 或启动证明。
- [x] 明确 GammaOS LK 固定版本的 H700 适配点：Cortex-A53 分支原本错误使用 `-mcpu=cortex-a9`；`arch_early_init` 会把 VBAR 重设到 MEMBASE；`ENABLE_THUMB=true` 时普通 C/C++ 对象默认 Thumb-2；现有 scheduler/critical section 是单核设计。
- [x] 固定 LK overlay 生成器完成六个文件的 H700 source overlay，生成结果通过 `git apply --check`；实际 GammaOS LK CRT0 + 原始 exception vectors 已在 host CI 完成 A32 ELF 编译与链接。该项不是完整 LK target link 或实机启动。
- [x] 建立独立 RG34XX H700 project/target/platform scaffold：设置 `H700_BUILD=yes`，平台选择 AArch32 Cortex-A53 / `arm-none-eabi-`，隔离 MTK platform/common、PAL 与不兼容模块；内存 layout gate 默认为 fail-closed。`LINKER_SCRIPT` 指向生成的 `system-onesegment.ld`，真实 pinned LK Makefile 变量展开回归已通过。
- [ ] 在内存 ownership/BL33 window 归属审计关闭后，才为 H700 target 填写 MEMBASE/MEMSIZE、heap、stack、MMU/linker ranges；目前不填任何猜测地址。
- [ ] 完整编译/链接 GammaOS LK H700 target，并以实际链接 map、ELF、image header 和尺寸检查替代 stub 布局探针；当前仍未批准 MEMBASE/MEMSIZE，完整 target link 必须等 memory ownership gate 关闭后进行。
- [ ] 在 RG34XX 实机验证 vendor BL31 → A32 LK BL33 的入口寄存器、CurrentEL/SPSR、栈、MMU/cache/TLB、GIC、VBAR、参数寄存器和 secondary-core 状态；host link 不替代该验收。
- [ ] 完成剩余保留区归属审查，尤其 `[0x7ec00000,0x7fc00000)`、顶部碎片、monitor 实际上界和 secure/DRM/DMA/display carveout；未批准前不得给 payload 分配地址。
- [ ] 实机重复冷启动、失败恢复和从 SD 只读加载测试；在完成内存 ownership 与 secondary-core gate 前，不写启动区、不执行 payload。

### C. Allwinner H700 基础 HAL

### RG34XX PI12 电源灯与游戏状态联动（2026-11-25）

- [x] 依据 RG34XX（非 SP）原厂启动设备树和 MustardOS 配置确认 `work_led=PI12`、`low_pwr_led=PI11`；当前只操作 PI12，不改变低电量灯。
- [x] 新增 PI12 GPIO C service：先置高输出锁存，再切换 GPIO output；带 pinmux/readback 检查，更新时 read-modify-write 保留 PI11 和同端口其他位。
- [x] 按 pinned GammaOS LK AArch32 真实顺序实现早期 MMU：`arm_mmu_init()`，映射 RG34XX 1 GiB DRAM 为 normal write-back，映射 `0x03000000–0x03100000` PIO/CCU 为 Device，再 `arch_enable_mmu()`；之后才进入 `target_early_init()` 点亮 PI12。该映射只设页表属性，不分配或批准 DRAM 使用。
- [x] 将 Linkslot 1.0.45 `Session` 游戏状态接入 PI12：`Playing/Polaroids` 熄灯，其余阶段亮灯；状态机抑制逐帧重复 GPIO 访问。默认启用，暂不增加设置项。
- [x] 加入 fake PIO host tests、Rust no_std 状态机 tests、AArch32/AArch64 对象编译和 LK source guard 接入；H700 AArch32/AArch64 compile check [#45524236936](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45524236936)、Linkslot runtime slices [#45524236833](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45524236833) 均通过。
- [ ] RG34XX 实机验证冷启动时灯亮时间是否足以作为松开电源键指示；验证进入游戏熄灭、退出游戏点亮，并确认 PI11 低电量灯不受影响。
- [ ] 实机验证进入 LK 早期钩子时 PIO/APB1 是否保持可访问；目前只确认 H616-family PIO 绑定 APB1、参考 CCU APB1 为 mux/divider 且无 PIO 专属 gate/reset，RG34XX Linux runtime APB1=100 MHz，不等于 BL33-entry 读回。
- [ ] 后续前端设置规划时再增加“游戏时熄灭电源灯”开关，默认开启；关闭时前端常亮。

- [x] 实现并 host-test H700 GPT 只读解析、H616-family sdc0 时钟/引脚配置计算、SD v2 识别状态机、CMD17 PIO sector read、FAT16/FAT32/exFAT 分区边界与目录/文件读取。
- [x] 建立 sdc0 → SD identify → CMD17 → GPT → partition-bounded FAT → Libretro VFS v3 的 fake-device host 端到端测试；明确所有测试均为合成媒体/寄存器，不代表实机 MMIO 或真实卡通讯。
- [x] 同一组 H700 HAL/存储 C 源码的 AArch64、AArch32 ARM-state、AArch32 Thumb-2 freestanding 编译矩阵通过；当前 GammaOS LK H700 target 使用 AArch32，Thumb-2 C/C++ + ARM-state startup/assembly 对齐其编译模式。AArch64 只是独立编译探针，不表示 BL33 使用 A64。
- [ ] 实现并验证目标机真实 sdc0 MMIO、PIO/APB1、CCU gate/reset、PF0-PF5 pinmux、卡供电和 400 kHz SD 初始化；没有现场读回前不得提高时钟或启用 DMA。
- [ ] 通过真实 CMD17 只读采集卡片 GPT、FAT/exFAT 卷及 ROM/BIOS 文件；验证块设备和分区 extent 与现场 inventory 一致。
- [ ] 实现有界写入、flush、原子替换和断电一致性后，才接入 save/state/battery 写路径。
- [ ] 逐项增加显示/framebuffer/backlight、GPIO input、timer、PCM audio、PMIC、电源关机、Wi-Fi/USB 调试服务；每项均有独立诊断和实机验收。

- [x] 审查 Linux 4.9 cpufreq core `show_scaling_cur_freq()` / `show_cpuinfo_cur_freq()` / `__cpufreq_get()`：target-based `cpufreq-dt` 的 `scaling_cur_freq` 为缓存 `policy->cur`，而 `cpuinfo_cur_freq` 调用硬件/clock getter；若不同，getter 路径会触发 `cpufreq_out_of_sync()` 并更新缓存。因此 v2 的“480 MHz cached value → 1008/1104 MHz cpuinfo”不能证明 PLL/driver 持续故障。新 collector 在 getter 前后复读缓存并收集 cooling-device 状态。
- [x] 复核 v2 CPU thermal zone0 46.637°C / first passive trip 60°C：固定 `power_allocator` 候选源码在低于 switch-on trip 时返回 maximum power；本次样本不支持 CPU-zone PA 已主动降频的判断，但 exact binary、其它 thermal cdev 和 policy constraints 仍待实测。
- [x] 补齐 LK `platform_init_mmu_mappings()` 完整调用契约与 fake-MMU 回归；1 GiB DRAM + PIO Device descriptor + MMU enable 顺序已由 H700 AArch32/AArch64/Thumb2 编译和 host regression 验证。 Pinned LK 的 16 KiB static section table 位于 `.bss`，实际 BSS/stack/heap placement 继续受 memory approval gate 约束。 已在 pinned `arch/arm/mmu.h` 与 `arch/ops.h` 核对声明/宏；仍未执行完整 LK target link。
- [x] 已在 RG34XX 实机运行并上传 collector v3：八组 uptime-bracketed getter 对账显示 `scaling_cur_freq` 可由 1008 MHz 在 getter 后更新至 1104 MHz；CPU thermal cooling devices 当前为 0/8 与 0/5，绑定 CPU zone0。v2 的 480 MHz 持续缓存异常被 v3 证据推翻为非原子/滞后读取，不是已证实的 governor clamp 或 clock-driver rejection。报告见 `reports/rg34xx-inventory-20261125-cpux-v3/`。仍不是完整 LK target link 或设备 DVFS 验证。

- [x] 新增 `tools/analyze_h700_opp_candidates.py`：基于 parser-v3 JSON，按 Linux 4.9 OPP core 模拟 no-mask / one-cell table-level `supported_hw`，区分 absent/empty 属性、多版本 OPP 属性、命名电压 suffix 与 base fallback，并与 runtime available frequency list 比对。工具不模拟 regulator selector table，不解码 SID，也不授权真实 OPP。
- [x] OPP candidate analyzer 的完整 u32 mask 假设、多版本 OPP cells、set-cover exact witness 与 voltage-triplet 回归通过 H700 Linux/Windows、storage-host regression 和 AArch64/AArch32 ARM/Thumb-2 编译：#45537478202、#45537478198；验证 HEAD `da41806e47dbd2e0d57e23e9ff6ff405a5514e29`。
- [x] 已用 parser-v4 `live-fdt.json` + 同次 `cpu-power-runtime.txt` 运行 OPP candidate analyzer：38 个 OPP entries 中 14 个 `-ab` 节点 `supported_hw_present=false`；五组 runtime list 一致为九档。静态唯一精确模型是 no table-level `supported_hw` + named voltage suffix `b1`。完整 shipping probe CFG 随后确认 `0x6c00/0x7c00` b0/b1 分支保持 mask=0 并跳过 `dev_pm_opp_set_supported_hw()`；OPP core 对 no-mask 的过滤规则和 named-voltage fallback 已还原。parser-v4 中 b1 变体恰好对应九档 runtime rates、五个被拒频点没有 b1，现已形成**高置信软件路径对账**；这不是实际 DVFS 授权。
- [x] 对照公开 `sun50iw9` cpufreq-NVMEM 候选：T507 / Lineage Linux 4.9.170 的 `0x6c00` 分别映射 mask `0x4` / `0x1`；Creality Linux 4.9.191 路径使用 `speed+Icpu`；Radxa speed-only 路径落入 `a0` + mask。另发现更接近的 Allwinner 家族实现：AvaotaSBC 与 JasonYANG 源码具有 SID offset `0x20/0x28`、pvalue/icpu 同样变换、SoC revision bits、named-property + supported-hw 双 setter/error unwind；但公开分支 speed 常量与 `a/b/c` 输出仍与匹配 binary 不同，且它们的 NVMEM reader 长度安全。故为强 lineage candidate，不是源码身份匹配。详见 CPUX audit §1.13。
- [x] 扩展只读 collector，增加 `kernel-driver-provenance.txt`：kernel release/version、`/sys/kernel/notes` hash/有限 notes bytes、相关 config、cpufreq platform driver/device bindings、模块名与过滤后的 OPP/NVMEM kallsyms symbol names（不输出地址）；H700 Linux/Windows audit + H700 cross-compile/storage regression 全通过 #45543371548、#45543371507。
- [x] 已运行并分析 RG34XX v4 inventory：GNU Build ID `a1d211f847a5fb69fe9898e15fbccdd3ea8d7b68`；Linux 4.9.170 #19 / Linaro GCC 5.3.1；`sun50i-cpufreq-nvmem` 与 `cpufreq-dt` 平台设备均实际绑定。运行时九档频率与 b1 named-voltage 子集严格对应；随后 full CFG 证明 b-prefix branch keeps supported-hw mask absent，目标 OPP core 的 no-mask filter 与该九档清单一致。报告见 `reports/rg34xx-inventory-20261125-cpux-v4/`。
- [x] 已从实机 stock `mmcblk0p4` 只读导出 64 MiB Android boot 分区，解压内核中的 GNU Build ID `a1d211f847a5fb69fe9898e15fbccdd3ea8d7b68` 与 live `/sys/kernel/notes` 精确相同；运行内核与 boot 分区内核二进制身份已关联。解压内核 17,461,256 bytes，SHA-256 `39cf8c3eb9a75ff0eda51b4074cd65860c5aa5386093bcaf69339819421d84b7`，前 16 位与公开 BaseOS survey 前缀一致。分区/压缩 kernel hashes 见 `reports/rg34xx-boot-p4-metadata.json` 与 `reports/rg34xx-boot-p4-verified.json`；不上传完整镜像。
- [x] 对本地解压 kernel Image 使用 `vmlinux-to-elf` 恢复 kallsyms / AArch64 ELF，恢复 89,675 symbols。首轮摘录已由完整 CFG 取代：报告 `reports/rg34xx-cpufreq-full-cfg.txt` / `.json`、`reports/rg34xx-opp-core-disasm.txt`。确认 speed cell 声明 2 bytes（捕获 `00 6c`），shipping probe 执行 32-bit load、忽略长度出参；这是超出声明长度的 logical read，不据此推断故障或 slab object 外访问。b0/b1 保持 mask=0 并跳过 supported-hw setter。
- [x] 完成 `sun50i_cpufreq_nvmem_probe()` 全量 CFG（纯离线，仅本地已符号化 ELF；无设备/`/sys`/`/dev/mem`/`sunxi_get_soc_ver` 调用，不重导出 p4）：函数 `0x…72ccc`–`0x…73148`（不含），287 指令、69 strict blocks、106 edges；`st_size=0` 故以 epilogue `ret` 与下一 FUNC/branch-target 关系判边界，连续覆盖无缝隙。**十个显式 speed 值**→mask/name：`0x2000`/`0x2800`→0/`a1`；`0x5c00`→0/`c0/c1/c2`；`0x6c00`/`0x7c00`→0/`b0/b1`；`0x2400`/`0x5000`/`0x5400`/`0x7400`→mask 1；`0x2c00`→mask 2；default→mask 8。b0/b1 跳过 `set_supported_hw`；name 为空跳过 `set_prop_name`，两 setter 互斥；setter 失败 unwind，cpufreq-dt 只在 CPU loop 成功后注册。
- [x] 恢复目标 OPP core 过滤逻辑：`_of_add_opp_table_v2` 中 `supported_hw==NULL` 会剔除所有带 `opp-supported-hw` 节点、保留无属性节点；注册 mask 时无属性节点仍接受，属性节点需 `(mask[i]&bit[i])!=0`；命名电压优先 `opp-microvolt-<name>`，缺失时回退 base。parser-v4 38 个 live FDT entries 中 b1 变体恰好对应九档 runtime list、五个被拒频点无 b1，no-mask+b1 得到 exact match；mask 1/2/8 会超过九档。NVMEM 2-byte/32-bit 为超出声明长度的 logical read，是否 fault/高半字具体值不作推断。与 Avaota 同血缘但非同一实现。报告见 `reports/rg34xx-cpufreq-full-cfg.txt` / `.json` / `reports/rg34xx-opp-core-disasm.txt`；ELF/boot 保留本地。
- [ ] 继续只读确认 live `sunxi_get_soc_ver()&7` 与 2-byte cell 32-bit load 的实际 operand（不得调用可能临时写 CE clock/reset 的 vendor getter；优先查现有 boot log/安全属性或离线确认 allocator 语义）。即使两项匹配 b1，仍不能授权 OPP/DVFS 写；温度、BL31 多核、timer 与 PMIC/CCU readback 门槛必须独立关闭。
- [ ] 在具备安全 BL33-entry 只读访问依据后读取/验证 AXP2202 DCDC1 与 CPUX PLL/CCU 实际状态；先证明 CPU1–CPU3 handoff、单核所有权和 timer continuity，完成前不允许任何共享 CPU PLL 写入。

### C1. CPU performance modes / H700 DVFS (architecture first)

- [x] 建立 [H700 CPU Performance Service / DVFS 设计](h700-cpu-performance-service.md)：定义 Power Saver、Balanced、Performance、Auto、受限 Boost 模式；C HAL 负责 OPP/PLL/PMIC/热限制，Rust/Linkslot 只提交模式请求。
- [x] 用 RG34XX 现场 inventory 确认实际 PMIC 是 AXP2202；现场 Linux 日志确认 I²C-5 / 地址 `0x34`。
- [x] 从两份 RG34XX 专用 DTS 交叉确认 `cpu@0 cpu-supply` 指向 AXP2202 DCDC1；2026-11-25 live FDT 又确认 CPU0 `cpu-supply` phandle `0x4f` 指向 axp2202-dcdc1（500000–1540000 µV，always_on/boot_on），Linux regulator debugfs 的 cpu0 consumer 报告 1160 mV。该值是 Linux regulator-framework 读数，不是 BL33-entry 原始 PMIC 寄存器值。
- [x] 检查 AXP2202 参考 U-Boot 驱动的 DCDC1 selector/control 结构（`0x83` / `0x80`），但尚未向 PMIC 执行任何写入。
- [x] 找到 SID speed cell (`0x03006000 + 0x00`, 2 bytes) 和 thermal calibration cell (`0x03006000 + 0x14`, 8 bytes)；2026-11-25 有界实机读取为 speed `0x6c00`、thermal bytes `181113ab191b16bb`。主线诊断 candidate bin 5 不是 OPP mask；两个公开 4.9.170 vendor candidates 对同一 raw 值分别映射 `supported_hw=0x4` 和 `0x1`，4.9.191 有另一个长度检查安全实现。目标 binary 对应 source/mask 未知，OPP authorization 仍为 `not_approved`。
- [x] 解析 mainline H616/H700 speed-bin decoder、CPU OPP table 和 CPUX PLL/mux 参考偏移；unknown bin 将在本项目中 fail closed，不照抄 Linux 的 bin-0 fallback。
- [x] 固定并审查 `qiaoweibiao/T507_Kernel@98fcc7d3f112e51b0edfb71536da89cb2115106f` 的 `sun50iw9` CPUX 时钟驱动：确认 `PLL_CPU=0x0000`、`CPU_CFG=0x0500`、CPU mux bits 24–26、AXI/CPUAPB divider fields、`pll_periph0` pre-rate 切换/`pll_cpu` post-rate 恢复，以及新模式 PLL lock polling 的实现；记录 PLL enable bit31 不应动态更改、锁定位受 EVB build macro 影响。审计详情见 [H700 CPUX clock-source audit](h700-cpux-clock-source-audit.md)。这是固定源码证据，不代表 RG34XX 实机 readback 或已授权 MMIO。
- [x] 确认该固定 vendor `clk-sun50iw9_tbl.c` 明确注释频率因子表不正确；因此不从该表生成 RG34XX MHz 档位或 OPP，也不把表格数值与 AXP2202 电压推断拼接。
- [x] 扩展 RG34XX 只读 inventory：从 Linux `clk_summary` 单独摘录 `pll_cpu`、`pll_periph0`、`cpux` 和 `cpu` 条目并保留父子树缩进；不执行 devmem、不触碰 CCU MMIO、不修改 clock policy。对应静态只读安全测试已加入 H700 AArch32/AArch64 workflow。
- [x] 扩展 `tools/inspect_h700_fdt.py`：OPP entries 现输出 `voltage_microvolt_variants`（解析 `opp-microvolt-<suffix>`），OPP table 现输出 `nvmem_cells` / `nvmem_cell_names`。`tools/test_inspect_h700_fdt.py` 增加 a1/b0 电压后缀与 speed-bin phandle 回归；workflow 已添加对应 push/PR path 和执行命令。提交 parser `fc84576d0d597abb6317ac64fefd62cf25dd63dc`、test `0ee82a5709418b8f81120cd337a75b8d237052ee`、workflow `9cbceb738ac2a8802784d0eac6ae6f4f2c5e1a5c`、fixture string 修正 `c5bd42ca547db2c49ec2018946275cd457ff24ee`。新 CI 结果待确认。
- [x] 已上传 parser-v2 与 parser-v3 inventory：`reports/rg34xx-inventory-20261125-cpux-v2/`、`reports/rg34xx-inventory-20261125-cpux-v3/`。v3 包含原始 live FDT 派生的 OPP property-presence、named voltage、table speed NVMEM reference 及同步 cpufreq getter/cooling-state 证据。
- [x] 扩展只读 inventory 为六轮、间隔 200 ms 的 uptime-bracketed 同步采样：每轮读取 policy0/cpu0 的 `scaling_cur_freq`、`cpuinfo_cur_freq` 和 CPUX 相关 `clk_summary` 行，用于区分缓存 policy 频率与 clock-framework 即时读数；仅读取 `/proc/uptime`、cpufreq sysfs 和 clk_summary，不改 governor、频率、电压或寄存器。静态 safety regression 已新增断言并由现有 H700 workflow 执行，结果待确认。
- [x] collector/synchronized-sample 变更的 H700 host regression 与 Linux/Windows boot-audit 已通过 #45526953772、#45526953774；这些只验证采集脚本和静态只读边界，不替代新一轮设备采集。
- [x] parser-v2 OPP/FDT regression 已通过 H700 AArch32/AArch64 与 Linux/Windows workflows #45529626035、#45529626080。parser-v3 与候选分析器的增量验证见本轮 OPP 专项记录。
- [x] 完成 2026-11-25 RG34XX CPUX live read-only inventory：Linux 观察到 CPU0–CPU3 online 且属于同一 cpufreq policy；`clk_summary` 显示 `pll_cpu → cpu → cpuapb`，CPU/CPUAPB 为 1512/378 MHz，AXI 756 MHz，独立 `pll_periph0` 为 600 MHz。该结果证明 Linux 运行态的父时钟关系，不证明 BL33-entry MMIO 状态。
- [x] 对照两个公开 Linux 4.9.170 cpufreq-nvmem candidates：T507 branch 对 raw `0x6c00` 无 case、default mask=0x4；Lineage branch 显式 case→mask=0x1。两者都以 `u32 *` 解引用 2-byte SID cell、未检查 `len`，存在潜在越界读取。另有 Creality Linux 4.9.191 candidate 显式映射 `0x6c00→mask 0x1` 并采用 length-checked `u8 *` read，但版本不是目标 4.9.170。公开 source candidates 彼此不一致，不能推定目标内核 mapping。
- [x] 复核 inventory v1 的 cpufreq/clk_summary 样本时序：per-CPU `scaling_cur_freq` 为 1416 MHz，后续 policy0 为 1320 MHz，稍后 `clk_summary` 为 1512 MHz；同版本 cpufreq-dt 候选显示 `cpuinfo_cur_freq` 通过 `clk_get_rate()` 读取、`scaling_cur_freq` 取缓存 `policy->cur`，所以不同时间的样本不能证明 clock provider 不一致。六轮 uptime-bracketed 同步采样已加入 collector，待实机重新采集。
- [x] 初步核对 live DT 的 shared OPP table 与电压采样：1008MHz/950mV、1104MHz/1000mV、1416MHz/1160mV 分别与若干 supported-hw/电压变体数值吻合，但这些相关性不认证 SID mask。parser-v1 未提取 `opp-microvolt-*` named voltage properties 和 table-level `nvmem-cells`，因此旧 JSON 的电压清单不完整。
- [x] 记录温度校准阻塞：SID 有 4 个 raw 12-bit calibration fields，但内核报告 `avcc is not calibrated`；zone 温度仅作诊断，不可用于 Auto/Boost 闭环。
- [x] 审查同版本 cpufreq 取频语义：候选 `cpufreq-dt` 的 `cpuinfo_cur_freq` 通过 `cpufreq_generic_get()` 读取 `clk_get_rate()`；`scaling_cur_freq` 对 target-based driver 返回缓存 `policy->cur`。inventory v1 将 cpufreq 属性和后续 `clk_summary` 分时读取，原先看到的差异可能来自 schedutil 变频时序，尚无同步快照证明 provider 不一致。
- [x] 更新 [RG34XX CPUX live read-only audit](rg34xx-cpux-live-audit-2026-11-25.md)，归档 live FDT / CPU power / SID / memory map 报告链接与结论。
- [x] 确认目标机有 H700 thermal sensor，但当前内核报告 `avcc is not calibrated`，所以自动温控和 Boost 仍禁止。
- [x] 实现纯 C OPP policy selector：省电/均衡/性能/自动/增强档位策略、speed-bin mask、板级电压上限、热频率上限、限制原因累计、严格 OPP 表校验和缺少硬件门控时 fail closed；测试使用合成 OPP，不是目标机频点表。
- [x] GitHub Actions [H700 CPU OPP / AArch32-AArch64 回归 #45381349400](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45381349400) 通过：CPU policy host tests + AArch64 / AArch32 ARM / AArch32 Thumb-2 freestanding object compile。
- [x] 增加版本化 H700 CPU Performance Service C ABI v1：固定大小 OPP profile、模式请求、策略规划结果、真实硬件遥测字段和明确的 `POLICY_ONLY` 返回值；当前 facade 不调用 MMIO/PMIC，`hardware_applied` 固定为 0。
- [x] 增加 `slot-cpu-performance` Rust `no_std` 客户端：通过不透明 C service 指针请求模式，按 ABI v1 解码 64-byte 状态快照，将计划 OPP 与可选实测遥测分开；并增加 `CpuPerformanceRuntime` token 获取静态 C owner，单个 token 以可变借用发放临时 client，runtime 未就绪时返回独立错误。runtime token 的 workspace host tests、AArch32/AArch64 `no_std` checks 和 Linkslot overlay compile 由 [H700 Linkslot runtime slices #45391108154](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45391108154) 验证通过。
- [x] 在 C host test 中固定 ABI v1 state 的 `sizeof=64`、`hardware_applied` offset=48 和 `reserved` offset=52；[AArch32/AArch64 compile check #45387293421](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45387293421) 通过。
- [x] 审查 Linkslot 1.0.45 quick settings UI 扩展点：系统设置 6 行；`QuickRow`/`QuickValue` 的 discriminant/index 被固定数组使用。CPU 行只能追加，不得重排旧索引；中英文 translation、App 的 quick value/change-setting 分支和 parity tests 必须同步更新。具体接入契约见 [H700 CPU DVFS 文档 §7.2](h700-cpu-performance-service.md)。
- [x] 在 H700 `slot-quick-menu` no_std overlay 中追加 CPU Performance QuickRow：`SYSTEM_SETTINGS` 6→7、`QuickRow::ALL` 87→88，保持 Language index=86、CPU row index=87，原有行索引不变；H700 App overlay 把 `QuickValue::CpuPerformanceUnavailable` 追加为 index=37 / `QuickValue::ALL` 38，并显示中英文“暂不可用 / Unavailable”。App 的左右切换在未连接 approved runtime 时立即返回、不 persist；A/open 也不打开子页。完整 Frontend overlay 编译和两项 App unit tests 由 runtime-slices [#45428654903](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45428654903) 通过；full feasibility [#45428664653](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45428664653) 也 71/71 通过。
- [x] 增加静态 H700 CPU runtime owner：平台静态持有唯一 C service，board-profile 校验、必需证据 gate 检查、四种可见策略预检、失败时撤销旧实例；host 测试使用明确标注的 synthetic OPP。此项只建立 C 生命周期边界，不代表已有 RG34XX profile 或硬件调频。
- [x] 将 H700 CPU policy、C service ABI 和 static runtime owner 的三个 C 对象加入 `platform/h700/rules.mk` 的 LK `OBJS`；扩展 LK project guard 检查对象项和源文件匹配。该项只登记未来 target link 的对象图，未运行完整 LK target link，也没有调用 owner 的 startup code 或真实 OPP profile。
- [x] 为 RG34XX H700 提供 LK 必需的 `target_early_init()` / `target_init()` 两个 target hooks，替代被 overlay 排除的 MTK generic target init 文件；两个 hooks 当前保持 no-op，不伪造 MTK fastboot/eMMC/memory scratch 能力。扩展 guard 和 AArch32/AArch64 compile probes 覆盖该对象。
- [ ] 在完整 H700 platform/target 服务逐项就绪后，把 board-approved profile 的加载放入 `target_init()`（`platform_init()` 之后、`apps_init()` 之前）；在硬件证据齐备前，禁止通过此 hook 初始化任何真实 CPU OPP profile。
- [x] CPU Performance QuickRow 以 display-only/unavailable 方式接入 H700 App overlay：App regression tests 验证 QuickValue/index、左右切换无效、无持久化写入。Runtime-slices [#45428654903](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45428654903) 20/20 与 full feasibility [#45428664653](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45428664653) 71/71 均执行并通过 2 项 App unit tests；完整产物 artifact [14937338188](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45428664653/artifacts/14937338188)。
- [ ] 在 `target_init()` 的真实 LK 单核启动路径中调用 runtime owner，并将静态服务指针接入 Linkslot Rust client/App；H700 `target_init()` hook 已存在但保持空实现，没有 board-approved RG34XX OPP profile，因此目标运行时仍 unavailable。当前 Quick Menu 只显示不可用占位，不允许选择/保存性能模式。
- [x] 扩展 CPU service host regression：覆盖无效 reinitialization 使旧实例失效，并增加 static runtime owner profile/lifecycle 测试；owner C 源加入 AArch64、AArch32 ARM-state、AArch32 Thumb-2 编译矩阵。服务与 owner 的 host regression/AArch compile 由 #45390257222 和 #45391642082 验证通过；target hooks 的三种架构编译由 #45392910394 验证通过。
- [x] 扩展只读 FDT/inventory：解析 CPU `cpu-supply`、`operating-points-v2`、regulator phandle、SID/NVMEM cell offset/size、thermal-sensor provider arguments；新增 `cpu-power-runtime.txt` 采集 Linux CPUFreq、thermal zone/trip、regulator、clock readback 及 NVMEM 元数据（不读取原始 fuse bytes）。[H700 boot audit tools #45396816432](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45396816432) 的 Ubuntu 与 Windows jobs 均通过。旧的 2026-11-22 live inventory 尚未重新采集，真实设备读回仍待验证。 采集器对旧 Linux 4.9 未导出的 sysfs 类节点显式输出 `NOT_PRESENT`，对节点存在但属性不可读输出 `NO_READABLE_*`，避免空白/缺失被误判为零值；安全回归与 Linux/Windows CI 由 #45397788581 验证通过。 现新增 `tools/inspect_h700_sid_cells.py` 与 Windows runner 集成，输出 `sid-cell-report.json`；仅解析两个限长 cell，speed raw 不映射 OPP mask，thermal raw 不认证校准。Linux/Windows 综合 host CI [#45399690374](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45399690374) 全部通过。固定 vendor cpufreq consumer 的 `u32*` / 2-byte NVMEM cell 长度问题已记录为后续 OPP 解码阻塞项。 通用 SoC revision 探测只查看 `/sys/devices/soc0/{machine,family,soc_id,revision}`，不读取 serial/chipid；禁止调用 sun50iw9 vendor `sunxi_get_soc_ver()` / `sunxi_info`，因为该函数路径可能临时写 CE clock/reset；对应 safety regression 已通过 [#45401913670](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45401913670)，最新 inventory audit [#45402146927](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45402146927) 也通过 Linux/Windows。 SID JSON 现同时保留 generic SoC bus 的 `machine/family/soc_id/revision` 原始值；缺失时标记 unavailable，但不作为 OPP 授权。该提取测试 [#45402936876](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45402936876) 及文档/审查 [#45403070519](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45403070519) 的 Linux/Windows jobs 均通过。 新增 mainline reference classifier：直接 raw values 只给诊断 candidate bin；`0x2400/0x7400/0x2c00/0x7c00` 需 ARM SMCCC ARCH_SOC_ID revision 才能区分 bin 1/2；主线在 `SMCCC_RET_NOT_SUPPORTED` 时默认 bin 2，本项目不采用该假设，revision 缺失/unsupported 时仍 ambiguous；未知值不走上游 default-to-bin-0；generic soc0 revision 不替代 SMCCC 值。所有结果仍 `opp_authorization=not_approved`。host parser tests #45404785874 和 source/docs audit #45405024561 通过。 新增 `mainline_reference_decode` 只作诊断，区分 direct raw→candidate bin、需要 SMCCC ARCH_SOC_ID revision 的歧义值和 unsupported raw；generic `soc0/revision` 不替代 SMCCC revision，所有分支仍 `opp_authorization=not_approved`。
- [ ] 从实机 live-FDT 导出完整 regulator subtree，确认 DCDC1 及该板 revision 的 CPU rail / 约束；只读 PMIC 电压与 enable-state readback。
- [ ] 从实机读取 SID speed-bin 原始值，验证 SoC revision 语义；unknown / unsupported raw values 必须禁用提升频点。
- [ ] 在取得 RG34XX live FDT / BL33 安全只读访问依据后，采集 `PLL_CPU` 与 `CPU_CFG` 实际值、当前 CPU parent、`pll_periph0` 频率、AXI/CPUAPB divider 和 timer source；同时确认 lock-status build configuration 与精确 CCU base。不得从 vendor source address 直接推导 LK MMIO 写路径。
- [ ] 证明 BL31→BL33 的 CPU1–CPU3 状态、共享 CPUX 时钟协调和 timer continuity；完成前不允许更改共享 CPU PLL。
- [ ] 校准 H700 thermal sensor / AVCC；在有效温度数据缺失时禁用 Auto / Boost 的自动升频分支。
- [ ] 实现只读 clock/voltage/bin/temperature telemetry；后续硬件 transition 必须通过升频先升压、降频先降压、失败回滚和滞回/最小驻留时间测试。
- [ ] 只有目标板 OPP/电压表、温度限制、次级 CPU 状态与实机持续负载测试全部通过后，才开放 Balanced/Performance/Boost 的实际 DVFS；模式不接受任意 MHz 输入，必须报告实际频率和被钳制原因。
### A. 工具链与工程基础（已完成第一步）

- [x] AArch64 freestanding C、C++ 和汇编编译探针。
- [x] 检查 C、C++ 和汇编产物确实为 AArch64 ELF relocatable objects。
- [x] GitHub Actions 自动编译检查。
- [ ] 增加统一构建参数与工具链版本记录。
- [ ] 明确调试日志输出方式和目标 ELF/镜像的检查规则。

### B. RG34XX / Allwinner H700 启动链与设备识别

交付：目标设备 SD 启动链调查报告和最小裸机入口规范。

- [x] 从 BaseOS-MultiFrontend 固件审计确认 RG34XX 型号、Allwinner H700、Cortex-A53、1 GB LPDDR4 和 `rg34xx_v1` 面板驱动。
- [x] 用 RG34XX 非 SP 现场 provenance 证明当前活动链是 stock Boot0 → BaseOS AArch64 native SPL → vendor BL31 → AArch32 SVC BL33 loader → AArch64 Linux；TOC1 U-Boot 属于 fallback。形成[启动交接审计报告](rg34xx-boot-handoff-audit.md)和活动 loader provenance 报告。此项只证明当前 BaseOS 链，不证明 GammaOS LK 已在机运行。
- [x] 建立[目标机只读 inventory 脚本](../scripts/collect-rg34xx-readonly-inventory.sh)、Boot0 eGON/checksum/DRAM 候选解析器、Allwinner TOC1 校验/提取器、H700 FDT/framebuffer 解析器、U-Boot env CRC 解析器、CMA/memblock 物理范围解析器及静态只读安全回归；主机合成测试覆盖各类输入。
- [x] 在 RG34XX 非 SP 实机运行首轮[只读采集流程](rg34xx-readonly-inventory.md)，完成 Boot0/TOC1 checksum、组件提取与 hash、live FDT、GPT、/proc/iomem 和 meminfo 检查；现场结论见[实机 Inventory 报告](rg34xx-live-inventory-2026-11-22.md)。
- [x] 从 live FDT 的 `boot_fb0` 与 bootargs `disp_reserve` 交叉确认 framebuffer 物理范围 `[0x7bf24800, 0x7c076000)`（720×480×32bpp，1,382,400 bytes）；新增 FDT parser JSON 字段及合成测试。
- [x] 读取完整 16 MiB U-Boot env GPT 分区：CRC 唯一匹配到 128 KiB 环境映像（data offset 4），25 个变量无解析警告；完整分区 hash 与 BaseOS prepared RG34XX env 区域一致。目标 U-Boot 静态字符串与 SD 自动切换路径相符，现场报告记录了证据边界。
- [x] 将两份现场 Boot0 的 32-word DRAM 参数与 BaseOS RG34XX profile 对照：LBA16 主 LPDDR4 块 32/32 匹配；LBA512 的 LPDDR3/LPDDR4 备用块也各自 32/32 匹配；LBA512 主块仅 word[26]/word[29] 不同。H616 BROM 公开搜索位置与 LBA16/LBA512 对应，LBA16 为高置信正常启动候选，但尚无直接读取轨迹。
- [x] 读取第四轮 memblock `memory/reserved`：确认 1 GiB memory window 与 46 条 reserved entries；识别 `[0x48000000,0x49000000)` 为与 BaseOS vendor monitor 研究窗口一致的 16 MiB 保留区，并将 `[0x77c00000,0x7bc00000)` 标为与 `CmaTotal=64 MiB` 精确匹配的高置信 CMA 候选；现场明细见 [Inventory 报告](rg34xx-live-inventory-2026-11-22.md)。解析器现支持 RG34XX 的逗号加 `size:...K` memblock 格式，且 PAGE_SIZE 缺失时仍能解析 memblock。
- [x] 2026-11-25 新一轮 inventory 再次捕获 1 GiB memory window、46 条 reserved ranges、64 MiB CMA 和 720×480×32bpp framebuffer `[0x7bf24800,0x7c076000)`；`[0x7ec00000,0x7fc00000)` 16 MiB 仍未归因，继续排除。CMA debugfs 在 Linux 4.9 未导出，但 meminfo 报告 `CmaTotal=64 MiB`；此快照不授权分配任何新 payload RAM。详见 [CPUX live audit](rg34xx-cpux-live-audit-2026-11-25.md)。
- [x] 反编译第四轮 live FDT 并与 memblock 交叉审查：将 `[0x42000000,0x42001000)` 与 FDT initrd 范围 `[0x42000000,0x42000200)` 关联，将 `[0x44000000,0x44023240)` 标记为高置信 live FDT blob 候选；识别 `/firmware/optee` SMC conduit 和 ION secure/CMA heap 声明；确认 ION heap base/size 为零、Linux OP-TEE UID 检查失败及 bootargs `androidboot.secure_os_exist=0`，但不据此声称安全世界/secure carveout 不存在。FDT parser 已增加 initrd、firmware_nodes、ion_heaps JSON 字段和合成测试。
- [ ] 完成剩余保留区归属审查：尤其 `[0x7ec00000,0x7fc00000)` 的 16 MiB 高地址区、top-of-RAM 37 条碎片、monitor 实际访问上界、其他 secure/DMA/display carveout 与 loader 生命周期。第四轮 `/proc/iomem` 未给该 16 MiB 区域标注 owner；与 Linux 4.9.170 / sun50iw9p1 同系列 Allwinner 参考驱动显示的 ION secure heap `GET_DRM_INFO` vendor SMC 路径仍只有待验证关联，不能仅凭配置或节点存在就给 carveout 定名。2026-11-25 16:30 +08:00 报告已捕获只读 `tee-ion-runtime.txt`：`CONFIG_ION/TEE/OPTEE=y`、`/dev/ion` 存在，`/dev/tee0` 与 `/dev/teepriv0` 缺失、标准 OP-TEE probe 为 `api uid mismatch`；ION secure heap 当前零分配，sys_user 有 7,397,376 bytes 当前分配（含 7,372,800 bytes `ion_disp2`），CMA 64 MiB（约 62 MiB free），FDT secure heap base/size 为零。此快照没有执行 vendor SMC/ioctl，因此不能证明 vendor DRM SMC 不存在或失败，也不能为 16 MiB 保留区定名。此项闭合前，16 MiB 继续作为未知保留区排除；ION `heap_secure` 的 FDT base/size 为零本身不能给任何物理地址定名。framebuffer `[0x7bf24800,0x7c076000)` 即使未出现在 memblock 列表中也必须排除。不得把 `memblock.memory - memblock.reserved` 直接当作 payload 可用 RAM。
- [x] 以 GammaOS Pico 固定 LK 源码（`9bf141a9`，AArch32）完成 A32 BL33 source overlay、入口/异常向量布局检查、A53 编译 flags/VBAR 调整，以及原生 H700 project scaffold。现场交接状态为 A32 SVC（`0x4a000000` / SPSR `0x1d3`），因此当前路线是 A32 LK 替换 BL33，不先做 A64 payload。
- [x] 查明 LK 的 `lk.img` 会经过 MTK `mkimage`，不能直接作为 BL33；新增 SHA 固定 reference-loader header wrapper，见 [BL33 image packaging](h700-bl33-image-format.md)。其 host regression 只证明字节/分支/向量处理，不证明真实 LK image 或实机启动。
- [ ] 完成 GammaOS LK target link、从 `lk-no-mtk-header.bin` 生成完整 BL33 image、检查实际 map/header/尺寸，并在内存 ownership 和恢复路径关闭前不安装或写入启动区域。
- [ ] 建立最小 A32 handoff adapter + AArch64 payload；实机验证 CurrentEL、SPSR、参数寄存器、栈、异常向量、GIC、MMU/cache/TLB 与 FDT/内存边界。
- [ ] 完成重复冷启动、第一帧/日志时间戳和失败恢复测试；恢复方式按电脑重写/修复 SD 卡，不依赖旧 Linux 自动回退。
- [x] 增加 RG34XX BaseOS-native BOL4 / Android boot v2 host 打包、检查与本地 p4 dump 提取工具；显式按本次已审计 capture 的 p4 +48 MiB / 0x829000 extent 提取活动镜像，并拆出 header、BOL4、AArch64 Image、initrd、DTB 与 SHA-256 manifest；固定 extent 同时校验 Android image FNV 与 BOL4 header fingerprint。14 项 host tests 覆盖压缩、容器、损坏输入、有界提取、不修改原始 dump，并以固定 BaseOS native C decoder 验证 BOL4 互操作；工具只处理普通文件，不操作 SD/Boot 区，也不代表镜像已能在实机启动。
- [x] 使用本地保留的 64 MiB `boot-partition.bin` 运行 `extract-inputs` 并完成 21 项交叉核验；`extracted-manifest.json`、`verification.json`、`SHA256SUMS.txt` 已归档到 `reports/rg34xx-active-inputs/`（8/8 checksum）；原始分区与提取二进制仍只保留本地。BOL4 解压所得 `kernel.Image` SHA-256 与主 Android gzip 内核逐字节一致，见 reports 分支的 `rg34xx-boot-partition-deep-audit.md` addendum。
- [x] 从同一份本地 p4 dump 提取 p4+40 MiB native firmware package（`0x02800000` / `0x46200`），并校验 SHA-256 `dc9dc3ae…f28e9`、FNV `0xaf0018cb`、四段连续边界；p4+40 DTB 与 p4+48 active image DTB（139,822 B）逐字节相同。提取清单及反汇编记录见 reports 分支 `rg34xx-native-firmware/`。
- [x] 将 p4+40 A32 loader 的 MOVW/MOVT 常量及 CMP 与 `native-handoff.h` 记录 ABI 对照：`image_bytes` 位于 record `+0xbc`，`image_fnv` 位于 `+0xc0`；loader 通过 `r7=0x4a300028` 的 `+0x94/+0x98` 读取这两个字段，正好对应当前活动镜像的 `0x00829000` / `0x04762a5f`。这证明部署 loader 与活动镜像的身份字段匹配，不证明 raw-capsule SPL 已被独立反汇编或整套 generation 可复现。
- [x] 完成原厂 BaseOS 启动链的只读来源核验：活动 Android v2/BOL4、p4+40 MiB native firmware、A32 loader 的 image length/FNV 检查，以及原厂 native generation/SPL/raw-capsule 的 host-only 可重复性比较。该工作只用于识别当前 RG34XX 的 stock boot chain 与配套关系；重建结果与 BaseOS 仓库已提交的原厂产物逐 hash/逐字节匹配，但**不是 GammaOS Pico/LK 的构建产物，也不会集成到本项目镜像**。后续只保留它作为启动链参考，不继续制作 BaseOS image/firmware candidate。

### C. Allwinner H700 基础 HAL

### 存储元数据准备（host-only）

- [x] 实现 H700 primary GPT 只读解析器：校验 protective MBR、GPT 1.0 header/CRC、分区数组 CRC、可用 LBA 边界、GUID 唯一性及分区不重叠；以 RG34XX 实测 GPT 布局建立合成磁盘回归测试。该项不包含 MMC 驱动、实机读卡、文件系统挂载或任何写操作，详见 [H700 GPT 只读发现](h700-gpt-readonly.md)。
- [x] 实现 H616/H700 MMC module-clock 纯计算器并加入 host 回归：按 pinned U-Boot 的 OSC24M/PLL6 parent 选择、M/N divider 算法生成 CCU register value；覆盖 400 kHz、25/50 MHz、可表示边界、无超频 rate sweep。该模块不访问 MMIO，不配置 CCU 或 card clock，详见 [sdc0 MMC 板级配置](h700-mmc-board-profile.md)。
- [x] 建立单扇区 CMD17 PIO 读取原语及模拟寄存器回归：SDHC/SDSC 地址换算、512-byte FIFO、错误与超时恢复；保持 IDMA 与写/擦除命令关闭。该项不是完整 MMC 驱动，尚未连接真实 sdc0 初始化和实机读卡，详见 [H700 MMC 板级配置](h700-mmc-board-profile.md)。
- [x] 将 CMD17 PIO 读取封装为 GPT `h700_block_reader`，容量/寻址方式取自 SD OCR/CSD；GPT sector/header scratch buffer 显式 4-byte aligned。此项仍是 host-only 模拟，没有实机 LBA0/GPT readback。
- [x] 建立合成 GPT over CMD17 端到端 host 测试：通过 PIO FIFO 模拟卡片扇区，调用真实 GPT 解析器识别 p6 UDISK/p7 primary，验证 no-write 路径。此项仍不是 RG34XX 实机扇区读取。
- [x] 实现 SD CSD 结构 0/1 只读容量解析与寻址类型校验；用 RG34XX 现场 30,375,936-sector 容量建立合成 R2 回归。尚未从实体卡采集 CSD 响应，详见 [H700 MMC 板级配置](h700-mmc-board-profile.md)。
- [x] 实现 SD v2 只读卡识别状态机及模拟寄存器回归：CMD0/CMD8/CMD55-ACMD41/CMD2/CMD3/CMD9/CMD7，SDSC 额外 CMD16；限定只读命令、400 kHz、1-bit 识别，不包含 pinmux/CCU 实机配置和真实卡通讯，详见 [H700 MMC 板级配置](h700-mmc-board-profile.md)。
- [x] 完成 H616-family PIO/APB1 24MHz（OSC24M）时钟选择 helper；根据 RG34XX live `sun50iw9p1` pinconf 编码解出 sdc0 PF0–PF5 为 ALT2、40mA（硬件 field=3）、pull-up（field=1），并将独立 host-tested pin helper 从通用 H616 30mA 更新为实机配置。寄存器读回与模拟调用顺序均有 host 回归；仍未进行实机 MMIO，详见 [H700 MMC 板级配置](h700-mmc-board-profile.md)。
- [x] 增加 H616-family sdc0 CCU gate/reset 与 400 kHz module/card clock 集成封装及主机寄存器回归；通过 CCU 偏移回调写入 `0x84c` / `0x830`，并复用 `MASK_DATA0` 时钟更新逻辑。PF0–PF5 有独立 host-tested ALT2/40mA/pull-up 配置 helper（pinconf 源码依据见 board profile）；目标 PIO/APB1 实机使能、卡供电和实际 MMIO 仍未测试，详见 [H700 MMC 板级配置](h700-mmc-board-profile.md)。
- [x] 建立统一的 sdc0 只读 bring-up→SD v2 识别→CMD17 block reader→primary GPT scan 入口，并以 fake H700 PIO/CCU/MMC/FIFO 完整模拟；确认 p6 UDISK、p7 primary 和无写命令。调用方仍需提供已启用的 PIO/APB1 时钟、卡供电和有效 MMIO 映射；尚未进行实机测试。
- [x] 将 CMD17 单扇区读取桥接到 FAT16/FAT32/exFAT 多扇区回调，使用对齐 bounce buffer；支持按 GPT 分区记录带精确 extent 挂载只读卷，并测试 FAT32 p7 的 GPT→CMD17→VBR 路径。
- [x] 建立 GPT→CMD17→FAT32 目录/文件主机端到端测试：通过模拟 FAT32 p7 挂载卷、枚举 `ROMS`、打开 `TITLE.GBA` 与 `BIOS.BIN` 并读回内容。全程只读、合成媒体；尚未访问实机卷。
- [x] 增加 C 侧 Libretro VFS v3 FAT provider：固定文件/目录 handle 表、挂载点/allowed-root path containment、read/seek/stat/readdir/get_path；写入和元数据变更回调全部 fail-closed。通过 fake `linkslot_vfs_install()` 测试 GPT→CMD17→FAT→VFS 文件读取。尚未由目标启动代码调用或在实机运行。
- [x] 增加统一 `h700_fat_vfs_scan_partition_and_install()` bootstrap：一次调用完成 sdc0 初始化、SD 识别、GPT 精确名称查找、持久 CMD17 block device、FAT 分区挂载和 VFS v3 注册；fake 8192-sector 卡端到端读 ROM/BIOS 并确认零写命令。目标 Linkslot 启动代码仍需在 `retro_init()` 前调用，尚未实机验证。
- [x] FAT16/FAT32/exFAT mount 增加 GPT/MBR 分区 extent 约束；新增显式 `fat_mount_partition()`，并在读取 FAT 表/根目录前验证 BPB/VolumeLength、FAT 容量、cluster heap 与 root cluster 不越界。新增 GPT p7、MBR、FAT16/32、exFAT host 测试；尚未挂载实机 p7。
- [ ] 在 sdc0 只读块设备路径完成后，读取真实磁盘 GPT 并对照现场分区布局；再以文件系统签名识别 ROM/user-data 卷，不依赖 Linux mmcblk 编号或 GPT 分区标签。

依赖 B 完成后再实现：

- 定时器、GPIO 输入、日志；
- SD 块设备与 FAT32/exFAT 文件系统；
- 显示 framebuffer 与背光；
- PCM 音频输出；
- PMIC 电量/充电状态与完全断电；
- USB 调试（优先 ADB gadget，至少保留可靠串口日志）。

每个硬件接口必须有独立探测/诊断和可重复测试，不以写入未经确认的寄存器地址代替硬件调查。

### D. Emulator Core 独立准备

不依赖 Linkslot UI 的工作可以先做：

- 审查 Gambatte、mGBA、gpSP 的 AArch32/AArch64 构建和 ABI；根据最终 LK/runtime 架构固定核心 ABI。
- 比较 Pico Core ABI 与当前核心上游接口；
- 建立 ROM、视频帧、音频采样、输入、即时状态、电池存档的核心测试夹具；
- 确认 GB/GBC/GBA 核心在 Cortex-A53 上的优化选项和内存需求；
- 存档兼容性以真实文件测试，不根据扩展名推断。

## Linkslot 大版本发布后

### L1：上游基线和适配审计

- 锁定新的上游 commit SHA。
- 通过 Git subtree 导入 `third_party/linkslot`。
- 重新做逐 crate 依赖审计，避免在即将废弃的旧版本上重复工作。
- 将上游代码分为可原样复用、需要替换后端、必须重写三类。
- 明确实际需要的最小 Rust/LK 运行时能力。

### L2：Linkslot 集成

- 先接入静态 UI、数据模型和游戏库。
- 再接入配置、存档、即时状态、MOD、Web 和云同步。
- 每次上游同步均运行差异检查、编译和兼容性回归。
- H700 代码不得散落到上游业务逻辑模块。

## 完整功能阶段（依赖上游集成与 HAL 完成）

### Emulator / UI

- GB/GBC/GBA 三个平台。
- Linkslot 游戏库、封面、最近游玩、收藏、核心选择和游戏内菜单。
- LCD 校色、调色板、滤镜、Overlay/Bezel/遮罩和金手指。
- 快进、倒带、即时存档/读档、按键映射和核心选项。

### 家庭 LAN Web 管理

- H700 Wi-Fi、DHCP/DNS、TCP Server、HTTP API 和 Linkslot Web 页面。
- 仅家庭 LAN 可达；默认不强制密码，可选密码/配对令牌。
- WebDAV 云同步保留 Linkslot 的哈希、版本仲裁、冲突提示和安全替换语义。
- 云同步只由用户显式触发。

### MOD 与调试

- 保留 Linkslot Lua MOD 目录/API 和执行时机。
- Lua 有内存/执行时间/请求数量上限。
- 调试构建可通过 USB 连接读取日志、执行命令和传输文件；正式版默认关闭远程调试。

### 完全关机与发行验收

- 长按电源键只写即时恢复状态；电池存档和云同步均不自动触发。
- 文件 flush 与断电顺序可审计。
- 正常关机后不进入 suspend/s2idle。
- 下一次开机可恢复上次游戏状态。
- 网络断开、云端冲突、MOD 故障不会阻止关机。
- 冷启动到可操作游戏库的时间有实测数据并纳入发布说明。

## 开发原则

1. 不跨阶段同时引入多个新硬件风险。
2. 先完成不依赖 Linkslot 版本的底层工作，避免跟进上游大版本时重复返工。
3. 每个驱动必须有独立探测/诊断页或可读日志。
4. 功能实现与验收测试同一个 PR 提交。
5. 任何跨核心存档兼容结论必须附真实文件、SHA-256、长度和复现步骤。
6. 先支持一台明确型号，再扩展其他 H700 设备 profile。


### pvaibhav/slot 120 Hz reference review (2026-11-25)

Reviewed the pinned [`pvaibhav/slot` implementation](https://github.com/pvaibhav/slot/tree/5206e3fa39aa4a9b702cd9048a07dab3b5ab95da). Its exact hardware gate, ordered PLL/divider writes, readback, 60-Hz rollback attempt and LCD-IRQ validation are valuable design references. It is not a compatible drop-in: its `rg34xx` gate requires `fog_fj035fhd05_v1` / 26 MHz / 820×536, whereas our live RG34XX identifies as `rg34xx_v1`; its non-SP RG34XX has no connected-device live test. The fork applies a global 120-Hz mode at startup/wake and does not switch UI/GAME modes. See [detailed review](slot-pvaibhav-120hz-review.md). The first read-only live capture is complete: UI and GAME both remained near 59.6 Hz; `high_refresh=1` did not produce a 120-Hz observation, and the live `rg34xx_v1` identity fails the fork's exact `fog_fj035fhd05_v1` gate. See [the live capture analysis](pvaibhav-slot-rg34xx-live-capture-2026-11-25.md). The live-FDT timing reparse and same-boot TCON/PLL readback are complete. The active 60-Hz pair is divider 11 + PLL raw `0x88001703`; a pinned public sun50iw9 formula estimates 288-MHz parent / 26.1818-MHz DCLK / 59.569 Hz. The pvaibhav 120-Hz candidate pair estimates 528-MHz parent / 52.8-MHz DCLK / 120.13 Hz at live 820×536 totals, but this external-source decode is not yet verified against exact kernel #19 and does not validate panel support. Next, disassemble the Build-ID-matched stock p4 kernel ELF to trace `rg34xx_v1 + lcd_type=old` panel init and PLL/parent mux; then measure UI present and armGBA committed frames independently. Do not port its register values or enable hardware writes.


### pvaibhav/slot live-device result (2026-11-25)

A same-device UI/GAME capture has been completed on the RG34XX non-SP with the multi-frontend BaseOS launch. Both phases remained at approximately 59.6 Hz by display IRQ counters (UI 59.614–59.648 Hz; GAME 59.632 Hz). The later live-FDT reparse confirms `rg34xx_v1` / RGB666 / 720×480 / 26-MHz DCLK property / 820×536 totals / `lcd_type=old`; same-boot TCON/PLL readback then confirmed divider word `0xf000000b` and PLL raw word `0x88001703`. Using the pinned sun50iw9 reference formula, the current PLL raw word decodes to a 288-MHz `pll_video0x4` rate; 288/11 gives an estimated DCLK of 26.1818 MHz and 59.569 Hz, agreeing with the IRQ proxy within about 0.1%. This decode is reference-source analysis, not yet verified against exact running kernel #19. `slot.state` has `high_refresh=1`, but no 120-Hz hardware application was observed. The running process is `/mnt/sdcard/System/slot`, GAME maps `armgba_libretro.so`, and UI presents / armGBA committed FPS remain unmeasured.

The live FDT is re-parsed and published with byte-matched DTB/JSON hashes. It confirms `rg34xx_v1`, RGB666, 720×480, DCLK property 26 MHz, HT/VT 820×536, H porches 46/2/52, V porches 20/5/31, and bootarg `lcd_type=old`; static DTB calculation is 59.155 Hz. The later same-boot TCON/PLL readback gives divider 11 and PLL raw `0x88001703`; using the pinned public sun50iw9 formula, the baseline PLL decodes to 288 MHz, leading to estimated DCLK 26.1818 MHz and 59.569 Hz. Exact PLL formula/clock parent still needs confirmation against kernel #19; readback is read-only. The pvaibhav fork's gate requires `fog_fj035fhd05_v1`, so it does not match `rg34xx_v1`. Do not weaken the panel allowlist or copy its register values. Hardware writes remain disabled.


The vendor U-Boot survey now resolves an additional identity nuance: RG34XX selects the `rg34xx_v1` panel struct, but that struct calls a common U-Boot init routine whose symbol name includes `fj035fhd05`. The routine label is not a replacement for the live DT driver name. The survey maps PE6 high → `lcd_type=old`, low → `boe`, matching the live `old` bootarg. Use target + DT driver + `lcd_type` + complete timing as the display-profile identity; do not equate the FJ035 init routine with support for pvaibhav's FJ035 Linux writer.


The [related RG34XX-H divider-mismatch report](https://community.muos.dev/t/rg34xx-h-incorrect-display-refresh-rate/1825) is retained as a caution, not a transferable profile: a different unit reported ~106.6 Hz from nominal ~60 Hz with a related fixed /6 TCON path, and a later DTS revision changed DCLK/HT to 54 MHz/1679. Another unit could not reproduce the symptom. The RG34XX non-SP's current DCLK divider is already read back as 11, matching the pvaibhav 60-Hz word. Resolve the exact kernel #19 PLL fields, runtime parent mux and panel init using the Build-ID-matched stock p4 ELF before considering any 120-Hz candidate. Never apply the MustardOS timing to this device by analogy.


The TCON DCLK / PLL readback is complete. The next display milestone is to validate the public sun50iw9 PLL bitfield calculation against the exact kernel #19 source, trace `rg34xx_v1 + lcd_type=old` panel initialization and TCON clock parent, and establish whether a panel-qualified 120-Hz timing exists. The external reference math predicts 528 MHz PLL parent / divider 10 = 52.8-MHz DCLK for the pvaibhav high-mode pair, or about 120.13 Hz at live 820×536 totals; this is a candidate calculation only, not a validated profile or authorization to write registers. Keep the hardware writer disabled.
