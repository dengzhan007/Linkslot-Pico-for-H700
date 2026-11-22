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
- [ ] **完整内存管理的前置审计（不阻塞 Image 内入口探针）：** 继续审查剩余保留区归属，尤其 `[0x7ec00000,0x7fc00000)` 的 16 MiB 高地址区、top-of-RAM 37 条碎片、monitor 实际访问上界及其他 secure/DMA/display carveout 与 loader 生命周期。第四轮 `/proc/iomem` 未给该 16 MiB 区域标注 owner；ION/TEE 的 `GET_DRM_INFO` 关系曾作为待验证线索，但不是产品目标。该区域在 owner 明确之前始终排除。CMA `[0x77c00000,0x7bc00000)`、monitor `[0x48000000,0x49000000)`、framebuffer `[0x7bf24800,0x7c076000)` 和其它 reserved entries 同样排除。此项**不阻止第一份完全限制在已装载 kernel Image 自身范围内的 entry probe**；但在归属审计通过前，不得建立通用 allocator、Image 外栈、SD DMA、显示双缓冲或其他外部内存用户。不得把 `memblock.memory - memblock.reserved` 直接当作 payload 可用 RAM。
- [x] 对当前 RG34XX p4 boot 分区建立只读 Android boot image v2 解析器：校验 ANDROID! header、page/component alignment、SHA-1 image ID、gzip kernel、AArch64 Image header 和 appended FDT；主机合成测试覆盖损坏 ID、截断组件、FDT totalsize 和只写新目录的提取行为。工具：`tools/inspect_android_bootimg.py`。
- [x] 对当前现场 U-Boot 二进制直接定位 AArch32→AArch64 monitor handoff：Thumb 路径在 `0x14E2` 从 `0x1510` 读取 SMC Function ID `0x8000FF04`，`0x14E4` BLX 到 A32 wrapper `0xB58`，并在 `0xB5C` 执行 `SMC #0`。调用前将 `images.ep`（`+0x60`）放入 `r1`，将 FDT 复制到 `0x44000000` 后放入 `r2`，`r3=1`。`images.ft_addr/+0x6c` 与 `images.ft_len/+0x70` 的读字段和 U-Boot v2018.05 `bootm_headers_t` 布局一致；另一条 A32 fallback 直接 `BLX r6` 并传 `(r0=0, r1=machine-id, r2=FDT)`，交叉确认 `r6=kernel entry`。`0x27DC` 检查全局 byte `0x4A000000+0x4E9` 是否等于 1，选择 vendor SMC 或 A32 fallback；`flag&0x200` 对应 `BOOTM_STATE_OS_FAKE_GO`，跳过实际交接。
- [x] 解析 SMC 前 CPU/cache 清理路径：`0x14D0 -> 0x0C7C` 传入 `r0=3`，最终路径屏蔽 IRQ/FIQ，临时清 SCTLR.I，失效 I-cache/branch predictor，调用 A32 set/way D-cache clean helper（`MCR p15,c7,c14,2`），再恢复 I-cache。该参数不进入会清 SCTLR.M/C 的另一分支，所以不能宣称 U-Boot 在 SMC 前关闭了 MMU 或 D-cache。现场 TOC1/U-Boot hash 和每条 raw offset 见 [RG34XX U-Boot handoff 二进制审计](rg34xx-uboot-handoff-binary-audit.md)。
- [x] 从当前现场 U-Boot 进一步定位 FDT 初始化/`bootm` 路径：`0xEF5C` 读取全局 `gd->fdt_blob`（`sb+0x5c`），依据 totalsize 预留约 4 KiB 并调用 FDT 扩容/复制 helper；`0x8680` 更新全局 `working_fdt`；`0x7F10` 的 `bootm` 路径从该 global 取 FDT 并进行 initrd/chosen 处理；最终 `boot_jump_linux` 将 `images.ft_addr` 复制到 `0x44000000`。BaseOS 同类 U-Boot trace 模型把初始 `working_fdt` 设为 TOC1 package DTB。当前高置信判断为 TOC1 DTB → working_fdt → bootm fixup → images.ft_addr；尚无运行时指针值捕获，不能把它表述为现场指针级证明。细节见 [RG34XX U-Boot handoff 二进制审计](rg34xx-uboot-handoff-binary-audit.md)。
- [ ] 完成 FDT 来源的本地属性级比对：使用 `tools/compare_rg34xx_fdt_sources.py` 比较 p4 appended DTB、TOC1 DTB、live FDT 的 node/property hash，确认 live tree 更接近哪一份以及 memory/chosen 的差异。原始 boot/TOC1/FDT 文件只留在本地，不上传；仅上传生成的 `rg34xx-fdt-source-comparison.json` 到 reports 分支。
- [ ] 完成剩余启动门槛：恢复精确 `CONFIG_SYS_BOOTM_LEN`；把现场 U-Boot cache-maintenance 结束状态与相同 hash vendor monitor 的 `0x8000FF04` ERET 仿真证据衔接，核对目标 EL、SPSR、寄存器、MMU/cache/TLB 与 DAIF。BaseOS verifier 的 `SPSR=0x3c5`、`x0=0x44000000`、`x1..3=0` 是实际 monitor 指令的仿真结果，不是 RG34XX 实机寄存器捕获；当前 stock U-Boot 的 `SCTLR_EL1` 仍未知。当前证据已足以把“保留原厂 Boot0/TOC1/monitor/U-Boot，只替换 p4 kernel component”设为首选候选，但还不批准制作/刷写设备镜像。详细边界见 [RG34XX stock boot.img payload 路径评估](rg34xx-stock-bootimg-payload-path.md)。
- [x] 选定首选候选：保留原厂 Boot ROM/Boot0/TOC1/vendor monitor/AArch32 U-Boot/env/GPT，只替换 p4 Android v2 kernel component。当前现场 binary 已确认 gzip kernel、kernel load/entry `0x40080000` 和 `0x8000FF04` SMC 调用路径。该方案仍须完成 monitor ERET 与 FDT provenance 评审，**不是已批准刷写方案**；若实证显示 stock SMC 不适用于 freestanding Image，则回退到 A32 BL33 adapter，再评估自有 A64 SPL。
- [x] 建立最小 AArch64 Image entry probe 的**主机静态构建**：`platform/h700/bootprobe/entry.S` + `payload.ld`，入口记录 x0..x3、原始 SP、CurrentEL、初始 DAIF、SCTLR_EL1、TTBR0/1、TCR、MAIR、VBAR、MPIDR；在写任何 Image 内记录/栈前，若 `SCTLR_EL1.M` 或 `.C` 仍开启就原地 WFE，不触碰数据内存；仅在 MMU/D-cache 均关闭时才建立 Image 内私有栈/记录区并 WFE。CI 构建 8 KiB Image（`code0=B +0x40`、`text_offset=0x80000`、`image_size=0x2000`）并检查 header/尺寸。该产物没有被放入 Android p4，也没有在设备上执行。
- [ ] 在 monitor ERET/FDT provenance 门槛通过并完成独立恢复评审后，才把 entry probe 放到单独测试卡上实机执行；记录 CurrentEL、SPSR、x0..x3、DAIF、原始 SP、SCTLR_EL1/MMU/cache/TLB 与 FDT。只使用 Image 内部栈/记录，不访问 MMIO 或 Image 外 RAM。若 stock handoff 实证不适用，才另建 A32 BL33 adapter。
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
