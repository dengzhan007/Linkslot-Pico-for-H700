# 项目方案与实施路径对目标匹配度审查 — 2026-11-27

对 `dengzhan007/Linkslot-Pico-for-H700` 在 `porting/linkslot-1.0.45-feasibility`
分支 HEAD `afdf9c2` 上的一次**只读文档与提交历史审查**。不连接设备、不构建、不修改受审仓。

审查对象：`README.md`、`docs/decisions.md`、`docs/architecture.md`、`docs/roadmap.md`、
`docs/PROJECT-STATUS.md`、`docs/h700-a53-single-core-startup-audit.md`、
`docs/rg34xx-rtc-soft-entry-twi-audit.md`、`docs/h700-bl33-memory-ownership.md`，
以及提交历史与 `.github/workflows/`。

## 结论

**技术方案与项目目标一致；实施路径的依赖顺序倒置。**

分层架构、设备边界、存档/关机语义、上游同步策略彼此自洽，"host 编译 ≠ 实机验证"的纪律
在全仓一致执行，这一点做得比多数同类项目严格。问题不在方案，而在**推进次序**：

- 在"系统能否启动"（G1–G3）尚未闭合时，先建成了大量**上层能力**（CPU 策略服务、显示刷新
  策略、无 I/O 前端业务 crate、电源灯），这些能力当前没有任何实机路径可以执行；
- 唯一朝下的垂直切片（sdc0/CMD17/GPT/FAT）停在 11-23；
- **可打破僵局的"32 KiB 诊断 BL33 payload"打包链已 host 验证通过，但其最后一步在 roadmap
  中仍未勾选**，而同期 39 轮 vendor BL31 静态审计持续产出且未改变任何门槛状态。

## 1. 目标基准（由仓库文档定义，无歧义）

| 来源 | 内容 |
|---|---|
| `README.md` 项目目标第 1 条 | 开机直接进入 Linkslot 游戏库，首期只支持 GB/GBC/GBA |
| `README.md` 原则 1 | **先**验证 SD 启动、显示、输入、SD 读写、完全关机路径，**再**接入完整 UI |
| `docs/roadmap.md` 阶段序 | B 启动链/BL33 → C HAL → D 模拟器 → 完整功能 → **完全关机与发行验收（最后）** |
| `docs/decisions.md` ADR-003 | 长按电源键只写即时状态后完全关机；不自动写电池存档、不触发云同步 |
| `docs/decisions.md` ADR-012 | C/ASM 底层 + Rust `no_std` 前端 + 静态 C/C++ 核心；host 测试通过不等于实机可用 |

判定"是否匹配目标"的标准因此应当很直接：**本轮工作是否让上述任一项可验收能力向前移动一格**。

## 2. 逐维度判定

| 维度 | 当前状态 | 判定 |
|---|---|---|
| 分层架构（C/ASM + Rust no_std + 静态核心） | ADR-012、`architecture.md`、实际目录三者一致 | 匹配 |
| 输入 | evdev + gpiolib debugfs 双路实测，17/17 键闭合；X/Y 丝印与 DT 节点名互换三次确认 | 匹配，关键路径上的真设备证据 |
| 存储 | sdc0/CMD17→GPT→FAT→VFS host 路径通过；实机 sdc0 MMIO 与写盘未验证 | 方向正确，未到设备 |
| 上游基线 | 1.0.49 已归档且 71/71 CI 通过；分支名仍为 1.0.45，文档中"已迁移"与"仍锁 1.0.45"并存 | 事实源不唯一 |
| 显示 | 60 Hz 完整闭合（divider 11 / PLL 288 MHz / 推算 59.569 Hz，与 IRQ 实测吻合）；120 Hz 结论为面板无资格 | 60 Hz 达标；120 Hz 属目标外 |
| 启动链 / BL33 | 打包补丁器与 32 KiB extent 已验证；诊断 payload 本体未产出；无实机启动 | **关键卡点，一步之遥** |
| 内存所有权 | `H700_MEMORY_LAYOUT_APPROVED` 仍 unset，fail-closed 保持 | 正确保守，但阻塞 G1 |
| 关机/待机（ADR-003） | 投入最大；静态已到 selector/TWI 级，运行时全部未知 | 严重超前于路线图阶段 |
| 纪律（host ≠ 设备） | 全仓一致：不写寄存器、不刷机、`hardware_applied=0` | 应继续坚持 |

## 3. 未闭合门槛台账（本次从各文档抽取，建议正式化）

仓库目前没有集中的门槛清单，以下为人工汇总。

| # | 门槛 | 状态 | 阻塞在 |
|---|---|---|---|
| G1 | 完整 LK H700 target link / 可刷写 BL33 镜像 | open | G2、G3 |
| G2 | `MEMBASE/MEMSIZE` + DRAM/heap/stack/framebuffer 所有权 | open（unset） | 缺 pre-BL33 观测手段 |
| G3 | CPU1–CPU3 在 BL31→BL33 时的停驻状态 | open | 需 pre-BL33 快照 |
| G4 | 显示 scanout / framebuffer ownership、面板实机点亮 | 部分 | G2 |
| G5 | 输入 producer 接线（PIO/IRQ 到 C HAL） | 部分（映射闭合，驱动未接） | G2、IRQ 模型 |
| G6 | timer / GIC PPI continuity | open | G3 |
| G7 | PCM DMA 音频 | 未开始 | G1 |
| G8 | 实机 SD 写盘与断电一致性 | 部分（host only） | G1 |
| G9 | PMIC / 完全关机（ADR-003） | open（静态超前） | 不可观测 |
| G10 | thermal calibration（AVCC 未校准） | open | 无可信温度源 |
| G11 | board-approved OPP 表 | open | G10、G9 |

关键结构：**G2、G3 都指向同一个缺失物——pre-BL33 的可信观测点。**

## 4. 产能分布：不是没干活，是方向朝上

按日统计（`platform`+`target` 为朝下代码，`integration` 为前端业务 crate，合计为该日全部提交）：

| 日期 | platform+target | integration | 当日总计 | 主要内容 |
|---|---|---|---|---|
| 11-23 | 104 | 0 | 270 | **sdc0/CMD17/GPT/FAT 只读读路径、CCU/APB1、PF0–PF5 pinmux** —— 唯一的真机 bring-up 垂直切片 |
| 11-24 | 33 | 106 | 459 | CPU OPP 策略 / Service ABI v1 / runtime owner、十余个 `no_std` 业务 crate、软件 rasterizer、LK project scaffold |
| 11-25 | 14 | 7 | 264 | 显示刷新策略（60/120 Hz policy）、PI12 电源灯、早期 MMU |
| 11-26 | 28 | 20 | 333 | BL33 入口快照 ABI、DRAM 容量 ABI、Data Abort 恢复、`SYS_CFG` dormant 读、PI12 gating |
| 11-27 | **0** | **0** | 163 | 全天提交均为 BL31 审计脚本与文档 |

11-24~11-27 四天合计 **1,219 次提交**；其中 11-27 整天没有任何朝下代码。

这些代码本身质量不低（fail-closed、ABI 版本化、host 回归齐全）。但结构上：

- 11-24~11-25 的绝大部分属于**上层**：CPU 策略服务、显示策略、无 I/O 前端业务 crate。
  它们只有在系统启动后才能被执行，当前处于"写好了但没人调用"的状态（文档亦自述
  `target_init()` 保持空实现、runtime 仍 unavailable）。
- 11-26 的 snapshot / abort-recovery / dormant SYS_CFG 属于**为审计提供观测手段的脚手架**，
  本身不产生任何验收。
- 真正朝下的推进停在 11-23 的 sdc0 读路径。

即：依赖顺序倒置——**在"能不能启动"之前，先建成了"启动后才有用"的能力**。

## 5. 审计链没有停止规则

- BL31 静态审计从 #94 跑到 #132，约 **39 次 CI 运行**。
- 真正闭合的子问题只有一条：selector=5 → R-TWI0（`afdf9c2` 的 SRAM relocation 结论），
  属干净的静态收敛，但它的下游（PMIC slave address、事务字节、GP_DATA2 消费者、最终
  power action）仍未知，且**在系统尚不能启动 BL33 之前无法被使用**，不能解锁 G1–G8 任一项。
- 39 轮运行没有使任一门槛由 open 变 closed；`PROJECT-STATUS.md` 中
  `未验证 / 未完成 / 不得 / 未知 / 未闭合` 共出现 **84 次**。
- 文档在更早一轮写过"下一步：不再重复静态 disassembly"，随后 30+ 次提交继续做
  disassembly；#129–#131 三次 CI 失败均源于审计脚本自身的测试夹具（合成 FDT NUL、
  helper 改名），而非固件问题——**工具已经变成产品**。

## 6. 范围蔓延与文档体量

**6.1 120 Hz。** `README.md` 项目目标中不存在刷新率条目（只有校色、LCD 滤镜、遮罩、金手指）。
120 Hz 由"显示刷新策略"扩展为 pvaibhav fork 对比、boot-package 对比、armGBA committed-frame
计数、panel qualification gate 等整套文档与实机实测，最终结论为负（面板无料号/无资格）。
负结论有价值，但该支线应冻结。

**6.2 体量。**

| 类别 | 行数 / 体积 |
|---|---|
| 主机侧 `scripts/` | 18,959 行（其中 `audit-rg34xx-*` 6,139 行） |
| 设备侧 `platform/` + `target/` C/H/S | 15,825 行（含测试） |
| `integration/` Rust | 7,598 行 |
| `docs/` | 1.4 MB |
| `docs/PROJECT-STATUS.md` | 293,681 B / 1718 行，最长单行 1368 字符 |
| 近三日修改次数（`PROJECT-STATUS.md` / `roadmap.md`） | 156 / 100 |

`PROJECT-STATUS.md` 已从"续接入口"退化为审计日志流：新会话须通读数万字才能判断哪些门
是开的，"续接"的实际成本成为交付风险。四天 1,219 次提交本身也使人工复核不再可能。

## 7. 最关键发现：僵局的钥匙已经造好，但没被使用

`docs/roadmap.md`「2026-11-26 Small LK BL33 payload into matched native SPL package」一节中：

- [x] 已确认活动 native loader extent：p4+40 MiB 包内 offset `0x3e200`，长度 `0x8000`
      （32 KiB），目标 `0x4a000000`；
- [x] 已有 pinned-source 补丁器，可把受 32 KiB 约束的 raw payload 插入 BaseOS native
      builder，重新生成 header/FNV 与匹配的 native SPL；
- [x] 补丁器已在完整 feasibility run 中做 apply / check / AST 解析 / 字节级还原，
      并保持 `git diff --exit-code` 干净；
- [ ] **"Produce a compact diagnostic payload with a deterministic proof-of-life signal,
      then build the matched native SPL/firmware/capsule locally."** ← 至今未勾选。

也就是说：**把一个小 payload 放进真实启动链的技术路径已经打通并验证**，缺的是一个带确定
存活信号（UART0 打印 / PI12 闪烁 / sdc0 单扇区读）的诊断 payload 本体，以及执行它的授权。
该 payload 一旦跑起来，即可回读 BL33 入口寄存器与 `C0_RST_CTRL`/RVBAR，直接对 G3 和 G2
提供**第一份 pre-BL33 现场证据**——这是当前 39 轮静态审计都拿不到的东西。

阻塞它的不是技术，而是决策：它与"未闭合内存与恢复门槛前不执行未验证 payload"的既有约束
正面冲突，需要项目所有者显式批准并定义恢复预案（SD 卡由电脑重写，ADR-011 已规定）。

## 8. 建议（按优先级）

1. **把 7 节那个未勾选项提升为唯一主线交付物**，作为本轮唯一验收目标；产出物保持
   non-installable 标记直到 BL31 门槛复核完成。先写一页 M1 规格（payload 尺寸 ≤32 KiB、
   存活信号、入口快照字段、失败时的可见行为、SD 恢复步骤），再实现。
2. **建立门槛台账**（第 3 节表格正式版）置于 `PROJECT-STATUS.md` 顶部 ≤200 行；当前
   11 道门散落在数万字叙述中，无法一眼判断剩余数量与阻塞点。
3. **为静态审计设停止规则**：同一链条连续 3 轮未改变任一门槛状态 → 标记"当前环境不可
   观测"并搁置，不得新开 scanner。
4. **冻结两条支线**：120 Hz 与 BL31 待机/TWI 移入 roadmap 的 deferred 区，写明重启条件
   （例如"BL33 首次启动成功后再启"）。
5. **统一基线事实源**：明确 Linkslot 1.0.49 为唯一基线，分支名仅作历史；清除文档中
   "1.0.45 仍锁"与"1.0.49 已通过"并存的表述。
6. **增加产能口径**：每轮至少一项变更落在朝下路径（`platform/` 的 MMIO/IRQ/启动链），
   否则该轮标记 audit-only，不计入阶段验收。

## 9. 边界说明

本审查只读取仓库内文档、提交历史与 workflow 配置；未连接设备、未执行构建、未修改受审仓。
所有量化指标可由同目录 `evidence.txt` 中的命令复现。本报告不构成任何硬件放行结论：
`H700_MEMORY_LAYOUT_APPROVED` 仍为 unset，`hardware_applied = 0`。
