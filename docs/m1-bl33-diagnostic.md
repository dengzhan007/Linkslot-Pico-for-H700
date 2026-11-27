# M1 — RG34XX AArch32 BL33 诊断启动规格

**状态：** M1 已改为无拆机 / 独立 TF1 测试路线；尚未生成 M1 设备 payload、尚未写入测试卡、尚未执行实机启动。  
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

**M1 首选资格化方向：PI12 工作灯；UART0 不再是前置条件。** PI12 必须有精确 RG34XX 非 SP DTS/source 依据，明确 pin mux、APB1/PIO clock 与 BL33 访问权限，限定寄存器位域，保留 PI11，并具备确定的异常 fail-stop 路径。PI12 可从机身外观察，不需要 UART 焊盘；但 BL33 handoff 时访问权限、abort 处理和实际极性尚未通过。

### PI12 access-gate evidence refinement (2026-11-27)

The exact target BL31 binary has a statically identified pre-BL33 security
setup routine that writes SPC decode-port set registers 0–13 to
`0xffffffff`, the CCU security switch at `0x03001f00`, the R_PRCM
security switch at `0x07010290`, and a DMA security mask. Pinned H616 TF-A
source describes the same family routine as preparing Non-secure peripheral
access, but warns that SPC changes may not take effect unless a fuse is
programmed. The target-specific write at `0x030080e0` remains unexplained.

This is positive static ordering evidence that BL31 attempts to prepare
Non-secure peripheral access before BL33. It does not identify the applicable
SPC decode port for PIO, prove the fuse condition, or establish APB1/PIO clock
and access state at BL33 entry. The latest device inventory did not capture
the relevant fuse. PI12 therefore remains default-off; no unverified SID
offset may be read. Details and pinned references are in
[h700-bl31-pio-ccu-audit.md](h700-bl31-pio-ccu-audit.md).

### 备用候选：UART0（静态证据较多，但不作为首启依赖）

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
  `serial_init()`。精确 reproduction 的 SPL symbol map 中有
  `board_init_f`、`clock_init_uart`、`preloader_console_init`、
  `serial_init`、`eserial1_device` 和 `eserial1_init`，没有 eserial2–6。
  legacy/non-DM `serial_ns16550.c` 仅在 COM1 被定义时生成
  `eserial1_device`，`default_serial_console()` 按
  `CONFIG_CONS_INDEX` 选择该设备；`sunxi-common.h` 将 COM1 映射到
  `SUNXI_UART0_BASE=0x05000000`。Kconfig 对 non-DM SPL serial 默认启用
  `SPECIFY_CONSOLE_INDEX` 且 `CONS_INDEX=1`。这些是强链接/源码推断；
  生成的完整 `.config` 未随 artifact 发布，不能称为直接 config readback。
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
M1-S1 仍 OPEN；FCC 共板声明不构成当前非 SP 实机的 PH0/PH1 焊盘映射或电气连线证据。

**当前明确的阻塞：**

- **板级焊盘/物理连线：** 2026-06-17 制造商签署的 FCC model-difference declaration（document 9400819，SHA-256 `28138824031a0f7e4ff395ce800b8b4bb9062184b114bf60d55884d3130a8bcf`）明确列出 RG 34XX，并声明与 RG34XXSP 等型号共用 PCB/电路/内部；FCC Internal Photos document 9400823 的第 3/4 页提供 SP 样机 H700 主板正反面照片。但附件没有 PH0/PH1 test-pad、TX/RX、GND、网络名或当前非 SP 实机 PCB revision 对照，故这是强共板旁证而非物理焊盘映射，不能关闭物理信号资格。不得直接采用其他 SP 设备的焊点说明。详见 [UART handoff audit](rg34xx-uart-bl31-handoff-audit.md)。
- H700 MMU hook 已 host-test 增加第二个 shared Device/XN section `[0x05000000,0x05100000)`，覆盖 UART0 register aperture `0x05000000/0x400`。这只是 ARM translation descriptor，不会访问 UART、启用 clock、改变 reset/pinmux，也不赋予 security/firewall 权限；section-table 粒度同时映射相邻 peripheral address space，但没有驱动会访问它们。
- 当前 GammaOS LK H700 项目仍没有自己的 UART console/driver；先前限定审查的 BaseOS A32 loader 文件没有 UART 初始化，但现在已补齐 inherited U-Boot SPL 与 vendor BL31 的初始化/复用路径，详见上述静态证据。目标设备 TOC1 的 stock-fallback `u-boot` item SHA-256 为 `4d60a22ff13eac00dba89e0ee7df52a49852c0d9d856bec545ce40126191d127`；`/chosen/uboot_message=2018.05(05/26/2026-13:55:09)` 是 BaseOS `native/build.py` 从该 TOC1 item 读取版本字面量并写入 bootargs 的元数据，不是该 U-Boot 执行过的证据（当前活动链由 native SPL 将 A32 loader 放在 BL33 slot）。BaseOS-MultiFrontend pin 确实包含 `rg34xx` prepared-source profile，v2026.10 是其 AArch64 SPL 构建使用的上游 U-Boot 源；但该实验的物理 `LIVE_TARGETS` 只有 RG34XX SP / RG40XX V。当前非 SP 的 native active path 已有现场证据，且 BaseOS workflow #45301188593 已在 host 侧精确重建 generation、loader、firmware、image 与 capsule（capsule byte-exact）；CI 明确未进行 hardware qualification，因此这不关闭 UART/BL33 状态资格；Boot0/BL31 输出串口日志也不能单独证明替换链在 BL31→BL33 时 UART clock、reset、security/firewall 权限保持不变。
- 当前 inventory collector 已新增 `uart-runtime.txt`。2026-11-27 已在目标 RG34XX（非 SP）实机采集并上传完整报告：[reports/rg34xx-inventory-20261127-uart](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-inventory-20261127-uart)，reports HEAD `7217907f57408b714c55a872c6afc5cfc34fdac9`。本机 Linux post-boot view 显示 `ttyS0`→UART0 (`0x05000000`)、115200 console、PH0/PH1 function=`uart0`、pull-up/20 mA drive，以及 `apb2`/`uart0` 均为 24 MHz。`/proc/tty/driver/serial` 不可读，因此没有 divisor/FIFO 回读。dmesg 的六条 `uart0_jtag` / muxsel(3) 告警与参考 DTS 的 PF0–PF5 `sdc0_pins_c` 组合组逐 pin function 映射一致，且 vendor pinctrl driver 对这种组级名称与单 pin 名称差异只发 warning 后继续建图；它更可能不是 PH0/PH1 的 JTAG 冲突。目标机 redacted live FDT 已直接解析出告警组来源 `sdc0@2`（PF0–PF5、group function `uart0_jtag`、muxsel 3），并解析出 UART0 `serial0` alias、0x05000000/0x400 aperture，以及 PH0/PH1 default muxsel 2 / sleep muxsel 7。该结果只证明 FDT 静态声明，不是 live PIO 或 BL33 handoff 状态；详见 UART 专项审计。报告仍不提供物理 pad、电气连线或 BL33 handoff 证明。报告已附 redaction 说明与 SHA256 manifest；采集未访问 `/dev/mem`、未读写 MMIO、未挂载 debugfs、未写存储。M1-S1 仍 OPEN：尚需精确板号/revision 的物理 test-pad/electrical 确认，以及能说明 BL31→BL33 时 UART clock/reset/pinmux/security access 的匹配 handoff source 或可信最早期只读证据。
- 本轮额外审查了固定 BaseOS-MultiFrontend source pin `cbe253a43f46cf551cc9a6c79a84015bde3686cf` 下的 `native/entry32.S`、`native/loader.c`、`spl/native-boot.c`、`spl/vendor-handoff.c` 与 `spl/handoff32.S`：这些限定文件中没有 UART 初始化/控制台调用或 `0x05000000` UART aperture 访问；`loader.c` 中的 `serial` 仅是从 SID 生成设备序列号字符串。该负面结果不覆盖 U-Boot 公共代码、Boot0/BL31、未包含的依赖或目标机实际部署的 SPL binary；部署 SPL 精确构建输入目前也未完全可重建。因此它不能作为 BL33 handoff state proof。 进一步核对发现，目标非 SP TOC1 的 U-Boot item 为 1 MiB、SHA-256 `4d60a22ff13eac00dba89e0ee7df52a49852c0d9d856bec545ce40126191d127`，live FDT `uboot_message` 报告 vendor U-Boot `2018.05(05/26/2026-13:55:09)`；而上述 BaseOS pin 的 SPL README 使用 upstream U-Boot v2026.10，`lab_devices.py` 仅列 RG34XX SP / RG40XX V。故它不是当前非 SP 设备的精确启动源，不能用其 UART 或 DRAM 路径作目标机证明。该 pin 文档中的 vendor-monitor A32 SVC entry `0x4a000000` / monitor `0x48000000` 仅作为同族 ABI 参考，仍需用当前设备实际 monitor/SPL 构建资料匹配确认。

**构建来源状态更新（2026-11-27）：** native generation provenance 已由 BaseOS host-only workflow #45301188593 精确重建，输入/输出哈希与 reference capsule 全部匹配；这使得后续 UART/BL33 分析可以使用精确 pin 的 loader/SPL 来源，不再需要以“build inputs 未知”作为阻塞。CI 仍明确 `hardware_qualified=false`，所以该结果不证明 BL33 入口时的 UART clock/reset/pinmux/firewall 状态，也不提供实机信号授权。详见 [native generation provenance](rg34xx-native-generation-provenance.md)。

因此 UART0 继续作为**可选备用诊断路径**，不是无拆机首启的前置条件。项目不因缺少 PH0/PH1 物理 test-pad 资料而阻塞 M1；首轮优先资格化 PI12。UART 未经单独批准仍不可写，也不得改变 PH0/PH1 pinmux/clock。

### 首轮输出优先级

1. **PI12 工作灯（首选待资格化）：** 精确 RG34XX 非 SP DTS 将 PMU work_led 映射到 bank I / PI12；现有驱动只改 PI12 DATA/MUX 位并保留 PI11。production wrapper 仍 default-off；BL33 APB1/PIO handoff 权限、访问失败路径和实际极性必须先闭合。
2. **UART0（备用）：** 保留现有静态审计结果，不要求拆机寻找 PH0/PH1 焊盘，也不作为首启阻塞项。
3. **LCD/framebuffer（后续）：** framebuffer/scanout ownership 尚未向 BL33 批准，不进入首轮。
4. **SD / DRAM marker：** 不作为 proof-of-life；首轮不访问存储控制器，也不写外部 DRAM scratch。

## 4. 必须维持的硬件与执行边界

M1 采用**专用介质可写、设备内部状态不改**的分层边界：

- 全局 `H700_MEMORY_LAYOUT_APPROVED=false` 必须保持；不提供或猜测通用 `H700_MEMBASE`、`H700_MEMSIZE`。M1 若需要运行时空间，只能建立独立、可审计的 BL33-slot-only 局部布局，不能批准完整 DRAM 或 Linkslot 应用 arena。
- 允许 host 侧构建、验证独立测试 TF 镜像；实际写卡只限一张可擦写的专用 TF1 测试卡。原系统卡永不作为写入目标。
- 仅当配对 builder、manifest、hash、extent、分区和精确写入范围审计全部通过，才允许将该完整 M1 测试镜像（包括其镜像定义中的 Boot0/TOC1/native package）写入专用测试卡。不得把单独 capsule 或猜测的 sector offset 当作通用刷写命令。
- 不修改 SoC fuse/OTP，不访问 PMIC，不发起 I2C/TWI transaction，不改 CPU PLL/电压，不执行设备 SMC。M1 仅允许在 PI12 路径单独资格化后访问其精确 PIO 位域；不扫描其他外设。
- 不读取/写入未知 DRAM，不使用候选 Linux arena、BOL4、BL31 或未知保留区。首轮只允许经 ELF/map 审计的 BL33-slot 运行区。
- 不启动 CPU1–CPU3，不调用 PSCI CPU_ON，不启用 SMP/GIC/timer/IRQ/DMA/SD/LCD/Wi-Fi/Bluetooth/音频/Linkslot。
- PIO 访问若无法得到确定的异常 fail-stop 行为，则不进入实机测试；不得以尝试其他外设地址作为恢复方式。
- 测试卡启动失败时关机并换回原系统卡；异常发热、异味或供电异常时立即停止。

未完成配对与写入范围审计的产物必须标记 `NON-INSTALLABLE / NOT FOR DEVICE`。只有逐项核验过的测试镜像才可标记 `M1-TEST-TF1-ONLY`，不得称为正式发行镜像。

## 5. M1 阶段验收

| 阶段 | 内容 | 通过标准 | 允许动作 |
|---|---|---|---|
| M1-S0 | 本规格与边界 | 范围/输出候选/禁止项/验收标准明确 | 文档 |
| M1-S1 | PI12 proof-of-life 资格 | 精确板型 DTS、APB1/PIO handoff 权限、abort fail-stop、PI11 保留与可观察极性均有证据 | 只读审查、host tests；不需 UART 焊盘 |
| M1-S2 | 最小 payload | A32 entry/vector 与 snapshot tests 通过；ELF/map/BSS/stack 边界有报告；raw ≤ 32768 bytes；8 KiB early stack 与 16 KiB page table 不重叠 | host 编译/链接/静态分析 |
| M1-S3 | 配对 package | pinned builder 生成 matched SPL/firmware/capsule；header、FNV、extent、padding 可复验；源树恢复干净 | 隔离 host checkout 构建；产物保持 NON-INSTALLABLE |
| M1-S4 | 独立 TF1 无拆机实测 | BL33-slot-only 布局、CPU0-only、PI12 probe/异常路径、配对镜像与恢复流程均通过审查 | 仅写入 M1-TEST-TF1-ONLY 测试卡；不拆机、不焊线 |

**策略调整不代表硬件 gate 已通过。** 当前实际状态：M1-S0 规格已建立；M1-S1 转为 PI12 首选资格化、仍 OPEN；UART0 降为备用；M1-S2/S3 尚未完成；尚无可写入测试卡的配对镜像或实机结果。

### 2026-11-27 host layout evidence (not device approval)

Feasibility [#921](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46018337655)
passed 75/75. Its actual LK early-MMU/PI12 source-layout map records:

- Raw flat ELF: 7,940 bytes; active native loader file extent: 32,768 bytes.
- Linked `_end=0x4a00c024`; runtime span from BL33 base: `0xc024`.
- Pinned LK CRT0 `abort_stack=[0x4a004000,0x4a006000)`, exactly 8 KiB; stack top `0x4a006000`, AAPCS aligned.
- Real AArch32 section table `tt=[0x4a008000,0x4a00c000)`, 16 KiB aligned.
- Stack/table intervals are disjoint with an 8 KiB gap. CI now asserts these symbols, extents, bounds, stack alignment and non-overlap.

This is only a link-map/layout check for the pinned CRT0/MMU objects. The `0x4a000000–0x4a0e0000` linker ceiling remains a researched bound, not a proven reservation or safe physical allocation. Boot0/BL31 access/lifetime, CPU1–CPU3 initial hold-off, APB1/PIO handoff permissions and LED polarity remain unresolved. This evidence does not mark M1-S1/S2/S3 complete and does not make the ELF installable.

## 6. 退出条件与后续

M1 的首个目标是独立 TF1 启动后通过 PI12 给出可观察信号；它只证明最小 BL33 控制流与该 LED 路径，不等于完整 LK、通用 DRAM、应用 arena 或 G2/G3 已获批准。M1 之后按路线图推进 HAL 垂直切片：先输入/显示/SD 读，再在文件系统/flush/断电风险审查后做受控写入。PMIC 完全关机、BL31 suspend/TWI、120 Hz 与 CPU DVFS 不会因 M1 通过而自动解冻。
