# Linkslot Pico for H700

面向 Anbernic H700 系列掌机的低功耗裸机游戏平台。

本项目以 GammaOS Pico / LK 的裸机运行架构为底层技术参考，以 [linkSlot](https://github.com/luguiyi/linkslot) 的产品界面和用户功能为来源；不完整移植 GammaOS Pico 的前端，也不把现有 Linux Linkslot 二进制直接搬进 LK。

## 项目目标

- 开机直接进入 Linkslot 风格的游戏库，首期只支持 GB / GBC / GBA。
- 采用不依赖 Linux 待机的运行方式；长按电源键时只保存即时恢复状态并完全关机。
- 电池存档由用户显式保存；云存档同步由用户显式触发。关机不自动写入电池存档，也不触发云同步。
- 保留 Linkslot 的云同步协议、Lua MOD 格式/API、家庭 LAN Web 管理的数据与交互设计。
- 保留 GammaOS Pico 已有的校色、LCD 滤镜、遮罩/显示效果、金手指等能力，并继续扩展 Linkslot 的游戏设置。
- 保留开发调试入口：优先评估 USB ADB；Wi-Fi SSH 作为网络服务完善后的可选通道。不得为了调试常驻高功耗待机。

## 当前开发基线

- UI / 用户功能来源：[luguiyi/linkslot](https://github.com/luguiyi/linkslot)
- 裸机 / LK 参考：[TheGammaSqueeze/lk-mt6785](https://github.com/TheGammaSqueeze/lk-mt6785)
- 首期平台：GB、GBC、GBA。
- 核心建议：GB/GBC 优先 Gambatte；mGBA 保留为 GB/GBC 可选核心。GBA 优先 mGBA，gpSP 作为备选；armGBA-pico 暂列实验候选，待 mGBA 存档兼容验证后再决定是否进入正式候选。
- Web 管理仅面向家庭 LAN，默认不强制密码；预留简单易用的可选密码/访问令牌设置。服务不得监听 WAN 或公网接口。

## 重要设计文档

- [总体架构与硬件边界](docs/architecture.md)
- [已确认的设计决策](docs/decisions.md)
- [模拟器与存档兼容性](docs/emulator-save-compatibility.md)
- [开发路线与阶段验收](docs/roadmap.md)

## 项目状态

目前为设计与硬件可行性验证阶段，尚无可刷写固件。任何构建产物在完成 H700 实机验证之前都不得标记为可用发行版。

## 原则

1. 先证明 H700 裸机启动、显示、输入、SD 读写和安全回退，再移植完整 UI。
2. 数据格式优先兼容 Linkslot；不得无必要改变 Games / Saves / States / Cheats / linkSlot 目录语义。
3. 任何自动存档、同步和关机行为必须遵守 `docs/decisions.md`。
4. 不能将 MT6785/MTK 特有驱动、寄存器或 DMA 参数直接用于 RK3566/H700。
5. 上游代码和第三方核心必须保留许可证、NOTICE、对应源码及构建来源。