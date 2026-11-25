# RG34XX 120 Hz 只读刷新审计（pvaibhav/slot）

- 设备：serial 已脱敏，`BASEOS_TARGET=rg34xx`，`BASEOS_VERSION=1.5.3`，内核 `4.9.170 #19`
- 方式：仅 ADB，`uid=0(root)` 现成 root，**未执行 `adb root`**
- 采集窗口：UI `2026-11-25 23:13:38 UTC` / game `2026-11-25 23:15:10 UTC`
- 结论先行：**未匹配 120 Hz。两种状态均为 ~59.6 Hz，PLL 因子字未变，面板 timing 只有一套。**

## 1. 两状态原始对照

| 项目 | UI | game（armgba 运行中） |
| --- | --- | --- |
| `lcd_driver_name` | `rg34xx_v1` | `rg34xx_v1` |
| 分辨率 | 720×480 | 720×480 |
| `lcd_ht` / `lcd_vt` | 820 / 536 | 820 / 536 |
| `lcd_dclk_freq` | 26 MHz | 26 MHz |
| `lcd_if` | 0 | 0 |
| `de_rate` | 696000000 | 696000000 |
| `ref_fps` | 59 | 59 |
| `lcd output` fps | 60.2 | 60.2 |
| IRQ 293 delta / 12 s | 716 | 715 |
| **实测刷新率** | **59.667 Hz** | **59.583 Hz** |
| `0x03001040` | `0x88001703` | `0x88001703` |
| `0x06511044` | read error: I/O error | read error: I/O error |
| `high_refresh` | 1 | 1 |
| Slot 进程 | PID 2301，仅 `slot` 映像 | PID 2301，**新增 `armgba_libretro.so`** |

`diff` 显示两份报告的全部实质差异只有三处：时间戳、`armgba_libretro.so` 的 4 条 maps、以及各计数值本身。**显示栈一个字节都没变。**

## 2. 是否匹配 pvaibhav fork 的面板 gate：不匹配

固定提交 `5206e3fa39aa4a9b702cd9048a07dab3b5ab95da` 的实现不要求 FDT 中预先存在第二套 timing；它在启动/唤醒路径检查 live panel tuple，通过后才使用 TCON divider 与 PLL 两个寄存器实施模式设置。因此，"FDT 只有一套 timing"本身不能证明设备无法运行时改频。

本机的实际 gate 结果是不匹配：

- `BASEOS_TARGET=rg34xx` 分支要求 `lcd_driver_name=fog_fj035fhd05_v1`；
- 数值条件为 720×480、HT=820、VT=536、DCLK property=26 MHz，本机这些数值匹配；
- 本机实际 `lcd_driver_name=rg34xx_v1`，不满足精确字符串条件。

按该固定源码，`PanelRefresh::probe()` 会在面板名检查处返回 unsupported，之后 `supports_high_refresh()` 为 false，App 不会调用启动时的 `set_high_refresh()`。这说明本机不满足该 fork 当前 allowlist；不能据此推断只改面板名字就安全，也不能将其寄存器序列移植到本机。

## 3. 是否应用了 120 Hz 寄存器值：没有

`0x03001040` 两次都是 `0x88001703`，与该 fork 的 60-Hz 基线寄存器值一致。

固定 fork 源码的 `registers(true)` 明确给出的高刷组合是：

- TCON divider：`0x06511044 = 0xf000000a`
- PLL：`0x03001040 = 0x88002b03`

其中高刷路径按先 divider、后 PLL 的顺序写入；60-Hz 路径按先 PLL `0x88001703`、后 divider `0xf000000b` 的顺序恢复。**`0x88001701` 不是该 fork 的高刷候选值**，不应作为本次设备应有的 readback 值。

本次两状态均读回 PLL `0x88001703`，因此未观察到 fork 的 120-Hz PLL 候选被应用。TCON 地址本次读取失败，不能把 divider 说成已直接读回；其是否变化只能结合其它来源或另一次安全可读回证据判断。

`0x06511044`（TCON dclk divider）两次都返回 `I/O error`，dump 接口对该地址不可读，因此**没有直接读到 divider**。旁证是显示栈自报 `fps:60.2` 与 IRQ 59.58–59.67 Hz 都和"288/11 = 26.18 MHz ÷ 439520"的 59.57 Hz 吻合，说明分频路径也没被改写。

## 4. IRQ 是否接近 120 Hz：没有，接近 59.6 Hz

| 状态 | before | after | delta/12 s | Hz |
| --- | --- | --- | --- | --- |
| UI | 25713 | 26429 | 716 | 59.667 |
| game | 31143 | 31858 | 715 | 59.583 |

理论值：`288 MHz / 11 = 26.1818 MHz`，`26.1818e6 / (820 × 536) = 59.569 Hz`。两次实测偏差分别 +0.16 % / +0.02 %，属计数窗口量化误差。**距 120 Hz 差 2.01 倍，任何一次采样都没有向 120 靠拢的迹象。**

顺带：IRQ 292（`dispaly`，另一路）同样只随 293 同步增长，未出现第二套垂直时序。

## 5. 为什么进游戏也不会变

- `slot` 进程 PID 在两种状态下**都是 2301**，没有重启；进入游戏只是同一进程 `dlopen()` 了 `/mnt/sdcard/SYSTEM/armgba_libretro.so`。
- 进程不重启 → 一次性 init 不会重跑，显示路径没有机会在下游重新配置。
- `slot.state` 中 `high_refresh=1` 在两种状态下恒为 1，说明它是一个**常驻配置项**，并不是"进入游戏才置位"的开关；而显示栈侧没有任何对应变化。
- 因此，这两轮实测与固定源码 gate 一致：当前 `rg34xx_v1` 不通过 `rg34xx → fog_fj035fhd05_v1` 的 allowlist，启动时高刷 setter 不会被调用；fork 的实现也没有 UI/GAME 场景切换逻辑，它是在支持的设备上于启动/唤醒阶段全局应用偏好。故本次 UI/GAME 都保持约 60 Hz。

## 6. 判断汇总

| 判据 | 结果 |
| --- | --- |
| 面板 gate 匹配 | 否 — 精确驱动名不匹配（数值 timing tuple 匹配） |
| 高刷 PLL 候选已应用 | 否 — PLL 仍为 60-Hz 基线 `0x88001703`，候选为 `0x88002b03` |
| IRQ ≈ 120 Hz | 否 — 59.667 / 59.583 Hz |
| `high_refresh` 已应用 | 否 — 两态恒为 1，但 gate 不通过、setter 被跳过 |
| 游戏内是否更接近 120 | 否 — 与 UI 差异在量化误差内（59.58 vs 59.67） |

**最终判定：在当前设备/固件上，pvaibhav fork 的高刷选项没有被实际应用，UI 与游戏都是约 59.6 Hz 的 60-Hz 路径。此结论由 live gate mismatch、PLL readback 与 IRQ proxy 支持；TCON divider 在本次 ADB 审计中未能直接读回。**

## 7. 合规与未取到项

- 全程只读：未访问 `/sys/class/sunxi_dump/write`，未写寄存器，未停进程，未注入按键，未改配置，未重启，未执行 `adb root`。
- `0x06511044` 连续两次 `I/O error` → TCON dclk divider **无运行时读数**，只能由 PLL 字 + fps + IRQ 三方旁证推断未变。
- 未做 `sunxi_dump` 之外的任何寄存器探测，地址严格限于任务指定的两个。

## 8. 证据

| 文件 | 字节 | SHA-256 |
| --- | --- | --- |
| `slot-refresh-audit.sh` | 2845 | `aa12dffeff12c16d96eaf65216c2efd8f83fe5fea31d412d799c983e0206ce17` |
| `slot-refresh-ui.txt` | 2732 | `b552abbe6eed97114e6a7ec04ba88030ae4154cad98cc894269eed9a79ecfb58` |
| `slot-refresh-game.txt` | 3182 | `6b85dc7bd42c8c238e91485886388ca7342129dd3e13cfad831f7e80d4b8a9ab` |

脚本与设备端 `/tmp/slot-refresh-audit.sh` 的 SHA-256 一致（推送后已核对）。

## 9. 复现

```sh
adb push slot-refresh-audit.sh /tmp/slot-refresh-audit.sh
adb shell sh /tmp/slot-refresh-audit.sh ui      # UI 状态
adb pull /tmp/slot-refresh-ui.txt .
adb shell sh /tmp/slot-refresh-audit.sh game    # 手动进入 GBA 游戏后
adb pull /tmp/slot-refresh-game.txt .
```

Windows/MSYS 环境需设 `MSYS2_ARG_CONV_EXCL='*'`，否则 `/tmp/...` 会被改写成 `C:/Users/.../Temp/...`。


## 10. 解释更正（2026-11-26）

本节修正上文第 2、3 节中两处不准确表述；**不改变本次实机“UI 与 GAME 都处于约 60 Hz、没有应用 120 Hz”的结论**。

1. “120 Hz 需要设备树里存在第二套 timing”表述过强。当前 FDT 只有一套基准 timing，并不能单独证明不能运行时改频。pvaibhav/slot 的 pinned writer 本身就是在启动/唤醒流程中直接重写 DIVIDER 与 PLL，不要求 FDT 再提供第二个 timing 节点。FDT 单套基准参数和面板身份仍是 probe 的输入，但不能把“没有第二个 FDT timing 节点”作为否决动态模式的独立依据。
2. 上文称“120 Hz 需要 576 MHz 编码”不正确。核对 pinned 源码 `pvaibhav/slot@5206e3fa39aa4a9b702cd9048a07dab3b5ab95da` 的 `crates/slot-power/src/refresh.rs`，其 `registers(true)` 实际写入顺序和值为：
   - TCON divider：`0x06511044 = 0xf000000a`（divider 10）
   - PLL：`0x03001040 = 0x88002b03`（按已记录的 sun50iw9 参考公式为 528 MHz）
   - 528 MHz / 10 = 52.8 MHz；在 820×536 totals 下约为 **120.131 Hz**。
   
   `0x88001701` 才是该参考公式下的 576 MHz 编码；它不是 pvaibhav fork 的实际 120-Hz candidate。576 MHz / 11 约为 119.138 Hz，576 MHz / 10 则约为 131.052 Hz。
3. 本轮 UI/GAME 观测到的 PLL 始终是 `0x88001703`（288 MHz reference decode），IRQ 分别 59.667 / 59.583 Hz，显示驱动均报约 60.2 fps；因此本轮仍有充分证据判定当前没有应用 pvaibhav 的 120-Hz pair。当前这一轮对 TCON 地址 `0x06511044` 的 dump 读取返回 I/O error，故**本轮没有直接读回 divider**；旧的同设备独立只读记录曾读到 divider 11，但不可把它冒充为本轮的读回结果。
4. 精确面板身份仍是 `rg34xx_v1`，与 pinned fork 对 target `rg34xx` 要求的 `fog_fj035fhd05_v1` 不同。不能通过改 allowlist 或仅凭 120-Hz 寄存器算术来开放 writer。

证据来源：pinned [refresh.rs](https://github.com/pvaibhav/slot/blob/5206e3fa39aa4a9b702cd9048a07dab3b5ab95da/crates/slot-power/src/refresh.rs)；本次设备数据保留在本目录两份原始报告中。