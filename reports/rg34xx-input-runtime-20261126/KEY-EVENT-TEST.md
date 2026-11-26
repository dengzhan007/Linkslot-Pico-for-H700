# RG34XX 物理按键运行时验证 — 2026-11-26

本目录记录 `reports/rg34xx-input-fdt-20261126/` 静态设备树结论的**运行时**对证：在新一轮只读采集的同一次上电会话内，逐个按下 17 个游戏按键（电源键不纳入），直接读取 `/dev/input/event1`（`ANBERNIC-keys`）的原始 `input_event` 流。

## 1. 方法：为什么不是 `getevent`

这台设备**没有 `getevent`**（`/system/bin`、`/system/xbin` 均不存在，采集器在 `input-runtime.txt` 里也记了 `NOT_PRESENT: getevent`）。因此改用等价手段：

```
adb -s <serial> shell "cat /dev/input/event1" > ev1-raw.bin   # 只读打开，最长 600 s
```

- 传输完整性先做对照验证：`adb shell "dd if=/bin/sh bs=32 count=1 | md5sum"` 与本地落盘文件的 md5 一致（`6fbeca97…5442`），证明这条通道是字节透明的，不会像 `exec-out` 那样 `error: closed`。
- 每个记录 24 字节：`struct input_event { struct timeval time; __u16 type; __u16 code; __s32 value; }`，本机用 Python `struct.unpack('<qqHHi')` 解码，**没有在设备上跑任何东西**。
- 采集窗口：`2026-11-26T04:57:35.938971Z` → `2026-11-26T04:58:18.328933Z`（UTC），共 72 条记录 / 1728 字节，字节数正好是 24 的整数倍。
- 时间戳是内核 `CLOCK_REALTIME`（epoch 秒），秒以下分辨率为微秒。

## 2. 观测到的按键序列

按时间顺序共 **18** 组按下事件（预期 17 组，实际 18 组 —— 见 §4 的误触记录）。

| # | t+ (s) | 按键 | type/code | 按下值 | 保持 (ms) | 引脚（DT） |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 0.000 | A (`BTN_SOUTH`, 304) | `EV_KEY`/`304` | 1 | 150 | `PA0` |
| 2 | 2.380 | B (`BTN_EAST`, 305) | `EV_KEY`/`305` | 1 | 160 | `PA1` |
| 3 | 4.890 | X（发 `307` = DT `keyY`） | `EV_KEY`/`307` | 1 | 140 | `PA2` |
| 4 | 6.640 | Y（发 `306` = DT `keyX`） | `EV_KEY`/`306` | 1 | 170 | `PA3` |
| 5 | 8.880 | L1 (`BTN_WEST`, 308) | `EV_KEY`/`308` | 1 | 150 | `PA10` |
| 6 | 10.790 | R1 (`BTN_Z`, 309) | `EV_KEY`/`309` | 1 | 140 | `PA12` |
| 7 | 13.110 | SELECT (`BTN_TL`, 310) | `EV_KEY`/`310` | 1 | 180 | `PA5` |
| 8 | 15.070 | START (`BTN_TR`, 311) | `EV_KEY`/`311` | 1 | 180 | `PA4` |
| 9 | 16.440 | R1 (`BTN_Z`, 309) | `EV_KEY`/`309` | 1 | 140 | `PA12` |
| 10 | 21.290 | MENU (`BTN_TL2`, 312) | `EV_KEY`/`312` | 1 | 240 | `PE3` |
| 11 | 29.230 | L2 (`BTN_SELECT`, 314) | `EV_KEY`/`314` | 1 | 180 | `PA11` |
| 12 | 31.110 | R2 (`BTN_START`, 315) | `EV_KEY`/`315` | 1 | 180 | `PA7` |
| 13 | 33.990 | 十字上 (`ABS_HAT0Y`, 17, 值 -1) | `EV_ABS`/`17` | -1 | 160 | `PA6` |
| 14 | 35.050 | 十字下 (`ABS_HAT0Y`, 17, 值 +1) | `EV_ABS`/`17` | 1 | 200 | `PE0` |
| 15 | 36.830 | 十字左 (`ABS_HAT0X`, 16, 值 -1) | `EV_ABS`/`16` | -1 | 140 | `PA8` |
| 16 | 37.930 | 十字右 (`ABS_HAT0X`, 16, 值 +1) | `EV_ABS`/`16` | 1 | 160 | `PA9` |
| 17 | 40.960 | VOL+ (`KEY_VOLUMEUP`, 115) | `EV_KEY`/`115` | 1 | 240 | `PE1` |
| 18 | 42.140 | VOL- (`KEY_VOLUMEDOWN`, 114) | `EV_KEY`/`114` | 1 | 250 | `PE2` |

## 3. 覆盖度对证（DT 17 键 vs 实际观测）

| DT 键 | evdev code | 观测 |
| --- | --- | --- |
| A | 304 | ✅ 有事件 |
| B | 305 | ✅ 有事件 |
| X | 306 | ✅ 有事件 |
| Y | 307 | ✅ 有事件 |
| L1 | 308 | ✅ 有事件 |
| R1 | 309 | ✅ 有事件 |
| SELECT | 310 | ✅ 有事件 |
| START | 311 | ✅ 有事件 |
| MENU | 312 | ✅ 有事件 |
| L2 | 314 | ✅ 有事件 |
| R2 | 315 | ✅ 有事件 |
| VOL- | 114 | ✅ 有事件 |
| VOL+ | 115 | ✅ 有事件 |
| 十字 HAT0X | 16 | ✅ 两向均出现（-, +） |
| 十字 HAT0Y | 17 | ✅ 两向均出现（-, +） |

17 个 DT 键**全部**产生实际事件，无一遗漏；`ANBERNIC-keys` 通道上也没有出现 DT 之外的按键码。这组数据把 `rg34xx-input-fdt-20261126` 的静态声明（code / 方向 / -1、+1 极性）与真实电气输入闭合起来了。

但要注意：**code 覆盖 ≠ 丝印对应**。`306` 与 `307` 都出事件，不代表按着"X"键出的就是 `306`；实测恰好相反，见 §4 与 [`XY-REPEAT.md`](XY-REPEAT.md)。

## 4. 与指令顺序的两处偏差（如实记录）

> **2026-11-26 13:04 UTC 更正**：第 1 条经 [`XY-REPEAT.md`](XY-REPEAT.md) 的 6/6 复测已改判为**键位互换**，不是操作顺序问题。下面的原文保留，改判理由见该文件。

1. **~~第 3、4 项顺序互换~~ → X/Y 键位互换**：指令表是 X → Y，落盘顺序却是 `307` 在前、`306` 在后。
   复测证明：操作者按"X"时机器发的是 `307`（DT `keyY`/`PA2`），按"Y"时发的是 `306`（DT `keyX`/`PA3`）。
   也就是说操作者当时按顺序是对的，是**面板丝印与 DT 节点名互换了**。两个键都正常触发，覆盖判定不受影响。
2. **多出一组 R1**：`311`(START) 之后、`312`(MENU) 之前出现了一组 `309`(R1)，间隔约 1.2 s。操作者确认这是按 MENU 时误触 R1，与本记录一致；该组事件被单独标注，不计入 17 键的覆盖判定。

除这两点外，其余 15 组的先后顺序与指令表完全一致。

## 5. 事件语义观察

- `EV_KEY` 键：`value=1` 按下、`value=0` 抬起，每次状态变化后紧跟一条 `EV_SYN/SYN_REPORT`（type 0 code 0）。
- 十字键不是 `EV_KEY`，而是 `EV_ABS`：按方向输出 `-1`/`+1`，松手回 `0`（不是 `-1` 的相反数）。因此上层必须把 `ABS_HAT0X/Y` 的 `0` 理解为“回中”，而不是“另一个方向键按下”。
- 每次按压的保持时间实测 139–249 ms，说明这是轮询式 `gpio-keys-polled` 的采样结果，短于一个轮询周期的轻触会被合并或丢失。
- 观测到 `js0` 也在 `ANBERNIC-keys` 的 handlers 里，说明同一设备同时以 joystick 形式导出；本测试只走了 evdev 字符设备，未走 `/dev/input/js0`。

## 6. 静态 DT 与运行时状态的三处不一致（需要后续注意）

1. `pinmux-pins` 里 `PE0`/`PE2`/`PE3` 被记成 `function csi1 group PE0/PE2/PE3`，而 DT 把 `keyDOWN`/`keyvol-`/`keyMenu` 声明在这三个脚上。三个键**实测都能出事件**，说明这个 `csi1` 标注并没有真正夺走 GPIO 功能（更可能是 pin 分组名残留或 CSI1 只声明未启用），但它确实是运行时状态与 DT 表面冲突的地方，不能拿 pinmux-pins 单独作为“谁占了脚”的证据。
2. `pinmux-pins` 对 PA0–PA12 报 `(MUX UNCLAIMED)`，同时 `pinconf-pins` 报 `input bias disabled`。而 DT 里这些脚是 `pull_config = 1`，按 Allwinner 内核头 `SUNXI_PINCTRL_PULL_UP = 1` 应读作**弱上拉**，不是无上下拉。三处说法两两不一致：DT 说弱上拉、pinconf 说 bias disabled、pinmux 说没被申请。功能上按键正常出事件，所以更可能是该内核对 gpio-keys-polled 的 gpiod 请求不回填调试视图、且 pinconf 未反映实际 pad 配置；但这意味着 **pinmux-pins / pinconf-pins 这两份调试视图都不能单独用来判定脚的真实状态**。
3. `/proc/bus/input/devices` 里还有第三个设备 `dierct-keys-polled`（`input2` → `event2`），它**不在** DT 的 `gpio_keys` 节点里，名字里的 `dierct` 也像是 `direct` 的拼写残留。本次测试没有向它发事件，也没有读取它的事件流；它是什么、由谁注册，尚未确认，不应假设它与 17 个游戏键有关。

## 7. 边界

- 只做**读取**：`cat /dev/input/event1` 是普通读打开，没有 `EVIOCGRAB`、没有改 keymap、没有写 `/dev/input/*`、没有 `sendevent`。
- 全程没有写 GPIO/pinctrl 寄存器，没有 mount debugfs（采集器只读已有挂载）。
- 本测试证明“按键 → evdev 事件”这一段通了，**不**证明这些事件被 Slot / 前端正确消费，也不证明 libretro 层的按键映射正确。这一段需要另做前端侧观测。
- 电源键（`axp2202-pek` → `input0`/`event0`）按要求未纳入，未读取其事件流。
