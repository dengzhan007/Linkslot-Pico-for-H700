# 项目当前状态与续接记录

> **每次开启新的移植对话，先读本文。** 本文是当前工作状态的交接入口；详细设计仍以各专项文档为准。每完成一轮代码、测试或仓库审查，都应同步更新本文、路线图及对应专项文档。

**最后核对时间：** 2026-11-24 11:43 UTC  
**仓库：** `dengzhan007/Linkslot-Pico-for-H700`  
**工作分支：** `porting/linkslot-1.0.45-feasibility`  
**创建本文前的分支 HEAD：** `760ceecae40cb686ddc74235e3986476ddfd1d17`  
**本轮代码验证基线：** `bdb229e6a3fae8f00f4d3e25d9128e108576250e`

## 1. 项目目标与固定约束

- 目标设备是 **Anbernic RG34XX（非 SP）/ Allwinner H700 / Cortex-A53**，不是 RG353PS/RK3566，也不是 RG34XX SP。
- 主线：**GammaOS LK → H700 AArch32 BL33 → H700 C HAL → Linkslot 1.0.45**。
- 长期语言分工：**C/ARM 汇编负责 LK、启动链和 H700 BSP/HAL；Rust `no_std` 负责 Linkslot 前端业务；C/C++ 静态模拟器核心通过窄 ABI 接入。**
- 首期可玩目标只做 GB/GBC/GBA；首期只运行一个活动核心，采用单核协作式事件循环。
- 不把 Linux Linkslot ELF、Linux `std`/线程/动态 `.so` loader、EGL/ALSA/evdev/sysfs 直接移进 LK。
- 不推测物理内存地址，不写 Boot/SD 启动区，不执行未经验证的 payload；硬件寄存器写入必须有精确板级证据和独立测试。
- `upstream/linkslot` 是不可变源快照；移植补丁在隔离工作树生成，不直接修改上游归档。

## 2. 分支与源码基线

| 分支 | 用途 | 规则 |
|---|---|---|
| `main` | 项目默认/整合分支 | 不把未验证的移植实验直接带入 |
| `porting/linkslot-1.0.36-feasibility` | 旧版本历史验证 | 保留作为历史记录，不作为当前开发基线 |
| `porting/linkslot-1.0.45-feasibility` | **当前唯一移植工作分支** | 新代码、测试、路线图和状态记录都提交到这里 |
| `upstream/linkslot` | 上游源码归档 | 保持只读，不向此分支提交移植改动 |

当前 Linkslot 正式基线为 **1.0.45**，归档 HEAD 为 `3b06d96adadd0375f071555ecb3cc20c3a5b53db`（该 SHA 是本项目归档提交，不是上游 GitCode commit）。根目录 main 与 `test/` 的 852 个源码文件逐 blob 一致；`DEV/` 为 1.0.42，与 1.0.45 有 26 个文件内容不同。详见 [1.0.45 移植基线更新](linkslot-1.0.45-portability-update.md)。

## 3. 当前总体进展

### 已完成并有 CI/主机证据

- 1.0.45 上游审查、三核心 GBA picker 状态机升级、Renderer overlay 基线更新。
- 多个纯业务 Rust crate 已拆为 `no_std + alloc` 形式；输入手势、核心选择、状态转换等有独立回归测试。
- Linkslot Renderer 抽象和 H700 软件 framebuffer compositor 已可在隔离上游工作树编译；尚未接入已授权的真实 scanout buffer。
- Linkslot `slot-retro` 静态核心接口、Libretro VFS v3 ABI、mGBA AArch32 静态 archive/newlib 构建和 host link probes 已完成。
- H700 SD→CMD17→GPT→FAT→VFS 的合成设备/主机端到端只读路径通过；尚未在 RG34XX 上完成真实 sdc0 MMIO 和实体卡读扇区。
- GammaOS LK H700 BL33 overlay 的部分 CRT0/异常向量、Cortex-A53 编译参数和 VBAR 处理通过 host/cross-build 检查；**不是完整 LK 镜像，也不是实机启动证明**。
- CPU OPP 纯 C 选择器已实现，覆盖 Power Saver、Balanced、Performance、Auto、受门控的 Boost、Speed Bin mask、电压/温度上限和 fail-closed 证据门。
- CPU Performance Service C ABI v1 已实现，细节见第 4 节。

### 仍未完成的主要系统门槛

- 没有完整的 GammaOS LK H700 target link/BL33 镜像，也没有实机 LK 启动。
- BL31→BL33 时 CPU1–CPU3 是否停驻尚未证明；在证明之前不能更改共享 CPUX PLL。
- DRAM/reserved-memory/heap/stack/framebuffer 所有权仍未全部闭合；未知区域继续排除。
- 实际 H700 显示扫描输出、GPIO/ADC 输入 producer、硬件 timer、PCM DMA audio、RTC、关机和真实存储写入服务尚未形成完整运行时。
- 无经 RG34XX 实机验证的 CPU OPP 表、PMIC 调压流程、CPU PLL 切换流程或温控闭环。

## 4. 当前 CPU 调频切片（本轮重点）

### 已有代码

- `platform/h700/power/h700_cpu_performance_policy.h/.c`：纯策略选择器，不访问 MMIO/PLL/PMIC。
- `platform/h700/power/h700_cpu_performance_service.h/.c`：版本化 C Service ABI v1。
- `platform/h700/power/tests/h700_cpu_performance_service_test.c`：服务 ABI、策略门控、遥测分离、温度 clamp、Boost gating 和错误 OPP 测试。
- `scripts/test-h700-cpu-performance-service.sh`：主机测试入口。
- `scripts/build-h700-objects.sh`：已将 Service C 源纳入 AArch64、AArch32 ARM-state、AArch32 Thumb-2 编译矩阵。
- `.github/workflows/h700-arch-check.yml`：Push / Pull Request 均纳入服务 host regression 和架构编译。

### ABI v1 语义

- C 服务持有调用方提供的固定大小 OPP 表；不使用 heap、线程或动态分配。
- 模式请求经纯 C selector 选择目标 OPP。
- 成功返回 `H700_CPU_PERF_SERVICE_POLICY_ONLY`：**仅表示策略目标已选出，不表示真实 CPU 频率/电压已改变。**
- 状态结构把 `planned_frequency_hz/planned_voltage_uv` 与 `actual_frequency_hz/actual_voltage_uv/temperature_mc` 分开。
- `hardware_applied` 在 ABI v1 中固定为 0。没有实际硬件读回时，不得把计划频率显示为实测 MHz。
- 更新温度/电压/频率限制后，当前有效请求会重新经过 policy selector。
- 当前测试使用合成 OPP 值，不是 RG34XX 已批准的运行频点表。

### 本轮 CI 验证

- [H700 AArch32/AArch64 compile check #45386479380](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45386479380)：**成功**。两个 jobs 均成功：
  - `storage-host-regression`：包含 CPU Service v1 host runner 和现有只读存储回归。
  - `compile-freestanding-objects`：包含新 CPU Service C 源的 AArch64、AArch32 ARM-state 和 AArch32 Thumb-2 编译。
- [此前完整 Linkslot 1.0.45 feasibility #45381590503](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45381590503)：成功；这是旧的 69-step 综合基线，证明 host/cross-build 范围，不证明实机运行。
- [CPU OPP policy 专项 #45381349400](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45381349400)：成功。

## 5. H700 CPU DVFS 硬件证据和安全门槛

已确认/交叉验证：

- RG34XX 现场 Linux 日志识别 PMIC 为 **AXP2202**，I²C-5、地址 `0x34`。
- Knulli 与 MustardOS 两份 RG34XX 专用 DTS 都把 `cpu@0 cpu-supply` 指向 AXP2202 DCDC1；这是强交叉证据，但本机 live-FDT 的完整 regulator phandle/subtree 尚未导出核实。
- 已找到 SID speed cell 参考位置 `0x03006000 + 0x00`（2 bytes）和 thermal calibration cell `0x03006000 + 0x14`（8 bytes）；本机原始值及 SoC revision 语义尚未读取。
- 现场内核报告 H700 thermal sensor 存在，但 **AVCC is not calibrated**，因此不能据此开放自动温控升频/Boost。
- H616/H700 CCU 源码可提供 PLL/mux 初始参考；精确 RG34XX CCU base、安全 parent-switch/lock sequence、divider 和计时连续性尚未完成板级核实。
- CPU1–CPU3 的 BL31→BL33 状态未知；CPU PLL 是共享时钟域，不是每个模拟器核心各自独立的频率。

**禁止事项：** 不复制 MT6785 的 PLL 地址/PCW 算法，不把 GammaOS Pico 600–2000 MHz 档位直接套到 H700，不使用 AXP717 参数替代 AXP2202，不把 H616 OPP 表当作 RG34XX 已批准 OPP，不在缺少次级 CPU 状态证明时写 CPUX PLL，不在缺少电压/热证据时升频。

## 6. 下一步任务（按顺序执行）

1. **实现 Rust `no_std` 侧的 CPU Service ABI adapter。** 把模式/状态映射为 Rust 类型，保留 policy-only 与 actual telemetry 的区别；增加 ABI layout、错误码、状态转换和 AArch32/AArch64 编译回归。
2. 将 Linkslot 设置/quick menu 的 CPU 模式请求映射到该 adapter；硬件未接入前只显示请求模式和“频率遥测不可用”，不显示伪实测频率。
3. 增加 RG34XX 只读硬件 inventory：导出完整 live-FDT regulator 子树、SID 原始 speed-bin、thermal calibration、CPU rail 映射和当前 clock/PMIC 状态。优先使用 Linux/已有只读工具取证，不在 LK 侧猜地址。
4. 证明 BL31→BL33 CPU1–CPU3 状态和安全的单核 invariant；审计 CPUX PLL parent-switch/lock/分频及 timer continuity。
5. 只有前述证据闭合后，才实现 C HAL 只读 telemetry 与最终受控硬件 transition；升频先升压，降频先降频后降压，必须有 readback、超时、故障回滚、热 clamp、滞回和实机持续负载验收。
6. 最后才开放实际 Performance/Boost。不得把 ABI v1 的 `POLICY_ONLY` 当作 DVFS 已完成。

## 7. 续接规则

新的工作轮次按以下顺序恢复上下文：

1. 先读取本文和 GitHub 当前分支 HEAD / 最新 Actions 状态；本文的 HEAD 是核对时快照，不应假设之后没有新提交。
2. 当前代码只在 `porting/linkslot-1.0.45-feasibility` 上推进；先检查分支差异与最新 CI，不重复建立已有分支/文件。
3. 依任务读取专项文档：
   - CPU 调频：本文 + [H700 CPU Performance Service / DVFS](h700-cpu-performance-service.md)
   - 总体路线： [roadmap](roadmap.md)
   - Linkslot 1.0.45 源码差异： [1.0.45 portability update](linkslot-1.0.45-portability-update.md)
   - LK/H700 runtime 架构： [Linkslot H700 runtime architecture](linkslot-h700-runtime-architecture.md)
   - BL33/内存交接： [RG34XX boot handoff audit](rg34xx-boot-handoff-audit.md)
4. 只把 CI 明确通过的提交标为已验证；host compile 不等于实机启动，合成 OPP 不等于真实可用频点。
5. 每轮完成后先更新本文，再更新 roadmap 和对应专项文档；把新 commit SHA、Actions run URL、失败及尚存风险写入本文。
