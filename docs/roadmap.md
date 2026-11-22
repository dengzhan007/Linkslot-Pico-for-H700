# 开发路线与阶段验收

本项目采用可验证的垂直切片推进。每一阶段都必须产出可重复构建的测试对象和明确的验收记录；不以“编译通过”代替设备验证。

## M0：项目基线

- [x] 建立独立仓库。
- [x] 确认产品方向：GammaOS Pico 裸机架构参考 + Linkslot 唯一前端。
- [x] 确认首期只做 GB/GBC/GBA。
- [x] 固化关机、手动电池存档、手动云同步、家庭 LAN Web 和调试规则。
- [x] 确认目标设备无 eMMC，只设计 SD 卡启动，不做旧系统自动回退。
- [x] 建立 RG34XX / Allwinner H700 Cortex-A53 的 AArch64 freestanding C、C++ 与汇编编译检查和 CI。
- [ ] 等待 Linkslot 即将发布的大版本后，重新锁定上游 commit 并完成逐 crate 复用审计。
- [x] 确认目标 SoC 是 Allwinner H700，不是 Rockchip RK3566；AArch64 编译 CPU target 已修正为 Cortex-A53。
- [x] 锁定第一台实机为 Anbernic RG34XX（非 SP），建立 Allwinner H700 / Cortex-A53 硬件基线。

## 当前工作：不依赖 Linkslot 上游版本的准备

### A. 工具链与工程基础（已完成第一步）

- [x] AArch64 freestanding C、C++ 和汇编编译探针。
- [x] 检查 C、C++ 和汇编产物确实为 AArch64 ELF relocatable objects。
- [x] GitHub Actions 自动编译检查。
- [ ] 增加统一构建参数与工具链版本记录。
- [ ] 明确调试日志输出方式和目标 ELF/镜像的检查规则。

### B. RG34XX / Allwinner H700 启动链与设备识别

交付：目标设备 SD 启动链调查报告和最小裸机入口规范。

- [x] 从 BaseOS-MultiFrontend 固件审计确认 RG34XX 型号、Allwinner H700、Cortex-A53、1 GB LPDDR4 和 `rg34xx_v1` 面板驱动。
- [x] 审查 BaseOS 已提交的 Boot0/TOC1/U-Boot/monitor 结构、AArch32 BL33 返回状态及 A32→A64 Linux SMC 交接；形成[启动交接审计报告](rg34xx-boot-handoff-audit.md)。该项是源代码/模拟证据审计，不代表目标掌机已验证。
- [x] 建立[目标机只读 inventory 脚本](../scripts/collect-rg34xx-readonly-inventory.sh)、Boot0 eGON/checksum/DRAM 候选解析器、Allwinner TOC1 校验/提取器、H700 FDT/framebuffer 解析器、U-Boot env CRC 解析器、CMA/memblock 物理范围解析器及静态只读安全回归；主机合成测试覆盖各类输入。
- [x] 在 RG34XX 非 SP 实机运行首轮[只读采集流程](rg34xx-readonly-inventory.md)，完成 Boot0/TOC1 checksum、组件提取与 hash、live FDT、GPT、/proc/iomem 和 meminfo 检查；现场结论见[实机 Inventory 报告](rg34xx-live-inventory-2026-11-22.md)。
- [x] 从 live FDT 的 `boot_fb0` 与 bootargs `disp_reserve` 交叉确认 framebuffer 物理范围 `[0x7bf24800, 0x7c076000)`（720×480×32bpp，1,382,400 bytes）；新增 FDT parser JSON 字段及合成测试。
- [x] 读取完整 16 MiB U-Boot env GPT 分区：CRC 唯一匹配到 128 KiB 环境映像（data offset 4），25 个变量无解析警告；完整分区 hash 与 BaseOS prepared RG34XX env 区域一致。目标 U-Boot 静态字符串与 SD 自动切换路径相符，现场报告记录了证据边界。
- [x] 将两份现场 Boot0 的 32-word DRAM 参数与 BaseOS RG34XX profile 对照：LBA16 主 LPDDR4 块 32/32 匹配；LBA512 的 LPDDR3/LPDDR4 备用块也各自 32/32 匹配；LBA512 主块仅 word[26]/word[29] 不同。H616 BROM 公开搜索位置与 LBA16/LBA512 对应，LBA16 为高置信正常启动候选，但尚无直接读取轨迹。
- [x] 读取第四轮 memblock `memory/reserved`：确认 1 GiB memory window 与 46 条 reserved entries；识别 `[0x48000000,0x49000000)` 为与 BaseOS vendor monitor 研究窗口一致的 16 MiB 保留区，并将 `[0x77c00000,0x7bc00000)` 标为与 `CmaTotal=64 MiB` 精确匹配的高置信 CMA 候选；现场明细见 [Inventory 报告](rg34xx-live-inventory-2026-11-22.md)。解析器现支持 RG34XX 的逗号加 `size:...K` memblock 格式，且 PAGE_SIZE 缺失时仍能解析 memblock。
- [x] 反编译第四轮 live FDT 并与 memblock 交叉审查：将 `[0x42000000,0x42001000)` 与 FDT initrd 范围 `[0x42000000,0x42000200)` 关联，将 `[0x44000000,0x44023240)` 标记为高置信 live FDT blob 候选；识别 `/firmware/optee` SMC conduit 和 ION secure/CMA heap 声明；确认 ION heap base/size 为零、Linux OP-TEE UID 检查失败及 bootargs `androidboot.secure_os_exist=0`，但不据此声称安全世界/secure carveout 不存在。FDT parser 已增加 initrd、firmware_nodes、ion_heaps JSON 字段和合成测试。
- [x] 完成 2026-11-22 20:47:42 的 ION/TEE 只读运行时采集：确认 `CONFIG_ION/TEE/OPTEE/CMA=y`、`/dev/ion` 存在、`ion-sunxi` 已绑定；`/dev/tee0` 与 `/dev/teepriv0` 不存在，标准 OP-TEE probe 报 `api uid mismatch`。`sys_user` heap 当前约 7.05 MiB 分配，CMA heap 中 `nextui.elf` 分配 4,153,344 bytes；CMA 总量 64 MiB、空闲 60,360 KiB；secure heap 当前没有活动分配，不能据此判断 heap 容量或安全区位置。原始字段见 [20:47 runtime 报告](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-inventory-20261122-204742)。
- [ ] 查明 Allwinner vendor `GET_DRM_INFO` SMC 的实机返回：同版本 4.9.170 参考源码显示 `ion-sunxi` 会在 secure heap 创建前独立调用该 SMC，并在成功时记录 `drm_base/drm_size/tee_base`，失败时记录 `drm config service not available`。该路径与标准 OP-TEE UID 握手不同；`api uid mismatch` 不能替代该结果。collector 已补齐失败日志筛选。拿到实际返回前，`[0x7ec00000,0x7fc00000)` 及 top-of-RAM 碎片继续视为未知保留区并排除；framebuffer `[0x7bf24800,0x7c076000)` 也必须排除。不得把 `memblock.memory - memblock.reserved` 直接当作 payload 可用 RAM。
- [ ] 选定保留原厂 U-Boot 还是采用 A64 SPL + vendor monitor + A32 BL33；先完成入口/恢复设计评审，不试写启动区域。
- [ ] 建立最小 A32 handoff adapter + AArch64 payload；实机验证 CurrentEL、SPSR、参数寄存器、栈、异常向量、GIC、MMU/cache/TLB 与 FDT/内存边界。
- [ ] 完成重复冷启动、第一帧/日志时间戳和失败恢复测试；恢复方式按电脑重写/修复 SD 卡，不依赖旧 Linux 自动回退。

### C. Allwinner H700 基础 HAL

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

- 审查 Gambatte、mGBA、gpSP 的 AArch64 构建和 ABI；
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
