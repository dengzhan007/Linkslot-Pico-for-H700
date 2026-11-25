# 项目当前状态与续接记录

> **每次开启新的移植对话，先读本文。** 本文是当前工作状态的交接入口；详细设计仍以各专项文档为准。每完成一轮代码、测试或仓库审查，都应同步更新本文、路线图及对应专项文档。

**最后核对时间：** 2026-11-26（Linkslot main 1.0.49 完整 feasibility 最新 run #45646868006 为 71/71 成功；H700 boot-audit #45646868087 两 job 成功；新增 ADB UI/GAME 实测均约 59.6 Hz。120-Hz writer 保持关闭；下一步独立采集 Slot successful presents 与 armGBA committed frames。）
**仓库：** `dengzhan007/Linkslot-Pico-for-H700`  
**下一项显示分析（优先级最高）：** FDT timing 与 TCON/PLL 只读回读已完成：live panel 为 `rg34xx_v1 + lcd_type=old`，TCON divider word `0xf000000b`，PLL raw `0x88001703`。固定参考 sun50iw9 时钟代码将 live PLL 解为 288 MHz；同一公式把 pvaibhav 高刷候选 `0x88002b03` 解为 528 MHz，配合 divider 10 得 52.8-MHz DCLK / 约 120.13 Hz（820×536），但这不是 kernel #19 已确认的 decode，也不证明面板支持。Anbernic 未公开该内核源码；项目已有 stock p4 解压 ELF 且 GNU Build ID 与 live kernel 精确匹配。**该离线分析已完成**（见下方第 4 条与 `reports/kernel19-display-reverse-audit/ADDENDUM.md`）：panel init、PLL bitfield、divider 来源、TCON parent 继承均已闭合，不再重复导出 p4 或重复读 divider。随后分别补 Slot 600-present trace 与 armGBA committed-frame 计数；LCD IRQ 仍只作 scanout proxy。禁止使用 `/sys/class/sunxi_dump/write`，不接入、不调用硬件 writer。
**工作分支：** `porting/linkslot-1.0.45-feasibility`

**2026-11-26 最新续接更新：**

- **ADB 显示复测：** reports 分支 `reports/slot-refresh-audit/` 保存 UI/GAME 两份只读原始输出。UI 12 s IRQ 293 Δ716 = 59.667 Hz；GBA/armGBA 12 s Δ715 = 59.583 Hz。两态 live panel 均为 `rg34xx_v1`、720×480、820×536、26-MHz DCLK property；PLL `0x03001040` 均读为 `0x88001703`。TCON divider `0x06511044` 本轮返回 I/O error，未取得直接读数。结论为两态均无 120-Hz 应用证据；IRQ 只是 scanout proxy，不是 UI present 或 game FPS。原始脚本没有访问 sunxi_dump/write。
- **校正高刷寄存器候选：** 固定 pvaibhav 源码 `5206e3fa39aa4a9b702cd9048a07dab3b5ab95da` 的高刷组合是 divider `0xf000000a` + PLL `0x88002b03`；60-Hz 组合是 PLL `0x88001703` + divider `0xf000000b`。ADB SUMMARY 中旧写的 `0x88001701` 不是该 fork 的高刷值，已在 reports/SUMMARY.md 校正。面板名 `rg34xx_v1` 不满足 fork 的 `fog_fj035fhd05_v1` 精确 gate，按该源码路径启动 setter 会被跳过；不能通过改字符串绕过。
- **Linkslot 1.0.49 全量 CI：** 最新 `#45646868006` 在当前检查 HEAD `018e0b30d14965b56003e23e06da5593a94808d6` 上 **71/71 steps 成功**，artifact `15038716689`。H700 boot-audit `#45646868087` 的 host-tools 与 windows-powershell 两个 job 均成功。两项均为 host/cross-build 和工具验证，不是完整可刷写固件或实机启动证明。
- **下一显示测量：** 1) 先核对设备已安装 `System/slot` SHA `8f0fd40da4ac161e5fbb9a56a99c254acf8b2f31806e2be6bdaa929b5ee28505` 与 `armgba_libretro.so` SHA `4ebae098cc4848fb459fc18299c39acbdbbbeb732db5c3f66d1ff3f6e142e889` 是否匹配官方 H700 release ZIP；2) 若 Slot 二进制与固定源码一致，利用启动前 `SLOT_TRACE_LATENCY` / `System/trace-latency.on` 启用已有 600 次成功 `surface.swap()` 计数，单独测 UI presents/s；3) armGBA 必须读取其专有 `gba->video.frameCounter`，不能复用 pvaibhav `probe.rs` 中指向 mGBA 的 `gba` selector；先验证 ELF symbol/ABI/offset，再做只读进程内存采样。不得用 IRQ 替代 FPS，也不得写显示寄存器。



**Linkslot 1.0.49 runtime-slices 迁移验证（2026-11-25）：** [#45624285300](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45624285300) **成功**。完整 job 通过：H700 no_std workspace host tests、AArch32/AArch64 freestanding checks、renderer + business overlay 应用、上游 input/model regression、Linkslot 1.0.49 完整 Frontend + SoftwareCompositor 的 `cargo check`、CPU Performance QuickRow unavailable/不写存储的 2 项回归，以及 H700 C input ABI/queue job。首次 1.0.49 run [#45623895416](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45623895416) 曾因 H700 `slot-link-kind` 未提供上游新增的 `armgba_link_carried` / `armgba_port` 失败；已补齐 no_std 实现、测试和 re-export，第二轮通过。此结果仍是 host/cross-build，不是 LK 完整镜像或实机运行。

**Linkslot 1.0.49 综合 feasibility CI（2026-11-25）：** 完整 workflow [#45625386428](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45625386428) 在 static mGBA selector test 失败；上游 1.0.49 已将联机参数改为 `Option<LinkSetup>`，旧测试 patcher 仍传 `Option<u8>`，引发两个 E0308 类型错误。已在提交 `d333eb30dc8a21b4e9b9509bc08a3760223c091a` 修复：helper 改用 `Option<LinkSetup>` 并与动态路径一致设置六键手柄。修复后的 workflow 正在重新验证；此前 1–31 步通过，32 步失败，之后步骤被跳过。不得把该 run 标为通过。

**Linkslot 1.0.49 第一轮迁移（2026-11-25）：** 持久化 `slot-model/src/core.rs` 已同步 main 1.0.49；`CoreSpec::netpacket` 已纳入 H700 no_std 模型，gpSP/armGBA=true、mGBA=false，并在 host contract test 中锁定。crate 版本已更新至 1.0.49。Frontend 与 renderer patcher 的 source pin/workspace version 已指向上游归档 HEAD `241e7c1974926fa22552b200852914ceaccbfd42` / 1.0.49；workflow [#45623895416](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45623895416) 中 no_std workspace tests、overlay、upstream input/model regression 已通过，完整 Frontend cargo check 截至记录时仍在运行。未标记完整迁移通过。详见 [更新审查](linkslot-1.0.49-1.1.0-refresh.md)。

**Linkslot 新基线的 H700 代码差异补充（2026-11-25）：** 持久化 `integration/linkslot/h700-frontend-core/slot-model/src/core.rs` 仍是 1.0.45 风格，没有 1.0.49 `CoreSpec::netpacket` 字段，armGBA 仍被注释为单人核心；而 1.0.49 已声明 gpSP/armGBA netpacket=true、mGBA=false。需同步核心元数据与测试，才能保留上游 1.0.47 起的 armGBA 联机语义。H700 `machine.rs` 保留六个首期机种是有意的；DEV Arcade/NeoGeo 不应在首期引入。现有两条 patcher 仍 pin 1.0.45，且 runtime workflow checkout 的 `upstream/linkslot` 已是 1.0.49 归档头；不更新 pin 和锚点前，严格版本门会拒绝新快照。详见 [更新审查](linkslot-1.0.49-1.1.0-refresh.md)。

**Linkslot 上游源码更新（2026-11-25）：** 用户已更新三个来源分支，均已归档在 `upstream/linkslot`：main 1.0.49（原始 commit `b84f37f1a6259a625e9095279dd132f5ef2116af`）、DEV 1.1.0（`a800adb2e3a51d05ffe80abeae2e1ec52247b951`）、test 1.0.49（`8c7a37055eb68f71170320d4a17cdb9251785186`）。本仓库归档 HEAD `241e7c1974926fa22552b200852914ceaccbfd42`。main/test 的 851 个源码文件逐 blob 一致；DEV 比 main 新增 21、删除 1、修改 46。稳定移植基线改为 main 1.0.49，DEV 1.1.0 单独审查，不将 Arcade/NeoGeo/FBNeo 纳入首期 GB/GBC/GBA。关键兼容信息：旧 1.0.45 与三份新快照的 `slot-retro/src/ffi.rs` blob SHA 完全一致（`2e7b1b0bb3e1665de01f6458b6dfc7fe65a61f34`）；但 `slot-model/src/core.rs` 和 `slot/src/app.rs` 已改变，旧 patcher 仍锁 1.0.45，须在 1.0.49 上重跑锚点、overlay、静态 API 和完整 workflow，不得只修改版本号。详情见 [Linkslot 1.0.49/1.1.0 更新审查](linkslot-1.0.49-1.1.0-refresh.md)。  

**最新显示刷新实机结论（2026-11-25）：** pvaibhav/slot 在同一台 RG34XX non-SP 的 UI/GAME 两轮均约 59.6 Hz；未观察到 120 Hz 或场景切换。完整 live FDT 已上传并解析为 `rg34xx_v1` / RGB666 / 720×480 / DCLK property 26 MHz / HT×VT 820×536 / `lcd_type=old`，DTB 派生 nominal 59.155 Hz，显示 IRQ 计数斜率约 59.63 Hz（仅为 scanout/vblank proxy，不等于已确认的面板物理刷新率），TCON0 DCLK divider 已读回为 11（`0xf000000b`），PLL raw value 为 `0x88001703`；实际 DCLK 仍是基于 288 MHz / 11 的推算值，未做独立物理测量。Pinned BaseOS survey 的 vendor profile 是 PE6 high→old、low→boe，目标 rg34xx 使用 `rg34xx_v1`；其 U-Boot panel struct 选择 `rg34xx_v1`，虽调用一个 symbol 名含 `fj035fhd05` 的共享 init routine，但该函数标签不是 FDT 驱动名。pvaibhav fork 的 `rg34xx→fog_fj035fhd05_v1` allowlist 因而不匹配标准目标 profile，不能直接移植 writer。设备 Slot/core 文件 SHA 已记录，但尚未与 release ZIP 解包文件比较。详情见 [实机分析](pvaibhav-slot-rg34xx-live-capture-2026-11-25.md)。硬件 writer 保持禁用、`hardware_applied=0`。  
**当前文档/工具验证：** 最近一次已完成的 H700 boot audit tools [#45617713806](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45617713806) 已成功：Linux host-tools、Windows PowerShell 两个 job 均通过；此前 #45617414175、#45616919646、#45616740899、#45616747528、#45616754133、#45616761051 也成功。完整 Linkslot/LK feasibility 的既有通过记录为 [#45606529437](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45606529437)；较新的再跑 [#45616741294](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45616741294) 后被更新提交触发的并发取消；文档更新触发的 [#45617402177](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45617402177) 在本次检查时仍为 in_progress，未计为通过。
**最近一次完整 feasibility 通过的 HEAD：** `8d44874db413c6bd5be71a293edb2b34682b4ba5`（[workflow #45579608578](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45579608578)，71/71 步骤成功；工件 ID `15005762318`）。
**当前显示刷新策略实现提交：** `40113045a685413fb6887c621d9e0e982aee9303`，新增独立 `game_120hz_enabled` C policy 开关（默认关闭）。对应 AArch64/AArch32 ARM/Thumb-2 编译和 storage host regression [#45581459152](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45581459152) 已通过；Linux/Windows host-tools audit [#45581459275](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45581459275) 已通过，其中 `test-h700-display-refresh-policy.sh` 明确 PASS。完整 Linkslot/LK feasibility [#45581815012](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45581815012) 尚未取得终态；不将新提交标为完整 feasibility 通过。
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
**历史 parser-v1 限制（已由 parser-v3/v4 采集关闭）：** 最初的 `live-fdt.json` 没有命名电压与 table-level speed NVMEM 信息；parser-v4 inventory 现已包含 38 个 OPP 节点的 property-presence、named-voltage variants 和 speed NVMEM reference。结合 matched shipping binary 的完整 CFG/OPP-core 反汇编，no-mask+b1 是高置信软件选择模型；不需要重新导出 boot 或 DTB。
**本轮只读 clock inventory 变更：** `scripts/collect-rg34xx-readonly-inventory.sh` 在原有完整 `clk_summary` 输出外，新增保留缩进的 `pll_cpu` / `pll_periph0` / `cpux` / `cpu` 重点摘录；`tools/test_inventory_script_safety.py` 增加只读回归，`.github/workflows/h700-arch-check.yml` 将两个文件加入 push/PR 路径并执行该测试。新摘录只读 Linux debugfs 文本，不直接读取 `PLL_CPU` / `CPU_CFG` MMIO。摘录片段单独通过 `sh -n` 语法检查；完整 Python 回归/Actions 尚未取得可核验执行结果。相关提交链：collector 摘录 `db9e190619b149901b817b67d899c0e937891190` → printf 格式修正 `c30dceea131f2a1cf168bdf08beac36884ce52a3` → 静态安全测试 `aad8671ece7f39124a58aa659c54bc3a2c0ff661` → workflow 接入 `96afe6d2fff811eab4edfa87b3f58606bdaf645c`；随后同步 DVFS 文档、路线图与状态记录。
**本轮 FDT OPP 解析器增强：** 旧 `live-fdt.json` 仅包含 base `opp-microvolt`，未解析 `opp-microvolt-*` 命名电压属性，也未输出 `opp_l_table` 自身的 `nvmem-cells` / `nvmem-cell-names`。`tools/inspect_h700_fdt.py` 现新增 `voltage_microvolt_variants`、OPP table NVMEM refs/name；合成回归覆盖 a1/b0 suffix 与 speed-bin phandle。提交：parser `fc84576d0d597abb6317ac64fefd62cf25dd63dc`、test `0ee82a5709418b8f81120cd337a75b8d237052ee`、workflow `9cbceb738ac2a8802784d0eac6ae6f4f2c5e1a5c`、fixture NUL 修正 `c5bd42ca547db2c49ec2018946275cd457ff24ee`。新增 Python/Actions 尚未取得可核验运行结果；须重新运行 inventory 生成 parser-v2 `live-fdt.json`。
**本轮同步频率采样增强：** 采集器现执行六轮、间隔 200 ms 的 uptime-bracketed 采样，分别读取 `policy0/cpu0` 的 `scaling_cur_freq`、`cpuinfo_cur_freq` 及同轮 CPUX `clk_summary` 行。静态安全测试增加该路径无 sysfs 写入的断言；workflow 已包含此 test path/command。采样脚本和回归源码已提交，完整 Python/Actions 结果尚未取得。
**同步采样代码提交：** collector `2726db1f2b4e356e632b958027cb11f9bf42bf49`（blob `94cce9b7a0a90a28a7e427cfb84d2a9f6be540b8`）；新增静态只读测试 `51ed3bae2fc24e88fc803b2e3e143fe5df8ba495`（blob `c320daec1fc6a970b19301aa61b0003b797409ae`）。采样 shell 片段与 Python 测试方法的语法检查通过；GitHub commit combined status 目前未返回 checks，完整测试/Actions 仍待确认。


**本轮 CPUX inventory v2：** 报告已上传至 [reports/rg34xx-inventory-20261125-cpux-v2](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-inventory-20261125-cpux-v2)，采集时间 2026-11-25 00:41:21 UTC。parser-v2 已解析 `/opp_l_table` 的 table-level SID `speed` 引用及 `opp-microvolt-a1/b0/b1/c0/c1/c2` 命名电压；启动时 600/792/1032/1296/1344 MHz 五档因电压属性未命中被内核丢弃。v2 的 480 MHz cached observation 已由 v3 复读证据重新解释，不再视为真实 clamp；SID raw `0x6c00` 未获 OPP mask 授权，AVCC 热校准仍未通过。
**本轮 CPUX inventory v3（2026-11-25 03:19:48 UTC）：** 报告已上传至 [reports/rg34xx-inventory-20261125-cpux-v3](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-inventory-20261125-cpux-v3)。原始 live FDT 与五个启动链二进制捕获的哈希与 v1/v2 一致；v3 新增 parser-v3 OPP property presence 和八组 cpufreq getter/clock/DCDC1 对账。五份 `scaling_available_frequencies` 均为 480/720/936/1008/1104/1200/1320/1416/1512 MHz；38 个 OPP 节点的唯一精确结构候选为 no table-level `supported_hw` + `b1` named voltage，全部 one-cell mask 模型没有 exact witness。九档 `b1` 电压为 900/900/900/950/1000/1050/1100/1150/1160 mV。八组 getter 证明 v2 的 480 MHz cached window 是 cpufreq sysfs 非原子/滞后读取，不是已证实的频率钳制；CPU thermal cdev 状态为 0/8、0/5，均绑定 CPU zone0。此结果仍不证明实际驱动源码、SID `0x6c00 → b1`、原始 PMIC/CCU 状态或 OPP 安全授权；AVCC 未校准、CPU1–CPU3 BL33 状态未知，所有 DVFS 写操作继续禁用。
**本轮 CPUFreq/kernel provenance collector（代码 HEAD `9cd001f2caf5d2f083e98abce7117494d09574c9`，安全断言修正 HEAD `e0ab50220939dfbbec95826ea61c15e7ec27c50c`）：** `scripts/collect-rg34xx-readonly-inventory.sh` 新增 `kernel-driver-provenance.txt`，只读采集 `/proc/version`、`/proc/sys/kernel/osrelease`、`/sys/kernel/notes` 的 hash 与前 256 bytes、可用 kernel config、cpufreq 相关 platform driver/device 的 `driver/device/of_node` 绑定、模块名和经筛选的 OPP/cpufreq/NVMEM kallsyms **符号名（丢弃地址）**。不执行 vendor getter、ioctl、SMC、modprobe/insmod，不读写 PMIC/PLL/SID 和 sysfs 控制项。静态测试先因注释中出现 `sunxi_get_soc_ver()` 文字而误报，已改为仅拒绝真实代码调用；同时修正写路径正则后，Linux/Windows 与 H700 3 ISA 矩阵全通过。
**本轮公开 OPP driver source 排除（2026-11-25）：** 新增对照 T507 4.9.170、Lineage Allwinner 4.9.170、Creality 4.9.191 与 Radxa `sun50iw9` 候选。前两者 raw `0x6c00` 分别映射 mask `0x4/0x1`，均无 `b1` named-property setter；Creality 需 `speed+Icpu` 且 `0x6c00` 仍走 mask；Radxa speed-only 候选落入 `a0` + mask。没有一个公开候选匹配 v3 的 no-mask + `b1` 指纹。本机 OPP table 只声明 `speed` cell；实际 kernel build ID/source commit 尚未取得，因此暂判断为未识别的 custom/vendor path，不推断 `0x6c00 → b1`。

**本轮 RG34XX CPUX inventory v4（2026-11-25 04:05:51 UTC）：** 报告已上传至 [reports/rg34xx-inventory-20261125-cpux-v4](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-inventory-20261125-cpux-v4)。运行内核为 Linux 4.9.170 #19，Linaro GCC 5.3.1，构建时间 2026-05-26 13:58:08 CST；从 `/sys/kernel/notes` 解析出 GNU Build ID `a1d211f847a5fb69fe9898e15fbccdd3ea8d7b68`（notes SHA-256 `6065c1a7ae894047f0e902551811b39d0c5a11b5afd894083eae46497d56e12d`）。config、kallsyms 和 sysfs 链接确认内建 `sun50i-cpufreq-nvmem`、`cpufreq-dt` 平台设备均成功绑定；live OPP 表只有 `speed` NVMEM cell，raw SID 为 `0x6c00`。五组运行频率列表严格对应九个带 `b1` named voltage 的 OPP，频率/电压采样亦与若干 `b1` 档位吻合。公开 Creality 4.9.191 候选因要求额外 `Icpu` cell 且将 `0x6c00` 映射 mask 1 而被排除；T507/Lineage 4.9.170 候选亦不匹配。结论更新为：运行时有效 OPP grid 强烈指向 `b1`，但不能因此声称 SID `0x6c00 → b1`，所有 OPP/DVFS 授权仍关闭。已从实机 stock `mmcblk0p4` 只读导出 64 MiB Android boot 分区；解压内核中 GNU Build ID `a1d211f847a5fb69fe9898e15fbccdd3ea8d7b68` 与运行内核 `/sys/kernel/notes` 完全相同，首次建立**运行内核与 boot 分区内核二进制的精确身份关联**。完整文件不上传，仅保存[元数据报告](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-boot-p4-metadata.json)；分区 SHA-256 `41cfaa8f9ade4a5e05a89188abb7db154243e38b8e19c53b730f0a45ff325516`，压缩 kernel payload SHA-256 `cc4b5769cb8c9dcf8d013602e57c494a1153eb6714d9786cf847e39e3ecdd3af`。另扫描到 `fe7e68cd7aa14945d71f36e4fbf8412d959f4cc1` 第二个 GNU-note-like ID，尚未定位归属，不作解释。

**本轮 RG34XX 电源灯实现：** 原厂 RG34XX（非 SP）设备树与 MustardOS RG34XX-H 配置均映射 `work_led=PI12`、`low_pwr_led=PI11`；仅控制 PI12。新增 `platform/h700/power/h700_rg34xx_power_led.c`，上电默认点亮；`target_early_init()` 早期点亮，`target_init()` 失败重试。新增 PIO 1 MiB Device MMU window mapping。Linkslot 1.0.45 `Session` 在 Playing/Polaroids 状态熄灭、退出后点亮；no_std 状态机仅在状态变化时调用 C ABI。暂不增加前端开关。提交：driver `af1922d0cecec278fc7bf2d81d63e4e621ab8685`、Session state `3cbc92669aac23b3e702c9518573d37adcd2fa54`、PIO mapping `c3a233e6f20defaf2ab0192f0de54f5c70921464`。Host tests 与 cross-compile 已通过：H700 AArch32/AArch64 compile check [#45524236936](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45524236936)（含 AArch64、AArch32 ARM、AArch32 Thumb-2 三组对象编译与 LED C host regression）；Linkslot runtime slices [#45524236833](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45524236833)（含 no_std 状态机测试和 Linkslot 1.0.45 Frontend overlay 编译）成功。实机极性、启动时机仍待验证；完整 LK target link/镜像仍受独立内存批准 gate 约束。

**本轮 LK MMU 顺序修正（`ca1d1ce33b588bff85dc4ab9c949971ee2736ff4`）：** 对照 pinned GammaOS LK `arch/arm/arch.c` / `platform/mt6580/platform.c` 发现 `platform_init_mmu_mappings()` 在 `target_early_init()` 前必须建立 section table/TTBR、映射 DRAM 和设备窗口、最后启用 MMU；此前仅添加 PIO descriptor 的实现不完整。现在按 RG34XX live DRAM `[0x40000000,0x80000000)` 建 1024 个 1 MiB normal write-back RW identity sections，再将 `[0x03000000,0x03100000)` 设为 Device RW/XN，最后 `arch_enable_mmu()`。`DRAM_PHY_ADDR=0x40000000` 只供 LK 初始 section map 使用，不是 `H700_MEMBASE/H700_MEMSIZE` 或任何分配授权。fake-MMU 回归通过。

**本轮 CPUFreq 诊断修正（`b421b077e5f27c95bf61679762b70a86d66131cb`）：** pinned Linux 4.9 候选 `cpufreq.c` 显示 `scaling_cur_freq` 对 `cpufreq-dt` target driver 返回缓存 `policy->cur`；读取 `cpuinfo_cur_freq` 会调用 driver getter，若读数不同，`__cpufreq_get()` 会通过 `cpufreq_out_of_sync()` 更新缓存并排队 policy update。旧 v2 先读 scaling 后读 cpuinfo，因此 480 vs 1008/1104MHz 不能证明持续硬件故障。v2 CPU zone0 46.637°C 且 first passive trip 60°C；固定 power_allocator 候选在低于开关温度时允许最大功率，但其他 cdev/策略及 exact shipping binary 仍未知。新只读 collector 增加 cpufreq path identity、getter 前后 scaling reread、thermal cooling_device cur/max state 与 zone binding、DCDC1/CPUX eight-sample capture。

**自动化验证：** MMU/PIO 修正后的 H700 AArch64、AArch32 ARM、AArch32 Thumb-2 compile + host regressions [#45526769732](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45526769732) 成功；最新 collector 诊断测试 H700 compile/regression [#45526953772](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45526953772) 和 Linux/Windows boot-audit tools [#45526953774](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45526953774) 成功。完整 Linkslot/mGBA/LK feasibility [#45526769421](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45526769421) 已成功完成；它覆盖 1.0.45 Rust Frontend、mGBA AArch32 static archive + newlib/VFS v3、真实 C ABI 链接探针、A32 BL33 入口/异常向量 prototype 与 memory fail-closed guard。Build report 明确说明这是 host/cross-build prototype，**不是完整 LK kernel target link、boot image 或实机启动**。

**本轮 PIO clock-boundary 审计：** H616-family 参考 PIO DTS 以 `CLK_APB1` 为主总线时钟且无独立 PIO reset property；参考 CCU `apb1_clk` 为 `SUNXI_CCU_MP_WITH_MUX`（offset `0x520`），无 PIO 专属 gate。RG34XX v2 Linux clk_summary 报告 APB1=100 MHz、PIO consumer=1。该证据支持不添加未经验证的 PIO gate/reset 写入，但 BL33-entry APB1 parent/rate 仍未实测；LED 代码不改 CCU。
**本轮 LK MMU static footprint：** pinned LK `arch/arm/mmu.c` 使用 `tt[4096]` 静态 section table，占 16 KiB、16 KiB 对齐的 BSS footprint。MMU hook 本身不动态申请 DRAM，但这块 BSS 和其余 LK BSS/stack/heap 的实际 placement 仍在内存 ownership 审批范围内。

**下一步 CPU/DVFS 工作：** p4 解压 SHA、公示 survey 前缀比对、live↔p4 GNU Build ID 精确匹配及 `vmlinux-to-elf` 恢复 AArch64 ELF 已完成，不重复采集。先完成匹配 binary 的全量 probe CFG、raw SID→mask/name 分支、两项 OPP setter 的返回处理和 cpufreq-dt 注册条件；随后恢复目标 OPP core 的 supported-hw / named-voltage filtering 逻辑，解释它与九档 runtime list 的冲突。保持完整 boot/ELF 本地，仅上传有限反汇编片段/JSON。再推进安全 BL33-entry 只读 PMIC/CPUX readback、CPU1–CPU3 handoff 与 timer continuity。AVCC 校准未解决前不开放 Auto/Boost；所有 DVFS 写操作保持禁用。

**本轮 OPP 候选分析工具：** 新增 `tools/analyze_h700_opp_candidates.py` 与 `tools/test_analyze_h700_opp_candidates.py`，按 Linux 4.9 OPP core 规则枚举无 `supported_hw` 与所有一单元 mask，准确处理 `opp-supported-hw` 属性缺失/空属性、命名电压属性 fallback，并与实际 `scaling_available_frequencies` 对比。工具要求 parser-v3 presence 字段；输出仅是 source/table fingerprint，不解码 SID、不授权 OPP，也不写硬件。

**本轮 OPP analyzer 自动化验证：** `da41806e47dbd2e0d57e23e9ff6ff405a5514e29` 通过 [H700 AArch32/AArch64 compile + host regression #45537478198](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45537478198) 和 [H700 boot audit tools #45537478202](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45537478202)。新测试覆盖完整 u32 位域、多版本 OPP cell、exact witness 约束和电压 min/target/max 顺序。前一中间提交 `300ae422` 的一项 test fixture 未禁止低位候选，已修正并在 `da41806` 两平台重跑通过。

**最近完成的 RG34XX 设备采集：** parser-v3 inventory 已上传至 `reports/rg34xx-inventory-20261125-cpux-v3/`，并完成 OPP exact-candidate analysis 与 cpufreq getter reconciliation。原始 binary 保留在采集机，未上传；报告可复查文本 JSON 与哈希清单。
**boot 二进制身份核对：** 已通过 [p4 元数据](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-boot-p4-metadata.json) 与 [解压 hash / aligned notes](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-boot-p4-verified.json) 完成 live kernel ↔ stock p4 kernel 精确 Build ID 匹配；解压 SHA-256 与公开 BaseOS survey 前缀核对成功。`vmlinux-to-elf` 和首轮 probe/OPP setter 反汇编也已完成，报告见 [cpufreq disassembly](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-cpufreq-driver-disasm.txt)。
**本轮 RG34XX cpufreq probe 完整 CFG + OPP core（2026-11-25，纯离线静态）：** 仅用本地已符号化 ELF，不接触设备、不读 `/sys`/`/dev/mem`、不调用 `sunxi_get_soc_ver`、不写寄存器、不重导出 p4。产物仅上传有界文本/JSON：[驱动关键函数反汇编](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-cpufreq-driver-disasm.txt) / [full CFG](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-cpufreq-full-cfg.txt) / [full CFG JSON](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-cpufreq-full-cfg.json) / [OPP core disasm](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-opp-core-disasm.txt)。ELF 与 boot 镜像保留本地。
  - `sun50i_cpufreq_nvmem_probe` = `0x…72ccc`–`0x…73148`（不含），287 指令、69 严格基本块、106 edges；`st_size=0`，边界取"闭合 epilogue 的 ret，其后继为不同 FUNC（`find_deepest_state`）且非本函数分支目标"，全程连续块覆盖无缝隙。
  - 十个显式 speed 比较值及分支表：`0x2000`/`0x2800`→mask 0 + 前缀 `a` + 后缀恒 1；`0x5c00`→mask 0 + `c` + 条件后缀 0/1/2；`0x6c00`/`0x7c00`→mask 0 + `b` + 条件后缀 0/1；`0x2400`/`0x5000`/`0x5400`/`0x7400`→mask 1；`0x2c00`→mask 2；其余→mask 8。
  - **b0/b1 确认保持 mask=0 且跳过 `set_supported_hw`**：`0x…72f24` 块不写 mask 槽，唯一 `str` 在 `0x…72f44`，b 路径经 snprintf 后跳 `0x…72f48` 绕过之；`0x…72fd8 cbz w1,[x29,#0x74]` 因此在 b 路径必然跳过 setter。据此 no-mask + `b1` 模型由运货二进制证实（此前仅为 DT/runtime 指纹）。
  - 零值保证：`0x…72fa8 cbz` 在名字空时跳过 `set_prop_name`；`0x…72fd8 cbz` 在 mask=0 时跳过 `set_supported_hw`。故 a/b/c 只设名字、mask 组只设 mask，两 setter 不会同时触发。`set_supported_hw` 失败打印 `"Failed to set supported hardware"` 并 unwind 至 `0x…73070`，不注册 cpufreq-dt；`platform_device_register_full("cpufreq-dt")` 在 `0x…7303c`，仅 CPU 循环结束后到达。
  - OPP core：`_of_add_opp_table_v2`（`0x…84bdac4`–`0x…84bdb78`）中 `supported_hw==NULL` 时凡带 `opp-supported-hw` 节点一律剔除、仅留无属性节点；注册 mask 时无属性节点仍接受且要求某 `(mask[i]&bit[i])!=0`；命名电压回退 `opp-microvolt-<name>`→`opp-microvolt`。parser-v4 的 live `live-fdt.json` 含全部 38 个 OPP entries、property-presence 与 named-voltage 值；b1 变体恰好对应九档 runtime rates，五个被拒频点没有 b1。因此 no-mask + b1 与运行列表完全对账；mask 1/2/8 会多纳入匹配节点而超过九档。
  - NVMEM：`nvmem_cell_read` 按 `cell->bytes`（speed 为 2）分配（含 `__GFP_ZERO`）并读 2 字节；probe `ldr w1,[x20]` 读 4 字节 = 超越声明长度的**逻辑越界读**（忽略 `&len`），多出 2 字节仍在同一 slab 对象内，不必然是故障/缺页；风险是高 2 字节非零会落空比较而走默认 mask=8。不要把该读法抄进新 HAL。
  - 源码家族：与 AvaotaSBC/linux `sun50iw9_nvmem_xlate` 同血缘（SID `0x20`→`<<5`、`0x28`→`&0x3ff`、两 setter），但常量集、`a/b/c` 条件后缀命名、`{0,1,2,8}` 掩码、DEFER 分类与逐 CPU setter 循环均不同，**不是同一实现**；offset 相同不构成源码匹配。
  - 仍开放：speed cell 2-byte allocation 被 32-bit `ldr` 读取时的实际比较高半字、live `sunxi_get_soc_ver()&7`（b1 要求非零）、精确公开源码 commit 与所有硬件安全/授权门槛。CPU 服务保持 `POLICY_ONLY`、`hardware_applied=0`，所有 DVFS 写禁用。

**stock kernel 二进制身份与初步 ELF 摘录（历史结果，已由上面的完整 CFG / OPP 报告取代）：** p4/live GNU Build ID 精确匹配；解压 SHA-256 `39cf8c3eb9a75ff0eda51b4074cd65860c5aa5386093bcaf69339819421d84b7` 与公开 BaseOS survey 的 16-hex 前缀一致；`vmlinux-to-elf` 恢复 89,675 个 symbols。后续不再导出 p4 或重建 ELF，只基于已经上传的有界 CFG / OPP 报告工作。

**本轮显示刷新策略与 Slot 前端审查（2026-11-25）：** 纯 C policy/profile 位于 `platform/h700/display/`：UI 默认请求 120 Hz、UI preference 可选 60/120；GAME 默认请求 60 Hz，独立 `game_120hz_enabled` opt-in 默认 0。该策略仅表达 request，不连接 Frontend、不等待 VBlank、不写硬件；前端设置行和持久化尚未接入。Slot 不主动设置 120 Hz 或切换 UI/GAME 物理模式。最新 live FDT 与同启动周期 TCON/PLL readback 已将当前 60-Hz 基线闭合到 divider 11 + PLL raw `0x88001703`；public sun50iw9 reference formula gives 288-MHz parent and estimated 26.1818-MHz DCLK / 59.569 Hz。pvaibhav 120-Hz candidate gives 528-MHz parent and 52.8-MHz DCLK / 120.13 Hz at live totals, and the offline kernel #19 audit has since closed the field layout, divider source and runtime parent inheritance (see item 4 and `ADDENDUM.md`), but panel support for that mode remains unverified。pvaibhav strict allowlist requires `fog_fj035fhd05_v1`, not our `rg34xx_v1 + lcd_type=old`；our live panel entry is `fog_fj035fhd05_v1_panel` registered under name `rg34xx_v1`, so the label mismatch is a naming rule, not a different init routine。UI present and armGBA committed-frame metrics are not yet measured；the next method is Slot's 600-present latency trace and an armGBA-specific reader only after binary/source/symbol layout matching。Hardware writer stays disabled and `hardware_applied=0`。
 
**本轮 ION/TEE 内存取证（2026-11-25 16:30 +08:00）：** 新报告的 `tee-ion-runtime.txt` 记录 `CONFIG_CMA/DMA_CMA/OF_RESERVED_MEM/ION/TEE/OPTEE=y`、`/dev/ion` 存在；`/dev/tee0` 与 `/dev/teepriv0` 不存在，标准 OP-TEE probe 报 `api uid mismatch`。ION debugfs 的 `secure` heap 当前零分配，`sys_user` 当前分配 7,397,376 bytes（含 `ion_disp2` 7,372,800 bytes）；CMA 为 64 MiB、约 62 MiB free；FDT secure heap base/size 为零。以上是运行时快照，不能证明 vendor DRM SMC 不存在，也不能给 `[0x7ec00000,0x7fc00000)` 16 MiB 保留区定名。该区域继续按未知保留区排除，`H700_MEMORY_LAYOUT_APPROVED` 保持 unset；未执行 SMC/ioctl/安全区读取。

## 1. 项目目标与固定约束

- 目标设备是 **Anbernic RG34XX（非 SP）/ Allwinner H700 / Cortex-A53**，不是 RG353PS/RK3566，也不是 RG34XX SP。
- 主线：**GammaOS LK → H700 AArch32 BL33 → H700 C HAL → Linkslot main 1.0.49**。当前已验证的 H700 前端 overlay/CI 仍基于历史 1.0.45，必须完成新基线重跑后才可把代码和验收状态标记为 1.0.49。
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

当前目标稳定移植基线为 **main 1.0.49**；`test` 也是 1.0.49，851 个源码文件逐 blob 完全一致。未来差异审查分支为 `DEV 1.1.0`（相对 main 新增 21、删除 1、修改 46，重点增加 schema 2 平台包、Arcade/NeoGeo 与 FBNeo）。最新来源的原始 GitCode commit、归档 commit 与文件 SHA 见 [上游基线](upstream-baselines.md) 和 [三分支更新审查](linkslot-1.0.49-1.1.0-refresh.md)。**当前 H700 no_std crates、renderer/frontend patcher 与历史 CI 仍锁在 1.0.45**，1.0.49 迁移及重跑尚未完成；旧记录仅是历史基线。

## 3. 当前总体进展

### 已完成并有 CI/主机证据

- 历史 1.0.45 上游审查、三核心 GBA picker 状态机升级、Renderer overlay 基线更新；1.0.49 的新源代码审查见 [更新报告](linkslot-1.0.49-1.1.0-refresh.md)，适配重跑待完成。
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

### 历史 Linkslot 1.0.45 综合验收（2026-11-24 15:48 UTC；不代表 1.0.49 已通过）

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
3. **已完成 RG34XX inventory v4 与 stock CPUFreq binary audit**：[inventory v4](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-inventory-20261125-cpux-v4)、[驱动关键函数反汇编](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-cpufreq-driver-disasm.txt)、[完整 CFG](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-cpufreq-full-cfg.json) 已确认 live/stock kernel GNU Build ID 精确匹配、probe 287 instructions / 69 blocks / 106 edges、十个 speed 比较值；`0x6c00` 走 mask=0 的 b0/b1 命名属性路径并跳过 `set_supported_hw`。b1 与九档 runtime OPP grid 对账，但 live `sunxi_get_soc_ver()&7` 和 2-byte cell 的 32-bit over-read 高半字尚未取证；不得调用可能写 CE clock/reset 的 getter。无需重复提取 p4、恢复 ELF 或重跑 CPUFreq CFG。
4. **显示路径主干与主要面板生命周期已恢复（kernel #19，离线）**：[首轮报告](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/kernel19-display-reverse-audit) + [第二轮 ADDENDUM](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/kernel19-display-reverse-audit/ADDENDUM.md)，基于 Build-ID `a1d211f847a5fb69fe9898e15fbccdd3ea8d7b68` 匹配的 stock ELF。第二轮通过内存中的 `.rela.dyn` overlay 恢复 91,760 个未应用的相对重定位，证明 `disp_panel_para = disp_lcd_private + 0xf0`，`panel_info+0x80 = lcd_dclk_freq`；按 `lcd_if` 选择 divider 表，RG34XX 基线 `lcd_if=0`、`lcd_dclk_freq=26` 触发特例 divider 11。TCON `base+0x44` 低 7 位来自 `tcon0_set_dclk_div`，高 nibble `0xf` 由 `tcon0_open` 设置，故读回 `0xf000000b` 的两个来源都已定位。PLL 请求为 26×11=286 MHz；报告称 PLL 表步进 12 MHz、取到 288 MHz，与同启动 `clk_summary` 一致；第三轮已逐行导出全部 187 行并裁决该矛盾：187 行、288–2520 MHz、步进 12 MHz 成立，`0x6400` 是索引 178 的中间项（2424 MHz），末项为 `0x6800`（2520 MHz）。选频为“夹取 + 最近邻 + 等距取低”：286 MHz 请求落在 288 MHz（`0x1702`），572→576 MHz / divider 11 是外部强制保持 divider=11 的反事实，并非 stock `lcd_dclk_freq=52` 路径（stock divider=6、PLL 312 MHz、DCLK 52 MHz、约 118.31 Hz）。另一个 divider=10 / PLL=528 MHz 的外部候选算得约 120.13 Hz，但未验证面板。详见 [PLL-TABLE.md](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/kernel19-display-reverse-audit/PLL-TABLE.md)。

面板表已恢复为 17 项，RG34XX 命中 `fog_fj035fhd05_v1_panel`（注册名 `rg34xx_v1`）；`disp_lcd_enable` 主顺序、开/关 panel callbacks 及 TCON config 路径已追出。**仍未完全闭合**：`LCD_OPEN_FUNC` 步骤表的运行时 executor、四个 `struct clk *` handle 的取得点、bootarg `lcd_type=old` 与显示 profile/interface 的实际联系。时钟方面，驱动在 init 时用 `clk_get_parent` 保存现有 parent，再在 enable 时 `clk_set_parent` 回写同一 parent；它并非选择一个新的 mux parent，但会按 `lcd_dclk_freq × divider` 调用 `clk_set_rate`。实测当前 `tcon_lcd` parent/rate 为 `pll_video0x4 / 288 MHz`，这是 runtime `clk_summary` 证据。

120 Hz 仍为**未验证/不支持启用**：未找到绑定 `rg34xx_v1` 的第二套 timing 或 runtime refresh switch；面板规格容差、VBlank/TE 切换边界、accepted divider、回滚路径仍缺。**不连接设备、不写 TCON/PLL/CCU/PMIC、不接入硬件 writer**。第二轮自检 82/82、第三轮自检 47/47 均通过；第三轮的检查包含 187 行逐行重算、表末项与 `0x6400` 位置、以及两个请求的选行结果。

**Linkslot 1.0.49 综合 feasibility CI：** [#45631521531](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45631521531) 已在修复 mGBA static selector 的 `Option<LinkSetup>` 类型后 **71/71 步骤成功**；该成功 run 的 head 为 `d333eb30dc8a21b4e9b9509bc08a3760223c091a`。通过内容是 host/cross-build、软件 renderer、VFS/storage regressions、真实 mGBA AArch32 static archive 与 Linkslot API link probes、BL33 layout probe；**不是可刷写完整固件、实机启动或游戏运行验证**。
5. 证明 BL31→BL33 CPU1–CPU3 状态、单核 invariant、备用父时钟切换和 timer continuity；同时解决 AVCC thermal calibration 或确认独立可信温度来源。前述门槛闭合后才设计 C HAL transition（升频先升压、降频先降频后降压、readback/timeout/rollback/hysteresis）。
6. 最后才开放实际 Performance/Boost。不得把 ABI v1 的 `POLICY_ONLY` 当作 DVFS 已完成。
7. **显示 120/60 Hz：** live FDT timing/interface reparse、TCON/PLL 只读读回与 kernel #19 离线逆向均已完成（[审计报告](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/kernel19-display-reverse-audit)）。60-Hz 状态：TCON word divider 11（`0xf000000b`）、PLL raw `0x88001703` 经 `calc_rate_video0` 解为 288 MHz，288/11 → 26.1818 MHz → 820×536 下 59.569 Hz，与 IRQ proxy 约 59.6 Hz 吻合。已确认 divider 11 是常量表按 **`lcd_if`** 索引（`0 → 6`）叠加 **`lcd_dclk_freq == 26` 的特例覆盖**（→ 11）得到的编译期结果，`lcd_type` 与 SoC 版本都不参与该分支；`lcd_dclk_freq` 虽不参与底层分频写入，但参与后续 `clk_set_rate` 目标速率计算（286 → 288 MHz）。镜像内没有 120 Hz 模式表、没有运行时刷新切换路径，因此 120 Hz 仍为**未验证/不支持启用**。下一步：用 Slot present 计数和 armGBA committed-frame 计数建立独立 UI/GAME FPS；在精确 panel/revision、120-Hz clock/timing、VBlank/TE boundary 与 scanout/cache ownership 闭合前，不接入硬件 writer、不把 120-Hz candidate 标成 supported。

## 7. 续接规则

新的工作轮次按以下顺序恢复上下文：

1. 先读取本文和 GitHub 当前分支 HEAD / 最新 Actions 状态；本文的 HEAD 是核对时快照，不应假设之后没有新提交。
2. 当前代码只在 `porting/linkslot-1.0.45-feasibility` 上推进；先检查分支差异与最新 CI，不重复建立已有分支/文件。
3. 依任务读取专项文档：
   - CPU 调频：本文 + [H700 CPU Performance Service / DVFS](h700-cpu-performance-service.md) + [H700 CPUX clock-source audit](h700-cpux-clock-source-audit.md) + [RG34XX CPUX live audit](rg34xx-cpux-live-audit-2026-11-25.md)
- FDT OPP parser: [`tools/inspect_h700_fdt.py`](../tools/inspect_h700_fdt.py) + [`tools/test_inspect_h700_fdt.py`](../tools/test_inspect_h700_fdt.py); parser-v2 needs a new live inventory capture.
- OPP/cpufreq source check details: [RG34XX CPUX live audit §1.5](rg34xx-cpux-live-audit-2026-11-25.md).
   - 总体路线： [roadmap](roadmap.md)
   - 最新 Linkslot 源码差异： [1.0.49 / 1.1.0 refresh](linkslot-1.0.49-1.1.0-refresh.md)；历史 1.0.45 适配记录：[1.0.45 portability update](linkslot-1.0.45-portability-update.md)
   - LK/H700 runtime 架构： [Linkslot H700 runtime architecture](linkslot-h700-runtime-architecture.md)
   - BL33/内存交接： [RG34XX boot handoff audit](rg34xx-boot-handoff-audit.md)
4. 只把 CI 明确通过的提交标为已验证；host compile 不等于实机启动，合成 OPP 不等于真实可用频点。
5. 每轮完成后先更新本文，再更新 roadmap 和对应专项文档；把新 commit SHA、Actions run URL、失败及尚存风险写入本文。


**第三方 H700 120 Hz 实现审查（2026-11-25）：** 已审查 [`pvaibhav/slot` `5206e3fa39aa4a9b702cd9048a07dab3b5ab95da`](https://github.com/pvaibhav/slot/tree/5206e3fa39aa4a9b702cd9048a07dab3b5ab95da)。该 fork 的 Linux 侧实现确实比我们当前 policy-only 完整：`PanelRefresh::probe()` 对 model/panel/DT timing 严格 allowlist，使用 `sunxi_dump` 写 `DIVIDER 0x06511044` / `PLL 0x03001040`，120 Hz 先 divider 后 PLL，60 Hz 先 PLL 后 divider，读回两寄存器并在失败时 best-effort 回滚 60 Hz；启动时应用，唤醒时在开背光前重应用。其测量工具读取 LCD IRQ，而非依赖 stale `ref_fps`。但 **不能直接移植到本机**：fork 对 target `rg34xx` 的 gate 要求 `fog_fj035fhd05_v1 + 720×480 + lcd_dclk_freq=26 + HT/VT=820×536`，而本机最新 live FDT 是 `rg34xx_v1`；fork 自己也注明非 SP RG34XX 没有连接实机做 live test。BaseOS fleet panel survey 同样将 RG34XX 标为 `rg34xx_v1`。此外该 fork 设定为全局默认 120 Hz，启动/唤醒恢复，不提供独立 UI 偏好与 GAME 120 Hz 开关；与我们的 per-scene policy 不同。详细分析见 [`slot-pvaibhav-120hz-review.md`](slot-pvaibhav-120hz-review.md)。结论：借鉴 strict gate、C HAL owner、write ordering、readback、rollback 和 IRQ 验证；不复制其地址/数值或修改 panel allowlist。live FDT timing/interface 与 TCON/PLL 只读读回均已完成；下一项改为匹配 kernel #19 的 `rg34xx_v1 + lcd_type=old` 初始化、PLL 位域与时钟父链，并完成 UI present / armGBA committed-frame 独立计数。硬件 writer 继续关闭，`hardware_applied=0`。


## 8. pvaibhav/slot RG34XX non-SP 实机运行采集（2026-11-25 18:44–18:45 +08:00）

报告已上传至 [reports/pvaibhav-slot-runtime](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/pvaibhav-slot-runtime)，同一台 RG34XX、同一系统分别采集 UI 29.624 s（27 samples）和 GAME 29.749 s（23 samples）。[专项分析](docs/pvaibhav-slot-rg34xx-live-capture-2026-11-25.md)。

- 两个显示 IRQ 均稳定约 59.6 Hz：UI IRQ292/293 = 59.648/59.614 Hz；GAME = 59.632/59.632 Hz。驱动 `fps` 只在 59.8/60.2 间变化，`ref_fps=59`。DE rate 696 MHz；两轮 framebuffer、display status 和 clock summary 相同。没有观测到 UI/GAME 刷新模式切换，当前实机显示扫描仍约 60 Hz。
- 新增 [FDT reparse 报告](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/pvaibhav-slot-runtime/fdt-reparse)：JSON 已脱敏并记录 hash，未公开 raw DTB。解析确认 `rg34xx_v1`、720×480、RGB666、DCLK property 26 MHz、HT/VT 820×536、H porch 46/2/52、V porch 20/5/31，静态派生 nominal 59.155 Hz；bootargs 为 `lcd_type=old`。同启动周期 TCON/PLL 只读回读随后得到 `0xf000000b` / `0x88001703`。若 `tcon_lcd=288 MHz` 直接馈入 divider 11，则 DCLK 推算 26.1818 MHz、refresh 59.569 Hz，与 IRQ 约 59.63 Hz 的差异缩至约 0.1%；这是时钟树假设下的推算，不是独立物理测量。Knulli RG34XX DTS 提供 26 MHz/820×536 配置参考；MustardOS 的 54 MHz/1679×536 是另一配置，不能混用。
- Slot PID 2842 两轮相同，cmdline/executable 映射路径是 `/mnt/sdcard/System/slot`（comm 被 loader 截断为 `ld-linux-aarch6`）。GAME 的 `maps` 确认加载 `armgba_libretro.so`。多前端路径不是进程名 `slot-pvaibhav`；后续 collector 必须按真实 `/System/slot` executable path 匹配。
- `slot.state` 的 `high_refresh=1`，但配置值本身不是硬件模式证明；IRQ/clock/readback 没有显示 120 Hz。UI present FPS 和 armGBA committed game FPS 均未测得，不能从 IRQ 推断游戏 FPS。
- pvaibhav fork 的目标 `rg34xx` gate 要求 `fog_fj035fhd05_v1`；本机实际是 `rg34xx_v1`，虽然 report README 所列的几何和时序数值匹配，但面板 byte identity 不匹配。其 `PanelRefresh::probe()` 会拒绝该设备、不会开放 writer；不可放宽 allowlist 或复制寄存器值。
- `sunxi_dump` write/dump 节点存在且 root 可写，但本次没有写入。保持硬件 writer 关闭、`hardware_applied=0`。

运行环境已确认 `BASEOS_TARGET=rg34xx` / `BASEOS_PLATFORM=h700`，Slot 进程 UID=0 且具备完整 effective capabilities；pvaibhav 的 `DevicePlatform` 以 `BASEOS_TARGET` 选择 panel gate，`supports_high_refresh()` 只在 `PanelRefresh::probe()` 成功时为 true。由于 `rg34xx_v1` 不等于要求的 `fog_fj035fhd05_v1`，reviewed source 会拒绝该面板，`high_refresh=1` 不会触发写入。设备端 `System/slot` 与 `armgba_libretro.so` 的 SHA 已记录，但尚未与 H700 release ZIP 解包文件逐字节比对。新增源码审查结果：Slot 在非零 `SLOT_TRACE_LATENCY` 或 `SLOT_ROOT/System/trace-latency.on` 启用时，每 600 次主 surface swap 输出 `presents/s`；armGBA 的 `gba->video.frameCounter` 在 VBlank 帧结束处递增。当前 probe 不支持 armGBA，需先匹配 installed core 的 release/build identity，再验证 ELF 中 `gba` 符号和 `struct GBA::video` 偏移，最后实现只读采样。UI present、armGBA committed frames 与 LCD IRQ 分开统计。GAME 120 Hz 维持默认关闭。

- 19:12 设备二进制 SHA-256 已记录：`System/slot`（18,718,728 B）`8f0fd40da4ac161e5fbb9a56a99c254acf8b2f31806e2be6bdaa929b5ee28505`；`SYSTEM/armgba_libretro.so`（2,021,992 B）`4ebae098cc4848fb459fc18299c39acbdbbbeb732db5c3f66d1ff3f6e142e889`。尚未与官方 H700 release ZIP 中提取的文件比对，因此安装二进制具体版本仍待确认。


**Panel profile nuance:** the pinned [BaseOS variant survey](https://github.com/pvaibhav/BaseOS/tree/63137afeda1c0bc4ff771be6f03012b7f8147aca/experiments/h700-boot-speed/variant-survey) maps target `rg34xx` to `rg34xx_v1` 720×480; `fog_fj035fhd05_v1` belongs to other targets/geometries. The live bootargs contain `lcd_type=old`, and that survey notes the Linux panel driver uses `lcd_type` to select its init sequence. Thus the reviewed pvaibhav fork's `rg34xx → fog_fj035fhd05_v1` allowlist is not a match for this standard non-SP profile. Do not substitute the string without proving the exact `rg34xx_v1 + lcd_type=old` timing and register sequence.


**RG34XX vendor panel-selection audit:** the pinned BaseOS compatibility contract maps target `rg34xx` to `rg34xx_v1` / 720×480, with vendor PE6 high → `lcd_type=old`, low → `boe`. Its U-Boot inventory selects the `rg34xx_v1` panel struct (`selected_by_dt=true`) but the struct calls a common init function whose symbol string contains `lcd_panel_uboot_fj035fhd05_v1_init`. That routine label is not the DT `lcd_driver_name`. The live `lcd_type=old` is consistent with the vendor PE6-high branch. This confirms the fork's `rg34xx → fog_fj035fhd05_v1` gate does not describe the standard target profile; do not change that allowlist or copy the Linux refresh registers without independent validation. Details and pinned source links are in [the live capture analysis](docs/pvaibhav-slot-rg34xx-live-capture-2026-11-25.md).

**pvaibhav 安装包版本核对入口：** GitHub tag `v1.16.0-pvaibhav.2` 经 annotated tag 解析后精确指向审查源码 `5206e3fa39aa4a9b702cd9048a07dab3b5ab95da`。官方 H700 ZIP `slot-v1.16.0-pvaibhav.2-h700.zip` 为 17,556,056 B，资产 SHA-256 `1f9bdaa1110eb4f6006cc9da6a8bb34bca694ea53555c3138dcf656f0dfd1fca`。下一步由本地 Agent 下载并核验 ZIP，再比较其中 `slot` / `armgba_libretro.so` 与设备哈希；目前没有声称两者相同。


**相关 H700 时钟分频风险参考（非本机证据）：** MustardOS 2026-11-02 的另一台 RG34XX-H 曾因相关 `sun50iw9` BSP 的 RGB/HV TCON 固定 /6 与 PLL 最低 288 MHz，出现 DTB 标称约 59.97 Hz、实测约 106.6 Hz；其 DTS 后来将该机 DCLK/HT 改为 54 MHz/1679，见 [讨论](https://community.muos.dev/t/rg34xx-h-incorrect-display-refresh-rate/1825) 和 [修正提交](https://github.com/MustardOS/internal/commit/76e454dfa9758a99f85115f80a29121dabb13a01)。另一台 RG34XX-H 无法复现，说明不同修订/内核组合有差别。该数据不能用于本机，只支持一个结论：必须从精确 kernel #19 追出最终 TCON DCLK divider，不能只信 DTB property 或复制 54MHz/1679。


**TCON divider readback complete:** the same-boot `sunxi_dump/dump` report records `0x06511044=0xf000000b` (divider 11) and `0x03001040=0x88001703` (PLL raw). The related pinned sun50iw9 BSP's generic HV divider 6 is not the active value on this device. Under the 288-MHz parent assumption, /11 predicts 59.569 Hz; this matches IRQ proxy within ~0.1%. Exact Linux 4.9.170 #19 PLL decode, parent chain and `rg34xx_v1 + lcd_type=old` initialization still require source provenance. No writes were performed.


**报告脱敏与历史状态（2026-11-25）：** 当前 `reports` HEAD `34ea683` 的 FDT JSON 对 bootargs 敏感字段使用 `[REDACTED]`，raw DTB 未公开。该提交说明此前包含未脱敏 blob 的三提交序列已被替换；核对当前 `reports` 分支最近 100 个祖先提交，旧文档列出的两次提交不在当前可达历史中。此项只证明当前分支历史状态，不代表旧克隆、fork、下载缓存或 GitHub 保留对象立即消失；继续避免在日志、聊天和后续报告中复制设备序列标识。


### 8.1 TCON / PLL 只读回读更新（2026-11-25 19:59 +08:00）

最新报告位于 [reports/pvaibhav-slot-runtime/tcon-readback](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/pvaibhav-slot-runtime/tcon-readback)，采集于 Linux 4.9.170 #19、与 FDT 重解析报告相同的启动周期：

- `0x06511044` 原始值 `0xf000000b`；按已审查 TCON 字段定义，低 7 bits 为 divider=11，高 nibble 为 enable。
- `0x03001040` 原始值 `0x88001703`；不解码 PLL N/M 位域。
- `clk_summary` 同时记录 `tcon_lcd=288 MHz` / `pll_video0x4=288 MHz`、`pll_video0=72 MHz`、`display_top=200 MHz`；显示驱动状态 `ref_fps=59`、`fps=60.2`。

pvaibhav/slot 固定源码 `crates/slot-power/src/refresh.rs` 的 60-Hz register pair 正好是 PLL `0x88001703` + DCLK divider `0xf000000b`，与本机只读值完全相同；其 120-Hz 写入候选则是 divider `0xf000000a` + PLL `0x88002b03`。这验证了该 fork 对当前 60-Hz 基线所用寄存器地址/值与本机状态相符，但**不授权移植 120-Hz writer**：该 fork 的 `rg34xx` allowlist 仍要求 `fog_fj035fhd05_v1`，本机真实 panel 是 `rg34xx_v1` 且 bootargs 为 `lcd_type=old`。

以 `tcon_lcd=288 MHz` 直接馈入 divider 为条件，DCLK 推算为 26.1818 MHz；除以 live FDT 的 820×536 totals 得 59.569 Hz，与 UI/GAME IRQ proxy 59.61–59.65 Hz 一致到约 0.1%。这解释了 live FDT 26 MHz 整数属性计算出的 59.155 Hz 与 IRQ proxy 的差距，但仍不是示波器/光学测量，也没有验证 120-Hz 模式。PLL raw 位域、精确 kernel #19 显示驱动源码对应关系以及时钟树完整父链仍待核验。

报告中的 FDT JSON 已脱敏；raw DTB 未发布。所有显示硬件 writer 继续关闭，`hardware_applied=0`。


### 8.2 UI present / armGBA frame-counter source audit (2026-11-25)

- Slot pinned source `pvaibhav/slot@5206e3fa39aa4a9b702cd9048a07dab3b5ab95da` implements a trace gate via nonzero `SLOT_TRACE_LATENCY` or `SLOT_ROOT/System/trace-latency.on` (default root `/mnt/sdcard`, resolved once at first use). With tracing enabled before process startup, `device_app.rs` logs `600 presents` plus `presents/s`, nominal frontend frame period and missed game periods. This is the direct UI/main-loop present metric; it is not LCD scanout frequency.
- armGBA pinned source `MagnominusGit/armGBA@fad6c4b0c0176a76e10667f4aa071a8feb4425c0` defines global `struct GBA* gba`; `struct Video` contains `uint32_t frameCounter`; `src/video/video.c` increments it at V-blank frame end and `retro_run()` waits for it. This is a source-confirmed committed emulation-frame counter.
- The current public `probe.rs` only supports gpSP and mGBA. Before reading armGBA through `/proc/PID/mem`, match the deployed core hash to an exact build/release, verify ELF symbol visibility and pointer mapping, and calculate `offsetof(struct GBA, video)+offsetof(struct Video, frameCounter)` from that exact source. No memory probe has been run yet.
- Required output columns remain separate: `slot_presents_per_s`, `armgba_committed_frames_per_s`, `lcd_irq_per_s`, plus CPU MHz and temperature. No one metric substitutes for another.


### 8.3 Public sun50iw9 PLL formula cross-check (reference only; 2026-11-25)

The related pinned `qiaoweibiao/T507_Kernel@98fcc7d3f112e51b0edfb71536da89cb2115106f` clock driver places the `pll_video0x4` N field at bits [15:8], D1 at bit 1, and calculates `24 MHz × (N+1)/(D1+1)`; its DTS assigns `tcon_lcd` parent `pll_video0x4`. Under that public BSP layout:

- Live raw `0x88001703`: N=23, D1=1 → `pll_video0x4=288 MHz`; `/11` → 26.1818-MHz DCLK → 59.569 Hz at 820×536.
- pvaibhav 120-Hz candidate `0x88002b03`: N=43, D1=1 → `pll_video0x4=528 MHz`; `/10` → 52.8-MHz DCLK → 120.13 Hz at 820×536.

This is a strong internal-consistency check for the reference pair, not a proven decode of shipping kernel #19. The exact binary's PLL layout and live TCON parent/mux still need source matching; panel tolerance and runtime transition safety are unproven. Keep hardware writes disabled and `hardware_applied=0`.


### 8.4 Exact kernel #19 reverse-engineering path (2026-11-25)

KNULLI's [RG34XX developer documentation](https://knulli.org/devices/anbernic/rg34xx/#extracting-the-kernel-and-bootloader) states that Anbernic has not published the RG34XX kernel/U-Boot source; KNULLI's release uses bootloader/kernel binaries extracted from stock firmware. Our `reports/rg34xx-inventory-20261125-cpux-v4` records Linux 4.9.170 #19's GNU Build ID, and the stock `mmcblk0p4` extracted kernel ELF was previously confirmed to have that exact same GNU Build ID. The complete ELF remains local; do not upload the 64-MiB partition or whole ELF. Continue offline using that already symbolized ELF: locate `sunxi_clk_factor_pll_video0x4`, PLL factor/CCU functions, `disp_al_lcd_get_clk_info`, `disp_al_lcd_cfg`, `tcon0_set_dclk_div`, TCON LCD init/driver callbacks and `lcd_type` branches; disassemble bounded function ranges and compare register offsets, masks, divider encoding, PLL N/D1 layout, clock-parent selection and call ordering against the live readback. Publish only bounded disassembly excerpts, symbol names and verification JSON. The public sun50iw9 reference is a hypothesis until this binary-level check closes.


## 9. 2026-11-26 pvaibhav/slot 120 Hz 复测与 FPS 采样下一步

### 新增 ADB 实测结果

最新证据已上传至 [reports/slot-refresh-audit](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/slot-refresh-audit)，reports 分支原始报告提交 `e320277a08f70524853a631881d2bc8bb7fe7745`；报告解释更正随后提交至 reports 分支（见该目录 `SUMMARY.md` §10）。

同一台 RG34XX non-SP、BaseOS 1.5.3 / Linux 4.9.170 #19，通过 ADB 分别在 UI 和实际运行 GBA 的场景采样 12 秒：
- IRQ293：UI **59.667 Hz**，GAME **59.583 Hz**；显示 driver 两态均为 `fps=60.2`、`ref_fps=59`、`de_rate=696 MHz`。
- PLL `0x03001040` 两态均为 `0x88001703`（按已记录的相关 sun50iw9 参考公式解为 288 MHz）；没有观察到 120-Hz PLL 编程。
- 本轮 TCON `0x06511044` 读取返回 I/O error，不能把本轮 divider 当作已读回；另一次独立同设备采集中曾读到 `0xf000000b`。
- 面板 `rg34xx_v1`、720×480、820×536、DCLK property 26 MHz、`lcd_if=0`；Slot PID 2301 在两态相同，GAME 只新增加载 `armgba_libretro.so`，显示栈没有重初始化。两态 `high_refresh=1`，但没有带来模式变化。
- 结论：这次运行 UI/GAME 均处于约 60-Hz 路径。LCD IRQ 只是 scanout/vblank proxy；UI presents 和 GBA core committed frames 仍未测量。

### 报告解释更正

新报告最初把“FDT 只有一套 timing”当成动态 120-Hz 模式的否决证据，并把 576-MHz raw word 写成 pvaibhav 的目标配置；这两处已在 reports 分支 [SUMMARY.md §10](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/slot-refresh-audit/SUMMARY.md) 修正。pinned `pvaibhav/slot@5206e3fa39aa4a9b702cd9048a07dab3b5ab95da` 的真实 high pair 是 divider `0xf000000a` + PLL `0x88002b03`（参考公式 528 MHz / 10，约 120.131 Hz）；动态 writer 不依赖 FDT 第二个 timing 节点。这不改变实测结论：设备仍读到 60-Hz PLL word，IRQ 约 59.6 Hz，而且本机 `rg34xx_v1` 不满足 fork 的 `fog_fj035fhd05_v1` 精确 allowlist。

### FPS 计数器审查与安全边界

- UI：pinned Slot 的 `device_app.rs` 在 tracing 开启时每 600 次主 surface swap 输出 `presents/s`。开关为非零 `SLOT_TRACE_LATENCY` 或 `$SLOT_ROOT/System/trace-latency.on`；`OnceLock` 在首次读取时缓存，所以标记文件必须在 Slot 进程启动前存在。下一轮先只读确认 PID 2301 的 stderr fd 去向和安装的 `System/slot` SHA，再决定如何开启/收集日志。
- GBA：pinned armGBA 源码在 VBlank 帧结束递增 `gba->video.frameCounter`，是 core committed-frame 候选。其 `libretro/link.T` 只导出 `retro_*`、对其余符号使用 `local: *`；源码全局 `gba` 并非可依赖的动态导出符号，禁止假设 `dlsym` / `nm -D` 可取得它。必须先将设备 `armgba_libretro.so` SHA 与确切 release/build 匹配，并检查完整 ELF symbol table。只有符号、load bias、指针映射和由 pinned `gba.h` 计算的 `offsetof(GBA, video)+offsetof(Video, frameCounter)` 全部闭合后，才允许做有界、只读的 `/proc/PID/mem` 采样；否则改为自建带诊断导出的 core，不做盲目内存扫描。
- 三项指标必须分别报告：`slot_presents_per_s`、`armgba_committed_frames_per_s`、`lcd_irq_per_s`。LCD IRQ 不代表 UI FPS 或模拟器 FPS。

### 当前 CI 与后续执行顺序

- Linkslot LK Rust feasibility [#45646868006](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45646868006)：**成功**，最新已核验 70 余步综合 host/cross-build 流程全通过（含完整 H700 BL33 overlay、Linkslot 1.0.49、mGBA VFS v3、存储只读回归）。
- H700 boot audit tools [#45646868087](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45646868087)：**成功**。
- 上述均是 host/cross-build 和工具回归，不是刷写固件或实机启动证据。

下一步先完成 release ZIP 与设备 `System/slot` / `armgba_libretro.so` 的 SHA 对照、ELF 符号表检查、armGBA C 结构偏移编译探针；并只读确认 Slot stderr 日志去向。完成这些前，不启用标记文件、不重启前端、不读进程内存。随后先测 UI presents，再在 GBA 运行时按已验证地址采样 committed frame counter。硬件 writer 继续禁用，`hardware_applied=0`。
