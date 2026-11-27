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

### 当前候选：UART0（未批准、不可写）

reports 中的活动 Linux bootargs 包含 `earlyprintk=sunxi-uart,0x05000000` 与 `console=ttyS0,115200`。目标 Linux dmesg 对 pinctrl 的报告包含 `uart0_jtag` 与 `uart0` mux 状态，不能据此推断 BL33 入口的 PH0/PH1 mux、UART clock 或寄存器访问状态。其他 RG34XX 资料中的 UART0/PH0/PH1 仅是交叉线索，不能替代本机板级资料与入口状态证据。

UART0 资格关闭前，不得向 `0x05000000` 写寄存器，不得改变 PH0/PH1 pinmux/clock，也不得将 UART 输出写入诊断 payload。若 UART0 最终不可用，应重新审查替代方案，不能自动改用 PI12、LCD 或 SD。

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
