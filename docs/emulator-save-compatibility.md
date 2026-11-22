# 模拟器选择与存档兼容性

## 1. GB / GBC：Gambatte 与 mGBA

| 项目 | Gambatte | mGBA |
|---|---|---|
| 定位 | 专用 GB/GBC 核心，准确性导向 | 多平台核心，支持 GB/GBC/GBA |
| 优势 | 对 GB/GBC 行为和时序关注度高；成熟的 GB 色彩化、SGB/GBC 调色板及 GBC LCD 色彩校正选项 | 一个核心覆盖三代平台；功能集较完整；支持 Game Boy Camera，而 Gambatte 不支持 Camera |
| 代价 | 主要围绕 GB/GBC；Camera 不支持 | 对 GB/GBC 用户来说是多功能统一核心，但不以专用 GB/GBC 调色板工作流为主要卖点 |
| 建议 | GB/GBC 默认核心 | 可选核心，给需要 Camera 或希望统一核心的用户 |

Libretro 文档将 Gambatte 描述为 accuracy-focused，并列出 GB Colorization、custom palettes 和 GBC color correction；mGBA 文档列出 GB/GBC/GBA 支持与 GB Camera 能力。两者都支持电池存档，Libretro 默认保存文件扩展名均为 `.srm`。扩展名相同不等于所有 ROM/核心版本的保存数据必然逐字节兼容，项目仍须对实际 dump 长度和数据做测试。

## 2. GBA：mGBA 与 armGBA

| 项目 | mGBA | armGBA-pico |
|---|---|---|
| 代码来源 | Linkslot 当前已经使用的 GBA 核心 | armGBA 上游的裸机加载核心；其 README 说明其渲染器、总线/I/O、save chips 等借用了 mGBA 的相关实现 |
| 运行方式 | 当前 Linkslot 的 Linux/Libretro 构建；本项目需要移植/编译为 H700 Core ABI | GammaOS Pico 已做成固定地址加载的 32-bit ARM core blob，具有 A32 dynarec 和 Cortex-A55 优化 |
| 优势 | 与 Linkslot 现有配置、核心设置和用户习惯衔接最直接；通用核心生态成熟 | 针对无 OS 的运行环境设计；A32 recompiler、NEON renderer、LCD ghosting、run-ahead 与 host save-state 接口已专门适配 |
| 限制 | Linux/Libretro 版本不能原样加载到裸机；要重新实现 host services | 现成 blob 针对 32-bit ARM/LK 固定内存布局，不能直接放进 RK3566 AArch64 host；需要移植、重编译或设计 AArch32 core boundary |
| 电池存档 | 项目当前 mGBA 文件格式/长度需实测并记录 | armGBA-pico 明确声明与 gpSP 共用 `/saves/gba` 下 128 KiB 电池存档缓冲区 |
| 即时存档 | 与 mGBA 自身版本绑定 | 与 gpSP 不兼容，armGBA 使用独立 `/states/armgba` |

**目前可确认的兼容性：armGBA-pico 与 gpSP 的 GBA 电池存档共享 128 KiB 布局；两者即时存档不兼容。** armGBA-pico 的文档没有声明与 Linkslot mGBA 存档兼容，因此不能单靠“都用 mGBA 衍生代码”推断兼容。

## 3. 本项目的初始选择

- GB/GBC：Gambatte 默认，mGBA 可选。
- GBA：mGBA 默认，gpSP 备选。
- armGBA-pico：保留为候选研究对象，但不进入初始正式核心列表，直至完成 mGBA ↔ armGBA 的电池存档实测和 AArch64 ABI 评估。

这遵守“先保持 Linkslot 存档连续性，不以未经验证的性能收益换取存档风险”的原则。

## 4. 存档互通测试矩阵

对同一 ROM、同一游戏进度，至少覆盖 SRAM、Flash 64K、Flash 128K、EEPROM 512B、EEPROM 8K（使用对应测试 ROM 或真实游戏验证）：

1. 核心 A 写入电池存档；退出后保存文件 SHA-256、文件长度和原始字节。
2. 核心 B 读取该文件，检查游戏内存档是否出现；继续游戏并写入不同标记。
3. 核心 B 保存后回到核心 A，检查数据是否完整。
4. 验证不同 save chip、RTC 附加数据、文件 padding/长度、大小端和 CRC 行为。
5. 即时存档另测，不允许把 savestate 可读误认为 battery save 可互通。

结果分三级记录：`BYTE_IDENTICAL`（字节完全一致）、`DATA_COMPATIBLE`（游戏存档数据可互通但长度/包装不同）、`INCOMPATIBLE`。只有前两级并且有自动化回归测试时才允许共用 Saves 路径；第二级必须提供无损转换和备份。

## 5. 上游参考

- [Gambatte Libretro 文档](https://docs.libretro.com/library/gambatte/)
- [mGBA Libretro 文档](https://docs.libretro.com/library/mgba/)
- [mGBA Save RAM 语义](https://github.com/mgba-emu/mgba/blob/master/src/platform/libretro/libretro.h)
- [armGBA-pico README](https://github.com/TheGammaSqueeze/armGBA-pico)
- [GammaOS Pico GBA save bridge](https://github.com/TheGammaSqueeze/lk-mt6785/blob/lk-gba-emu-sd-card-kpa/emu/gba/gba_sd_save.c)
- [GammaOS Pico GBA Core ABI](https://github.com/TheGammaSqueeze/lk-mt6785/blob/lk-gba-emu-sd-card-kpa/emu/gba/gba_core_abi.h)

上述结论基于已检查的上游文档与源码；最终兼容性仍以本项目针对目标核心版本运行的 byte-level 测试为准。