# 项目当前状态与续接记录

> **每次开启新的移植对话，先读本文。** 本文是当前工作状态的交接入口；详细设计仍以各专项文档为准。每完成一轮代码、测试或仓库审查，都应同步更新本文、路线图及对应专项文档。

**最后核对时间：** 2026-11-25（已分析 CPUX inventory v2；新增 RG34XX PI12 电源灯早期驱动与 Linkslot 游戏生命周期联动，CI/实机验证待完成）  
**仓库：** `dengzhan007/Linkslot-Pico-for-H700`  
**工作分支：** `porting/linkslot-1.0.45-feasibility`  
**最近一次成功 CI 验证的代码基线：** `0e33da4a50b28583aac8c7e815dac7857b6ef43a`（完整综合 workflow #45428664653，71/71 steps；早于本轮只读 inventory 脚本增强）。**本轮 collector/CI 配置提交：** `96afe6d2fff811eab4edfa87b3f58606bdaf645c`，含 read-only clock-summary 摘录、静态安全测试和 workflow path/test 接入；随后又有 DVFS 专项文档、路线图和状态入口同步提交。对应 push workflow 尚未取得可核验结果。  
**本轮 CPU runtime owner 最新代码提交：** `072793f8b4bfce00683aa3d303eee62613fe41ab`（7 个连续提交）  
**本轮 Rust runtime bridge 最新代码提交：** `0d381d7ba66592b5ab64ba67e0b228039bb9eec0`  
**本轮 H700 build graph / guard 最新代码提交：** `6e847c0121431c681062053c52f28824715e78a6`（platform OBJS）与 `ef07a0e06c1d55e2d2f49175d4e976e8337e1ee4`（project guard）。  
**本轮 H700 target lifecycle hooks：** `target/rg34xx_h700/init.c` / `target/rg34xx_h700/rules.mk`；三种目标 ISA 编译由 #45392910394 验证通过。  
**本轮 CPU inventory/FDT parser：** 解析器与 provider argument 测试由 #45396816432 通过。  
**本轮 CPU power collector missing-state 增强：** `273895925554446bff3eb3f61ca1dde093d15d03`；对应 safety regression 提交 `f2ea2b9d5611b5461546bc4f07fe014f9b4d80b2`，两平台 CI 均通过。  
**本轮 SID 窄读取与解析器：** `tools/inspect_h700_sid_cells.py` / `tools/test_inspect_h700_sid_cells.py`；PowerShell runner 输出 `sid-cell-report.json`；Linux/Windows CI #45399902110 成功。  
**本轮 SoC revision 取证边界：** collector 只探测通用 `/sys/devices/soc0/{machine,family,soc_id,revision}`；不调用 vendor `sunxi_get_soc_ver()` / `sunxi_info`，因为固定 `sun50iw9` 驱动路径可能短暂写 CE CCU clock/reset 且 sys_info 暴露 serial/chipid。新 safety regression #45401913670 与最新版 inventory audit #45402146927 均通过。  
**本轮 SID JSON SoC identity：** `sid-cell-report.json` 现附带 generic SoC bus 的 machine/family/soc_id/revision 原始字符串（可用时），仅用于识别，不参与 OPP 放行；相关 host/Windows tests #45402936876、#45402977130 与 inventory audit #45403070519/#45403377522 均通过。  
**本轮 mainline speed-bin reference decode：** 固定上游 Linux `8d7fe9885c62fc9a95bf55d9db244d9672a8aadc` 的候选映射现加入 `sid-cell-report.json`，但所有结果仍 `opp_authorization=not_approved`；#45404785874/#45405024561 已通过。  
**本轮 Linkslot/mGBA 静态闭环：** #45413653400 成功（70/70）；真实 Linkslot 1.0.45 `slot-retro::Api` Rust `no_std` AArch32 staticlib、21 个 mGBA namespaced exports、真实 mGBA VFS v3 archive、H700 newlib adapter 与 `newlib/libm/libgcc` 最终 ELF32/ARM 链接通过；artifact `14929808289`。该探针不执行核心，也不是完整 LK/固件/设备验证。  
**本轮 CPU Performance QuickRow App regression：** 两条 workflow 都实际执行 `h700_cpu_performance_quick_row_tests`，2 passed / 0 failed；runtime-slices #45428654903 成功（20/20），完整 feasibility #45428664653 成功（71/71）。测试验证 unavailable 显示、左右请求不改变值、不创建持久化配置。
**本轮 RG34XX CPUX live inventory（采集于 2026-11-24 23:37:46 UTC）：** 已分析 `reports/rg34xx-inventory-20261125-cpux/`。live FDT phandle `0x4f` 将 CPU0 `cpu-supply` 指向 AXP2202 DCDC1；Linux regulator debugfs 报告 cpu0 1160 mV。CPU0–CPU3 在 Linux 中属于同一个 cpufreq policy 且全部 online；当前 Linux clock tree 为 `pll_cpu → cpu → cpuapb`（1512/1512/378 MHz），AXI 756 MHz，独立 `pll_periph0` 600 MHz。首轮 cpufreq 与 clk_summary 分时读取，不能据此认定 provider 频率不一致。SID speed raw `0x6c00`；mainline candidate bin 5 仅为诊断，不是 OPP 授权。不同厂商 4.9.170 源对 `0x6c00` 分别映射 mask 4 / mask 1，4.9.191 另有长度检查的 mask 1 实现，当前运行内核来源仍未知。温度 AVCC 未校准。完整结论：[RG34XX CPUX live read-only audit](rg34xx-cpux-live-audit-2026-11-25.md)；原始文件位于 [reports 分支](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-inventory-20261125-cpux)。
**本轮 CPUX vendor-source audit（2026-11-25）：** 固定审查 `qiaoweibiao/T507_Kernel@98fcc7d3f112e51b0edfb71536da89cb2115106f` 的 `sun50iw9` clock driver；源码给出 `PLL_CPU=0x0000`、`CPU_CFG=0x0500`、`pll_periph0` pre-rate parent switch / `pll_cpu` post-rate restore、PLL lock 新模式以及 PLL bit31 boot-enable / bit27 output-gate 约束。`LOCKBIT(x)` 受 EVB build config 影响；厂商 CPU factor table 自身标注不正确，禁止用作 RG34XX OPP。审查报告：[H700 CPUX clock-source audit](h700-cpux-clock-source-audit.md)。**仅源码审计，不是目标板寄存器读回，不关闭 DVFS 硬件门，也没有新增 CI 结果。**
**本轮 Linux 4.9.170 cpufreq/OPP 源码核查：** 两个相同版本的公开 vendor tree 对 raw `0x6c00` 的映射不同：T507 候选无 case、default=`supported_hw=0x4`；Lineage 候选显式映射 `supported_hw=0x1`。两者都以 `u32 *` 读取 2-byte SID cell 且不校验 `len`。Creality 4.9.191 候选映射为 mask 1，但使用长度检查的 byte-wise read，不能证明目标 4.9.170 binary 的实现。版本号不够识别 running source，任何 mask 均未授权。
**本轮 OPP parser-v1 报告边界：** 现有上传的 `live-fdt.json` 不能用于声称某些 OPP 没有电压；它未包含 `opp-microvolt-a1/b0/c*` 命名属性和 OPP table-level speed NVMEM reference。parser-v2 已补齐这些字段，必须用设备原始 live FDT 重新运行采集器后再计算 OPP mask/property suffix。
**本轮只读 clock inventory 变更：** `scripts/collect-rg34xx-readonly-inventory.sh` 在原有完整 `clk_summary` 输出外，新增保留缩进的 `pll_cpu` / `pll_periph0` / `cpux` / `cpu` 重点摘录；`tools/test_inventory_script_safety.py` 增加只读回归，`.github/workflows/h700-arch-check.yml` 将两个文件加入 push/PR 路径并执行该测试。新摘录只读 Linux debugfs 文本，不直接读取 `PLL_CPU` / `CPU_CFG` MMIO。摘录片段单独通过 `sh -n` 语法检查；完整 Python 回归/Actions 尚未取得可核验执行结果。相关提交链：collector 摘录 `db9e190619b149901b817b67d899c0e937891190` → printf 格式修正 `c30dceea131f2a1cf168bdf08beac36884ce52a3` → 静态安全测试 `aad8671ece7f39124a58aa659c54bc3a2c0ff661` → workflow 接入 `96afe6d2fff811eab4edfa87b3f58606bdaf645c`；随后同步 DVFS 文档、路线图与状态记录。
**本轮 FDT OPP 解析器增强：** 旧 `live-fdt.json` 仅包含 base `opp-microvolt`，未解析 `opp-microvolt-*` 命名电压属性，也未输出 `opp_l_table` 自身的 `nvmem-cells` / `nvmem-cell-names`。`tools/inspect_h700_fdt.py` 现新增 `voltage_microvolt_variants`、OPP table NVMEM refs/name；合成回归覆盖 a1/b0 suffix 与 speed-bin phandle。提交：parser `fc84576d0d597abb6317ac64fefd62cf25dd63dc`、test `0ee82a5709418b8f81120cd337a75b8d237052ee`、workflow `9cbceb738ac2a8802784d0eac6ae6f4f2c5e1a5c`、fixture NUL 修正 `c5bd42ca547db2c49ec2018946275cd457ff24ee`。新增 Python/Actions 尚未取得可核验运行结果；须重新运行 inventory 生成 parser-v2 `live-fdt.json`。
**本轮同步频率采样增强：** 采集器现执行六轮、间隔 200 ms 的 uptime-bracketed 采样，分别读取 `policy0/cpu0` 的 `scaling_cur_freq`、`cpuinfo_cur_freq` 及同轮 CPUX `clk_summary` 行。静态安全测试增加该路径无 sysfs 写入的断言；workflow 已包含此 test path/command。采样脚本和回归源码已提交，完整 Python/Actions 结果尚未取得。
**同步采样代码提交：** collector `2726db1f2b4e356e632b958027cb11f9bf42bf49`（blob `94cce9b7a0a90a28a7e427cfb84d2a9f6be540b8`）；新增静态只读测试 `51ed3bae2fc24e88fc803b2e3e143fe5df8ba495`（blob `c320daec1fc6a970b19301aa61b0003b797409ae`）。采样 shell 片段与 Python 测试方法的语法检查通过；GitHub commit combined status 目前未返回 checks，完整测试/Actions 仍待确认。


**本轮 CPUX inventory v2：** 报告已上传至 [reports/rg34xx-inventory-20261125-cpux-v2](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-inventory-20261125-cpux-v2)，采集时间 2026-11-25 00:41:21 UTC。parser-v2 已解析 `/opp_l_table` 的 table-level SID `speed` 引用及 `opp-microvolt-a1/b0/b1/c0/c1/c2` 命名电压；启动时 600/792/1032/1296/1344 MHz 五档因电压属性未命中被内核丢弃。八组 150 ms 样本中 `scaling_cur_freq` 连续至少 1.1 s 显示 480 MHz，而 `cpuinfo_cur_freq` / `pll_cpu` 为 1008/1104 MHz，DCDC1 为 950/1000 mV；仍需区分 policy/IPA 限制与 clock-driver 拒绝。SID raw `0x6c00` 未获 OPP mask 授权，AVCC 热校准仍未通过。

**本轮 RG34XX 电源灯实现：** 原厂 RG34XX（非 SP）设备树与 MustardOS RG34XX-H 配置均映射 `work_led=PI12`、`low_pwr_led=PI11`；仅控制 PI12。新增 `platform/h700/power/h700_rg34xx_power_led.c`，上电默认点亮；`target_early_init()` 早期点亮，`target_init()` 失败重试。新增 PIO 1 MiB Device MMU window mapping。Linkslot 1.0.45 `Session` 在 Playing/Polaroids 状态熄灭、退出后点亮；no_std 状态机仅在状态变化时调用 C ABI。暂不增加前端开关。提交：driver `af1922d0cecec278fc7bf2d81d63e4e621ab8685`、Session state `3cbc92669aac23b3e702c9518573d37adcd2fa54`、PIO mapping `c3a233e6f20defaf2ab0192f0de54f5c70921464`。host/Actions 与实机极性、启动时机仍待验证；完整 LK build 仍受独立内存批准 gate 约束。

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
- Linkslot `slot-retro` 静态核心接口、Libretro VFS v3 ABI、mGBA AArch32 静态 archive/newlib 构建和 host link probes 已完成；另已把真实 Linkslot Rust `Api` 表与真实 mGBA archive/newlib adapter 链接为无未解析符号的 AArch32 ELF（仅链接探针，不执行核心）。
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

### 最新 Linkslot 1.0.45 综合验收（2026-11-24 15:48 UTC）

- [H700 Linkslot runtime-slices #45428654903](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45428654903)：**成功，20/20 steps**。no_std business + Linkslot integration job 13/13，C input service ABI + queue job 7/7。
- [Linkslot LK Rust feasibility #45428664653](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45428664653)：**成功，71/71 steps**，代码 HEAD `0e33da4a50b28583aac8c7e815dac7857b6ef43a`。artifact [14937338188](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45428664653/artifacts/14937338188) 已上传，约 387 MB，包含 maps、ELFs、日志和报告。
- 覆盖 GammaOS LK H700 BL33 overlay/CRT0/exception vector、native BL33 header contract、memory/project fail-closed guard、H700 software renderer/SoftwareCompositor、完整 Linkslot 1.0.45 frontend + no_std business crates、Input ABI/queue、slot-retro VFS v3/static API、mGBA static Rust binding/前端选择、SD→GPT→FAT16/32/exFAT→VFS host path，以及 mGBA AArch32/newlib 构建和链接。
- **CPU QuickRow App tests 现已真实执行：** `cargo test --manifest-path .ci/upstream-linkslot/Cargo.toml -p slot --lib --no-default-features --features device,h700-software h700_cpu_performance_quick_row_tests -- --nocapture`；两项测试 `cpu_performance_row_is_visible_but_explicitly_unavailable` 和 `unavailable_cpu_performance_request_does_not_write_to_storage` 均通过（2 passed / 0 failed，486 filtered out）。runtime-slices 与 full feasibility 两条 workflow 都包含此步骤。
- 第一次新增测试的 run [#45428037808](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45428037808) 曾失败，因为默认 `cargo test -p slot` 额外编译了上游 `tests/core_picker.rs`，该旧 integration test 与 1.0.45 `CorePicker<C>` 泛型 API 不匹配（E0107/E0308），不是本次 CPU App 测试失败。命令改为 `--lib` 后，[runtime-slices #45428654903](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45428654903) 和 [full feasibility #45428664653](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45428664653) 均成功。
- 真实 Rust `slot-retro::Api` `no_std` AArch32 staticlib、21 个 mGBA namespaced exports、真实 mGBA VFS v3 archive、H700 newlib adapter 和 `newlib/libm/libgcc` 最终 ELF32/ARM `--no-undefined` 链接通过；BL33 header/vector layout probe 通过。mGBA ELF 仍未执行，BL33 image 仍只是 tiny `kmain`/exception stub 布局探针。
- 这些结果仍是 host/cross-build 验收，不是完整 LK target link、可刷写固件或 RG34XX 实机启动。
### 现阶段仍然未完成

- 没有完整 GammaOS LK H700 target link、可刷写 BL33 image 或实机启动。
- newlib adapter 虽已参与 ABI 级最终链接，但其 LK heap/console/RTC 回调仍未在目标启动路径安装；文件 I/O VFS 与核心启动生命周期尚未接入完整 LK App。
- 没有执行 Rust+mGBA ELF、没有 ROM gameplay、无视频/音频/输入/存档实机验证；当前工作只证明 AArch32 静态符号闭合。
- CPU runtime/真实 DVFS 仍受第 4–6 节的硬件证据门控约束，不因 mGBA/Linkslot CI 成功而改变。
## 4. 当前 CPU 调频切片（本轮重点）

### 已有代码

- `platform/h700/power/h700_cpu_performance_policy.h/.c`：纯策略选择器，不访问 MMIO/PLL/PMIC。
- `platform/h700/power/h700_cpu_performance_service.h/.c`：版本化 C Service ABI v1。
- `docs/h700-cpux-clock-source-audit.md`：固定 `sun50iw9` vendor 时钟驱动审查，记录 PLL/mux 偏移、parent-switch 与 lock 参考序列、bit31/bit27 安全约束和 factor table 不可信限制；只作源码证据，不提供 MMIO 实现。
- `scripts/collect-rg34xx-readonly-inventory.sh` 现额外输出 CPUX clock-summary 摘录；静态安全测试接入 H700 workflow，结果待确认。
- `platform/h700/rules.mk`：已把 policy selector、C service ABI 和 static runtime owner 三个对象加入 LK `OBJS`；`scripts/test-h700-lk-project-guard.py` 检查对象项和对应 C 源文件。该项只建立 build graph，不代表完整 LK target 已链接。
- `target/rg34xx_h700/init.c` / `target/rg34xx_h700/rules.mk`：提供 LK `kernel/main.c` 必须调用的 `target_early_init()` 与 `target_init()`，两者当前为空实现；`scripts/build-h700-objects.sh` 已将其加入 AArch64/AArch32 ARM/Thumb 编译矩阵。H700 overlay 排除 MTK generic `target/init.c`，project guard 验证 MTK boot app 不进入 H700 project。
- `platform/h700/power/tests/h700_cpu_performance_service_test.c`：服务 ABI、策略门控、遥测分离、温度 clamp、Boost gating 和错误 OPP 测试。
- `scripts/test-h700-cpu-performance-service.sh`：主机测试入口。
- `scripts/build-h700-objects.sh`：已将 Service C 源纳入 AArch64、AArch32 ARM-state、AArch32 Thumb-2 编译矩阵。
- `.github/workflows/h700-arch-check.yml`：Push / Pull Request 均纳入服务 host regression 和架构编译。
- `integration/linkslot/h700-frontend-core/slot-cpu-performance`：新建 Rust `no_std` typed client；C service 作为不透明指针传入，模式通过固定枚举映射，C ABI v1 的 64-byte state 结构按 `repr(C)` 解码。
- Rust 客户端新增 `CpuPerformanceRuntime` token：通过 C runtime status/getter ABI 获取静态 owner；在 runtime 尚未 ready 时返回独立的 `RuntimeUnavailable(status)`，并通过 `&mut self` 发放临时 client，减少 Rust 侧重复可变访问。获取 token 是 unsafe 操作，要求 C 启动已成功、生命周期内不得重新初始化 owner，且不得存在其他 C/Rust 写入者。
- `platform/h700/power/h700_cpu_performance_runtime.h/.c`：新增静态 H700 runtime owner，提供单实例生命周期、RG34XX board profile ABI、四种用户策略预检和 fail-closed accessor；无有效 profile 时返回 unavailable/NULL。
- `platform/h700/power/tests/h700_cpu_performance_runtime_test.c`：覆盖默认 unavailable、NULL/未批准/错误 board/缺少 gate/缺少 OPP 拒绝、静态 owner 稳定性、policy-only 状态和服务撤销。
- `tools/inspect_h700_fdt.py`：扩展静态 FDT 解析，输出 CPU `cpu-supply` / `operating-points-v2`、regulator phandle、NVMEM cell offset/size 和 thermal-sensor provider arguments；明确标注这些只是 DT 声明，不是动态读回或安全 OPP 认证。
- `tools/inspect_h700_fdt.py` 另已扩展 OPP parser：每个 OPP 输出 `voltage_microvolt_variants`（`opp-microvolt-<suffix>`）并输出表级 `nvmem_cells` / `nvmem_cell_names`；对应 synthetic test `tools/test_inspect_h700_fdt.py` 与 H700 workflow 已更新。待 Actions 验证和设备重新采集。
- `tools/inspect_h700_sid_cells.py` / `tools/test_inspect_h700_sid_cells.py`：从 `cpu-power-runtime.txt` 验证且解析固定长度 speed 2-byte cell 与 thermal calibration 8-byte cell；speed 仅解码 little-endian raw `u16` 与 vendor literal 匹配标记，输出 `opp_authorization=not_approved`；thermal 拆成四个 `u16` 和诊断字段，输出 `calibration_status=not_certified`。未知值/缺失值不放行 OPP。Windows runner 自动生成 `sid-cell-report.json`。
- SID parser 的 `mainline_reference_decode` 使用 pinned mainline H616/H700 decoder；`0x6c00→candidate bin 5` 仅是主线诊断参考，不是 vendor mask 或 OPP authorization。另已发现两个 4.9.170 vendor candidates 对 `0x6c00` 分别映射 0x4/0x1，进一步证明版本号不足以唯一识别 SID decoder。所有报告仍固定 `opp_authorization=not_approved`。
- `sid-cell-report.json` 同时保留 generic SoC bus 的 machine/family/soc_id/revision 原始值，缺失时 unavailable；这些字符串不作为 vendor SID revision 语义的替代，也不改变 `opp_authorization=not_approved`。
- `scripts/collect-rg34xx-readonly-inventory.sh`：新增 `cpu-power-runtime.txt`，只读采集 Linux CPUFreq、thermal zone/trips、regulator class/debugfs、`clk_summary` 和 NVMEM 元数据；仅在 `of_node` 精确匹配 `sunxi-sid-ng@03006000` 时，读取 DTS 声明的 speed cell（provider offset 0、2 bytes）与 thermal calibration cell（offset `0x14`、8 bytes），不 dump 完整 NVMEM 区域，也不修改 governor、频率、电压或时钟。旧的 2026-11-22 报告未包含该新输出，必须在设备上重新采集。
- 另探测通用 SoC bus 的 `/sys/devices/soc0` 与 `/sys/bus/soc/devices/soc0`，仅读 machine/family/soc_id/revision，不读 serial/chipid。固定 vendor sun50iw9 `sunxi_get_soc_ver()` 可能通过 CE 临时开关 clock/reset，因此明确不调用 `sunxi_info/sys_info` 或 `/dev/sunxi_soc_info` ioctl。
- 该 collector 另探测通用 SoC bus 的 `/sys/devices/soc0` 与 `/sys/bus/soc/devices/soc0`，仅收集 machine/family/soc_id/revision，不收集 serial/chipid。固定 vendor `sunxi_get_soc_ver()` 在 sun50iw9 可能走 CE clock/reset 暂时开关路径，因此 `sunxi_info/sys_info` 与 `/dev/sunxi_soc_info` ioctl 均明确不调用。
- 另只读探测通用 SoC bus 的 `/sys/devices/soc0` 与 `/sys/bus/soc/devices/soc0`，仅收集 machine/family/soc_id/revision，不读取 serial/chipid。固定 vendor `sunxi-sid.c` 的 sun50iw9 分支未定义 `SUNXI_SOC_ID_IN_SID`，`sunxi_get_soc_ver()` 可能调用 `sid_rd_soc_ver_from_ce()` 临时写入/恢复 CCU CE clock/reset；`sunxi-sysinfo` 的 `sys_info` 也会输出 serial/chipid，因此 collector 明确不调用 sys_info 或 `/dev/sunxi_soc_info` ioctl。
- 针对旧 Linux 4.9 sysfs 兼容性，CPUFreq、thermal、regulator、NVMEM 各分组现会明确写 `NOT_PRESENT`（节点类不存在）或 `NO_READABLE_*`（节点存在但没有可读属性）；不会把空列表解释成零频率/零电压。 `tools/test_inventory_script_safety.py` 同步覆盖这些分支。
- 修复 C service 的失败重初始化语义：对完整 service storage，在校验新配置前先撤销旧实例；ready 检查同时验证 ABI struct size，避免无效重初始化仍保留旧服务。
- 已核对 Linkslot 1.0.45 `slot-ui/src/quick_menu.rs` / `slot/src/app.rs` 的 CPU 模式 UI 扩展点；当前 System Settings 为 6 行，`QuickRow`/`QuickValue` 使用固定数组和 enum index。UI 扩展要求在末尾追加新 row/value、同步中英文 key 和 parity tests，不能重排既有设置索引。设计约束见 CPU DVFS 文档 7.2。

### ABI v1 语义

- C 服务持有调用方提供的固定大小 OPP 表；不使用 heap、线程或动态分配。
- 模式请求经纯 C selector 选择目标 OPP。
- 成功返回 `H700_CPU_PERF_SERVICE_POLICY_ONLY`：**仅表示策略目标已选出，不表示真实 CPU 频率/电压已改变。**
- 状态结构把 `planned_frequency_hz/planned_voltage_uv` 与 `actual_frequency_hz/actual_voltage_uv/temperature_mc` 分开。
- `hardware_applied` 在 ABI v1 中固定为 0。没有实际硬件读回时，不得把计划频率显示为实测 MHz。
- 更新温度/电压/频率限制后，当前有效请求会重新经过 policy selector。
- Rust 客户端把 C service pointer 作为 opaque pointer；把 planned OPP、actual clock/voltage/temperature 遥测、policy flags 和错误码转换为 Rust 类型。
- Rust 客户端校验 ABI version、struct size、reserved bits、telemetry valid flags 和 `hardware_applied == 0`；C 侧 `_Static_assert` 固定 state size=64、`hardware_applied` offset=48、reserved offset=52。
- 当前测试使用合成 OPP 值，不是 RG34XX 已批准的运行频点表。静态 runtime owner 已实现，H700 `target_init()` hook 也已存在，但 hook 保持空实现、没有 board-approved RG34XX profile，也未调用 owner；因此目标机 service 仍 unavailable。Rust 客户端尚未接入 Linkslot `App` 设置界面。
- 不同 vendor `sun50i-cpufreq-nvmem.c` 对 SID raw `0x6c00` 的 literal mapping 不一致；两份 4.9.170 候选均有对 2-byte SID cell 的 `u32 *` 长度风险，4.9.191 另有 length-checked byte-wise 实现。不得把任何一个公开分支当成目标运行内核的精确映射。

### 本轮本地验证与 CI 状态（2026-11-24 13:38 UTC）

- [Linkslot LK Rust feasibility #45403377689](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45403377689)：**成功，69/69 steps**；H700 BL33 overlay、memory fail-closed guard、renderer/framebuffer、Linkslot、mGBA、newlib 和只读 MMC/GPT/FAT/exFAT 综合 host/cross-build 通过。
- [Linkslot LK Rust feasibility #45405198205](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45405198205)：**cancelled**，被后续提交触发的并发运行取代；取消时 69 steps 中 34 步完成且没有测试失败，不作为成功或失败证据。
- [Linkslot LK Rust feasibility #45405599463](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45405599463)：截至 13:41 UTC **in progress**，head `b7ac4a5ddae666b475654f36e13c7071c1723d33`；cross-compile job 68 steps 中 11 步完成、未见失败；GammaOS LK H700 BL33 overlay、native BL33 header、H700 project/memory fail-closed guard 已通过，后续 renderer/Linkslot/storage/mGBA 步骤仍在执行。
- [H700 boot audit tools #45405024561](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45405024561)：**成功**；SID parser mainline reference classifier / Windows runner / 7 项 synthetic tests，Linux 6/6、Windows 8/8。
- [H700 boot audit tools #45405197973](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45405197973)：**成功**；roadmap/inventory docs batch，Linux 6/6、Windows 8/8。
- [H700 boot audit tools #45405590516](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45405590516)：**成功**；inventory schema docs batch，Linux 6/6、Windows 8/8。
- [H700 boot audit tools #45405607986](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45405607986)：**成功**；latest PROJECT-STATUS handoff check，Linux 6/6、Windows 8/8。
- [H700 boot audit tools #45405197973](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45405197973)：截至 13:38 UTC 两个平台 audit jobs 仍在运行；Linux host-tools 3/5、Windows 3/7 steps 已完成，未见失败。此 run 触发于 roadmap/inventory 文档更新。
- [H700 boot audit tools #45404785874](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45404785874)：**成功**；mainline speed-bin direct/ambiguous/unsupported reference 分类的 7 项 parser tests 和两平台 wrapper tests 全通过。
- [H700 boot audit tools #45404714998](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45404714998)：**成功**；SID mainline reference decoder implementation 的 initial host/Windows audit 通过。
- [Linkslot LK Rust feasibility #45393436818](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45393436818)：**成功，69/69 steps**；上一轮 H700 LK/Linkslot/mGBA/storage 综合基线。
- [Linkslot LK Rust feasibility #45399902083](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45399902083)：**成功，68/68 steps**；SID collector/parser 集成阶段综合回归。
- [H700 AArch32/AArch64 compile check #45392910394](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45392910394)：**成功**；RG34XX H700 target lifecycle hooks 在 AArch64、AArch32 ARM-state、AArch32 Thumb-2 编译通过。
- 最新 raw decoder 仅作为 Linux mainline reference 诊断。它不修复或信任旧 vendor `u32*` / 2-byte NVMEM cell consumer，不给设备 OPP mask，不生成真实频率/电压表，也不打开 DVFS。
- 当前仍没有 RG34XX 新版实机 `live-fdt.json` / `cpu-power-runtime.txt` / `sid-cell-report.json`；旧 2026-11-22 inventory 未包含新证据项。没有实机 CPU rail readback、完整板级 OPP/voltage policy、可信 SMCCC revision 或 thermal calibration pass，CPU runtime owner 仍 unavailable。
### 本轮 CI 验证

- [H700 boot audit tools #45397765612](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45397765612)：**成功**；collector missing-node `NOT_PRESENT` 状态覆盖测试通过，Linux/Windows jobs 均成功。
- [H700 boot audit tools #45397788581](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45397788581)：**成功**；明确 missing/unreadable CPU power sysfs status 的安全回归通过。
- [H700 boot audit tools #45397828044](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45397828044)：**成功**；inventory documentation and read-only semantics checks passed.

- [H700 boot audit tools #45396816432](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45396816432)：**成功**；Ubuntu host-tools 与 Windows PowerShell jobs 均通过，包含新增 CPU FDT phandle 测试、read-only CPU power inventory 测试。
- [H700 boot audit tools #45396493076](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45396493076)：**成功**；验证 CPU power collector / FDT provider metadata 的主机测试与 Windows wrapper 检查。
- [H700 AArch32/AArch64 compile check #45392910394](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45392910394)：**成功**；包括 H700 target lifecycle hooks 三种 ISA 编译矩阵。
- [Linkslot LK Rust feasibility #45393436818](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45393436818)：**成功，69/69 steps**；本轮 target hook 前一版综合 BL33 overlay/Linkslot/mGBA/storage host-cross-build baseline。

- [H700 AArch32/AArch64 compile check #45386479380](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45386479380)：**成功**。两个 jobs 均成功：
  - `storage-host-regression`：包含 CPU Service v1 host runner 和现有只读存储回归。
  - `compile-freestanding-objects`：包含新 CPU Service C 源的 AArch64、AArch32 ARM-state 和 AArch32 Thumb-2 编译。
- [H700 Linkslot 1.0.45 runtime slices #45387192778](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45387192778)：**成功**。两个 jobs 均成功；新增 Rust CPU performance crate 的 workspace host tests、AArch32/AArch64 `no_std` check 和 Linkslot renderer/business overlay compile 均通过。
- [H700 AArch32/AArch64 compile check #45387293421](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45387293421)：**成功**。包含 C Service ABI v1 状态布局 `_Static_assert`、host regressions 和 AArch64/AArch32 ARM/Thumb-2 compile。
- [H700 boot audit tools #45387673284](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45387673284)：**成功**，覆盖当轮项目状态文档和报告工具检查。
- [Linkslot LK Rust feasibility #45387540498](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45387540498)：**cancelled**，由后续提交触发的并发运行取代；没有报告测试失败，不能当作通过或失败证据。
- [H700 boot audit tools #45388212345](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45388212345)：**成功**，覆盖最新路线图中的 CPU 设置 UI 扩展约束。
- [Linkslot LK Rust feasibility #45388212358](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45388212358)：**成功**，是本轮之前的一次完整 overlay/cross-build 综合基线。
- 初次 runtime-slices 运行 #45387187588 在仅提交 workspace manifest、Rust crate 源文件尚未提交的中间 commit 上失败（找不到 `slot-cpu-performance/src/lib.rs`）；完整 crate 提交后的 #45387192778 已通过，该中间态失败不代表最终 crate 测试失败。
- [此前完整 Linkslot 1.0.45 feasibility #45381590503](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45381590503)：成功；这是旧的 69-step 综合基线，证明 host/cross-build 范围，不证明实机运行。
- [CPU OPP policy 专项 #45381349400](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45381349400)：成功。

## 5. H700 CPU DVFS 硬件证据和安全门槛

已确认/交叉验证：

- RG34XX 现场 Linux 日志识别 PMIC 为 **AXP2202**，I²C-5、地址 `0x34`。
- 2026-11-25 live FDT 已直接确认 CPU0 `cpu-supply` phandle `0x4f` 指向 AXP2202 DCDC1（500000–1540000 µV，always_on/boot_on）；同次 Linux regulator debugfs 显示 `cpu0 @1160mV`。这是本次 live FDT + Linux regulator-framework 证据，不是 BL33-entry PMIC 原始寄存器读回。
- 通用 SoC `soc0` sysfs revision 即使存在，也仅作为 OS 导出的识别线索；vendor `sunxi_get_soc_ver()` 路径当前不调用，因为它在该 H700 配置可能临时切换 CE clock/reset。SID JSON 对 generic revision 值保持原样，不把它映射成 H700 speed-bin 档位。
- 2026-11-25 新版 inventory 已按 DTS 声明限长读取 SID speed cell（2 bytes，raw `0x6c00`）与 thermal calibration cell（8 bytes，raw `181113ab191b16bb`）。主线诊断 decoder 给 candidate bin 5，但公开 vendor source candidates 对 `0x6c00` 的 mask 解释不一致（4.9.170 分别为 0x4/0x1）；generic SoC identity unavailable，所有结果仍 `opp_authorization=not_approved`。
- 现场内核报告 H700 thermal sensor 存在且有 4 个 SID raw calibration fields，但明确提示 **AVCC is not calibrated**；CPU/GPU 当前温度只作诊断，不可用于自动温控升频、Auto 或 Boost。
- 固定 `sun50iw9` vendor-source audit 给出 `PLL_CPU=0x0000`、`CPU_CFG=0x0500` 和 alternate-parent/lock 流程；2026-11-25 Linux `clk_summary` 观察到 `pll_cpu → cpu → cpuapb`、`pll_periph0=600MHz`。同版本 cpufreq-dt 候选源表明 `cpuinfo_cur_freq` 是 clock-framework `clk_get_rate()` 读数，而 `scaling_cur_freq` 是缓存策略值；首轮数据未同步采样，不能认定 clock provider 有永久冲突。六轮 uptime-bracketed read-only sampler 已加入采集器，待重新采集。
- Linux runtime snapshot 显示 CPU0–CPU3 全部 online 且属于一个共享 cpufreq policy，确认 CPU 频率不是按每个模拟器核心独立设置；但 CPU1–CPU3 在更早 BL31→BL33 handoff 时的状态仍未知。

**禁止事项：** 不复制 MT6785 的 PLL 地址/PCW 算法，不把 GammaOS Pico 600–2000 MHz 档位直接套到 H700，不使用 AXP717 参数替代 AXP2202，不把 H616 OPP 表当作 RG34XX 已批准 OPP，不在缺少次级 CPU 状态证明时写 CPUX PLL，不在缺少电压/热证据时升频。

## 6. 下一步任务（按顺序执行）

1. **在真实启动路径调用 owner，但不提供虚构板级配置**：H700 `target_init()` hook 已实现，下一步是在硬件证据齐备后接入 board-profile 加载；当前必须保持 hook 空实现、runtime unavailable。SoC revision 只能来自已验证的只读来源；不要调用会临时改变 CE clock/reset 的 vendor `sunxi_get_soc_ver()` 路径。
2. CPU Performance QuickRow 已完成 display-only/unavailable 接入和 App 回归测试。后续只有在 board-approved RG34XX profile、C owner 初始化和真实只读 telemetry 路径齐备后，才把 `CpuPerformanceRuntime` typed client 接入 App；在此之前继续保持不可选、不可 persist、无计划 MHz 展示。

Linkslot 1.0.45 UI 当前已追加 display-only CPU row：`SYSTEM_SETTINGS` 为 7 行，`QuickRow::ALL` 为 88 项，Language index=86、CPU Performance index=87；`QuickValue::ALL` 为 38 项，新 unavailable value index=37。两项 App unit tests 已在 runtime-slices/full feasibility CI 实际通过；Rust typed client 仍未接入 App。
3. **在 RG34XX 上重新运行 parser-v2 inventory**：取得包含 `opp-microvolt-*` named variants、OPP table-level SID phandle/name 的新 `live-fdt.json`；随后再核对 raw `0x6c00` 对应的实际 OPP voltage suffix 与 `scaling_available_frequencies`。
4. **确认实机正在运行的 cpufreq-nvmem/OPP 精确源码和 build**：目前公开 4.9.170 source candidates 的 raw `0x6c00` mapping 不一致；需找到目标镜像对应 source commit/patch 或可识别的只读驱动证据，不能推定 mask。
5. 证明 BL31→BL33 CPU1–CPU3 状态、单核 invariant、备用父时钟切换和 timer continuity；同时解决 AVCC thermal calibration 或确认独立可信温度来源。前述门槛闭合后才设计 C HAL transition（升频先升压、降频先降频后降压、readback/timeout/rollback/hysteresis）。
6. 最后才开放实际 Performance/Boost。不得把 ABI v1 的 `POLICY_ONLY` 当作 DVFS 已完成。

## 7. 续接规则

新的工作轮次按以下顺序恢复上下文：

1. 先读取本文和 GitHub 当前分支 HEAD / 最新 Actions 状态；本文的 HEAD 是核对时快照，不应假设之后没有新提交。
2. 当前代码只在 `porting/linkslot-1.0.45-feasibility` 上推进；先检查分支差异与最新 CI，不重复建立已有分支/文件。
3. 依任务读取专项文档：
   - CPU 调频：本文 + [H700 CPU Performance Service / DVFS](h700-cpu-performance-service.md) + [H700 CPUX clock-source audit](h700-cpux-clock-source-audit.md) + [RG34XX CPUX live audit](rg34xx-cpux-live-audit-2026-11-25.md)
- FDT OPP parser: [`tools/inspect_h700_fdt.py`](../tools/inspect_h700_fdt.py) + [`tools/test_inspect_h700_fdt.py`](../tools/test_inspect_h700_fdt.py); parser-v2 needs a new live inventory capture.
- OPP/cpufreq source check details: [RG34XX CPUX live audit §1.5](rg34xx-cpux-live-audit-2026-11-25.md).
   - 总体路线： [roadmap](roadmap.md)
   - Linkslot 1.0.45 源码差异： [1.0.45 portability update](linkslot-1.0.45-portability-update.md)
   - LK/H700 runtime 架构： [Linkslot H700 runtime architecture](linkslot-h700-runtime-architecture.md)
   - BL33/内存交接： [RG34XX boot handoff audit](rg34xx-boot-handoff-audit.md)
4. 只把 CI 明确通过的提交标为已验证；host compile 不等于实机启动，合成 OPP 不等于真实可用频点。
5. 每轮完成后先更新本文，再更新 roadmap 和对应专项文档；把新 commit SHA、Actions run URL、失败及尚存风险写入本文。
