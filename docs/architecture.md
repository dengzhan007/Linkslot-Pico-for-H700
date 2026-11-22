# 总体架构与硬件边界

## 1. 项目定位

目标是运行在 Anbernic RG34XX / Allwinner H700（四核 Cortex-A53）上的独立裸机游戏环境：启动后直接进入 Linkslot，不启动完整 Linux，也不依赖 suspend/s2idle 保持游戏状态。GammaOS Pico 提供裸机运行时、Core ABI、渲染与输入组织方式的参考；Linkslot 是本项目唯一前端及用户功能的来源。

当前没有多前端需求，不建设前端插件系统、动态前端加载器或通用 Frontend ABI。架构优先保证 Linkslot 上游更新时，H700 适配代码保持独立、上游业务代码尽量少改。

GammaOS Pico 参考树面向 MT6785，不能原样作为 RG34XX 的 SoC/板级实现。RG34XX 使用 Allwinner H700，Boot0/TOC1/vendor monitor/U-Boot 启动链与 MTK 平台不同；MTK 寄存器、CONNSYS、DMA 描述符及电源时序均不能用于 H700。RG34XX 的具体硬件边界见 [RG34XX 硬件基线](rg34xx-hardware-profile.md)。

## 2. 启动方式

目标为 RG34XX / Allwinner H700。当前可确认的原厂 SD 启动链为 Allwinner Boot ROM → eGON.BT0 Boot0 → TOC1 → vendor monitor 与 AArch32 U-Boot → AArch64 Linux。Boot0 负责 DRAM training；AArch32 U-Boot 并不是一个可直接跳转到任意 AArch64 程序的通用入口。BaseOS 的 U-Boot 模拟 trace 中 `0x40080000` 和 `0x44000000` 分别是 Linux Image 与 FDT 的研究地址，不是 Linkslot payload 的入口或已确认可用内存。

BaseOS-MultiFrontend 还包含一条更深入的 H700 原生启动研究路线：AArch64 mainline SPL 负责 SD、PMIC、DRAM 和预加载；保留 vendor monitor；由 monitor 进入 RAM 中的 AArch32 SVC BL33 适配器；适配器再经厂商 SMC 进入 AArch64。这个路线说明 A32 过渡层与 monitor 可能是实现 AArch64 payload 的可行桥接点，但厂商 SMC 的已验证目标是 Linux 启动契约，不等于通用裸机 ABI。具体证据、地址限制和覆盖范围见 [RG34XX 启动交接审计](rg34xx-boot-handoff-audit.md)。现场只读采集命令与 TOC1 解析步骤见 [RG34XX 只读启动链信息采集](rg34xx-readonly-inventory.md)。

**当前优先路径：保留原厂 Boot ROM、Boot0、TOC1、vendor monitor、AArch32 U-Boot、env 和 GPT，只在主机侧研究替换 p4 Android boot image 的 kernel component。** 现场 inventory 已完成；对当前设备实际 U-Boot binary 的反汇编已找到 `0x8000FF04` → `SMC #0` handoff、`images.ep`/FDT 参数流、AArch32 fallback，以及 Android gzip kernel selector。当前 p4 的 `kernel_addr=0x40080000` 已与 U-Boot `bootm` 解压目标和 `images.ep` 对应。证据和未决边界见[原厂 boot.img kernel-slot 路径评估](rg34xx-stock-bootimg-payload-path.md)与[现场 U-Boot handoff 二进制审计](rg34xx-uboot-handoff-binary-audit.md)。

第一份 payload 不建设通用 allocator：使用 AArch64 Image header，入口自行建立只落在该 Image 已装载范围内的私有栈和诊断记录；不访问 MMIO、不启用设备驱动、不分配 Image 之外的 RAM，不执行 Linux 初始化代码。首要验收是确认 monitor ERET 的目标 EL/SPSR、x0..x3、DAIF、`SCTLR_EL1`/MMU/cache 状态，以及 FDT 指针与内容。当前 monitor 的 `SPSR=0x3c5` 等结果来自实际 vendor 指令的仿真，并非 RG34XX 实机寄存器捕获；必须保留此证据边界。

未知 reserved region（特别是 `[0x7ec00000,0x7fc00000)`）、CMA、monitor、framebuffer 和 top-of-RAM 碎片继续排除。它们的 owner 审计**不阻止第一份完全位于 kernel Image 自身范围内的入口探针**，但在归属明确前，绝不建立通用 allocator、SD DMA、显示双缓冲或其他 Image 外部内存用户。若 stock `0x8000FF04` handoff 无法安全进入 freestanding A64 payload，再回退到 A32 BL33 adapter；只有该路线也不可行时才评估自有 A64 SPL。

目标设备无 eMMC，本项目只考虑从 SD 卡启动，不设计 eMMC 安装、双系统选择或自动回退到原 Linux/BaseOS 的产品流程。启动失败后的恢复按项目约定通过电脑重写/修复 SD 卡，或使用经确认的实际 Boot ROM/USB 烧录入口；不把其他设备的自动恢复测试当作 RG34XX 已具备的功能。冷启动到第一帧、进入可操作游戏库的耗时必须实测。

## 3. 分层

| 层 | 职责 | 设计约束 |
|---|---|---|
| Boot handoff | U-Boot / SPL 装载、A32 适配器与 A64 payload 交接 | 只考虑 SD 卡启动；入口契约必须实机验证 |
| H700 HAL | 时钟、计时器、MMU/cache、GPIO、PMIC、背光、显示、音频、SD、USB、Wi-Fi | 所有寄存器与时序按 Allwinner H700 和 RG34XX 具体板型重新确认 |
| Runtime | 任务/线程、锁、事件、内存分配、日志、文件系统封装 | 提供 Linkslot 实际需要的服务；不为兼容 Linux 而建设完整 POSIX 模拟层 |
| Emulator Host | GB/GBC/GBA Core ABI、帧调度、输入、音频、视频、持久化 | 模拟器核心与 Linkslot UI 解耦，核心可替换 |
| Linkslot App | 唯一前端：卡带 UI、游戏库、配置、MOD、云同步入口 | 尽量跟随上游；平台差异经适配层提供，不散落到业务逻辑 |
| LAN Services | HTTP Server、云同步 WebDAV 客户端、可选调试服务 | Web 管理仅家庭 LAN；云同步只在用户手动触发时运行 |

## 4. Linkslot 上游同步边界

Linkslot 的 UI 与用户功能是唯一产品前端，不需要为前端切换设计抽象层。需要隔离的是 Linkslot 上游代码与 H700/LK 平台实现。

- 上游源码单独维护在 `third_party/linkslot`，通过 Git subtree 跟进 `luguiyi/linkslot`；正式构建锁定上游 commit SHA，不跟随浮动分支。
- H700 板级驱动放在 `platform/h700`；LK 运行时服务放在 `runtime/lk`；Linkslot 集成适配放在 `integration/linkslot`。
- 优先原样复用数据模型、用户交互逻辑、目录语义、Web 静态资源、云同步规则和 MOD 协议；文件、时钟、任务、显示、输入、音频、网络等平台后端由 H700 适配层实现。
- 只有无法通过外部适配解决时，才局部修改 subtree 内上游文件；每个改动都要记录原路径、上游基线、原因和测试，持续控制本地补丁规模。
- 不把整个 Linkslot Cargo workspace 无差别合并为 H700 根 workspace；先逐 crate 审查依赖，再通过窄 feature/path dependency 引入确实需要的部分。

特别注意：Linkslot 目前是 Rust 工作区，部分代码依赖 Rust `std`、线程、文件系统、网络、动态库和 Linux 进程接口；GammaOS Pico/LK 不是 Linux 运行环境。不能假设将 Linkslot crate 直接加入 LK 工程就能编译运行。首先必须验证所需 Rust 运行时服务和工具链方案，确定可原样复用的 crate，以及必须替换的平台后端。不要为了减少几处改动而建设庞大的 Linux syscall/POSIX 兼容层。

详细的 subtree 同步规则、源码分级、CI 检查和每次上游更新流程见 [Linkslot 上游同步与移植边界](linkslot-upstream-sync.md)。

## 5. 关键 HAL 接口

- Display：输出 framebuffer，统一旋转、缩放、色彩变换、滤镜与 Overlay 合成；避免模拟器自行控制屏幕。
- Input：统一按键扫描、消抖、长按/组合键及热键映射；设备 profile 描述按键 GPIO 与极性。
- Audio：统一 PCM 提交、采样率转换、音量、静音、功放使能和关机时序。
- Storage：统一 SD 卡块设备和 FAT32/exFAT 文件 API；提供同步写、临时文件、rename/replace、flush/barrier。
- Power：长按电源请求交给应用层；应用保存即时状态后调用板型专属的 PMIC 完全断电流程。不得实现待机作为正常关机路径。
- Debug：保留串口/早期日志；开发构建评估 USB ADB；联网服务稳定后再提供可选 Wi-Fi SSH。

这些接口只覆盖 Linkslot 与模拟器实际需要的能力，不提前实现完整 Linux 兼容层。

## 6. Emulator Core ABI

Host 不直接依赖某个模拟器的私有全局变量。定义版本化 ABI，至少包含 `init/start/reset/frame/input/video/audio/state_save/state_load/battery_read/battery_write/cheats/options/shutdown`。所有核心均通过统一的 host-owned allocator、文件访问和时间接口运行。

即时存档与电池存档是两个独立接口和文件类别。即时存档格式带核心 ID 与版本，不承诺跨核心通用；电池存档只有通过逐字节兼容测试后才允许共享。

## 7. 网络架构

GammaOS Pico 的 `pico_net` 包含 IPv4/ARP/DHCP/DNS/TCP 客户端等能力，`pico_tls` 包含 TLS 1.2/1.3 客户端实现；`pico_netsvc` 提供异步网络服务接口。这些是网络层的参考，不代表 H700 无线驱动可复用。

H700 Wi-Fi 必须先识别目标设备的无线芯片、总线、固件格式、供电与天线配置，再实现对应驱动。Pico 网络栈当前公开的 TCP API 是 client connect/send/recv，没有完整 listen/accept 服务端接口；Web 管理需要补充 TCP Server 或替换为适合裸机的服务端实现。

网络工作分为两条互不耦合的路径：

1. WebDAV 客户端：手动同步时启动 Wi-Fi、DNS、TCP、TLS 和 WebDAV 请求；结束后按用户设置关闭无线电。
2. 家庭 LAN HTTP Server：仅在设备开机且 Wi-Fi 接入家庭 LAN 时启动；退出设备或关机时停止。

Web Server 默认不强制密码，但仅绑定 Wi-Fi 私网地址，不监听 WAN/公网。可选密码或配对令牌必须可在设备/Web 设置中快速启停和修改。不能把“家庭 LAN”误当成公网安全边界。

## 8. Web、MOD 与云同步

- Web UI：尽量沿用 Linkslot 的静态 HTML/CSS/JS 和 API 语义；服务端重新实现，不复用 `std::net::TcpListener`、进程调用或 Linux 文件路径代码。
- Cloud Sync：保留 Linkslot 的 ROM SHA-256 身份、WebDAV 布局、版本仲裁、冲突决策、备份与原子替换规则；移植网络、文件系统和任务执行后端。
- MOD：保留 `mod.json`、`action.json`、目录结构、执行时机及 Lua Host API。Lua VM 和受控 Host 函数须独立于 Linux/`mlua`；保持内存/时间/API 限额。

## 9. 数据布局

首期兼容 Linkslot 的卡根目录语义，至少保留：

```
Games/GB/
Games/GBC/
Games/GBA/
Saves/GB/
Saves/GBC/
Saves/GBA/
States/GB/
States/GBC/
States/GBA/
Cheats/
Labels/
Overlay/
linkSlot/Config/
linkSlot/Metadata/
linkSlot/mods/
```

其他 Linkslot 目录（BIOS、Guides、Wallpapers、Music 等）可以继续保留，但不为未支持的平台创建模拟器入口。

## 10. 硬件差异与设备 profile

H700 设备不能只用一个屏幕分辨率或一套按键定义。至少建立设备 profile，记录板型、屏幕型号/时序、GPIO 按键、音频 codec/功放、PMIC、Wi-Fi 芯片/总线、USB 端口用途和电源域。一个 profile 通过实机验证后才能标记为受支持。

## 11. 目标机型

首期只支持 Anbernic RG34XX（非 SP）。其 CPU 是 Cortex-A53，因此所有 H700 C/C++/汇编和模拟器核心交叉编译均使用 AArch64 Cortex-A53 target。不能将 RG34XX SP 的 2 GB LPDDR4、`rg34xxsp_v1` 面板驱动或 Boot0 DRAM 参数套用到 RG34XX。

详细硬件基线及证据见 [RG34XX 硬件基线](rg34xx-hardware-profile.md)。

## 12. 参考代码

- [GammaOS Pico LK branch](https://github.com/TheGammaSqueeze/lk-mt6785/tree/lk-gba-emu-sd-card-kpa)
- [Pico network stack](https://github.com/TheGammaSqueeze/lk-mt6785/tree/lk-gba-emu-sd-card-kpa/platform/common/pico)
- [Pico GBA driver and Core ABI](https://github.com/TheGammaSqueeze/lk-mt6785/tree/lk-gba-emu-sd-card-kpa/emu/gba)
- [Linkslot](https://github.com/luguiyi/linkslot)
- [Linkslot Web server](https://github.com/luguiyi/linkslot/tree/main/crates/slot/src/web)
- [Linkslot MOD runtime](https://github.com/luguiyi/linkslot/tree/main/crates/linkslot/src/mods)
