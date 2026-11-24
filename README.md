# Linkslot Pico for H700

面向 Anbernic H700 系列掌机的低功耗裸机游戏平台，首台目标设备为 **Anbernic RG34XX（非 SP）**。

本项目以 GammaOS Pico / LK 的裸机运行架构为底层技术参考，以 [linkSlot](https://github.com/luguiyi/linkslot) 作为唯一前端和用户功能来源；不完整移植 GammaOS Pico 的前端，也不把现有 Linux Linkslot 二进制直接搬进 LK。

## 目标语言与运行时架构

**长期目标：以 C 为底层、Rust 为前端业务层、C/C++ 为模拟器核心的裸机系统。** 这是后续设计、代码归属和验收的统一方向：

- **C + ARM 汇编：** GammaOS LK/BL33、启动交接、H700 BSP/HAL、时钟/中断/内存/显示/输入/音频/SD/电源等硬件服务，以及必要的 newlib/Core ABI 适配。硬件寄存器访问、启动入口和板级资源所有权由这一层控制。
- **Rust：** Linkslot 前端业务层及 LK 运行时集成，优先复用上游数据模型、输入状态机、UI 状态转换、游戏库/菜单业务、配置和渲染命令；设备目标采用 no_std，只通过明确的 C ABI/HAL 服务访问底层，不携带 Linux std、线程、进程或动态库假设。
- **C/C++：** GB/GBC/GBA 模拟器核心，以静态链接方式接入统一 Core ABI；核心与 Rust 前端隔离，ROM、输入、视频、音频、即时状态和电池存档通过窄接口传递。
- **Python/Shell：** 仅用于开发机上的源码审计、补丁生成、构建编排、固件/报告检查和 CI；不得进入设备启动链、常驻进程或游戏运行路径。

目标不是把 Linkslot 全量翻译成 C，也不是让 Rust 重新实现硬件驱动。Rust/C 之间的边界必须显式、可测试；模拟器核心保持独立，避免前端或 HAL 依赖某一个核心的内部实现。当前 renderer、VFS 和 mGBA 结果仍属于 host/CI 验证，不代表裸机系统已完成或可启动。

## 项目目标

- 开机直接进入 Linkslot 游戏库，首期只支持 GB / GBC / GBA。
- 本项目只有一个前端，不建设多前端框架或动态前端插件系统。
- 采用不依赖 Linux 待机的运行方式；长按电源键时只保存即时恢复状态并完全关机。
- 电池存档由用户显式保存；云存档同步由用户显式触发。关机不自动写入电池存档，也不触发云同步。
- 保留 Linkslot 的云同步协议、Lua MOD 格式/API、家庭 LAN Web 管理的数据与交互设计。
- 保留 GammaOS Pico 已有的校色、LCD 滤镜、遮罩/显示效果、金手指等能力，并继续扩展 Linkslot 的游戏设置。
- 保留开发调试入口：优先评估 USB ADB；Wi-Fi SSH 作为网络服务完善后的可选通道。不得为了调试常驻高功耗待机。

## 当前开发基线

- **首台目标设备：** Anbernic RG34XX（非 RG34XX SP）。
- **硬件平台：** Allwinner H700，四核 Cortex-A53，Mali-G31 MP2，1 GB LPDDR4，720×480 屏幕，无 eMMC。
- **启动介质：** 只考虑 SD 卡启动；不设计 eMMC 安装或回退到原 Linux/BaseOS 的启动流程。
- **硬件证据：** [RG34XX 硬件基线](docs/rg34xx-hardware-profile.md)，其中区分了 BaseOS 已验证行为、固件分析结果和仍待裸机验证的项目。
- UI / 用户功能来源：[luguiyi/linkslot](https://github.com/luguiyi/linkslot)
- 裸机 / LK 参考：[TheGammaSqueeze/lk-mt6785](https://github.com/TheGammaSqueeze/lk-mt6785)
- 首期平台：GB、GBC、GBA。
- 核心建议：GB/GBC 优先 Gambatte；mGBA 保留为 GB/GBC 可选核心。GBA 优先 mGBA，gpSP 作为备选；armGBA-pico 暂列实验候选，待 mGBA 存档兼容验证后再决定是否进入正式候选。
- Web 管理仅面向家庭 LAN，默认不强制密码；预留简单易用的可选密码/访问令牌设置。服务不得监听 WAN 或公网接口。

## 当前工程进展

- 已建立 RG34XX / Allwinner H700 / Cortex-A53 的 AArch64 freestanding 编译检查及 GitHub Actions。
- 当前检查只生成并验证 C11、C++17 和 AArch64 汇编目标文件，不是可启动 payload；加载地址、入口、EL、栈和 AArch32-to-AArch64 交接仍待启动链审计。
- 已归档用户提供的 linkSlot 1.0.36 源码快照（`upstream/linkslot`，快照 commit `8590d6ff`）；正在审查逐 crate 复用范围和 LK Rust runtime 可行性。该 ZIP 不含原始 Git 历史，不能把快照 commit 当作上游 commit。
- 已完成第一轮 Linkslot Rust 裸机可行性构建：`slot-input` 与 `slot-model` 的纯逻辑临时副本通过 AArch32/AArch64 `no_std + alloc` 交叉编译；输入手势 47 项和纯机种数据 4 项回归测试通过。
- 已用固定 GammaOS LK commit 的 `include/lib/heap.h` 完成 AArch32 `heap_alloc/heap_free` 签名/链接兼容探测；这不是真实 LK heap 实现。LK 参考源码的临界区只证明本核 IRQ 屏蔽，不能当作多核堆锁。
- `slot-i18n` 的 12 份双语 JSON 已通过静态表 no_std 探针：1,763 个 key/语言，原版 18 项测试与候选 13 项测试通过，AArch32/AArch64 编译通过；尚未接入 UI。
- 已完成静态 Libretro API registry 探针：两个前缀化 mock core 在 AArch32/AArch64 生成无未解析符号的 ELF，主机回调/状态/内存往返通过；这还不是 mGBA/gpSP 实际核心构建。
- 已将固定版本真实 mGBA 编译为 AArch32 static Libretro archive（Linux、Generic、VFS v3 glibc 及 VFS v3 arm-none-eabi/newlib 变体）；VFS v3 newlib archive 为 94 个对象、ELF32/ARM EABI5、25/25 个 API 导出前缀化、52 个真正外部依赖；旧 stdio/dirent/Linux descriptor/host config directory 依赖已移除。
- mGBA VFS v3 适配仍是 CI 临时源码改造：Linkslot 的 `slot-retro::environment()` 尚未接入函数表，H700 SD/FAT backend 也未实现。whole-archive 诊断显示 newlib/libm/libgcc 只剩 11 个 syscall hooks；加入 libnosys 后可链接，但只是 dummy syscall stub，不是有效 runtime。真实 mGBA VDir 适配的 host fake-VFS 回归已通过；下一步为 heap/RTC/console 等 hooks 实现 LK/H700 服务。


- A32 LK-as-BL33 的镜像入口/头区/向量布局探针通过，但它使用测试 stub 和合成头，不是完整 LK 固件；LK 参考分支目前为单核调度，A53 编译标志/SMP 启动路径未适配，secondary CPU 状态仍未知。

- 当前启动证据显示厂商 BL31 默认进入 AArch32 SVC BL33（`0x4a000000`，SPSR `0x1d3`）；当前移植路线按 AArch32 GammaOS LK 替换该 BL33 入口推进。LK 完整 target link、可用内存归属、secondary-core 状态和实机 handoff 仍未通过验收。

本地运行 AArch64 编译检查：

```sh
sudo apt install clang llvm
bash scripts/build-h700-objects.sh
```

详细说明见 [H700 AArch64 构建基础](docs/h700-build-scaffold.md)。

## Linkslot 上游同步

项目不要求直接使用原版 Linkslot Linux 发布包。目标是尽量保持上游业务逻辑和数据语义，将 H700/LK 适配放在独立目录，减少每次更新时需要重新移植的范围。

**当前源码基线：** 用户提供的 linkSlot 1.0.36 已归档到本仓库 `upstream/linkslot` 分支。原始输入是无 `.git` 的 ZIP，原始上游 commit/tag 不可得；仓库中的 `8590d6ff` 是快照归档 commit，不是上游源码 commit。当前先完成逐 crate 复用审计和 LK Rust ABI/工具链验证，尚不把整个 workspace 导入 `third_party/linkslot`。

特别注意：Linkslot 当前 `device` feature 仍是 AArch64 Linux/`std` 后端，不是 LK/bare-metal。后续以窄 `no_std` Rust 业务运行时承载前端逻辑，通过 H700 C HAL 和静态 Libretro C ABI 接入设备服务；不得把 Linux `device_app` 当作裸机入口。GammaOS LK 参考分支和当前 BL33 候选为 AArch32，Rust 与模拟器静态核心必须按该 ABI 构建。

具体规则见 [Linkslot 上游同步与移植边界](docs/linkslot-upstream-sync.md) 和 [1.0.36 可移植性审计](docs/linkslot-1.0.36-portability-audit.md)。

## 重要设计文档

- [RG34XX 硬件基线](docs/rg34xx-hardware-profile.md)
- [RG34XX 启动交接审计](docs/rg34xx-boot-handoff-audit.md)
- [RG34XX 首轮实机只读 Inventory](docs/rg34xx-live-inventory-2026-11-22.md)
- [总体架构与硬件边界](docs/architecture.md)
- [已确认的设计决策](docs/decisions.md)
- [模拟器与存档兼容性](docs/emulator-save-compatibility.md)
- [开发路线与阶段验收](docs/roadmap.md)
- [H700 AArch64 构建基础](docs/h700-build-scaffold.md)
- [H700 newlib/LK 运行时适配与堆安全边界](docs/h700-newlib-runtime.md)
- [上游源码基线](docs/upstream-baselines.md)

## 项目状态

目前为设计与硬件可行性验证阶段，尚无可刷写固件。任何构建产物在完成 RG34XX 实机验证之前都不得标记为可用发行版。

## 原则

1. 先验证 RG34XX SD 启动、显示、输入、SD 读写和完全关机路径，再接入完整 Linkslot UI。
2. 数据格式优先兼容 Linkslot；不得无必要改变 Games / Saves / States / Cheats / linkSlot 目录语义。
3. 任何自动存档、同步和关机行为必须遵守 `docs/decisions.md`。
4. 首期目标是 RG34XX / Allwinner H700 / Cortex-A53；不能将 MT6785/MTK 特有驱动或其他 SoC 的寄存器、DMA 参数直接用于 H700。
5. 上游代码和第三方核心必须保留许可证、NOTICE、对应源码及构建来源。
