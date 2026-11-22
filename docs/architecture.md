# 总体架构与硬件边界

## 1. 项目定位

目标是运行在 Anbernic H700 / Rockchip RK3566 上的独立裸机游戏环境：启动后直接进入 Linkslot 风格的 UI，不启动完整 Linux，也不依赖 suspend/s2idle 保持游戏状态。GammaOS Pico 提供裸机运行时、Core ABI、渲染与输入组织方式的参考；linkSlot 提供 UI、目录语义和用户功能来源。

这不是把 MT6785 版 LK 原样移植到 RK3566。两者 SoC、启动链、PMIC、显示控制器、音频和 Wi-Fi 硬件不同；MTK 寄存器地址、CONNSYS、DMA 描述符及电源时序不能用于 H700。

## 2. 启动与回退

首选开发路径为保留设备现有 Boot ROM / SPL / U-Boot 启动链，在 SD 卡上加载独立 AArch64 裸机 payload。首轮实验不得写入 eMMC 的 bootloader、分区表或 recovery。启动入口必须具备：

- 正常启动新运行时；
- 按键选择回到原有 Linux / BaseOS；
- payload 校验失败、SD 缺失或初始化失败时自动回退；
- 独立恢复/调试入口，避免测试固件把设备变成不可启动状态。

具体使用 U-Boot `bootelf`、自定义命令还是 kernel-like Image handoff，须先检查目标机 U-Boot 配置与可用命令后决定；当前不假定某一种入口已经可用。

## 3. 分层

| 层 | 职责 | 设计约束 |
|---|---|---|
| Boot handoff | U-Boot 加载、内存交接、回退 | 保留原启动链；首期只从 SD 启动 |
| H700 HAL | 时钟、计时器、MMU/cache、GPIO、PMIC、背光、显示、音频、SD、USB、Wi-Fi | 所有寄存器与时序按 RK3566 和具体板型重新确认 |
| Runtime | 线程/任务、锁、事件、内存分配、日志、文件系统封装 | 统一同步 I/O 与后台任务接口；不暴露 Linux API |
| Emulator Host | GB/GBC/GBA Core ABI、帧调度、输入、音频、视频、持久化 | 模拟器核心与 UI 解耦，核心可替换 |
| Linkslot App | 卡带 UI、游戏库、配置、MOD、云同步入口 | 兼容 Linkslot 的目录与配置语义 |
| LAN Services | HTTP Server、云同步 WebDAV 客户端、可选 SSH | Web 管理仅家庭 LAN；云同步只在用户手动触发时运行 |

## 4. 关键 HAL 接口

- Display：输出 framebuffer，统一旋转、缩放、色彩变换、滤镜与 Overlay 合成；避免每个模拟器自行控制屏幕。
- Input：统一按键扫描、消抖、长按/组合键及热键映射；设备 profile 描述按键 GPIO 与极性。
- Audio：统一 PCM 提交、采样率转换、音量、静音、功放使能和关机时序。
- Storage：统一 SD 卡块设备和 FAT32/exFAT 文件 API；提供同步写、临时文件、rename/replace、flush/barrier。
- Power：长按电源请求交给应用层；应用保存即时状态后调用板型专属的 PMIC 完全断电流程。不得实现待机作为正常关机路径。
- Debug：保留串口/早期日志；开发构建评估 USB ADB；联网服务稳定后再提供可选 Wi-Fi SSH。

## 5. Emulator Core ABI

Host 不直接依赖某个模拟器的私有全局变量。定义版本化 ABI，至少包含 `init/start/reset/frame/input/video/audio/state_save/state_load/battery_read/battery_write/cheats/options/shutdown`。所有核心均通过统一的 host-owned allocator、文件访问和时间接口运行。

即时存档与电池存档是两个独立接口和文件类别。即时存档格式带核心 ID 与版本，不承诺跨核心通用；电池存档只有通过逐字节兼容测试后才允许共享。

## 6. 网络架构

GammaOS Pico 的 `pico_net` 包含 IPv4/ARP/DHCP/DNS/TCP 客户端等能力，`pico_tls` 包含 TLS 1.2/1.3 客户端实现；`pico_netsvc` 提供异步网络服务接口。这些是网络层的参考，不代表 H700 无线驱动可复用。

H700 Wi-Fi 必须先识别目标设备的无线芯片、总线、固件格式、供电与天线配置，再实现对应驱动。Pico 网络栈当前公开的 TCP API 是 client connect/send/recv，没有完整 listen/accept 服务端接口；Web 管理需要补充 TCP Server 或替换为适合裸机的服务端实现。

网络工作分为两条互不耦合的路径：

1. WebDAV 客户端：手动同步时启动 Wi-Fi、DNS、TCP、TLS 和 WebDAV 请求；结束后按用户设置关闭无线电。
2. 家庭 LAN HTTP Server：仅在设备开机且 Wi-Fi 接入家庭 LAN 时启动；退出设备或关机时停止。

Web Server 默认不强制密码，但仅绑定 Wi-Fi 私网地址，不监听 WAN/公网。可选密码或配对令牌必须可在设备/Web 设置中快速启停和修改。不能把“家庭 LAN”误当成公网安全边界。

## 7. Web、MOD 与云同步

- Web UI：尽量沿用 Linkslot 的静态 HTML/CSS/JS 和 API 语义；服务端重新实现，不复用 `std::net::TcpListener`、进程调用或 Linux 文件路径代码。
- Cloud Sync：保留 Linkslot 的 ROM SHA-256 身份、WebDAV 布局、版本仲裁、冲突决策、备份与原子替换规则；移植网络、文件系统和任务执行后端。
- MOD：保留 `mod.json`、`action.json`、目录结构、执行时机及 Lua Host API。Lua VM 和受控 Host 函数须独立于 Linux/`mlua`；保持内存/时间/API 限额。

## 8. 数据布局

首期兼容 Linkslot 的卡根目录语义，至少保留：

```text
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

## 9. 硬件差异与设备 profile

H700 设备不能只用一个屏幕分辨率或一套按键定义。至少建立设备 profile，记录板型、屏幕型号/时序、GPIO 按键、音频 codec/功放、PMIC、Wi-Fi 芯片/总线、USB 端口用途和电源域。一个 profile 通过实机验证后才能标记为受支持。

## 10. 参考代码

- [GammaOS Pico LK branch](https://github.com/TheGammaSqueeze/lk-mt6785/tree/lk-gba-emu-sd-card-kpa)
- [Pico network stack](https://github.com/TheGammaSqueeze/lk-mt6785/tree/lk-gba-emu-sd-card-kpa/platform/common/pico)
- [Pico GBA driver and Core ABI](https://github.com/TheGammaSqueeze/lk-mt6785/tree/lk-gba-emu-sd-card-kpa/emu/gba)
- [Linkslot](https://github.com/luguiyi/linkslot)
- [Linkslot Web server](https://github.com/luguiyi/linkslot/tree/main/crates/slot/src/web)
- [Linkslot MOD runtime](https://github.com/luguiyi/linkslot/tree/main/crates/linkslot/src/mods)