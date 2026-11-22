# Linkslot Pico for H700

面向 Anbernic H700 系列掌机的低功耗裸机游戏平台，首台目标设备为 **Anbernic RG34XX（非 SP）**。

本项目以 GammaOS Pico / LK 的裸机运行架构为底层技术参考，以 [linkSlot](https://github.com/luguiyi/linkslot) 作为唯一前端和用户功能来源；不完整移植 GammaOS Pico 的前端，也不把现有 Linux Linkslot 二进制直接搬进 LK。

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

- 已建立 RG34XX / Allwinner H700 / Cortex-A53 的 AArch64 freestanding C/C++/汇编编译检查及 GitHub Actions。
- 已新增只用于静态审查的 8 KiB AArch64 Image entry probe 构建：私有栈和入口记录区均位于 Image 内；入口先检查 `SCTLR_EL1.M/C`，若 MMU 或 D-cache 仍开启则不写内存而停机。CI 验证 `ARM\x64` header、`code0` 分支、`text_offset=0x80000` 和 Image size。尚未在设备上执行，也没有写入 p4。
- 当前 stock boot.img kernel-slot 路线是首选候选：现场 U-Boot 已确认 gzip selector、`0x40080000` kernel load/entry 与 `0x8000FF04` monitor handoff；FDT 来源和 monitor ERET 实机状态仍待最终确认。
- Linkslot 即将发布大版本，暂缓导入/同步上游源码和逐 crate 复用审计，避免对即将过时的版本重复适配。

本地运行 AArch64 编译检查：

```sh
sudo apt install clang lld llvm
bash scripts/build-h700-objects.sh
```

详细说明见 [H700 AArch64 构建基础](docs/h700-build-scaffold.md)。

## Linkslot 上游同步

项目不要求直接使用原版 Linkslot 发布包升级。重点是尽量保持上游业务代码原样、将 H700/LK 适配放在独立目录，通过固定 commit 的 Git subtree 跟进上游，减少每次更新时需要手工修改的代码和重新适配范围。

**当前等待 Linkslot 即将发布的大版本后再确定导入基线。** 在此之前优先推进不依赖 Linkslot 内部 API 的 H700 构建基础、启动链和硬件调查。

具体同步规则见 [Linkslot 上游同步与移植边界](docs/linkslot-upstream-sync.md)。

## 重要设计文档

- [RG34XX 硬件基线](docs/rg34xx-hardware-profile.md)
- [RG34XX 启动交接审计](docs/rg34xx-boot-handoff-audit.md)
- [RG34XX 原厂 boot.img 内核槽位裸机 payload 路径评估](docs/rg34xx-stock-bootimg-payload-path.md)
- [RG34XX 现场 U-Boot AArch32 → AArch64 handoff 二进制审计](docs/rg34xx-uboot-handoff-binary-audit.md)
- [RG34XX 首轮实机只读 Inventory](docs/rg34xx-live-inventory-2026-11-22.md)
- [总体架构与硬件边界](docs/architecture.md)
- [已确认的设计决策](docs/decisions.md)
- [模拟器与存档兼容性](docs/emulator-save-compatibility.md)
- [开发路线与阶段验收](docs/roadmap.md)
- [H700 AArch64 构建基础](docs/h700-build-scaffold.md)
- [上游源码基线](docs/upstream-baselines.md)

## 项目状态

目前为设计与硬件可行性验证阶段，尚无可刷写固件。任何构建产物在完成 RG34XX 实机验证之前都不得标记为可用发行版。

## 原则

1. 先验证 RG34XX SD 启动、显示、输入、SD 读写和完全关机路径，再接入完整 Linkslot UI。
2. 数据格式优先兼容 Linkslot；不得无必要改变 Games / Saves / States / Cheats / linkSlot 目录语义。
3. 任何自动存档、同步和关机行为必须遵守 `docs/decisions.md`。
4. 首期目标是 RG34XX / Allwinner H700 / Cortex-A53；不能将 MT6785/MTK 特有驱动或其他 SoC 的寄存器、DMA 参数直接用于 H700。
5. 上游代码和第三方核心必须保留许可证、NOTICE、对应源码及构建来源。
