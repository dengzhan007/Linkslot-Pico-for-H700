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

## 当前工作：GammaOS LK → H700 BL33 → H700 HAL → Linkslot main 1.0.49

### 当前源码基线升级：从历史 1.0.45 迁移至 main 1.0.49（host/cross-build feasibility 已通过，实机 HAL 仍在开发）

- [x] 归档并比较 Linkslot 最新三分支：main 1.0.49 与 test 1.0.49 的 851 个源码文件逐 blob 一致；DEV 1.1.0 相对 main 新增 21、删除 1、修改 46。当前唯一稳定移植基线为 main 1.0.49；原始来源 commit、归档 commit 和差异见 [上游源码基线](upstream-baselines.md) / [1.0.49 与 1.1.0 更新审查](linkslot-1.0.49-1.1.0-refresh.md)。
- [x] H700 no_std `slot-model/core.rs` 已同步 1.0.49 CoreSpec，包括 `netpacket` 字段；gpSP/armGBA netpacket=true、mGBA=false，contract test 已补齐。
- [x] Frontend 与 renderer patcher source pin 已更新到归档 HEAD `241e7c1974926fa22552b200852914ceaccbfd42` 和 workspace 1.0.49。
- [x] 1.0.49 runtime-slices workflow [#45624285300](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45624285300) 成功：no_std host tests/A32/A64、renderer/business overlay、input/model regression、完整 Frontend + SoftwareCompositor cargo check、CPU QuickRow fail-closed tests 与 C input ABI/queue 全部通过。首轮 #45623895416 因缺少 armGBA `armgba_link_carried` / `armgba_port` 接口失败，已修复后复验通过。
- [x] Linkslot main 1.0.49 完整 feasibility workflow [#45646868006](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45646868006) 已在 HEAD `018e0b30d14965b56003e23e06da5593a94808d6` **71/71 steps 成功**；静态 mGBA selector 的 `Option<LinkSetup>` 类型错误已由 `d333eb30dc8a21b4e9b9509bc08a3760223c091a` 修复，`#45631521531` 和最新 run 均通过。此结果只覆盖 host/cross-build、overlay、storage/VFS 和 ELF/link probes，不是完整 LK 固件或实机验证。artifact `15038716689`。
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
- [x] 新增 RG34XX 纯 C 显示刷新策略、UI preference / GAME 120-Hz opt-in 与 frame-period helper；该模块只请求策略，不连接 Frontend、不等待 VBlank、不写硬件。 原始 TheGammaSqueeze boot-package 直接审查已确认：两套签名 boot_package 以 54MHz / HT840 (120Hz) 与 54MHz / HT1679 (60Hz) 的 sunxi.fex timing 区分；刷写脚本直接写 raw mmc offset 0x1004000 并重启，README 仅声明 Stock firmware 测试。与 pvaibhav/slot 的 userspace PLL/TCON writer 是不同路径；二者都不提供 UI/GAME 场景自动切换，也都不能直接移植到当前 rg34xx_v1 + lcd_type=old BaseOS。按 kernel #19 反汇编推导，54MHz 若进入 Linux panel-init 会走 lcd_if=0 默认 divider6、PLL 请求324MHz（精确表项），不同于当前26MHz特例 divider11/288MHz；未在 BaseOS 刷包实测。Nominal 54MHz timing arithmetic 与 README reported rate 有 <0.011Hz 差异，专项文档已标注。live FDT timing/interface 已实机重解析为 `rg34xx_v1` / RGB666 / 720×480 / 26-MHz DCLK property / HT×VT 820×536 / `lcd_type=old`；TCON/PLL 只读回读为 `0xf000000b` / `0x88001703`。按 288-MHz 直接馈入 divider 11 推算当前 DCLK 26.1818 MHz、scanout 59.569 Hz，与 IRQ proxy 约 59.6 Hz 一致；只闭合当前 60-Hz 基线，不验证 120 Hz。pvaibhav 120-Hz 候选 pair 为 divider `0xf000000a` + PLL raw `0x88002b03`，但其 gate 要求 `fog_fj035fhd05_v1`，不匹配 live `rg34xx_v1`；ROCKNIX firmware 另有 52-MHz / 813×533 120-Hz reference，但 totals 也不等于 live 820×536。外部 `rgxx_boot_package_mods` 的 54-MHz / HT840 / VT536 与 54-MHz / HT1679 profiles 继续作为参考候选，不作最终板型 profile。无持久化、无真实 TCON/CCU/PLL writes，`hardware_applied=0`。已有 policy/period arithmetic/host tests 与三 ISA freestanding compile 分别由 #45569849692 / #45572079415 验证；inventory collector host/cross compile 与 Windows safety 由 #45573555763 / #45573555742 验证。
- [x] 完成 RG34XX non-SP 的 TCON0 DCLK divider / PLL **只读**实机回读（2026-11-25 19:59 +08）：`0x06511044=0xf000000b`（divider 11）、`0x03001040=0x88001703`；与 pvaibhav/slot 的 60-Hz register pair 完全相同。结合 `tcon_lcd=288 MHz`，若该 clock 直接进入 divider，则 DCLK 推算 26.1818 MHz、820×536 下 59.569 Hz，与 IRQ proxy 约 59.6 Hz 吻合。**只确认当前 60-Hz 状态；没有写寄存器，也没有验证 120 Hz。** 详见 [readback report](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/pvaibhav-slot-runtime/tcon-readback)。
- [x] 对与 Build ID 匹配的 Linux 4.9.170 #19 kernel 做离线恢复/反汇编（2026-11-25 完成）：panel init、PLL 位域、时钟父链与最终 DCLK 已闭合（见 [ADDENDUM](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/kernel19-display-reverse-audit/ADDENDUM.md)）；PLL 因子表已逐行导出 187 行并恢复选频算法，286 MHz → 288 MHz（`0x1702`），572→576 MHz（`0x1700`）仅是外部强制 divider=11 的反事实；stock `lcd_dclk_freq=52` 会选 divider 6、请求 312 MHz（DCLK 52 MHz / 118.31 Hz）；另有强制 divider=10 / 528-MHz 候选（见 [PLL-TABLE](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/kernel19-display-reverse-audit/PLL-TABLE.md)）。pvaibhav/slot 的 120-Hz pair 仅作候选对照，不直接写入。
- [ ] 分别实现/验证 UI present FPS 与 armGBA committed-frame FPS 的只读测量。2026-11-26 ADB 实测 UI/GAME IRQ293 为 59.667/59.583 Hz，PLL raw 两态均为 `0x88001703`；这是 scanout proxy，不是 UI/game FPS。UI `presents/s` tracing 需在 Slot 进程启动前置位（OnceLock 缓存），先核对 stderr 去向和安装二进制 SHA。armGBA 的 `gba` 全局被 `libretro/link.T` 的 `local: *` 隐藏，不能依赖 `dlsym`/`nm -D`；先匹配 core release、检查完整 ELF symtab，并用 pinned source 编译 `offsetof` 探针。未闭合符号/load-bias/指针/结构偏移前，禁止读 `/proc/PID/mem`。LCD IRQ 不得冒充任一 FPS；GAME 120 Hz 默认关闭，所有硬件 writer 保持关闭。

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
- [x] 为 RG34XX H700 提供 LK 必需的 `target_early_init()` / `target_init()` 两个 target hooks，替代被 overlay 排除的 MTK generic target init 文件；early hook 点亮 PI12，post-platform hook 安全重试；heap readiness marker 仅在 normal 已初始化 heap 的启动路径设置。扩展 guard 和 AArch32/AArch64 compile probes 覆盖该对象。
- [x] 将 runtime/lk_heap_service.o 加入 H700 LK OBJS，闭合 target_init() 对 lifecycle marker 的实际链接依赖；对固定 GammaOS LK kernel/main.c 启动顺序、heap_get_info() 临界区保护和 heap free-list 初始化增加 source-pinned guard。host regression 同时验证 normal bootstrap 与 ENABLE_NANDWRITE（该路径不初始化 heap，必须保持 unavailable）。此项只验证 heap readiness 服务，不批准 heap 物理范围或内存预算。
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



### 2026-11-26 original RG34XX boot-package vs pvaibhav/slot comparison

The original [TheGammaSqueeze RG34XX branch](https://github.com/TheGammaSqueeze/rgxx_boot_package_mods/tree/rg34xx) was inspected at HEAD `64300b3b6137f7676b63b2e969e58db2870db48b`. It builds a signed Allwinner boot package containing U-Boot, monitor, DTBO and the selected sunxi.fex. The 60/120 profiles both set DCLK=54 MHz and VT=536, changing HT 1679→840; the resulting package is flashed to raw mmc offset `0x1004000` and the device reboots. README says stock-firmware testing only. This is a global boot configuration, not a userspace/Slot option or scene-aware mode switch.

pvaibhav/slot instead retains the live 26-MHz / 820×536 tuple and attempts runtime divider/PLL writes during app startup and resume, behind an exact DT panel gate plus register readback and rollback. Its high pair (divider 10 / PLL 528 MHz) is not equivalent to the original package's 54-MHz / HT840 mode. The original package's FDT panel label is fog_fj035fhd05_v1 while our live Linux FDT says rg34xx_v1; neither external recipe is approved for this exact panel/revision. Keep boot-package writes, sunxi_dump writes and HAL writer disabled. See [comparison review](slot-pvaibhav-120hz-review.md).

### pvaibhav/slot live-device result (2026-11-25)

A same-device UI/GAME capture has been completed on the RG34XX non-SP with the multi-frontend BaseOS launch. Both phases remained at approximately 59.6 Hz by display IRQ counters (UI 59.614–59.648 Hz; GAME 59.632 Hz). The later live-FDT reparse confirms `rg34xx_v1` / RGB666 / 720×480 / 26-MHz DCLK property / 820×536 totals / `lcd_type=old`; same-boot TCON/PLL readback then confirmed divider word `0xf000000b` and PLL raw word `0x88001703`. Using the pinned sun50iw9 reference formula, the current PLL raw word decodes to a 288-MHz `pll_video0x4` rate; 288/11 gives an estimated DCLK of 26.1818 MHz and 59.569 Hz, agreeing with the IRQ proxy within about 0.1%. This decode is reference-source analysis, not yet verified against exact running kernel #19. `slot.state` has `high_refresh=1`, but no 120-Hz hardware application was observed. The running process is `/mnt/sdcard/System/slot`, GAME maps `armgba_libretro.so`, and UI presents / armGBA committed FPS remain unmeasured.

The live FDT is re-parsed and published with byte-matched DTB/JSON hashes. It confirms `rg34xx_v1`, RGB666, 720×480, DCLK property 26 MHz, HT/VT 820×536, H porches 46/2/52, V porches 20/5/31, and bootarg `lcd_type=old`; static DTB calculation is 59.155 Hz. The later same-boot TCON/PLL readback gives divider 11 and PLL raw `0x88001703`; using the pinned public sun50iw9 formula, the baseline PLL decodes to 288 MHz, leading to estimated DCLK 26.1818 MHz and 59.569 Hz. Exact PLL formula/clock parent still needs confirmation against kernel #19; readback is read-only. The pvaibhav fork's gate requires `fog_fj035fhd05_v1`, so it does not match `rg34xx_v1`. Do not weaken the panel allowlist or copy its register values. Hardware writes remain disabled.


The vendor U-Boot survey now resolves an additional identity nuance: RG34XX selects the `rg34xx_v1` panel struct, but that struct calls a common U-Boot init routine whose symbol name includes `fj035fhd05`. The routine label is not a replacement for the live DT driver name. The survey maps PE6 high → `lcd_type=old`, low → `boe`, matching the live `old` bootarg. Use target + DT driver + `lcd_type` + complete timing as the display-profile identity; do not equate the FJ035 init routine with support for pvaibhav's FJ035 Linux writer.


The [related RG34XX-H divider-mismatch report](https://community.muos.dev/t/rg34xx-h-incorrect-display-refresh-rate/1825) is retained as a caution, not a transferable profile: a different unit reported ~106.6 Hz from nominal ~60 Hz with a related fixed /6 TCON path, and a later DTS revision changed DCLK/HT to 54 MHz/1679. Another unit could not reproduce the symptom. The RG34XX non-SP's current DCLK divider is already read back as 11, matching the pvaibhav 60-Hz word. Resolve the exact kernel #19 PLL fields, runtime parent mux and panel init using the Build-ID-matched stock p4 ELF before considering any 120-Hz candidate. Never apply the MustardOS timing to this device by analogy.


The kernel #19 display audit now has a two-round offline reverse-engineering result: [first-round report](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/kernel19-display-reverse-audit) and [ADDENDUM](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/kernel19-display-reverse-audit/ADDENDUM.md). Using an in-memory overlay of 91,760 unapplied `.rela.dyn` entries, the second round recovered the panel/config pointers and the 17-entry panel callback table. `panel_info+0x80` is `lcd_dclk_freq`; the divider table is indexed by `lcd_if`, and baseline `lcd_if=0` with `lcd_dclk_freq=26 MHz` selects divider 11. The TCON divider word is composed from `tcon0_open`'s `0xf0000000` upper bits and `tcon0_set_dclk_div`'s low 7 bits. The current live chain remains 288 MHz / 11 → 26.1818 MHz → 59.569 Hz at 820×536, consistent with the ~59.6-Hz scanout IRQ proxy (not game FPS or independently measured panel refresh). Linux captures the existing clock parent with `clk_get_parent` and re-applies that same parent during enable; it then calls `clk_set_rate` from `lcd_dclk_freq × divider`. The third round exported every PLL factor row and closed that summary conflict: 187 rows, 288 to 2520 MHz, uniform 12-MHz step, every row satisfying `24 MHz x (N+1)/(D2+1)` with zero deviation. `0x6400` is interior row 178 (2424 MHz); the real last row is `0x6800` (2520 MHz). Rate selection is clamp + nearest neighbour with ties going low, not floor and not ceil: `pllvideo0x4_max` is seeded from the last table rate (2.52 GHz) by `sunxi_clk_factor_initlimits`, the request is truncated to whole MHz, and a request below the first row maps to 288 MHz. So 286 MHz selects row 0 (`0x1702`). The 572→576 MHz result assumes an externally forced divider 11; the stock path with `lcd_dclk_freq=52` selects divider 6 and requests 312 MHz (52-MHz DCLK / 118.31 Hz). A separate forced divider-10 / 528-MHz candidate calculates to ~120.13 Hz, but neither forced-divider case proves panel support. See [PLL-TABLE.md](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/kernel19-display-reverse-audit/PLL-TABLE.md). Panel timing, accepted 120-Hz divider, panel tolerance, TE/VBlank boundary handling and rollback are still absent, so **120 Hz remains unverified and must not be enabled**. Keep all hardware writers disabled. After the exact factor table is reconciled, the display work returns to separate UI present and armGBA committed-frame accounting; do not use LCD IRQ as either FPS metric.


## Linkslot 1.0.49 / 1.1.0 三分支更新（2026-11-25）

上游三分支已更新并归档：main 1.0.49、test 1.0.49、DEV 1.1.0。main/test 的 851 个源码文件一致，作为稳定移植基线只适配一次；DEV 相对 main 有 21 个新增、1 个删除、46 个修改，新增 schema 2 平台扩展包、Arcade/NeoGeo 机柜、FBNeo 与 DAT 数据支持。

路线调整：

1. 先把此前 1.0.45 的已验证 H700 no_std 业务切片、renderer overlay 和 mGBA 静态链接探针迁移到 main 1.0.49。Libretro FFI blob 未变是兼容依据，但不能代替完整编译。
2. 更新并回归 H700 持久化核心元数据表，保留 no_std/alloc 分离；重点检查 armGBA 元数据与 netpacket 联机语义。
3. 对 `slot/src/app.rs`、`slot-frontend`、`slot-gfx` 的 patcher 逐个重新验证锚点和语义，不能只改 1.0.45 SHA / Cargo version。
4. DEV 1.1.0 作为第二阶段差异审计，重点核查平台枚举的落盘序号兼容、平台包安装/卸载/损坏回退、核心可用性和扫描器边界。Arcade/NeoGeo/FBNeo 不进入首期可启动切片。
5. 上游 Linux/glibc 文件、shell、动态核心加载、网络和 Web 服务不得直接纳入 LK；继续采用 C HAL + C ABI + Rust no_std 业务层 + C/C++ 静态核心。

本次仅完成来源核对与文档同步，不代表新基线已完成 H700 编译、LK 链接或实机验证。

### 2026-11-26 pvaibhav/slot ADB follow-up

Two same-device ADB captures now confirm UI and active armGBA gameplay remain on the approximately-60-Hz scanout path: IRQ293 is 59.667 Hz in UI and 59.583 Hz in game; PLL remains `0x88001703` in both, while the attempted TCON read in this capture returns I/O error. `high_refresh=1` is configuration state, not proof of applied mode. The report's two interpretation errors are explicitly corrected in [reports/slot-refresh-audit/SUMMARY.md §10](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/slot-refresh-audit/SUMMARY.md).

The next display-validation phase is not another IRQ capture: resolve the Slot trace log route for its 600-present `presents/s` counter, then identify the exact installed armGBA ELF before considering a frame-counter read. The pinned source's `gba` variable is local under `libretro/link.T`; a dynamic-symbol lookup is not valid. Use hash-matched ELF evidence and a compiled layout probe; do not scan process memory. Keep `hardware_applied=0` and all display writers disabled.


### Original RG34XX boot-package 120 Hz implementation review (2026-11-26)

Reviewed [TheGammaSqueeze/rgxx_boot_package_mods `rg34xx`](https://github.com/TheGammaSqueeze/rgxx_boot_package_mods/tree/rg34xx) at `64300b3b6137f7676b63b2e969e58db2870db48b`. The two FDT blobs are 137,728 bytes and differ in exactly one property: `lcd_ht=840` (120-Hz package) versus `1679` (60-Hz package); both use panel `fog_fj035fhd05_v1`, `lcd_dclk_freq=54`, `lcd_vt=536`, and 720×480. U-Boot, monitor and p1.dtbo artifacts are identical between modes. The boot package is flashed at raw offset `0x1004000` and rebooted; this is a static per-boot mode, not a runtime switch. README claims Stock-firmware testing only. Detailed artifact/hash and safety audit: [rgxx-boot-package-120hz-review.md](rgxx-boot-package-120hz-review.md).

This differs from pvaibhav/slot's userspace PLL/TCON writer, which expects the 26-MHz / 820×536 baseline and writes its 528-MHz / divider-10 high pair. The original boot package instead selects 54-MHz DCLK and changes HT. Slot's exact timing probe would reject the boot-package FDT. More importantly, both third-party paths use/require `fog_fj035fhd05_v1`, while our live standard RG34XX is `rg34xx_v1 + lcd_type=old`. Do not flash or transplant either path, and do not weaken the panel allowlist. Neither third-party design implements UI-120 / GAME-default-60 automatic scene switching. Keep hardware writers disabled.


### armGBA committed-frame layout probe and binary identity gate (2026-11-26)

The pinned armGBA source `fad6c4b0c0176a76e10667f4aa071a8feb4425c0` increments `gba->video.frameCounter` at VBlank, but the libretro linker script localizes non-`retro_*` symbols, including `gba`. A compile-time layout probe has been added (`tools/armgba-frame-layout-probe.c`, `scripts/test-armgba-frame-layout.py`) and wired into the Linkslot feasibility workflow. Its host, AArch32 ARM, AArch32 Thumb-2 and AArch64 static-assert compile step passed in [#45722327333](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45722327333); that aggregate run was later cancelled by a superseding push after 33/74 steps, but its artifact was uploaded and read. The measured pinned-source layout is `GBA.video=11944`, `Video.frameCounter=4`, combined `gba->video.frameCounter=11948` bytes. Replacement run [#45722659918](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45722659918) was cancelled during LLVM tool installation before the layout step and is not evidence for or against the probe. Numeric source offset is known; installed ELF identity and the runtime `gba` pointer/load-bias are still unverified.

The pinned Slot armGBA build recipe downloads a caller-specified source commit, applies `link-stop-queued-once.patch`, builds with `-mcpu=cortex-a53`, and emits a `.meta` file recording source commit and patch hash. Before any target process-memory reader is added, match the device core SHA to the official H700 release, extract this metadata if packaged, inspect full `.symtab` and `.dynsym`, and establish the in-memory `gba` pointer without blind scans. UI `presents/s` needs its own trace collection and verified stderr path. No marker file, frontend restart or `/proc/PID/mem` access yet.


### Official H700 binary provenance and high-refresh capability gate (2026-11-26)

The new read-only ADB report under [reports/slot-refresh-audit/binary-identity](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/slot-refresh-audit/binary-identity) closes the binary identity: device `System/slot` and `armgba_libretro.so` hashes exactly match the official H700 v1.16.0-pvaibhav.2 ZIP. The packaged `armgba-*.meta` pins the actual core to `MagnominusGit/armGBA@df6b2631291f249d54a00dd14f49bc893e438feb`, built with `-mcpu=cortex-a53` and the recorded link-stop patch SHA. This supersedes the earlier tentative layout pin `fad6c4b0...`; its 11948-byte offset is not valid evidence for this release binary. The layout probe and CI checkout were updated to exact commit `df6b2631291f249d54a00dd14f49bc893e438feb`. Dedicated layout step in [#45724910634](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45724910634) **passed**; artifact ID `15072724723` reports `GBA.video=11944`, `Video.frameCounter=4`, combined offset **11948 bytes**. The aggregate run was later cancelled by a superseding documentation push after the probe had completed; do not call the whole workflow a pass. Exact result: [armgba-frame-counter-layout-v1.16.0.md](armgba-frame-counter-layout-v1.16.0.md).

The reason `high_refresh=1` does not change the live RG34XX is now established from the pinned Slot source and the captured FDT. `DevicePlatform.refresh` is `Option<PanelRefresh>`; for target `rg34xx`, `PanelRefresh::probe` requires exact `lcd_driver_name=fog_fj035fhd05_v1`. Our live device is `rg34xx_v1`, so `refresh=None`; `supports_high_refresh()` is false and `set_high_refresh()` does not invoke the writer. Do not bypass the gate or force third-party timings.

The armGBA ELF is unstripped. Its `.symtab` contains two local objects named `gba` at `0x4c16a8` and `0x4259b8`, while `.dynsym` exports none. The source has a primary global pointer in `libretro.c` and a separate file-local pointer in `libretro/link.c`; disassembly must map references to the correct one before any pointer dereference. Slot PID 2301's stdout/stderr both go to `/mnt/sdcard/Logs/slot.log`, but tracing is off and its marker is absent. Next only gather `/proc/PID/maps` and PT_LOAD mapping facts; do not read process memory, create the marker or restart Slot in this read-only phase.


### 2026-11-26 armGBA runtime mapping closure

The exact official v1.16.0-pvaibhav.2 core now has a verified runtime load bias and primary GBA pointer: three PT_LOAD-backed maps agree on bias `0x7f64070000`; `gba@0x4259b8` is the libretro main pointer, while `gba@0x4c16a8` is the link-cable cache. The exact-source committed-frame field is at +11948 bytes from the primary `struct GBA *`. See [runtime mapping report](armgba-runtime-mapping-v1.16.0.md). No process memory was read and no frame rate was measured yet. Next phase requires explicit permission for Slot trace enable/restart and bounded read-only frame-counter sampling. UI presents, core frames and LCD IRQ remain separate; no refresh writer is enabled.


### 2026-11-26 runtime rates measured: presents / committed frames / IRQ

The earlier gates are now closed with real device data in [fps-sampling](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/slot-refresh-audit/fps-sampling): `slot_presents_per_s` 59.345 mean (4 pace lines), `armgba_committed_frames_per_s` 59.722 (1184 frames / 19.825 s, 79 bounded 12-byte samples, 948 B total), and `lcd_irq_per_s` 59.590 (1203 / 20.188 s). The game ran on the hash-matched official armGBA core with primary pointer `gba@0x4259b8`; per-sample maps and three-line bias checks all agreed on `0x7f55ae0000`. Actual sample rate was 3.934 Hz, not the 5 Hz plan; quantization and interval statistics are in the report.

Marker was created only for the UI trace and removed afterwards; Slot was restarted only through the confirmed PID 1 respawn chain and restored to UI (PID 8064, tracing off, no armgba mapping). A diagnostics helper blocked the first restart and was terminated specifically, with the failure recorded rather than hidden. No refresh writer was enabled: `hardware_applied=0`.

Next display work still cannot claim 120 Hz: panel acceptance, timing margin, TE/VBlank behaviour and rollback remain unverified, and all hardware writers stay disabled. Do not substitute IRQ or UI presents for the core's committed-frame metric.


#### FPS sample audit clarification (2026-11-26)

The complete game launch log adds **31** 600-swap windows during armGBA gameplay (mean **59.561 presents/s**) and 25 pre-game windows (mean **59.503 presents/s**); the README's 59.345 average describes only the first four trace windows. The 79 frame samples independently recompute to 1184 / 19.8252277 s = **59.722 frames/s** at an actual 3.934-Hz sampling cadence; IRQ293 is **59.590/s**. See [independent report analysis](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/slot-refresh-audit/fps-sampling/ANALYSIS.md).

The original sample PID 22514's exact first/last `/proc/PID/maps` snapshots were not recovered. A separate maps-only follow-up used the new Slot PID 8064 and uploaded two complete 425-line snapshots. Its RX/R/RW PT_LOAD rows independently resolve to the same new bias `0x7f339a0000`; this validates the mapping method but does not retroactively prove PID 22514's earlier `0x7f55ae0000` bias. See [MAPS-SUPPLEMENT.md](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/slot-refresh-audit/fps-sampling/MAPS-SUPPLEMENT.md). The 13-file SHA256 manifest has been independently verified. The process exited back to UI with no armGBA mapping, marker absent, trace env unset; no FPS was resampled and no process memory or hardware registers were accessed.



#### 2026-11-26 armGBA maps-only supplement

The report branch now includes `armgba-maps-supplement-first.txt`, `armgba-maps-supplement-last.txt`, and `MAPS-SUPPLEMENT.md`. This is a new-PID 8064 capture, not recovered PID 22514 evidence. Both files contain 425 lines; the four armGBA mapping rows match exactly, and RX/R/RW independently calculate to `0x7f339a0000`. All 13 SHA256 manifest entries pass independent verification. Slot returned to UI, armGBA mappings are zero, trace marker/env remain absent. No frame sampling was repeated, no `/proc/PID/mem` read, and no hardware write occurred. Original FPS measurements remain unchanged; all metrics remain around 60/s, with no 120-Hz evidence. Keep all hardware writers disabled and `hardware_applied=0`.


## RG34XX 120-Hz panel qualification gate (2026-11-26)

The kernel #19 reverse audit and the separate UI/GAME/core frame-rate measurements are complete. Runtime rates remain around 60/s; the current live 60-Hz PLL/TCON pair is `0x88001703` / `0xf000000b`. The next blocker is no longer software timing discovery: it is the exact LCD module identity and its electrical timing specification.

Anbernic's [official RG34XX specifications](https://win.anbernic.com/product/517.html) publish a 3.4-inch IPS 720×480 panel but no module part number or refresh ceiling. Slot maintainer discussion in [issue #147](https://github.com/BrandonKowalski/slot/issues/147) characterizes the 120-Hz mode as a display-driver tweak for panels slightly below 60 Hz and later skips the hack due to stutter reports/complexity. This is contextual evidence only, not a panel datasheet or proof for this exact `rg34xx_v1 + lcd_type=old` board. A separately published [RG34XX-SP hardware bring-up repository](https://github.com/fotlim/android_device_anbernic_rg34xxsp_h700_lineage) identifies `FOG FJ035FHD05` only for the SP model and its `anbernic,rg34xx-sp-panel` compatible; it must not be used as the part-number identification for this non-SP board.

Next gate: identify the exact panel vendor/module/revision and obtain a matching datasheet or written vendor confirmation for pixel clock, HT/VT, porch and refresh/overclock limits. Until then 60 Hz is the only qualified mode. Keep the 120-Hz policy request-only, GAME 120-Hz opt-in default-off, all hardware writers disabled, and `hardware_applied=0`. Do not relax the pvaibhav `fog_fj035fhd05_v1` gate or transplant either its PLL/TCON pair or the raw boot-package modification.


## 2026-11-26 LK runtime floor: persistent event-step integration

The previously implemented `slot-lk-runtime` had not been included in the H700 no_std workspace. It has now been added as the 14th workspace member and its package version aligned to Linkslot 1.0.49. The existing event-step is intentionally narrow: C monotonic time → bounded H700 input drain → upstream gesture actions → application-owned action handler → one app tick, all on the single LK thread and using the same timestamp. It owns no scheduler, heap, device driver, VFS, display, PCM or emulator lifecycle.

The 1.0.49 H700 runtime-slices workflow now checks the full 14-member workspace on host and `armv7a-none-eabi` / `aarch64-unknown-none` without default features. Its first run exposed missing root re-exports for the H700 logical input constants; `slot-input/src/lib.rs` was corrected in `a544492a428996a5bcbb981f114b3b117e001653`. The repaired run [#45739871351](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45739871351) **passed**: both workspace host test modes, both freestanding target checks, upstream input/model regressions, full Linkslot 1.0.49 Frontend build, CPU Performance QuickRow fail-closed test, and C input ABI/queue host + AArch32/AArch64 checks. The three `slot-lk-runtime` unit tests passed in both host test modes. The full feasibility workflow [#45740026647](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45740026647) is still in progress; an earlier aggregate run was cancelled by superseding commits and is not counted.

Next application step: implement an allocation-bounded `ActionHandler`/app router over the already extracted `slot-quick-menu`, `slot-game-search`, `slot-model` and frontend state crates. Keep side effects behind typed C service handles; do not add filesystem calls, raw MMIO, framebuffer ownership or core dynamic loading to Rust. Following that, wire read-only ROM/core access through the existing VFS v3 provider and static-core API. Display/PCM/GPIO and persistence remain independent hardware/memory gates; no target boot or physical hardware access is claimed.


## 2026-11-26 Quick Menu and Search phase app-router integration

The 15th persistent workspace member, `slot-app-router`, now extracts the upstream Linkslot 1.0.49 Quick Menu and SearchResults phase routing. Quick Menu produces typed cursor, setting, open-row, parent and close commands; SearchResults uses `SEARCH_ROWS=7`, circular vertical navigation, seven-item horizontal paging, viewport retention, hit selection, return-to-keyboard and close commands. The caller owns App phase, query, Pinyin composition/candidates, SearchHit, shelf/folder selection and all side effects. This is intentionally not a global key dispatcher: Shelf's MENU opens Quick Menu, Playing's MENU opens the in-game utility menu, and search/overlay contexts have their own priority handlers.

The crate is `no_std`, has no third-party dependencies, and includes **12 tests**. Focused run [#45741150698](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45741150698) **passed** with both host test modes, both no-default freestanding checks, upstream input/model regressions, full Linkslot 1.0.49 Frontend overlay build, CPU Performance QuickRow fail-closed test, and C input ABI/queue checks. Both app-router test groups run in the workspace. This proves host/cross-build correctness only, not device boot.

The search keyboard/Pinyin slice is now extracted and tested. It routes candidate focus, page/selection wrap, commit, UI cursor intents, layout cycle, activate-key, backspace, confirm and close while leaving Cursor geometry, PinyinProvider/dictionary, query and candidates with the App. The 15-member run [#45741616363](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45741616363) **passed**, including the seven keyboard-router tests in both host modes. A phase-aware dispatcher `route_app_input(owner, action)` has since been added in commit `8c4efd5de6250d421067e14f7b96df648ad1aaeb`: it preserves Shelf MENU vs Playing MENU and dispatches only the already-resolved QuickMenu/SearchResults/SearchKeyboard owner, otherwise delegating to the existing App handler. It deliberately does not resolve modal priority or patch App::apply itself.


### SearchKeyboard/Pinyin input route and CI (2026-11-26)

The `slot-app-router` now routes QuickMenu, SearchResults and SearchKeyboard input phases with **19 host tests**. The new SearchKeyboard path mirrors upstream 1.0.49 action precedence for Select candidate focus, candidate paging/selection, candidate commit, UI cursor movement intent, layout cycling, key activation, backspace, confirm and close. It receives a small snapshot of candidate metadata only; App retains query/IME composition, PinyinProvider, candidate text, keyboard Cursor geometry and all side effects.

Focused workflow [#45741616363](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45741616363) passed the 15-member workspace, both host test modes, both freestanding targets, complete Linkslot Frontend overlay and C input ABI/queue checks. The pure routes are now connected through the isolated App::apply overlay; keep VFS and device I/O in the C-owned platform services.


### Phase-aware App input dispatcher and focused CI (2026-11-26)

A phase-aware dispatcher route_app_input(owner, action) composes the three phase routers. Shelf MENU opens QuickMenu; Playing MENU opens the in-game menu; Shelf X opens GameSearch; already-resolved QuickMenu/SearchResults/SearchKeyboard owners are passed to their typed routers; other owners delegate to App. scripts/apply-linkslot-app-router.py now applies the adapter to actual Linkslot 1.0.49 App::apply() in an isolated pinned checkout; it preserves power/upgrade/intro/sync, search/keyboard, popup/utility and game-menu precedence, and keeps Shelf CorePicker ahead of routing.

The dispatcher and four new tests are in commit `8c4efd5de6250d421067e14f7b96df648ad1aaeb`. Focused 15-member runtime-slices run [#45742086671](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45742086671) **passed**: both host test modes, AArch32/AArch64 no-default checks, full Frontend overlay, QuickRow fail-closed regression, upstream input/model and C input ABI/queue checks. Total app-router tests: **23**.

App::apply integration is complete as a CI source overlay, not a persistent change to the immutable upstream/linkslot tree. The static guard and full Frontend build passed in runtime-slices #45745328247; full feasibility #45746788491 passed 75/75 steps. The generic H700RuntimeServicesV1 / H700RuntimeSession::bootstrap() boundary is now implemented and tested: validate time/input/VFS descriptors, build event loop, install the process-lifetime VFS backend, then open the static core. Runtime-slices #45748496564 and full feasibility #45748496582 passed, including C ABI layout checks on AArch32/AArch64 and the full mGBA/VFS link chain. A source-pinned LK heap readiness bridge is now object-linked and guarded for the normal and NAND-write startup paths; this is not a heap budget approval. Next, implement C-owned H700 monotonic-time and GPIO/ADC input providers using device-specific evidence, then connect the coordinator to LK main only after core-arena/stack/BSS/display/PCM/secondary-core evidence is approved. No target SD/MMC access or ROM execution has been performed.


### H700 runtime service bootstrap (2026-11-26)

The C/Rust ABI v1 aggregate (runtime_services.h, vfs_service.h) and no_std H700RuntimeSession::bootstrap() are now committed. The coordinator guarantees validation and event-loop construction before VFS install, and VFS install before the generic static-core opener. It rejects invalid ABI/version/size or missing callbacks, fails closed on VFS install failure, and leaves process-lifetime VFS installed when core open fails. Five startup/lifecycle tests and AArch32/AArch64 ABI assertions passed in runtime-slices #45748496564; full feasibility #45748496582 passed 75/75 steps.

The RG34XX live FDT and Linux evdev input map are now corroborated by a complete gpiolib debugfs level capture: all 17 physical game keys caused one clean active-low press/release on their expected line. The pinmux/pinconf debug views still disagree and are not permission to rewrite pad configuration. Do not use /dev/mem again on this image (all tested offsets returned EFAULT). The LK PA_DAT/PE_DAT sampler remains isolated and unexecuted on target; the remaining input gate is a reversible, separately gated diagnostic in a bootable LK image, not another Linux userspace probe. The Generic Timer provider still needs actual CNTFRQ/counter-delta execution in LK; do not enable timer IRQ. Do not reuse MTK GPT/GPIO/AUXADC code or guess MMIO. The heap readiness bridge is linked and guarded. Keep VFS read-only. The coordinator is still not called from LK main; no bootable image or device runtime exists.


## 2026-11-26 RG34XX input-controller source audit and live-FDT parser

Two independent RG34XX non-SP device-tree sources (KNULLI boot-package and
MustardOS RG34XX-H) agree on a 17-button GPIO matrix. The six GPIO cells are
decoded using the pinned sun50iw9p1 vendor provider contract; all entries are
active-low in those references. Reference polling differs (20 ms vs 4 ms), so
the LK polling interval remains undecided. Both references mark GPADC and
`keyboard_1350mv` disabled; the PMIC power key is separate.

The read-only FDT inspector now reports PIO controllers, gpio-keys children,
GPIO phandle arguments, analog keypad nodes and PMIC power-key nodes under
`input_devices`. Synthetic parser regression covers the GPIO cell decoder,
active-low flag, EV_ABS directional event, disabled ADC nodes and PMIC key.
The original parser validation was source/host-only; subsequent target reports
have now supplied the active live FDT, Linux evdev path and a full 17-key
gpiolib debugfs readback. The readback establishes the expected Linux GPIO
lines and active-low press/release behavior, while the conflicting
pinmux/pinconf debug labels remain unresolved and are not write authorization.
The LK sampler source exists, but direct PA_DAT/PE_DAT reads in LK have not
been executed.

Remaining evidence: a bootable, reversible LK diagnostic must validate the
BL33-entry PIO state and direct PA_DAT/PE_DAT read path before a scanner is
connected to any normal runtime loop. Do not write PIO mux/pull registers,
request GPIO lines, or add EINT/IRQ handling as part of this readback work.


## 2026-11-26 RG34XX 输入设备 live-FDT 闭合与软件边沿转换层

- [x] 从目标机活动 live FDT 确认 `gpio-keys-polled` 父节点及 17 个按键子节点均为 `okay`，声明轮询周期 20 ms；所有键映射到主 PIO A/E bank，六 cell GPIO tuple 的 mux/pull/drive/active-low 已按 pinned Allwinner provider 解码。
- [x] 核对 pull enum：`pull=1` 是弱上拉，不是无上下拉；GPADC 和 ADC keyboard 禁用；AXP2101-PEK power key 独立。
- [x] 建立只接收 PIO A/E 电平快照的纯软件 map/edge 转换器，向既有 bounded input queue 产生逻辑 press/release edges，不实现 GPIO 寄存器读写或硬件扫描。
- [x] Host regression 通过；AArch64、AArch32 ARM-state、Thumb-2 编译通过。
- [x] 实机只读采集活动 live FDT、`/proc/bus/input/devices` 与原始 event1；设备缺少 `getevent`，使用已验证的 event1 char-device 原始记录离线解码。pinmux/pinconf 状态作为观察项保留，不写配置。
- [x] 通过已挂载的 `/sys/kernel/debug/gpio` 对 17 个实体键进行 1,200 次/126 秒只读采样；17/17 条线各出现唯一 active-low press/release，未发现其他 GPIO 变化。
- [x] 实现只读 PIO bank single-shot sampler（PA_DAT 0x10 / PE_DAT 0xA0）及 mock/cross-build 验证；该代码没有 write callback，也尚未执行于 LK 实机。
- [ ] 待获得可启动 LK 镜像和可逆诊断流程后，在 BL33 early-boot 状态下验证直接 PA_DAT/PE_DAT readback；在此之前不接入 normal runtime。
- [ ] 根据 direct-LK 采样结果及更高频/抖动证据，再决定 20 ms polling 与 debounce；当前约 105 ms 的 debugfs capture 不足以量化接点 bounce。

当前安全边界：FDT、Linux evdev 与 Linux gpiolib 电平读回已确认 17 键实体映射和 active-low press/release；**LK 直接 PIO MMIO、早期 pin ownership、调度周期及去抖仍未验证**。禁止把静态 FDT 或 Linux driver readback 当作 LK 初始化/寄存器路径已经通过。


## 2026-11-26 RG34XX 实机输入链路与只读 PIO sampler

- [x] 活动 live FDT 的 17 个 GPIO 输入与 /proc/bus/input/devices 的 ANBERNIC-keys/event1 对上；72 条原始 evdev 记录覆盖 17 个实体键（含一次人工误触 R1）。设备无 getevent，改用只读 event char-device 原始采集并离线解码。
- [x] X/Y 实体标签与 DTS 节点交叉：实体 X=PA2/code307/keyY；实体 Y=PA3/code306/keyX。X/Y 专项重复按压 6/6 一致；软件逻辑映射已修正。
- [x] 记录 pinmux/pinconf 的不一致（PA bank MUX UNCLAIMED、部分 PE 显示 CSI1、pinconf 均 bias disabled）；按键实际工作，不把调试视图当成写寄存器授权。未知 dierct-keys-polled/event2 独立隔离。
- [x] 新增 read-only single-shot PIO sampler：只读取 PA_DAT 0x10 / PE_DAT 0xA0（base 0x0300b000），无 write callback，不改 mux/pull/drive，不启用 EINT/GIC。
- [x] Mock-PIO host tests 覆盖寄存器地址、空闲态、实体 X/Y press/release 顺序、ABI 错误拒绝；H700 编译/storage host regressions [#45761004754](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45761004754) 与 boot audit [#45761004741](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45761004741) 均通过。
- [ ] 在目标设备上通过受控诊断路径实测 PA_DAT/PE_DAT 只读快照与按键同步关系；在此之前 sampler 不接入 LK runtime。
- [ ] 评估 20 ms cadence 与去抖；当前 debugfs 约 105 ms 采样只证明稳定 press/release，不足以测量接点 bounce 或子 100 ms 边沿；LK 直接 PIO 高频采样与输入调度尚未验证。

当前边界：Linux evdev 输入链路已实测；PIO 只读采样仅完成 host/mock 和 cross-build，**没有目标机 MMIO 执行、没有 LK 输入循环、没有 LK 启动/游戏运行验证**。


### RG34XX input hardware readback gate (2026-11-26)

- [x] Linux runtime evidence confirms all 17 GPIO gamepad keys on `ANBERNIC-keys/event1`; physical X/Y labels are independently confirmed and the H700 logical map is corrected.
- [x] Reports-side `/proc/iomem` confirms the PIO resource range `0300b000-0300b3ff` at `/soc@03000000/pinctrl@0300b000`.
- [x] Add a separate bounded `/dev/mem` **read-only** collector for exactly PA_DAT `0x0300b010` and PE_DAT `0x0300b0a0`; gate it on root, readable `/dev/mem`, and exact `/proc/iomem` resource identity. Static safety tests and shell syntax passed on Linux + Windows [#45762940878](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45762940878), 89 tests per platform.
- [x] Run the bounded `/dev/mem` collector on the RG34XX; the attempt failed closed at the first read with EFAULT, and the userspace path has been retired for this image.
- [x] Use the already-mounted gpiolib debugfs view as a read-only alternative. The complete 1,200-snapshot report confirms 17/17 physical keys and 34 exact transitions; see the [device report](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-gpio-debugfs-20261126b/GPIO-DEBUGFS-TEST.md).
- [ ] Compare direct LK PA_DAT/PE_DAT edges with the Linux gpiolib/evdev evidence through a separately gated target diagnostic; only then select an LK polling cadence/debounce policy.
- [ ] Do not connect the scanner to LK main loop until direct-LK readback is complete and repeatable. No pinmux/pull/drive writes, EINT/GIC setup or changes to the GPIO kernel owner are in scope.

Linux GPIO driver readback is now device-executed and repeatable for this one-at-a-time test. The raw LK PIO sampler remains host/mock and cross-build validated, not device executed. This is not an LK runtime bring-up result.


### 2026-11-26 PIO readback: /dev/mem EFAULT and gpiolib debugfs fallback

- [x] Execute the bounded PA_DAT/PE_DAT collector on the RG34XX as root. Preconditions passed (/dev/mem character node readable; exact PIO resource present), but the first 4-byte read returned EFAULT at sample 0.
- [x] Verify the refusal is offset-independent: offset 0, 4, 0x03000000, PIO base, PA_DAT, PE_DAT, one-byte read and streaming cat /dev/mem all return EFAULT. Device config has CONFIG_DEVMEM=y, CONFIG_STRICT_DEVMEM unset, and selinux=0; no workaround was attempted. No PA/PE values or key PIO samples were obtained. See the [device failure report](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-pio-input-capture-20261126/PIO-CAPTURE-FAILURE.md).
- [x] Full Linkslot/LK host and cross-build feasibility at the preceding work-branch HEAD be8523e08052 completed 75/75 steps [#45763055313](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45763055313). This remains host/cross-build only, not a booted LK runtime.
- [x] Verify the generic Linux v4.9 gpiolib debugfs implementation: its default /sys/kernel/debug/gpio view reports current chip->get() values for requested lines as in hi/lo; a vendor gpio_chip.dbg_show callback can override that output, so the device format must be inspected rather than assumed. References: [gpiolib.c v4.9](https://github.com/torvalds/linux/blob/v4.9/drivers/gpio/gpiolib.c) and [GPIO legacy documentation v4.9](https://github.com/torvalds/linux/blob/v4.9/Documentation/gpio/gpio-legacy.txt).
- [x] Extend ordinary read-only inventory to capture the already-mounted /sys/kernel/debug/gpio file when readable; do not mount debugfs.
- [x] Add scripts/collect-rg34xx-gpio-debugfs-readonly.sh: bounded timed snapshots of that file, timestamps from /proc/uptime, no /dev/mem, no gpio export/request, no sysfs/debugfs writes. Linux and Windows host jobs both pass [#45765497473](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45765497473): 94 Python tests per platform plus shell syntax validation.
- [x] Run the new collector on RG34XX. All 17 requested lines are present in the existing gpiolib debugfs dump and each physical key produced exactly one `hi→lo→hi` sequence across 1,200 samples / 126.0 seconds; no non-key line changed. The device report is [GPIO-DEBUGFS-TEST.md](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-gpio-debugfs-20261126b/GPIO-DEBUGFS-TEST.md).
- [x] Independently parse the complete 2,982,090-byte sample file: 1,200 sample blocks, 34 single-line transition points, no sample with multiple GPIO changes, and no unexpected non-key GPIO transitions.
- [ ] Once a target LK image exists and the memory/boot safety gates are approved, create a separately gated and reversible LK diagnostic using the existing PIO Device mapping and `h700_rg34xx_input_scan_once()`; keep it outside normal boot/runtime. This is required to verify direct LK MMIO, not to re-prove Linux GPIO behavior.
- [ ] Do not retry `/dev/mem` with other offsets, use `/dev/kmem`, `sunxi_dump`, GPIO sysfs export, a custom kernel module or pinctrl debug write. No pinmux/pull/drive/EINT/GIC changes are authorized by this readback task.


## 2026-11-26 Pinned LK scheduler timer source audit

A direct audit of the immutable GammaOS LK pin
`9bf1419a49e0af6dba8b05916df382c9c9c6d0a8` found a prerequisite earlier
than the Linkslot runtime bootstrap:

- `kernel/main.c` initializes heap, threads and DPC, then calls
  `timer_init()`; only afterward does its bootstrap thread call
  `platform_init()`, `target_init()` and `apps_init()`.
- `kernel/timer.c::timer_init()` unconditionally registers a periodic
  `platform_set_periodic_timer()` callback (10 ms on the ordinary build).
  Software timer scheduling reads `current_time()`.
- The existing H700 Generic Timer component is a read-only ABI service only.
  It is not wired to LK's platform timer contract and does not provide the
  scheduler tick. The MT6785 GPT5 timer implementation is explicitly not
  portable to H700.
- The heap readiness bridge is later in the lifecycle (`target_init()`),
  so it cannot gate the timer initialization point.
- This is not a successful LK target link: the memory ownership approval
  gate remains closed and no boot image/device execution has occurred.

The source audit also confirms that LK calls `platform_early_init()`
before heap/thread/timer startup. The pinned generic weak fallback is empty;
MTK's override initializes GIC before its GPT4 early timebase. MTK's later
scheduler tick uses GPT5 and its MTK IRQ path. The pinned MTK GICv2 driver
depends on MTK register bases/helpers and is excluded by the H700 overlay.
The checked-in Linux snapshot shows `GIC-0 27 arch_timer` and
`wakeupgen 48 sun4i_timer0`, but those Linux runtime mappings do not prove
BL33 timer-PPI ownership. The uploaded FDT summary omits interrupt-controller
nodes and the original live DTB is not in the repository. A CI source guard,
`scripts/audit-h700-lk-timer-prerequisites.py`, now checks this upstream
contract without reading or writing hardware registers.

The device-tree audit tooling has also been extended: the FDT inspector
now preserves interrupt-controller compatible/register ranges, phandles,
interrupt-cell counts, inherited interrupt parents, and raw timer interrupt
specifiers (including `interrupts-extended`). Linux and Windows parser tests
pass. The previously uploaded inventory contains only `live-fdt.json`, not
the original `live-fdt.dtb`, so the newly exposed fields still require one
fresh read-only device inventory. Re-run the Windows ADB inventory wrapper,
upload both raw DTB and parsed JSON to reports, then review the actual GIC
and timer declarations. Do not interpret Linux IRQ numbers as BL33 PPI
numbers or configure any interrupt registers from the DT alone.

**Next required work:** qualify the BL31→BL33 architected-counter access and
actual RG34XX interrupt-controller identity/register layout, timer-counter access and timer-PPI handoff. Only then implement
the H700 LK `current_time()`, periodic-timer and delay platform hooks. Do
not add a no-op timer, guess a PPI, or configure timer/GIC registers before
the handoff is understood. In parallel, the RG34XX PA_DAT/PE_DAT scanner
remains a separately gated, read-only LK diagnostic and is not part of the
normal input loop yet.


## 2026-11-26 Fresh RG34XX live FDT: GICv2 and timer mapping closed

The new read-only inventory is uploaded at reports/rg34xx-inventory-20261126-timer/. The redacted live DTB is 143,936 bytes / 573 nodes, compatible allwinner,h616 + arm,sun50iw9p1, SHA-256 620b36d84fb7e714dcb56891f50802048d83c64ca924222a4a566f340ac7242e.

The main controller /interrupt-controller@03020000 is ARM Cortex-A15/A9 GICv2-compatible, with GICD 0x03021000/0x1000, GICC 0x03022000/0x2000, GICH 0x03024000/0x2000 and GICV 0x03026000/0x2000. /timer_arch maps secure physical PPI13 → INTID29, non-secure physical PPI14 → INTID30, virtual PPI11 → INTID27, and EL2 physical PPI10 → INTID26. Flags 0xf08 encode CPU mask 0xF and active-low level-sensitive. Linux GIC-0 27 Level arch_timer is the virtual-timer INTID27 and is active on all four CPUs. A distinct Allwinner sun4i-a10-timer at 0x03009000/0x400 uses sunxi-wakeupgen input48, matching Linux wakeupgen 48 sun4i_timer0.

These live-DT/Linux-runtime mappings close the hardware identity and PPI declaration question, not BL31→LK ownership. Pinned LK's ARM IRQ vector calls platform_irq() and schedules when it returns nonzero. H700 still lacks platform_irq() and a GICv2 provider; the pinned GIC implementation is MTK-specific and excluded.

**Next gate:** run a non-invasive LK-target probe to record exception/security mode, CNTFRQ, two CNTVCT samples, GICD/GICC/GICH/GICV identification and control state, Group0/Group1NS ownership, EL2 counter/timer trap state, CNTVOFF, and whether PPI27 is accessible to BL33. Do not write GIC/timer registers or start an interrupt before readback establishes the handoff. Only then implement H700 early GICv2 setup, platform_irq() IAR/EOI, LK time/delay hooks and scheduler tick. Initial IRQ bring-up stays single-core and must not reassign security groups or clear unrelated pending interrupts.

The full feasibility CI at 8e7f8db completed 76/76; it tests host/cross-build contracts and does not constitute a full LK target link or device boot.


## 2026-11-26 A32 BL33 first-entry state snapshot

- [x] Add an A32 CRT0 capture at the first instruction of lk_bl33_entry: CPSR/SP/LR are copied to shared callee-saved r5/r6/r7 and original r4 remains in r4, before VBAR setup or startup mode/stack changes. This avoids FIQ-banked r8-r14.
- [x] Preserve the original r4 loader argument; change CRT0's temporary boot-argument and optional TrustZone symbol-reference registers to r0 so the r6/r7 snapshots survive; publish SP/LR/r4/CPSR to a five-word BSS object after the BSS clear, with magic written last.
- [x] Add a C-only snapshot reader with null/unpublished failure results; no CP15, timer, GIC, or MMIO accesses are made by the reader.
- [x] Extend the A32 BL33 overlay linker/layout probe and project guard, add C ABI host regression, and compile the new object in AArch64 / AArch32 ARM-state / Thumb-2 variants.
- [x] H700 AArch64/AArch32 ARM-state/Thumb-2 compile + host regression passed (#45779682341); full Linkslot/LK feasibility passed 76/76 (#45779682226), including A32 BL33 layout, snapshot BSS bounds, CRT0/exception links and H700 memory fail-closed guard; latest boot-audit Linux/Windows passed (#45780174078).
- [ ] After memory ownership, firmware pairing, and a reversible boot diagnostic are approved, execute the LK entry path on RG34XX and read the captured registers. Do not infer Security state from CPSR alone, and do not start GIC/timer programming from this snapshot.

The snapshot only becomes available after CRT0 reaches the post-BSS publication point; it is not a crash logger for earlier faults. Until an authorized device run exists, all entry values remain unknown and the existing SPSR=0x1d3 remains a loader/monitor model, not a measurement of this replacement LK.


## 2026-11-26 RG34XX BL33 memory lifetime source trace

- [x] Cross-check pinned BaseOS native SPL writes: firmware package at 0x42e00000; firmware extents include BL31 at 0x48000000, DTBO/DTB at 0x4a100000/0x4a200000 and A32 loader at 0x4a000000; BOL4 staging at 0x45000000; native record at 0x4a300000; final EL3 reset uses RVBAR=0x48000000.
- [x] Cross-check the later A32 Linux-loader writes: entry stack 0x4a0f0000, translation table 0x4a0f8000..0x4a0fc000, kernel output 0x40080000..0x42000000 (max 0x01f80000), initrd 0x42000000, final FDT 0x44000000.
- [x] Record that the audited pinned native SPL handoff path has no explicit destination in the proposed 0x40080000..0x42000000 application candidate; its subsequent A32 Linux-loader stage does use that candidate region, so a direct replacement BL33 must bypass that loader path.
- [x] Match and statically triage the exact paired BL31 monitor payload. SHA-256 matches both the native firmware extent and TOC1 inventory; a control-flow-aware AArch64 scan decoded 1,849 reachable instructions and 268 address references (188 in the 107,216-byte payload, 80 beyond its file end in the adjacent runtime/page-table window). It recovered TTBR0_EL3=0x4803cb40, TCR_EL3 config source=0x4803c8e0 and per-core TPIDR_EL3 base=0x4803cd40. See [audit #45795120056](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45795120056) and the updated [memory ownership report](h700-bl33-memory-ownership.md).
- [x] Decode BL31 EL3 translation-map calls and verify the exact matched monitor/SPL pair: unconditional requests cover [0x48000000,0x48041000); matched monitor word at file +0x18 is 0x00000000; pinned native SPL patches only +0x24, +0x30 and +0xb0, so the conditional [0x48200000,0x4a000000) and nested 4 MiB calls are skipped for this pair. Raw mapping attributes remain uninterpreted and BootROM/earlier indirect mutation is not fully proved.
- [ ] Complete BL31 runtime lifetime/ownership proof beyond static direct-control-flow references: conditional flag provenance, indirect calls/branches, exception returns, secure-world services, DMA, cache aliases, runtime scratch and secondary-core state. The static scan does not prove the candidate is free.
- [ ] Build a complete reviewed memory map for LK image/BSS/stack/heap/page tables, firmware handoff records, framebuffer/PIO, SD/DMA reservations and the separate Linkslot+mGBA application footprint; verify secondary-core and cache/MMU behavior before any MEMBASE/MEMSIZE assignment.

Safety boundary: 0x40080000..0x42000000 remains candidate-only. Keep the unconditional BL31-associated mapping [0x48000000,0x48041000) excluded; do not count the zero-flag skipped [0x48200000,0x4a000000) request as an active map. The 0x4a000000 endpoint is the native BL33 loader slot. Do not set H700_MEMORY_LAYOUT_APPROVED, reuse the unknown 0x7ec00000..0x7fc00000 reservation, execute a payload, or write Boot0/TOC1/SD as part of this source trace.


## 2026-11-26 Small LK BL33 payload into matched native SPL package

- [x] Confirm active native firmware loader extent from uploaded firmware manifest: p4+40 MiB package; loader offset 0x3e200; exact extent 0x8000 bytes (32 KiB), destination 0x4a000000.
- [x] Add a pinned-source patcher for BaseOS native/build.py, limited to a clean BaseOS-MultiFrontend checkout at cbe253a43f46cf551cc9a6c79a84015bde3686cf. The raw LK binary is constrained to the active 32 KiB extent and A32 entry/vector/header contract.
- [x] Keep native loader extent, offsets and extent table fixed; the patched builder adds the raw payload to generation and persistent source hashes, inserts the paired header, pads exactly to 32 KiB, regenerates firmware FNV/header and then builds the matching native SPL.
- [x] Add host regression for raw image validation, builder patch shape, hash-input ordering and idempotence.
- [x] Patcher host regression and real pinned BaseOS source apply/check/AST-parse/byte-exact restore passed in full feasibility run #45785521254; `git diff --exit-code` and clean `git status` both passed.
- [ ] Produce a compact diagnostic payload with a deterministic proof-of-life signal, then build the matched native SPL/firmware/capsule locally. Keep the artifacts non-installable until the BL31 and memory/recovery gates are reviewed.

Hard size boundary: this first integration path is only for a small diagnostic payload <= 32 KiB. The 5,148-byte CRT0/exception layout probe is not yet such a diagnostic because it has no approved visible signal and uses stubs. It does not make the complete LK or Linkslot/mGBA fit the loader extent.




## 2026-11-26 Matched BL31 PSCI CPU_ON callback

- [x] Audit the exact matched 107,216-byte monitor platform operation table
  at 0x480165f8. The standard TF-A plat_psci_ops_t second slot (+8) is
  pwr_domain_on and points to 0x4800254c.
- [x] Verify registration, global pointer load, slot +8 selection, indirect
  branch, callback call at 0x4800255c, and low-level CPU power-on routine
  0x48002d78. The routine programs a per-core RVBAR and has SoC-specific
  core power/reset sequencing. Exact binary verifier and regression are
  scripts/audit-rg34xx-bl31-psci-cpu-on.py and
  scripts/test-audit-rg34xx-bl31-psci-cpu-on.py.
- [x] Combined CI [#45816600012](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45816600012)
  passed. The matched monitor exposes a PSCI CPU_ON-capable callback.
- [x] Selector parity audit [#45817598323](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45817598323)
  verifies BL31/SPL RVBAR selection. Matching VER_REG low3 values: 0, 2;
  mismatching values: 1, 3, 4, 5, 6, 7. The device value itself remains
  unknown and the mismatch remains a hard gate.
- [ ] Do not equate the presence of a PSCI CPU_ON callback with automatic
  secondary startup during the initial BL31-to-BL33 handoff. The live
  CPU1-CPU3 state before the first BL33 instruction is still unknown.
- [x] Add `scripts/read-h700-rvbar-selector-readonly.py` and host
  regression. The device probe opens `/dev/mem` with `O_RDONLY`, reads
  only `0x03000024`, and outputs only `VER_REG & 7` plus the SPL/BL31
  bank comparison. It does not inspect CPU reset state or write MMIO.
- [ ] Run that probe on the actual H700 unit and record only the JSON output
  in the reports branch. Then read back pre-BL33 CPU reset/status/RVBAR state
  using a trusted, read-only early environment.
- [ ] BL31 selects the alternate RVBAR bank for any nonzero low3, while
  pinned native SPL selects it only for value 2; they agree only for values
  0 and 2 and diverge for 1, 3-7. Do not use PSCI CPU_ON or execute a new
  BL33 until the target value is known and both stages use one identical
  target-verified RVBAR bank-selection rule. H616 documentation identifies
  VER_REG at SYS_CFG+0x24 but leaves bits [7:0] undefined; H616 open-source
  tooling's die selector semantics are family evidence, not proof of the
  H700-specific mapping.
- [ ] Keep diagnostic LK single-core: do not issue PSCI CPU_ON, start a
  secondary, or enable SMP until A53 coherency, per-core GIC/timer setup,
  scheduler state, and cross-core locks are deliberately implemented and
  validated. The current full feasibility work remains host-only.


## 2026-11-26 Full Linkslot 1.0.49 H700 feasibility pass

- [x] Latest full host-only feasibility workflow [#45795119939](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45795119939): 74/74 main steps passed; artifact uploaded (387,922,330 bytes).
- [x] H700 physical DRAM provider is part of the actual H700 `OBJS` list; the full Make project guard parses the corrected rules; AArch64, AArch32 ARM-state and Thumb-2 object checks passed in [#45795119838](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45795119838). Host test reports 1 GiB capacity only; it does not approve use of all memory.
- [x] GammaOS LK H700 CRT0/exception-vector overlay link; BaseOS native builder override apply/restore and header package regressions; no_std renderer/Linkslot frontend and input integrations; read-only sdc0 -> CMD17 -> GPT -> FAT32 -> Libretro VFS path and FAT/exFAT host regressions passed.
- [x] Pinned mGBA AArch32 namespaced static archive, VFS v3 and bare-metal newlib builds; actual VDir adapter test; H700 newlib adapter; real mGBA exports bound to Rust static API and linked in the probe; final AArch32 LK-as-BL33 entry/vector layout probe passed.
- [ ] This is still host/cross-compile/static-link feasibility, not a complete installable native firmware image, not a proof of SMP heap safety, and not a device boot. The 32 KiB diagnostic payload and deterministic visible proof-of-life signal remain unimplemented.
- Build report: `linkslot-lk-rust-feasibility-45795119939`; preserve the linked report/artifact with the CI run. No device, SD, boot media or payload was accessed.


## 2026-11-26 Boot0 / BL31 CPU0-only startup static audit

- [x] Decode the exact 128 KiB primary Boot0 binary (SHA-256
  `bf886995745a0eb9b5030296f9f6163035e164b3a82183716e67ff876319289f`,
  eGON checksum valid). Helper at `0x231e0`, caller at `0x23224`;
  caller gates helper calls to core2/core3 on bit29 of `0x03006248`.
- [x] Confirm helper writes `C0_RST_CTRL &= ~(1 << core)`; under H616
  reference semantics, bit zero asserts reset. Core1 is not passed to helper.
  The native SPL sets the primary RVBAR pair only. The original first-entry
  reachable BL31 scan recovered CPU0-mode/subsystem accesses only; a separate
  full-image/function-table audit below subsequently found the indirect PSCI
  CPU_ON path.
- [x] Add reproducible CI and report:
  [#45808401081](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45808401081),
  all audit steps passed. Details:
  [h700-a53-single-core-startup-audit.md](h700-a53-single-core-startup-audit.md).
- [ ] Device-verify pre-BL33 CPU state. H616 reference POR default
  `C0_RST_CTRL=0x13ff0101` implies CPU0 released and CPU1-3 reset, but H700
  live state and bit29 gate value have not been read. The gate word
  `0x03006248` sits at SID window offset `0x248`; its field is not defined
  by the public H616 register tables. A future approved probe should return
  only `(readl(0x03006248) >> 29) & 1`, not a full SID dump. A Linux register
  read after SMP bring-up is not proof of the pre-BL33 state.
- [ ] Resolve Boot0's unusual OR of core2/core3 one-hot bits into
  `0x09010044` (H616 manual calls this CPU0 `RVBARADDR0_H`) and its write
  to `DBG_REG1.C_DBGPWRUPREQ` (H616 manual marks these bits read-only).
  Do not alter either register until the H700 semantics are identified.
- [ ] Keep SMP disabled and the single-core LK gate open until a trusted
  pre-BL33 read-only register snapshot or equivalent reviewed reset-hold proof
  demonstrates CPU1-3 remain held off throughout LK execution.



### 2026-11-26 — RVBAR selector device-probe result

- [x] Attempt the bounded read-only VER_REG[2:0] collector on the RG34XX.
  It ran as root under the bundled CPython 3.12.16, but /dev/mem read
  0x03000024 returned EFAULT; independent dd reproduced it. The earlier PIO
  probe confirms this kernel's userspace /dev/mem read path fails at
  unrelated offsets too. No JSON or selector value was produced. Raw report:
  [reports/rg34xx-rvbar-selector-20261126](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-rvbar-selector-20261126).
- [x] Mark userspace /dev/mem as a blocked method for this kernel; do not
  retry with devmem, dd, mmap or another wrapper. No register write, module
  load, debugfs mount or boot-argument change was made.
- [ ] Establish a supported kernel-side read path for SYS_CFG, or prepare a
  reviewed read-only early-boot capture. A runtime VER_REG sample can settle
  the selector choice; a pre-BL33 capture is separately required for CPU
  reset/RVBAR handoff state. Keep the selector mismatch unresolved and keep
  the single-core/SMP gate closed until evidence is valid.


## 2026-11-26 continuation — SYS_CFG/RVBAR and LK MMU coverage

- [x] Review the latest `reports` branch kernel-module preflight (`2963c8c4`). The running Linux 4.9.170 image supports modules but contains no matching build tree, `Module.symvers`, or kernel headers; debugfs/regmap exposes no SYS_CFG instance. Do not retry userspace `/dev/mem` and do not build a module against an unverified generic BSP source tree.
- [x] Cross-check the existing GammaOS LK H700 MMU hook: its 1 MiB identity-mapped Device section at `0x03000000` covers both SYS_CFG VER_REG `0x03000024` and RG34XX PIO `0x0300b000`. Add compile-time assertions and host regression checks for this coverage. This is mapping evidence only, not proof of bus/firewall access.
- [ ] Design a single-word, read-only VER_REG probe with an explicit Data Abort recovery path and a minimal result channel. Do not issue a bare volatile read from a boot hook before the abort behavior is contained and reviewed.
- [ ] Keep the separate pre-BL33 CPU reset/RVBAR snapshot gate open. A later LK read of VER_REG cannot establish CPU state at the earlier BL31→BL33 handoff.
- Hardware state remains unchanged: no module load, MMIO write, boot-argument or boot-media modification; `hardware_applied=0`.


### Data Abort recovery gate (pinned GammaOS LK source audit)

- [x] Review the exact GammaOS LK pin: `arm_data_abort` saves the fault frame and branches to `arm_data_abort_handler`; the generic handler calls `exception_die(..., -8, ...)`, reaches `halt()`, and loops. It does not return from the exception.
- [x] Record that the current H700 Device MMU mapping does not make an MMIO read safe from Data Abort or prove BL31/firewall access permission.
- [ ] Design a probe-scoped AArch32 exception-return trampoline that matches only the exact diagnostic load PC, restores the saved frame/SPSR, and resumes at a known recovery label; retain fatal handling for all other aborts.
- [ ] Choose a result channel that does not assume UART, panel, heap, Linux, or filesystem readiness.
- [ ] Add host/source/disassembly guards for the exception frame, PC semantics, return path and fatal fallback before wiring any SYS_CFG or PIO read.


## 2026-11-26 final acceptance — probe-scoped Data Abort recovery

- [x] Add A32 Data Abort vector recovery only for the explicit SYS_CFG read; retain the pinned LK fatal fallback for every non-match.
- [x] Enforce exact matcher: active cookie, SVC/ARM origin, LR_abt = probe load + 8, DFAR = 0x03000024, short-descriptor synchronous read-side DFSR allowlist, and nonzero recovery label.
- [x] Mask IRQ/FIQ before abort-frame save; preserve the original frame pointer in r4; align the temporary Abort C-call stack; restore SPSR/GPR/PC through the positive-match-only `LDM^` trampoline.
- [x] Keep the single-register probe dormant: no startup, input, timer, scheduler, or default runtime call sites.
- [x] H700 AArch64/AArch32 ARM/Thumb-2 compile + host regressions passed: [#45834208274](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45834208274).
- [x] H700 boot-audit Linux + Windows checks passed: [#45834060935](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45834060935).
- [x] Full Linkslot/LK feasibility passed **81/81, 0 skipped**: [#45834208280](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45834208280), artifact [15123653120](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45834208280/artifacts/15123653120).
- [x] Review actual linked A32 test ELF disassembly: A32→Thumb C `BLX`, Thumb→ARM probe `BLX`, probe LDR label/literal address, masked IRQ/FIQ vector entry and positive-only exception return are all visible and linked.
- [ ] Review/build a separately packaged diagnostic within the actual vendor 32 KiB loader extent; prove header/package pairing, memory ownership, installation rollback and proof-of-life before considering device execution.
- [ ] Obtain a trustworthy pre-BL33 CPU reset/RVBAR snapshot. A later Linux/LK VER_REG sample cannot substitute for the pre-BL33 CPU state.
- Hardware remains unchanged: `hardware_applied=0`; no device MMIO read/write, module load, boot-argument or boot-media change, flash, or payload execution.


### 32 KiB native loader raw-file extent regression

- [x] `scripts/test-h700-lk-overlay.py` rejects a minimal A32 overlay raw flat image above `0x8000`; it reports raw-file headroom separately from the `_end` runtime-memory footprint.
- [x] `scripts/test-baseos-native-lk-override.py` accepts exactly 32,768 bytes and rejects 32,769 bytes; the opt-in native builder pads an accepted payload to exactly 32 KiB and regenerates the matched firmware/SPL metadata.
- [x] Full Linkslot/LK feasibility [#45834208280](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45834208280) passed **81/81, zero failures, zero skipped**, artifact [15123653120](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45834208280/artifacts/15123653120) (387,946,289 bytes).
- [x] H700 three-ABI compile/host check [#45834208274](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45834208274) passed; boot-audit Linux/Windows [#45834060935](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45834060935) passed.
- [ ] Build and package an actual visible proof-of-life diagnostic in the 32 KiB extent using the matching vendor header and a newly regenerated native firmware/SPL pair; review BSS/stack ownership and rollback before any device execution.
