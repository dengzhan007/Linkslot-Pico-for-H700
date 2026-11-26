# Slot UI presents + armGBA committed-frame 采样报告（RG34XX）

日期 2026-11-26（UTC）。全部设备操作经 ADB；除下方明示的 marker、SIGTERM 与 evdev 按键注入外，
未修改设备配置、ROM、存档、刷新率参数或任何硬件状态。`hardware_applied = 0`：未写 PLL/TCON/CCU/PMIC，
未操作 `/dev/mem`，未刷启动分区，未改面板 allowlist，未启用 120 Hz writer。

## 1. 三个指标必须分开

| 指标 | 实测 | 采样窗口 / 口径 |
| --- | --- | --- |
| `slot_presents_per_s` | 58.67 / 59.57 / 59.57 / 59.57，均值 **59.345** | UI 主 surface 成功 swap；4 条 `slot: pace:` 记录，nominal 60.00 fps，stale 全 0，missed game periods 10 |
| `armgba_committed_frames_per_s` | **59.722** | `1184` 帧 / `19.825228` s，来自 `gba->video.frameCounter` 差分 |
| `lcd_irq_per_s` | **59.590** | IRQ 293 `1203` 次 / `20.187913 s`，只读 `/proc/interrupts` |

三者不是同一个量：presents 是前端合成提交，committed frames 是模拟器核心完成的帧，IRQ 是 LCD 中断。

## 2. PID / bias / 地址

| 阶段 | PID | 说明 |
| --- | --- | --- |
| 预检 | 2301 | PPID 1331（`/sbin/frontend-session`），其 PPID=1；SIGTERM 后由 init respawn |
| UI presents | 22514 | marker 已存在，`fd/2 -> /mnt/sdcard/Logs/slot.log`，`SLOT_TRACE_LATENCY` 仍 absent |
| 游戏 + 帧采样 | 22514 | 同一进程 `dlopen` armgba，未重启；ROM `/mnt/sdcard/Games/GBA/CT特种部队1.gba`，日志确认 `opening ... on armgba` 与 `core /mnt/sdcard/System/armgba_libretro.so` |
| 清理后 UI | 8064 | marker 删除后再次 respawn；`presents/s` 行数回到 0，tracing 已关闭 |

帧采样期间每轮重扫 PID 与 maps：PID 恒定 22514，bias 恒定 **`0x7f55ae0000`**，
映射签名恒定 `8f7db48f8bb8ea5f`。bias 由 `r-xp`(off 0)、`r--p`(off 0x175000)、
`rw-p`(off 0x178000) 三行独立解出且完全一致；`---p` 空洞行不参与。
主指针变量 = bias + 0x4259b8，指针值恒为 `0x7f1c0ea4d0`，8-byte 对齐；
计数器 = 指针 + 0x2eac（11948），其 4 字节范围每轮都验证落在 rw 映射内。

## 3. 采样误差与实际频率

- 目标 5 Hz / 20 s；实际 **79** 个样本，起止 `2026-11-26T01:16:22.654510Z` → `2026-11-26T01:16:42.478397Z`，
  跨度 `19.825228 s`，实际 **3.934 Hz**（adb 往返使周期约 0.254 s）。
- 间隔 min/mean/max = 0.249146 / 0.254170 / 0.266060 s，
  stdev 0.003333 s；瞬时 fps 范围 56.378–65.836。
- 计数器是 uint32 整数，单次差分有 ±1 帧量化误差，约 ±3.93 fps；瞬时值因此抖动。
  汇总帧率用总帧数 / 总时长，比逐点均值稳健。未发生 uint32 环绕（wraparound_events=0）。
- 时间戳取每次 adb 往返中点的 host `time.monotonic_ns()`，不是设备时钟，也不是计划采样周期。

## 4. 内存读取边界

每次严格只读 8 字节主指针 + 4 字节计数器 = 12 B，共 **948 B**（预算 4096 B）。
无扫描、无 ptrace attach、无 gdb、无写入进程内存；权限正常，未做绕过。maps 是文本元数据，不计入该预算。

## 5. marker 与 UI 恢复

- 预检 marker 不存在；本轮创建 `$SLOT_ROOT/System/trace-latency.on`，采集后 **已删除**，最终状态 ABSENT。
- 首次 SIGTERM 后 `frontend-session` 进入 `baseos-control-center --diagnostics` 并阻塞，60 s 内没有新 PID；
  这是实际发生的失败，未伪造后续数据。B 键把画面切到前端选择器，MENU 无效；
  只对该辅助进程发 SIGTERM，init 随即 respawn Slot（PID 22514）。用户后来提示在选择器上选中
  `slot-pvaibhav` 按 A 也能返回，此时已完成 respawn，未再改动。
- 游戏用长按 Menu（BTN_TL2=312）退出，`armgba` 映射由 4 行回到 0；随后删除 marker 并再次 SIGTERM +
  init respawn（PID 8064）。新进程 `fd/2` 仍是 `/mnt/sdcard/Logs/slot.log`，`presents/s` 为 0，UI 正常。
- 未重启设备，未触碰用户配置、ROM、存档。

## 6. 文件

| 文件 | 内容 |
| --- | --- |
| `README.md` | 本文件 |
| `preflight.txt` | 单设备、uid、BASEOS_TARGET、挂载、inittab、进程链、marker、core SHA、日志原始状态、ROM 清单 |
| `slot-ui-trace.log` | UI presents 采集时从 `fd/2` 指向的日志全文（4 条 pace 记录） |
| `ui-presents.json` | 逐条解析、汇总与 respawn 经过 |
| `armgba-frame-samples.csv` | 逐样本 host monotonic 时间、PID、bias、主指针、counter、差分与实际间隔 |
| `armgba-frame-summary.json` | 实际采样率、帧率、误差、IRQ、错误列表 |
| `lcd-irq-samples.txt` | IRQ 293 前后原始行与计数 |
| `slot-game-launch.log` | 游戏启动阶段日志，证明 .gba 与 armgba core |
| `recovery-log.txt` | 首次 60 s 失败与最终清理 respawn 的原始状态文件 |
| `SHA256SUMS.txt` | 上述文件 SHA-256（不含自身） |
