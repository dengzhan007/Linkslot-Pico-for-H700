# 方向与任务目标偏离复查 — 2026-11-28

对 `dengzhan007/Linkslot-Pico-for-H700` 在 `porting/linkslot-1.0.45-feasibility`
分支 HEAD `7877b838` 的**只读文档与提交历史复查**，间隔上一次审查（2026-11-27 18:21，
HEAD `e87de1e6`）约 23 小时。不连接设备、不构建、不修改受审仓。

复查对象：`README.md`、`docs/PROJECT-STATUS.md`、`docs/roadmap.md`、
`docs/m1-bl33-diagnostic.md`、`docs/no-disassembly-tf-bringup-plan.md`、
`docs/m1-oneshot-capsule-audit.md`、`docs/m1-candidate-artifact-accounting.md`，
以及 `e87de1e6..7877b838` 的提交历史与新增源码。

## 结论

**方向纠偏已经落地，任务目标没有偏离；执行层出现新的过度建设。**

上一次提出的五条建议全部被采纳并写进文档（M1 主线化、G1–G11 门槛台账、审计停止规则、
冻结 120 Hz 与 BL31 suspend/TWI、audit-only 轮次不计入硬件进展）。M1 的边界写得很扎实：
32 KiB 是文件上限而非内存授权、CPU0-only、不碰 Linux arena/BOL4/BL31、PIO 无法 fail-stop
就不上机。

但同期 421 次提交里，M1 从"32 KiB payload 塞进现有启动链"扩写成"自建最小 U-Boot SPL +
一次性 capsule"，并且卡在一个尚未决策的安全边界（PMIC/TWI）上：21 小时净增设备侧代码
542 行，**诊断 payload 仍为 0**，M1-S1 仍 OPEN。

## 1. 上次建议的落地核对

| 上次建议 | 现状 | 文档位置 |
|---|---|---|
| M1 为唯一主线 | 已落地。M1-S0…S4 分阶段，逐阶段限定允许动作 | `PROJECT-STATUS.md`「唯一近期主线：M1」 |
| 建门槛台账 | 已落地，且沿用原 G1–G11 编号，含 PARTIAL / DEFERRED / NOT STARTED | 「当前验收门槛台账」 |
| 静态审计停止规则 | 已落地：同一子问题连续 3 轮无新原始证据、不改变 G 门槛 → `DEFERRED / NOT OBSERVABLE`，不得新建 scanner | 「冻结项与审计停止规则」 |
| 冻结 120 Hz 与 BL31 suspend/TWI | 已落地：120 Hz 仅保留 request-only、GAME opt-in 默认关；BL31 TWI 停在 #132，待 BL33 可观测后按 ADR-003 恢复 | 同上 |
| audit-only 轮次不计硬件进展 | 已落地：纯 host/tooling/文档轮次标记 `audit-only` | 同上 |
| （超出建议）无拆机路线 | 新增：PI12 工作灯作 proof-of-life、专用 TF1 卡、不焊 UART；UART0 降为备用 | `no-disassembly-tf-bringup-plan.md` |

这一层的方向判断与边界纪律是准确的，值得保留。

## 2. 23 小时的量化产出

| 指标 | 数值 | 说明 |
|---|---|---|
| 提交数 | 421 | `e87de1e6..7877b838` |
| 变更文件 | 66 | +7 872 / −155 行 |
| 纯文档提交 | **228（54%）** | 仅触及 `docs/` |
| `platform`+`target`+`integration` 净增 | **+542 / −0 行** | 其中 `platform/h700/boot/` 8 个文件全为 preflight 适配器 |
| M1 诊断 payload | **0** | 无 A32 入口/向量/LED 代码 |
| `PROJECT-STATUS.md` | 293 KB → **436 KB**，2 448 行 | 单日 +48%；H1 位于第 34 行，其上已有 33 行内容 |
| 重复提交标题 | 19 个，最多 4 次 | 如 `feat(m1): add read-only firmware package preflight model` ×4 |
| CI 运行编号 | #891 → #1070，约 179 次 | 含 SPL 链接失败迭代 #1057–#1070 十余轮 |

按日拆分：11-27 共 267 次提交、其中 `platform`+`target` 仅 1 次；11-28 共 323 次，
其中 29 次。也就是说，实际设备侧代码集中在最后一天。

## 3. 三个真实风险

### 3.1 M1 被重定义为"自建 SPL 工程"

为绕开 stock 路径的写盘/看门狗与 v2 capsule 的 RTC GP14/GP15，当前路线需要新建
U-Boot v2026.10 的最小 SPL profile：裁剪 Kconfig、清理 BSS、在 SPL 内解析 GPT 并校验
p4+40 MiB 包。十余轮 CI 迭代集中在链接与 Kconfig 字符串上（`fix(m1): correct SPL
Makefile object-rule regex` 一类），而不是在诊断 payload 上。

这是第二个 bootloader 的 bring-up，成本与原计划（复用现有 native SPL、只替换 32 KiB
A32 loader）不在一个量级。原计划在上一轮审查时已 host 验证通过。

### 3.2 PMIC/TWI 边界自相矛盾，且卡在只能由项目所有者做的决定上

- `docs/m1-bl33-diagnostic.md` §4：**不访问 PMIC，不发起 I2C/TWI transaction**。
- `docs/m1-oneshot-capsule-audit.md` §2.2：部署中的 native SPL board-init 就是
  AXP717 + SPL I2C + DCDC2/DCDC3 电压写 + DRAM training + CPU PLL，**任何复用现有 DRAM
  初始化路径的方案都带 PMIC/TWI**。
- 文档因此给出两个 profile（stock-init parity / 严格 no-PMIC），并明确写着
  "profile 未选、未授权"、"直到选定并复核前，不得声称 `pmic_access=false`"。

这个未决项直接挡住 M1-S3（配对 package 构建）。

### 3.3 一次性 capsule 的收益可能小于代价

stock 路径的持久写入只落在**专用测试卡**上，ADR-011 的恢复模型（换回原卡）已覆盖；
唯一设备侧状态是 RTC GP14/GP15，而这正是原机每次正常启动都在使用的机制。为回避它去
自建 SPL，换来的"无状态"收益有限，新增的未验证面却很大。

## 4. 次要问题

- **状态文档结构已断裂**：`# 项目当前状态与续接记录` 在第 34 行，其上被前置了 3 个新章节；
  436 KB 的单文件已不适合作为"续接入口"。
- **提交卫生**：同一改动最多 4 次提交；`docs:` 前缀同时存在带冒号与不带冒号两种写法
  （152 次 / 73 次），使按类型筛选失效。
- **CI 成本**：约 179 次运行 / 23 小时，完整 feasibility 单次 75+ 步、artifact 数百 MB。

## 5. 建议

1. **立刻定 profile，建议选 A（stock-init parity）**：保留现网 board-init 与 DCDC2/DCDC3
   原值，构建清单显式声明 `pmic_access=true` / `twi_access=true`，不新增 rail、clock 或
   PMIC scratch 写入。选 B 需要"DRAM 已可用"的独立证据，而该证据当前拿不到，会变成第二个
   死锁。
2. **重新评估 one-shot 是否必需**：若接受 stock RTC/WDT（在专用测试卡前提下），可直接复用
   现有 native SPL，只替换 32 KiB A32 loader，M1-S3 当天可出。
3. **给 M1-S2 设硬截止**：先产出 ≤32 KiB payload（PI12 闪 + 复用 `h700_bl33_entry_snapshot`）
   并完成 ELF/map/BSS/stack 审计，再回头做 SPL 洁净度，避免继续在 Kconfig 里循环。
4. **修文档结构**：把 H1 之前的 33 行移回标题之下；把"台账/冻结项"与"按日审计日志"拆成
   两个文件。
5. **提交卫生**：同一改动一次提交；`docs:` 前缀统一；audit-only 轮次合并。

## 6. 需要项目所有者决策的两件事

1. M1 SPL profile 选 **A（复用现网 board-init，声明 PMIC/TWI）** 还是 **B（严格 no-PMIC，
   需另证 DRAM 可用）**。
2. 是否接受 **stock RTC GP14/15 + 看门狗协议**。若接受，自建 SPL 的必要性大幅下降，M1
   可回到"只做 payload"。

## 7. 边界说明

本复查只读取仓库内文档、提交历史与新增源码；未连接设备、未执行构建、未修改受审仓。
所有量化指标可由同目录 `evidence.txt` 中的命令复现。`H700_MEMORY_LAYOUT_APPROVED` 仍为
unset，`hardware_applied = 0`；本报告不构成任何硬件放行结论。
