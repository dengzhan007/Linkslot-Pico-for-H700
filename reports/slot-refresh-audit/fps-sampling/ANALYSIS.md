# RG34XX FPS 采样报告复核（2026-11-26）

## 1. 结论

本次实际采样的三个独立速率都落在约 60-Hz 区间，没有任何一项支持 120-Hz 运行：

| 指标 | 结果 | 证据口径 |
| --- | ---: | --- |
| Slot UI 主 surface presents | 4 条摘要窗口均值 59.345 presents/s；完整日志的 25 个游戏启动前窗口均值 59.503 presents/s | 每条 600 次 surface swap |
| Slot 游戏运行期间 presents | **59.561 presents/s** | `slot-game-launch.log` 中进入 armGBA 之后 31 个 600-swap 窗口 |
| armGBA committed frames | **59.722 frames/s** | frameCounter 1184 帧 / 19.8252277 s |
| LCD IRQ 293 | **59.590 IRQ/s** | 1203 次 / 20.187913 s |

游戏态三项测量的接近程度很高：Slot presents 约 59.56、armGBA 核心完成帧约 59.72、LCD IRQ proxy 约 59.59。它们仍代表不同层次，不能互相替代。LCD IRQ 不是光学实测刷新率，但结合此前读取到的 60-Hz PLL/TCON 基线，本轮进一步支持设备仍在 60-Hz 路径，而非 120-Hz。

## 2. CSV 数值独立复算

从 `armgba-frame-samples.csv` 重新解析全部 79 行：

- 样本 index 为 0–78；有效间隔数为 78。
- 首尾 monotonic 时间差为 19.8252277 s。
- 首末计数器从 11263 增至 12447，累计差 1184；逐行 `delta_frames` 总和也等于 1184。
- 每个 `delta_frames` 均与相邻 uint32 计数器差一致；无 wraparound。
- 总帧率 = 1184 / 19.8252277 = **59.7218866 fps**。
- 实际采样频率 = 78 / 19.8252277 = **3.934381 Hz**，低于目标 5 Hz；间隔均值 0.254170 s，范围 0.249146–0.266060 s。
- 瞬时 fps 56.378–65.836 的抖动符合单次采样差分只有 15/16 帧、±1 帧量化误差的特征；应以总帧差 / 总时间作为主要结果，不能将瞬时最小/最大解释成真实速度突变。
- 每轮内存读取 8-byte 主指针 + 4-byte frameCounter，共 12 B；79 轮合计 **948 B**，低于 4096 B 预算。

主指针全程恒定 `0x7f1c0ea4d0`，PID 22514、load bias `0x7f55ae0000`、map signature `8f7db48f8bb8ea5f` 均稳定。**原采样 PID 22514 的原始 maps 首尾快照仍未找回**，因此该 PID 的 bias 仍是原报告摘要级证据；不能把后续 PID 8064 的新 maps 当成 PID 22514 的回溯证明。2026-11-26 的独立 maps-only 补采使用新 PID 8064，两份 425 行快照中的 armGBA 四条映射逐字节一致，RX/R/RW 三行独立计算均得到新 bias `0x7f339a0000`，验证了同一 ELF 的 PT_LOAD 对齐与 bias 解算方法。详见 [MAPS-SUPPLEMENT.md](MAPS-SUPPLEMENT.md)。该补采没有读取 PID 8064 的进程内存，也没有重采 FPS。

## 3. Slot presents 日志复核

`slot-ui-trace.log` 中的 4 条最初 UI pace 行为 58.67、59.57、59.57、59.57，平均 59.345；全部 60.00 nominal fps、stale=0、每窗口 600 presents。

完整 `slot-game-launch.log` 还保留更多窗口：
- 进入游戏前 25 个窗口平均约 **59.503 presents/s**；
- 从 `launch: opening CT特种部队1 on armgba` 起，游戏态 31 个窗口平均约 **59.561 presents/s**；首个游戏态窗口 59.27 且 stale=4，其余窗口均约 59.57–59.58。

这说明 UI trace 的四条均值代表最初采样片段，不是本次所有 UI 窗口的总平均；游戏态 presents 结果是由完整 launch log 追加提取的分析值。

## 4. 设备身份与 ADB 标识

预检中的设备现场输出为 Linux `rg34xx`、Linux 4.9.170 #19、DT SoC `sun50iw9`、`BASEOS_TARGET=rg34xx`、`BASEOS_PLATFORM=h700`，SD 路径、Slot 日志和 armGBA SHA 也与该 RG34XX 运行环境相符。

同一预检的 `adb devices -l` 却显示 `product:occam model:Nexus_4 device:mako`。这个 ADB descriptor 是一组产品属性，不应单独当成物理设备型号；公开的嵌入式 Linux 设备案例也出现过相同 `occam/Nexus_4/mako` 标识。因此它本身不足以推翻 Linux/DT/发行版身份，但后续采集应继续使用同一个明确的 ADB serial，并同时记录 `/proc/device-tree/model`、`compatible` 和 `/proc/cpuinfo` 作设备侧确认。参考：[Linux 设备出现相同 ADB descriptor 的社区记录](https://community.home-assistant.io/t/tuya-smatek-t6e-and-s6e-4-inch-control-panels-for-ccs-central-control-system-android-based-in-wall-4-inch-smart-display-with-touch-screen-now-sold-under-many-different-brands/473366/61)。

## 5. 恢复与安全检查

- 预检确认 Slot 是 init respawn 的前端子进程；首次 SIGTERM 后进入 diagnostics helper 并在 60 秒内未 respawn Slot，是一次真实恢复异常，不应从报告中省略。
- 后续清理时删除本轮创建的 `System/trace-latency.on`，再结束 Slot 和阻塞的 diagnostics helper；init respawn 得到 PID 8064。
- 清理后 marker ABSENT、`SLOT_TRACE_LATENCY` 未设置、新 Slot 的 `presents/s` 行数为 0、`armgba` 映射为 0，报告记录 UI 正常。
- 未重启整机；未修改 ROM、存档、用户配置和刷新率参数；未读写 PLL/TCON/CCU/PMIC；`hardware_applied=0`。
- `SHA256SUMS.txt` 中列出的 **13 个文件**已独立复算 SHA-256，全部与清单逐字节匹配；包含新增两份 maps 快照和 `MAPS-SUPPLEMENT.md`。

## 6. 与项目决策的关系

1. 不再把 UI FPS、core FPS、LCD IRQ 合并成一个 FPS。三项结果现在都有真实运行时测量。
2. 这台 RG34XX 本次仍表现为 UI/GAME 约 60 Hz / 60 fps；120 Hz 不仅未启用，也没有任何数据支持已进入 120-Hz 模式。
3. 后续 120-Hz 工作应继续先闭合 panel allowlist 与 kernel/TCON/PLL 兼容性；不得因为 FPS 采样结果接近 60 就放宽面板 gate 或试写寄存器。
4. 原始 PID 22514 的 maps 快照无法回溯；新 PID 8064 的 maps-only 补采已独立验证 ELF 映射行与 bias 算法，但不替代旧 PID 的原始证据。无需重跑内存 sampler 或提高采样频率。


## 7. maps-only 独立补采（2026-11-26）

原始 PID 22514 的两份完整 `/proc/PID/maps` 快照未在本地采集缓存中找回，故继续将旧 PID 的 `0x7f55ae0000` bias 保留为摘要级证据，不将新采集混作旧采集。

本次独立补采使用同一 RG34XX 的新 Slot PID **8064**，同一 ROM `CT特种部队1.gba` 与同一官方 armGBA core。首尾各保存一份完整 maps（均 425 行）；两份快照的 armGBA 四条映射逐字节相同，SHA-256 分别为：

- `armgba-maps-supplement-first.txt`: `5b2ff0a9cf21350ef314f7c1d07fddffdcdaedbf42b98a27b6a6095a97ff2e75`
- `armgba-maps-supplement-last.txt`: `401362c79fe7dcbf3e37cc2502280741155105a5378e8f74317f09e6e593682d`

三行 PT_LOAD 映射交叉计算：RX `0x7f339a0000 - 0x0`、R `0x7f33b25000 - 0x185000`、RW `0x7f33b28000 - 0x188000`，均为 **`0x7f339a0000`**。这是新进程 ASLR 下的独立验证，不应与旧 PID bias 相等，也不能反推旧 bias 的原始 maps。

采集后长按 Menu 退出游戏；PID 8064 仍存活并回到 UI，armGBA 映射归零，marker ABSENT，`SLOT_TRACE_LATENCY` 未设置。未重采 FPS、未读取 `/proc/PID/mem`、未重启或杀死 Slot、未改用户数据；无任何 PLL/TCON/CCU/PMIC 写入，`hardware_applied=0`。设备身份与完整安全边界见 [MAPS-SUPPLEMENT.md](MAPS-SUPPLEMENT.md)。