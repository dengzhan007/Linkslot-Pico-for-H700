# M1 — RG34XX AArch32 BL33 诊断启动规格

**状态：** M1-S0 规格草案；未生成诊断 payload，未构建设备可安装固件，未执行设备测试。  
**目标设备：** Anbernic RG34XX（非 SP），Allwinner H700，AArch32 SVC BL33 handoff。  
**上游/构建 pin：** Linkslot main 1.0.49（本阶段不链接 Linkslot）；BaseOS-MultiFrontend native builder `cbe253a43f46cf551cc9a6c79a84015bde3686cf`。

## 1. 交付目标

准备一个极小的 AArch32 BL33 诊断 payload，用于验证受控 BL33 启动路径的入口与可观测性；不引入完整 Linkslot、mGBA、Linux loader、PMIC、电源管理或多核功能。

活动 native firmware 中 BL33 loader 的文件 extent 是 `0x8000` bytes（32 KiB），目标装载地址为 `0x4a000000`。这是 raw input 文件边界，**不是**对整个 1 MiB BL33 区间、BSS、stack、heap、page table、DMA 或应用 DRAM 的内存授权。

## 2. payload 的最小功能

1. AArch32 ARM-state entry/vector contract，与当前活动 BL33 loader 的头区及入口偏移匹配。
2. 在改变处理器模式、VBAR、stack 或启动运行时之前捕获入口寄存器；复用现有 `h700_bl33_entry_snapshot` ABI（CPSR、SP、LR、r4 与 publish-last magic），并保持字段/发布顺序回归测试。
3. 仅在有资格且经审查的 proof-of-life 输出接口确定后，发出一个固定、可重复识别的短信号；不得扫描设备、探测未知总线或循环访问外围设备。
4. 不包含业务调度器、动态内存分配、外部应用载入、文件系统、模拟器或 UI。

入口快照要到 BSS 清理后的 publish point 才存在，不能记录 publish point 之前的早期故障，也不能单独证明 CPU1–CPU3 的 reset/RVBAR 状态。

## 3. Proof-of-life 信号资格

**当前没有获准的硬件输出接口。** 选择输出之前必须同时满足：对应精确 RG34XX 非 SP 板型的可信原理图/PCB test-pad 资料或同版本启动源代码；明确 pin mux、clock/reset、供电域与 BL33 入口时访问权限；不覆盖 Boot0/SPL/BL31 必需功能；输出不依赖未批准的 DRAM、framebuffer、GIC、timer、DMA、PMIC 或存储控制器；信号实现和恢复方案经项目所有者审查。

### 当前候选：UART0（有板级配置旁证，仍未批准、不可写）

**已取得的正向证据：**

- reports 分支当前设备的 Linux bootargs 包含 `earlyprintk=sunxi-uart,0x05000000` 与 `console=ttyS0,115200`；当前 dmesg 中 pinctrl 对 `uart0_jtag` / `uart0` mux 有状态报告。
- 匹配 RG34XX（非 SP）路径的 KNULLI boot DTS（固定 commit [`57afa662009641d181f57241bcc480119b131530`](https://github.com/knulli-cfw/knulli-linux/blob/57afa662009641d181f57241bcc480119b131530/package/boot/uboot-h700/rg34xx/boot_package/rg34xx.dts)）声明 `uart@05000000` 为 `okay`，寄存器窗口 `0x05000000/0x400`、UART0 clock phandle `0x29`、default pinctrl phandle `0x1a0`。该 pin group 将 PH0/PH1 配置为 `uart0_tx/uart0_rx`、muxsel 2；sleep pinctrl 则使用 muxsel 7。该 DTS 还将 UART3（`0x05000c00`）设为 disabled。
- 独立 RG34XX [bootlog/pinctrl capture](https://gist.github.com/macromorgan/3e12146a63b53c5b61e4e3c7d4547a0e) 中有 Boot0、BL31、U-Boot 的串口输出；启动后的 pinmux snapshot 显示 PH0/PH1 由 uart0 claim。这支持 UART0 在至少一台 RG34XX 上曾作为实际调试控制台工作，不等于当前测试机的 PCB 焊盘位置或 BL33 可访问性已经验证。

**新增的 SPL→BL31→BL33 静态链证据（2026-11-27）：**

- BaseOS pinned U-Boot v2026.10 H616 `board_init_f()` 在进入 native
  handoff 之前按顺序调用 `clock_init()`、`gpio_init()`、
  `preloader_console_init()`。H616 的 UART0 console-index-1 分支把 PH0/PH1
  设为 UART0 mux（mux value 2），PH1 pull-up；`clock_init_uart()` 将
  APB2 配为 24 MHz、打开 UART gate 并解除 reset；console init 调用
  `serial_init()`。精确 reproduction 的 SPL symbol map 中确实有
  `board_init_f`、`clock_init_uart`、`preloader_console_init` 和
  `serial_init`。生成的完整 `.config` 未随 artifact 发布，因此 console
  index 的 1 是该 U-Boot Kconfig 默认/源代码路径，不能冒充独立 config
  文件读回。
- 目标机 exact vendor monitor（SHA-256
  `676bc0206e3c9a7f229d93ea7f3286095f669940ad91f3c4f3d37bdb18696c31`）
  在 BL33 准备路径调用 UART gate selector：若 SPL 留下已有 gate bit，
  BL31 选择最低已启用 UART、缓存其 base 并跳过 clock/reset/divisor
  重配置；若未启用则走 APB2/16550 fallback（24 MHz / 115200）。
- 同一 BL31 路径还调用 SPC/CCU/PRCM/DMA security setup；其意图是让外围
  资源可被 non-secure world 使用，但 H616 TF-A 明确警告 SPC 写入可能
  需要特定 fuse 才生效。目标 fuse、实际 permission readback 与额外
  `0x030080e0=1` 写入的语义未知。

这补齐了此前只检查 BaseOS `native-boot.c` / `loader.c` 时漏掉的继承
U-Boot SPL 和 vendor BL31 层；但它仍然是 source/target-binary 静态证据，
不是 BL33 入口寄存器或物理引脚的实机读回。详细偏移、寄存器和固定
source pin 见 [UART handoff audit](rg34xx-uart-bl31-handoff-audit.md)。
M1-S1 仍 OPEN。

**当前明确的阻塞：**

- H700 MMU hook 已 host-test 增加第二个 shared Device/XN section `[0x05000000,0x05100000)`，覆盖 UART0 register aperture `0x05000000/0x400`。这只是 ARM translation descriptor，不会访问 UART、启用 clock、改变 reset/pinmux，也不赋予 security/firewall 权限；section-table 粒度同时映射相邻 peripheral address space，但没有驱动会访问它们。
- 当前 GammaOS LK H700 项目仍没有自己的 UART console/driver；先前限定审查的 BaseOS A32 loader 文件没有 UART 初始化，但现在已补齐 inherited U-Boot SPL 与 vendor BL31 的初始化/复用路径，详见上述静态证据。目标设备 TOC1 的 stock-fallback `u-boot` item SHA-256 为 `4d60a22ff13eac00dba89e0ee7df52a49852c0d9d856bec545ce40126191d127`；`/chosen/uboot_message=2018.05(05/26/2026-13:55:09)` 是 BaseOS `native/build.py` 从该 TOC1 item 读取版本字面量并写入 bootargs 的元数据，不是该 U-Boot 执行过的证据（当前活动链由 native SPL 将 A32 loader 放在 BL33 slot）。BaseOS-MultiFrontend pin 确实包含 `rg34xx` prepared-source profile，v2026.10 是其 AArch64 SPL 构建使用的上游 U-Boot 源；但该实验的物理 `LIVE_TARGETS` 只有 RG34XX SP / RG40XX V。当前非 SP 的 native active path 已有现场证据，且 BaseOS workflow #45301188593 已在 host 侧精确重建 generation、loader、firmware、image 与 capsule（capsule byte-exact）；CI 明确未进行 hardware qualification，因此这不关闭 UART/BL33 状态资格；Boot0/BL31 输出串口日志也不能单独证明替换链在 BL31→BL33 时 UART clock、reset、security/firewall 权限保持不变。
- 当前 inventory collector 已新增 `uart-runtime.txt`。2026-11-27 已在目标 RG34XX（非 SP）实机采集并上传完整报告：[reports/rg34xx-inventory-20261127-uart](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-inventory-20261127-uart)，reports HEAD `7217907f57408b714c55a872c6afc5cfc34fdac9`。本机 Linux post-boot view 显示 `ttyS0`→UART0 (`0x05000000`)、115200 console、PH0/PH1 function=`uart0`、pull-up/20 mA drive，以及 `apb2`/`uart0` 均为 24 MHz。`/proc/tty/driver/serial` 不可读，因此没有 divisor/FIFO 回读。dmesg 的六条 `uart0_jtag` / muxsel(3) 告警与参考 DTS 的 PF0–PF5 `sdc0_pins_c` 组合组逐 pin function 映射一致，且 vendor pinctrl driver 对这种组级名称与单 pin 名称差异只发 warning 后继续建图；它更可能不是 PH0/PH1 的 JTAG 冲突。目标机 redacted live FDT 已直接解析出告警组来源 `sdc0@2`（PF0–PF5、group function `uart0_jtag`、muxsel 3），并解析出 UART0 `serial0` alias、0x05000000/0x400 aperture，以及 PH0/PH1 default muxsel 2 / sleep muxsel 7。该结果只证明 FDT 静态声明，不是 live PIO 或 BL33 handoff 状态；详见 UART 专项审计。报告仍不提供物理 pad、电气连线或 BL33 handoff 证明。报告已附 redaction 说明与 SHA256 manifest；采集未访问 `/dev/mem`、未读写 MMIO、未挂载 debugfs、未写存储。M1-S1 仍 OPEN：尚需精确板号/revision 的物理 test-pad/electrical 确认，以及能说明 BL31→BL33 时 UART clock/reset/pinmux/security access 的匹配 handoff source 或可信最早期只读证据。
- 本轮额外审查了固定 BaseOS-MultiFrontend source pin `cbe253a43f46cf551cc9a6c79a84015bde3686cf` 下的 `native/entry32.S`、`native/loader.c`、`spl/native-boot.c`、`spl/vendor-handoff.c` 与 `spl/handoff32.S`：这些限定文件中没有 UART 初始化/控制台调用或 `0x05000000` UART aperture 访问；`loader.c` 中的 `serial` 仅是从 SID 生成设备序列号字符串。该负面结果不覆盖 U-Boot 公共代码、Boot0/BL31、未包含的依赖或目标机实际部署的 SPL binary；部署 SPL 精确构建输入目前也未完全可重建。因此它不能作为 BL33 handoff state proof。 进一步核对发现，目标非 SP TOC1 的 U-Boot item 为 1 MiB、SHA-256 `4d60a22ff13eac00dba89e0ee7df52a49852c0d9d856bec545ce40126191d127`，live FDT `uboot_message` 报告 vendor U-Boot `2018.05(05/26/2026-13:55:09)`；而上述 BaseOS pin 的 SPL README 使用 upstream U-Boot v2026.10，`lab_devices.py` 仅列 RG34XX SP / RG40XX V。故它不是当前非 SP 设备的精确启动源，不能用其 UART 或 DRAM 路径作目标机证明。该 pin 文档中的 vendor-monitor A32 SVC entry `0x4a000000` / monitor `0x48000000` 仅作为同族 ABI 参考，仍需用当前设备实际 monitor/SPL 构建资料匹配确认。

**构建来源状态更新（2026-11-27）：** native generation provenance 已由 BaseOS host-only workflow #45301188593 精确重建，输入/输出哈希与 reference capsule 全部匹配；这使得后续 UART/BL33 分析可以使用精确 pin 的 loader/SPL 来源，不再需要以“build inputs 未知”作为阻塞。CI 仍明确 `hardware_qualified=false`，所以该结果不证明 BL33 入口时的 UART clock/reset/pinmux/firewall 状态，也不提供实机信号授权。详见 [native generation provenance](rg34xx-native-generation-provenance.md)。

因此 UART0 目前是**优先资格化的候选**，不是已批准的 proof-of-life。资格关闭前，不得向 `0x05000000` 写寄存器，不得改变 PH0/PH1 pinmux/clock，也不得将 UART 输出写入诊断 payload。若 UART0 最终不可用，应重新审查替代方案，不能自动改用 PI12、LCD 或 SD。

### 当前不选择的输出

- **PI12 LED：** production PIO wrapper 当前 default-off；BL33 入口 APB1/PIO 权限未闭合。
- **LCD/framebuffer：** framebuffer 与 scanout ownership 尚未向 BL33 批准。
- **SD 单扇区读：** 需要控制器/时钟/pinmux 与 buffer 权限，不能当作无副作用的存活信号。
- **内部 DRAM marker：** 无外部观察者，且会依赖尚未批准的 DRAM scratch 区域，不能构成合格 proof-of-life。

## 4. 必须维持的硬件与执行边界

在 G2/G3 等相关门槛正式关闭之前：

- `H700_MEMORY_LAYOUT_APPROVED=false`；不提供或猜测 `H700_MEMBASE`、`H700_MEMSIZE`。
- 不执行设备 SMC/MMIO，不读取/写入未知 DRAM，不发起 I2C/TWI transaction，不访问 PMIC。
- 不写 SD、Boot0、TOC1，不刷写 native SPL/firmware，不替换用户卡上的 loader。
- 不执行新 BL33 payload；CI、ELF link、package/FNV 检查全部限 host。
- 不启动 CPU1–CPU3，不调用 PSCI CPU_ON，不启用 SMP。
- 不启动 GIC/timer/IRQ、DMA、SD、LCD/PIO、Wi-Fi、Bluetooth、音频或 Linkslot。
- 任何后续设备测试前，必须具体化并审查失败恢复路径；不得假设设备有 eMMC 或自动回退。

`NON-INSTALLABLE / NOT FOR DEVICE` 必须作为诊断产物文件名、报告与 CI summary 的显式标识。任何 host 构建成功都不得被表述为可刷写、可安装或可运行。

## 5. M1 阶段验收

| 阶段 | 内容 | 通过标准 | 允许动作 |
|---|---|---|---|
| M1-S0 | 本规格与边界 | 范围/输出候选/禁止项/验收标准明确 | 文档 |
| M1-S1 | proof-of-life 资格 | 精确板型信号链有可信资料支撑，入口访问权限与 pin/clock 状态可解释 | 只读资料审查、host tests |
| M1-S2 | 最小 payload | A32 entry/vector 与 snapshot tests 通过；ELF/map/BSS/stack 边界有报告；raw ≤ 32768 bytes | host 编译/链接/静态分析 |
| M1-S3 | 配对 package | pinned builder 生成 matched SPL/firmware/capsule；header、FNV、extent、padding 可复验；源树恢复干净 | 隔离 host checkout 构建；产物保持 NON-INSTALLABLE |
| M1-S4 | 设备 proof-of-life | G2/G3 与 handoff/recovery gates 已书面关闭；信号经批准；有本次具体测试的明确授权 | 另行制定、审批的单次设备测试 |

**M1-S0 完成不代表 M1-S1 通过。** 当前实际状态：M1-S0 规格建立；M1-S1 的 UART0 候选未资格化；M1-S2/S3 尚未执行；M1-S4 未授权。

## 6. 退出条件与后续

只有最小 BL33 诊断能产生经资格确认的确定信号，且可观测入口快照可复验，才算完成 M1 的 host/packaging 准备。设备执行仍需要 G2/G3 等 gate 和明确审批。M1 之后按路线图推进 HAL 垂直切片：先输入/显示/SD 读，再在文件系统/flush/断电风险审查后做受控写入。PMIC 完全关机、BL31 suspend/TWI、120 Hz 与 CPU DVFS 不会因 M1 通过而自动解冻。
