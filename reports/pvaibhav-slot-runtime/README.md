# pvaibhav/slot RG34XX 实机运行阶段只读采集（UI vs GAME，2026-11-25 18:44）

本目录保存同一台 RG34XX、同一系统环境下 **UI 30 秒** 与 **GAME 30 秒** 两轮只读采集的完整产物，
以及它们的逐字段对比。两轮中间由用户在设备上手动切换状态（进入游戏），采集期间不注入输入、
不切换刷新率、不写 MMIO。

- 设备：`RG34xx` / `sun50iw9p1`，Linux `4.9.170`，ADB shell `uid=0`（已有 root 权限，**未执行 `adb root`**），
  串号已在下述文本中替换为 `[REDACTED]`
- 采集脚本：本次使用的本地 PowerShell 执行副本为 `build/_slot-capture.ps1`；`tools/collect-pvaibhav-slot-runtime.ps1` 在当时本地工作树中有对应版本，但核对当前 GitHub 工作分支时尚未找到已提交文件，不能视为已版本化工具。
- 输出：`20261125-184406-ui/`（53 文件）、`20261125-184508-game/`（49 文件）

## 0. 采集命令与两处必要的运行偏差

```
# 第一轮：设备停在 slot-pvaibhav 主界面
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\collect-pvaibhav-slot-runtime.ps1 -Phase ui   -Seconds 30 -NoPrompt
# 第二轮：进入游戏并保持运行
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\collect-pvaibhav-slot-runtime.ps1 -Phase game -Seconds 30 -NoPrompt
```

本次运行有两处**环境/脚本层面**的偏差，均不改变采集语义（全部仍为只读命令）：

1. 本机调用环境里 `$PSScriptRoot` 为空，脚本默认值 `$RepoRoot = Split-Path -Parent $PSScriptRoot` 求值失败，
   因此显式传入等值的 `-RepoRoot`。脚本文件本身未改动。
2. 脚本存在 5 个缺陷，直接运行会在采样阶段抛异常或产出错误字段。为不改动仓库脚本，使用逐字复制后的
   执行副本（`build/_slot-capture.ps1`）并在副本内做最小修正。缺陷与修正如下，建议上游吸收：

| # | 位置 | 现象 | 修正 |
|---|---|---|---|
| 1 | `Invoke-AdbText` 传参 | 本机 adb（36.0.0 / 33.0.0 均如此）不对 `shell` 多参数做转义，`shell sh -c <带空格命令>` 到设备端退化成 `sh -c cat`，`cat` 读 stdin 永久阻塞 → 20 s 超时 | 把该组合折叠成**单个** argv `sh -c '<payload>'`，并转义内部单引号 |
| 2 | 同上，跨行 payload | `.ps1` 为 CRLF，多行脚本把 `\r` 带到设备端，`for …; do` 报 `syntax error (expecting "do")` | payload 先做 CRLF→LF 规范化 |
| 3 | `Get-DisplayIrqRows` | `if (A -and $Matches[3] -match B)` 中第二个 `-match` 覆盖 `$Matches`，随后 `$Matches[3].Trim()` 对 null 取值 → 采第一个样本即抛错 | 先用局部变量保存结构化匹配结果，再做标签判定 |
| 4 | 采样循环调用点 | 函数 `return ,$rows` 叠加调用处 `@(...)` 造成**嵌套数组**：每轮只生成 1 个聚合行，`total` 变成数组，`display-irq-rates` 计算抛 `Object[]→Int64` | 调用点去掉 `@()` |
| 5 | 进程识别 / fps 正则 | ① 进程扫描模式只认 `slot-pvaibhav` / `.system/slot`，本机前端实际是 `/mnt/sdcard/System/slot`（comm=`ld-linux-aarch6`），采不到 `proc-*/maps`；② `(?i)fps\s*:` 会命中 `ref_fps:59`，把 `ref_fps` 当成驱动 fps | ① 追加 `*System/slot*` 模式，并排除扫描 shell 自身 PID；② 正则加 `(?<![A-Za-z0-9_])` 负向断言 |

修正后 `slot-pids.txt` 稳定命中 1 个 PID（`2842`，`/mnt/sdcard/System/slot`），
`proc-2842/{cmdline,comm,status,stat,maps,mountinfo,cwd.txt,exe.txt,selected-environ.txt}` 全部落盘。

## 1. 只读声明

全程仅执行读取类命令：`cat /sys/**`、`cat /proc/**`、`ls -l`、`uname`、`ps`、`adb pull /sys/firmware/fdt`。
未执行：`adb root`、重启、刷新率/模式切换、`sunxi_dump` 写入、MMIO 写入、任何配置或 ROM/存档修改。
`/sys/class/sunxi_dump/{write,dump}` 存在且**可写**（`rw-r--r-- root`，见 `refresh-control-availability.txt`），
本次**未写入**；该事实仅作为"设备具备写通道、本次未使用"的记录。

## 2. 显示链路对比（核心结论）

| 项目 | UI 30 s | GAME 30 s |
|---|---|---|
| 采样数 / 时间跨度 | 27 / 29.624 s | 23 / 29.749 s |
| IRQ 292 `wakeupgen 88 Level dispaly` | Δ1767 → **59.648 Hz** | Δ1774 → **59.632 Hz** |
| IRQ 293 `wakeupgen 64 Level dispaly` | Δ1766 → **59.614 Hz** | Δ1774 → **59.632 Hz** |
| 驱动上报 `fps`（采样点分布） | `60.2` × 23，`59.8` × 4 | `60.2` × 20，`59.8` × 3 |
| `ref_fps`（驱动字段，可能陈旧） | 59（全部样本） | 59（全部样本） |
| `de_rate` | 696,000,000 Hz | 696,000,000 Hz |
| `mgr0` 模式行 | 720x480 rgb / cs[0x204] / err[0] / iommu[1] | 与 UI **完全一致** |
| `err` / `skip` | 0 / 0 | 0 / 0 |
| `backlight` | 8 | 8 |
| Live FDT SHA-256 | `6d1301e6…6ed16` | **同一 blob**（逐字节相同） |
| `high_refresh`（slot.state） | **1** | **1** |

**面板与 timing（来自 live FDT `lcd0@01c0c000`，静态属性，非运行时测量）**

| 字段 | 值 |
|---|---|
| `lcd_driver_name` | `rg34xx_v1`（面板型号标识） |
| `compatible` | `allwinner,sunxi-lcd0` |
| 有效/总分辨率 | `lcd_x`=720，`lcd_y`=480；`lcd_ht`=820，`lcd_vt`=536 |
| 前后沿/同步 | `lcd_hbp`=46，`lcd_hspw`=2，`lcd_vbp`=20，`lcd_vspw`=5 |
| `lcd_dclk_freq` | 26 MHz（DTB 声明值） |
| 接口 | `lcd_if`=0（CPU/RGB），`lcd_hv_if`=0，`lcd_lvds_if`=0 |
| 其他 | `lcd_frm`=1，`lcd_pwm_used`=1 / `lcd_pwm_freq`=50000，`lcd_backlight`=50，`smart_color`=90，`lcdgamma4iep`=22 |

**阶段性判断（只陈述测到的东西）**

- 两轮扫描中断速率**在测量分辨率内无差异**（59.61–59.65 Hz），`mgr0` 模式行、`de_rate`、FDT 全部相同
  → 本次 UI→GAME 切换**没有触发刷新率或显示模式变化**；`high_refresh=1` 在两轮都成立，但**未观测到任何模式切换**，
  该设置在此环境下的实际生效方式**未被本次采集证明**。
- 用 DTB 声明的 26 MHz 与 820×536 推算为 59.155 Hz；若实际 dclk 取自 `pll_video0x4`(288 MHz)/11 = 26.182 MHz，
  则为 59.569 Hz，与实测 59.63 Hz 更接近。这属于**推算**，仅作量级一致性说明，不作为已测得频率。
- **后续已补上这个推算的前提**：19:59 用 `sunxi_dump` 的 dump 请求节点只读回读，
  `0x06511044` = `0xf000000b`（divider 11，与 fork 标注的 60 Hz 分频字逐字节相同），
  `tcon_lcd` = 288 MHz，见 `tcon-readback/`。即 288/11 → 26.18 MHz → 59.57 Hz，
  与实测中断速率吻合；该结果仍是"分频字 + 上游时钟"推算，不是直接测得的 DCLK。
- 驱动上报的 `fps` 在 60.2 / 59.8 之间跳变，两轮**都有**这两种取值，与阶段没有对应关系，属采样瞬间的驱动计数抖动。

## 3. 非显示侧对比（同环境、同设备）

| 项目 | UI | GAME |
|---|---|---|
| Slot PID | 2842 | 2842（未重启） |
| `maps` 行数 / 加载 `.so` 数 | 863 / 12 | 895 / 14 |
| 新增映射库 | — | **`armgba_libretro.so`**、`libz.so.1` |
| `scaling_cur_freq` 取值集 | 1008 / 1104 / 1512 MHz（中位 1104） | 1104 / 1200 / 1320 / 1512 MHz（中位 1200） |
| `cpu_thermal_zone` 采样区间 | 47.37 – 49.55 °C | 48.91 – 50.77 °C |
| 其他热区（快照） | gpu 47.85 / ve 47.04 / ddr 47.45 / battery 30.0 °C | gpu 48.66 / ve 48.42 / ddr 47.93 / battery 30.0 °C |
| governor / online CPU | `schedutil` / `0-3` | 同（同一系统环境） |

`armgba_libretro.so` 说明第二轮运行的是 **libretro GBA 核心**，与"进入游戏"一致；但本次**没有**读取任何
核心帧计数，所以不能由它推断游戏帧率。

## 4. 两轮产物清单（要求检查的项均已落盘）

| 要求检查的产物 | UI | GAME | 说明 |
|---|---|---|---|
| `live-fdt.json` | ✅ 38,955 B | ✅ 38,955 B | 两轮内容一致（同一份 FDT） |
| `display-state.txt` | ✅ | ✅ | `/sys/class/disp/disp/attr/sys` 原文 |
| `display-irq-rates.txt` | ✅ Δ1767/1766 | ✅ Δ1774/1774 | 内核中断计数差，非 FPS |
| `samples.json` | ✅ 27 样本 | ✅ 23 样本 | 每样本含 fps/ref_fps/cpu/temp/IRQ 计数 |
| `proc-<pid>/maps`（含 cmdline/status/stat/mountinfo…） | ✅ `proc-2842` | ✅ `proc-2842` | 9 个文件/进程 |
| `slot-state-candidates.txt` | ✅ 3 处 `slot.state` | ✅ 3 处 | 均含 `high_refresh=1` |
| 其他 | `framebuffer.txt`、`display-clocks.txt`、`kernel-and-cpu.txt`、`thermal-zones.txt`、`refresh-control-availability.txt`、`process-snapshot.txt`、`slot-process-scan(-end).txt`、`interrupt-samples/interrupts-*.txt`、`SUMMARY.md` | 同 | |

`live-fdt.dtb`（143,936 B，两轮同一 SHA-256 `6d1301e6…`）**保留在本地**，未上传；其中的面板 timing 已抽取到本文 §2。

## 5. 缺失字段（未测得，因此不做推断）

以下数值**本次没有测到**，报告中不给出任何估计值：

- **游戏实际帧率**（libretro 核心提交帧率）：没有读取 `armgba_libretro.so` 的帧计数/`retro_run` 计数，
  即没有 per-core committed-FPS 探针。中断速率只能作为扫描/vblank 代理。
- **UI 呈现帧率**（合成/上屏队列统计）：该栈上没有 surfaceflinger/`dumpsys` 类统计可用，未测量。
- **`high_refresh` 的实际生效状态**：slot 设置文件为 1，但两轮 timing 完全相同，未观察到模式切换；
  也没有可读的"当前刷新率"节点可交叉验证。
- **精确实际 dclk**：DTB 声明 26 MHz，与实测 IRQ 速率存在约 0.5% 偏差；`clk_summary` 只有 `pll_video0x4`/`tcon_lcd`
  的 288 MHz 与 `pll_video0` 72 MHz，未见 lcd 分频后的最终 dclk 字段。未作定论。
- **CPU/GPU 占用率、生效中的 OPP/电压**：未采集（避免触碰 DVFS/PMIC 路径）。
- **每 CPU 的中断分布**：`interrupt-samples/interrupts-*.txt` 保留了原始每 CPU 计数，但本次未做拆分统计。

## 6. 结论边界

- 本轮只说明"这台机器、这套系统、这两个阶段"的**观测值**：扫描中断速率、驱动上报 fps、显示模式行、
  静态面板 timing、进程映射、CPU 频点与温度。
- 不构成任何刷新率切换、MMIO 写入、性能调节的可行性依据；不声称任何游戏帧率或掉帧结论。
- 全程未修改设备状态，未执行 root 提权、重启、写入或绕过任何权限检查。


## 7. Artifact and source-path follow-up

The `live-fdt.json` files in the two 18:44/18:45 rounds expose only panel driver and geometry; their `panels[0]` objects do not include the `interface` or `timing` keys, because the parser version used at that time never emitted them. The report's 26 MHz / HT 820 / VT 536 values were extracted from the local raw `live-fdt.dtb`, which was not uploaded.

**Resolved in `fdt-reparse/` (2026-11-25 19:09).** The parser was extended to emit `panels[].interface` and a full `panels[].timing` block and the live FDT was pulled again, so the timing record is now machine-readable and complete. Only the **redacted** `live-fdt.json` is published: the raw `live-fdt.dtb` (SHA-256 `65801950f6d2d3c7139111d35c6cc0dd64cc249f8ba64ffb5b6f1c7fd9c39173`) is kept local because its `bootargs` carry `snum=` and `androidboot.serialno=`. The published JSON has both of those keys replaced by `<serial-redacted>`. Two caveats recorded there: the 19:09 pull belongs to a **later boot** (`bootargs` now carries `bootreason=unknow` instead of `button`; uptime ≈ 350 s; only three `/chosen` properties differ, all boot-time records — every `lcd0@01c0c000` property is byte-identical), and the DTB-declared nominal refresh `59.155 Hz` is superseded by the divider readback below.

Both process environments show `BASEOS_TARGET=rg34xx`, `BASEOS_PLATFORM=h700`, `BASEOS_PACKAGE_ID=slot-pvaibhav`, and `BASEOS_FRONTEND_ID=slot-pvaibhav--default`. The Slot process runs as UID 0 with full effective capabilities. In the reviewed pvaibhav source, `DevicePlatform::new()` reads `BASEOS_TARGET`, `PanelRefresh::probe()` requires `fog_fj035fhd05_v1` for `rg34xx`, and startup mode application is gated by `supports_high_refresh()`. Our observed `rg34xx_v1` panel therefore fails the exact identity gate; the 60-Hz observation is not explained by missing root permission. The installed binary hashes are now captured (`System/slot` = `8f0fd40da4ac161e5fbb9a56a99c254acf8b2f31806e2be6bdaa929b5ee28505`, 18,718,728 B; `SYSTEM/armgba_libretro.so` = `4ebae098cc4848fb459fc18299c39acbdbbbeb732db5c3f66d1ff3f6e142e889`, 2,021,992 B, both recorded in `fdt-reparse/device-sha256.txt`), but no upstream build was hashed for comparison, so the exact source revision is still unverified.
