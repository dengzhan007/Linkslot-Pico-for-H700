# RG34XX 无拆机 / TF1 专用实机 Bring-up 方案

**决策日期：** 2026-11-27  
**状态：** 已确定为首选路线；尚未生成 M1 设备 payload、尚未写入测试卡、尚未进行实机启动测试。  
**目标：** 不拆开 RG34XX、不焊接 UART，通过独立 TF1 测试卡验证最小 AArch32 BL33。

## 1. 设备与风险边界

目标设备 RG34XX（非 SP）通过 TF 卡启动，没有板载 eMMC 系统盘。当前已核对的活动链为：

Boot ROM → eGON Boot0 → BaseOS native AArch64 SPL → vendor BL31 → AArch32 SVC BL33。

这改变的是**启动介质失败后的恢复方式**：只要试验仅修改可拆卸 TF 测试卡，启动失败后关机、换回原系统卡即可恢复原有启动环境。它不代表设备绝不会损坏；异常供电、错误电压/时钟、PMIC/TWI 写入、SoC fuse/OTP 或不受控的外设访问仍然存在板级风险。

执行原则：

- 不拆机、不焊接、不为首次启动寻找 PH0/PH1 UART 焊盘。
- 原系统 TF 卡取出并单独保管，永不作为开发写入目标。
- 只使用另一张可擦写的独立 TF1 开发卡。
- 不访问 SoC fuse/OTP，不发起 PMIC/TWI transaction，不改变 CPU PLL/电压。
- 不因无 eMMC 而放宽未知 DRAM、任意 MMIO 或外设权限限制。

## 2. 首发 proof-of-life：PI12 工作灯

首选通过机身外部可见的工作灯验证 BL33 已执行：

- RG34XX 非 SP 启动 DTS 将 PMU work_led 映射到 PIO bank I / PI12；low_pwr_led 映射到 PI11。
- 仓库已有 platform/h700/power/h700_rg34xx_power_led.c，驱动只更新 PI12 DATA bit 12 和 MUX1 对应位域；read-modify-write 保留 PI11 与其他 GPIO 位。
- host fake-register 测试已覆盖写入顺序、位保留、读回失败和默认关闭行为。
- 当前物理 MMIO wrapper 默认关闭。BL33 入口时 APB1/PIO clock、non-secure access/firewall 与 abort 处理尚未资格化，因此不能直接启用宏。
- RG34XX 的 LED 实际极性仍须通过不拆机的冷启动观察确认；同族 active-high 资料仅作为旁证。

首轮成功指示应固定为肉眼可辨、可重复的简单亮灭/短闪状态，不依赖尚未批准的 Generic Timer、GIC、IRQ、DMA 或调度器。

UART0 降级为可选备用。已有 PH0/PH1 静态配置和 SPL/BL31 handoff 证据，但没有必要为首次启动拆机寻找测试点。LCD/framebuffer 因 ownership 未批准而延后；SD 不是首启可视信号。

## 3. 首轮 payload：CPU0-only、仅限 BL33 slot

活动 A32 loader 位于 0x4a000000，输入文件 extent 为 0x8000 bytes（32 KiB）。这是文件长度限制，不是整个 DRAM 的使用许可。

首轮诊断 payload 必须：

- 使用 AArch32 SVC entry；raw payload 不超过 32 KiB。
- 在 ELF/map 中逐项审计代码、向量、BSS、stack、页表及异常路径，所有实际运行区必须处于经审查的 BL33 slot 局部范围。
- 不使用 Linux kernel arena [0x40080000, 0x42000000)、BOL4 staging [0x45000000, 0x46000000)、BL31 [0x48000000, 0x49000000) 或未知保留区。
- 不启用 heap、Linkslot、mGBA、应用加载、文件系统、SD controller、GIC/timer/IRQ、DMA、SMP、音频、Wi-Fi、Bluetooth 或 DVFS。
- 不调用 PSCI CPU_ON，不启动 CPU1–CPU3，不改 CPU PLL/电压。
- PIO 访问只允许涉及 PI12 所需的精确寄存器位；如果 Data Abort 不能被安全界定并恢复为 fail-stop，则禁止进入实机测试。

全局 H700_MEMORY_LAYOUT_APPROVED=false 必须保持不变；不得提供通用 H700_MEMBASE/H700_MEMSIZE。M1 只可建立一个独立的 BL33-slot-only 局部布局审计，不能替代完整 LK、Linkslot、heap、mGBA 或 DMA 的内存批准。

### 已测量的 host-only BL33 内部布局（2026-11-27）

Full feasibility #921 passed 75/75 and its actual LK early-MMU/PI12 link map
records the following current layout:

| Region | Interval | Size |
|---|---|---:|
| LK early/abort stack | `[0x4a004000,0x4a006000)` | 8 KiB |
| Initial stack top | `0x4a006000` | aligned |
| Gap | `[0x4a006000,0x4a008000)` | 8 KiB |
| AArch32 section table | `[0x4a008000,0x4a00c000)` | 16 KiB |
| Remaining linked BSS | ends at `0x4a00c024` | `_end=0x4a00c024` |

The raw ELF is 7,940 bytes, while its runtime span through `_end` is
`0xc024` bytes. CI now reads the actual `abort_stack`,
`abort_stack_top`, and `tt` symbols, validates the pinned 8 KiB stack
size, stack alignment, stack/table containment and disjointness. This removes
an implicit layout assumption from the host test.

**Important limit:** these intervals are inside the candidate BL33 address
window, but static linking cannot establish that opaque Boot0/BL31 code,
secure services or DMA never use them. The check does not approve the
physical region and does not close the global memory gate. It only proves
that the current pinned LK objects do not overlap each other in this
specific host-linked layout.

## 4. 独立 TF1 测试卡流程

### 写卡前

1. 原系统卡取出并单独保管。
2. 识别独立测试卡的设备节点、容量与介质身份；保存测试卡原始镜像及 SHA-256。
3. 使用固定 BaseOS-MultiFrontend builder commit cbe253a43f46cf551cc9a6c79a84015bde3686cf，保留原始 prepared bundle、Boot0、TOC1、firmware/template 的全部来源 hash。
4. 构建匹配的 M1 payload、native firmware、SPL/capsule 与最终 TF 镜像。
5. 在 host 侧检查 GPT/分区、精确写入区间、header、32 KiB loader extent、FNV、generation、padding、manifest 与 SHA-256；逐项审查写入目标。未完成这些检查前，不提供通用 dd 或原始 sector 写卡命令。

### 首轮实机

- 只把经审计且明确标记为 M1-TEST-TF1-ONLY 的完整配对镜像写入独立测试卡。
- 插入 TF1，按设备正常 SD 启动方式上电；不拆机、不接 UART。
- 肉眼观察 PI12 是否产生 M1 规定的成功指示，并记录视频/观察结果、测试镜像 SHA 与设备启动行为。
- 成功仅证明最小 BL33 控制流及 PI12 输出路径；不代表完整 LK、显示、输入、存储、Linkslot 或模拟器通过。
- 若无灯、黑屏或卡住，关机后移除测试卡并换回原系统卡；失败卡在主机重新制作。

### 停止条件

出现异常发热、异味、供电异常或任何疑似硬件故障，立即停止上电试验。不得因可换 TF 卡而尝试未知 PMIC、时钟、电压、fuse/OTP 或外设访问。

## 5. 里程碑

| 阶段 | 目标 | 通过标准 |
|---|---|---|
| M1-T0 | PI12 handoff/权限/异常资格 + BL33 slot 局部布局 | 精确 PIO 位域、APB1/PIO 可访问性证据、abort fail-stop 与 map 审计边界明确 |
| M1-T1 | 最小 A32 PI12 proof payload | entry/vector/异常路径通过；ELF/map/BSS/stack 审查；raw ≤ 32 KiB |
| M1-T2 | 匹配 native package | 同一代 payload/firmware/SPL/capsule/TF image，所有 hash、extent、FNV、generation 一致 |
| M1-T3 | 独立 TF1 无拆机验证 | 以 PI12 确认 BL33 入口；失败可换回原系统卡 |
| M1-T4 | 扩展完整 LK | 补齐 GIC/timer、heap、显示、输入、存储等启动必需服务 |
| M1-T5 | 接入 Linkslot/模拟器 | 独立应用 DRAM arena 获得单独批准后再推进 |

**当前实际位置：** native package provenance 已在 host 侧精确复现；PI12 host service/tests 已存在；完整 H700 LK target、M1 设备 payload、最终 TF 镜像和 TF1 实测均未完成。当前下一项是 M1-T0，不是直接启用物理 PIO wrapper。

## 6. 关键参考

- M1 诊断规格：docs/m1-bl33-diagnostic.md
- PI12 生命周期：docs/h700-power-led-lifecycle.md
- BL33 内存所有权：docs/h700-bl33-memory-ownership.md
- native generation provenance：docs/rg34xx-native-generation-provenance.md
- BL33 image format：docs/h700-bl33-image-format.md
