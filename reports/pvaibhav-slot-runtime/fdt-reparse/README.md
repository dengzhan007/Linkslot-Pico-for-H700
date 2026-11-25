# RG34XX 运行时 FDT 重解析（fdt-reparse）

只读采集：从运行中的 RG34XX 拉取 `/sys/firmware/fdt`，用仓库内的 `tools/inspect_h700_fdt.py`
重新解析，用于确认面板 `timing` 字段是否已完整输出。

- 采集时间：2026-11-25 19:09（GMT+8）
- 设备序列号：`[REDACTED]`（已脱敏；原值同时出现在 FDT `bootargs` 的 `snum=` 与
  `androidboot.serialno=` 中，所以原始 DTB 不随报告发布，见下节）
- 内核：`Linux 4.9.170 #19 SMP PREEMPT Tue May 26 13:58:08 CST 2026 aarch64`
- boot_id：`e0ca1c1b-6d0f-45db-a9be-2744d7fe1f0d`（采集时 uptime 约 350–415 s）
- 采集方式：`adb pull /sys/firmware/fdt` + `python -B tools/inspect_h700_fdt.py`，
  未执行 root 提权、未写入设备、未修改任何配置或权限。

## 脱敏说明

| 项 | 处理 |
| --- | --- |
| `live-fdt.dtb`（原始 FDT） | **不发布**，仅保留本地（`build/fdt-reparse-local/live-fdt.dtb`）。`bootargs` 内含 `snum=` / `androidboot.serialno=`，改字节就破坏 DTB 结构与哈希，因此整份不发布 |
| `live-fdt.json` → `chosen_bootargs` | 两处序列号替换为 `<serial-redacted>`，其余字节未改；新增 `redactions` 字段记录替换了哪些键 |
| `device-sha256.txt` | 删除 `serial` 行，改为 `[REDACTED]`，并保留设备型号/内核/uptime/boot_id 用于对齐采集批次 |
| 哈希 | 脱敏后 JSON 的 SHA-256 与原始解析结果不同（原始 38537 B 不再存在），下表给的是**已发布脱敏版**的哈希；原始 DTB 的 SHA-256 仍记录，用于证明本地留档与当时解析的是同一份 blob |

本目录在脱敏前曾提交过原始 DTB 与未脱敏 JSON（`cb1b87b`、`c528833`）；这两次提交仍在
`reports` 分支历史里，彻底撤回需要重写该分支历史（见目录末尾的"历史遗留"）。

## 产物与哈希

| 文件 | 字节 | SHA-256 |
| --- | --- | --- |
| `live-fdt.json`（已脱敏，已发布） | 38866 | `8d6e6e1477c187a692cd79c473823f39fdd19794aa745fc503012af0b5470c46` |
| `live-fdt.dtb`（未发布，本地留档） | 143936 | `65801950f6d2d3c7139111d35c6cc0dd64cc249f8ba64ffb5b6f1c7fd9c39173` |
| `inspect_h700_fdt-panel-timing.patch` | — | 解析器两个提交的原始补丁（`b48e579` + `04298e1`） |
| `device-sha256.txt` | — | 设备侧 slot / 核心哈希原始记录（序列号已脱敏） |
| `file-hashes.txt` | — | 本地文件哈希原始记录 |

`live-fdt.json` 内部 `sha256` 字段与本地 `live-fdt.dtb` 的 SHA-256 一致，说明解析器读的是同一份
143936 字节 FDT（`totalsize` 143936，`node_count` 573，`model` `sun50iw9`）。

> 换行与哈希：`pathlib.write_text` 默认按平台换行写出，同一份 DTB 在 Windows 上会得到
> 39887 字节的 CRLF JSON、在其它平台是 38537 字节的 LF JSON，任何记录的 SHA-256 只能描述其中一种。
> 本次同时把输出固定为 LF（提交 `04298e1`），因此上表 `live-fdt.json` 的哈希对任何平台重跑都成立
> （脱敏替换是确定性的字符串替换，重跑同一份 DTB 后再做一次替换即可复现同一哈希）。

## panels[0]：`timing` 不再缺失

此前的 `live-fdt.json`（18:44 / 18:45 两轮）中 `panels[]` 只有
`node / driver / width / height / status`，没有 `interface` 与 `timing`。
原因是 `tools/inspect_h700_fdt.py` 的面板分支只提取了 `lcd_driver_name / lcd_x / lcd_y`，
从未读取 `lcd_dclk_freq / lcd_ht / lcd_vt / lcd_hbp / lcd_hspw / lcd_vbp / lcd_vspw`。
本次给解析器补上了这两组字段（`_panel_timing()` / `_panel_interface()`），
提交 `b48e579`（分支 `porting/linkslot-1.0.45-feasibility`，纯新增 71 行、无删改），
另有 `04298e1` 把 JSON 输出固定为 LF；两个提交的原始补丁随本目录保存为
`inspect_h700_fdt-panel-timing.patch`（`git am` 可直接套用）。
重新解析同一份 DTB 后 `panels[0].timing` 为完整对象：

```json
{
  "node": "/soc@03000000/lcd0@01c0c000",
  "driver": "rg34xx_v1",
  "width": 720,
  "height": 480,
  "used": true,
  "interface": "rgb666",
  "timing": {
    "pixel_clock_hz": 26000000,
    "pixel_clock_mhz": 26,
    "h_active": 720, "h_total": 820,
    "h_back_porch": 46, "h_sync_pulse": 2, "h_front_porch": 52,
    "v_active": 480, "v_total": 536,
    "v_back_porch": 20, "v_sync_pulse": 5, "v_front_porch": 31,
    "nominal_refresh_hz": 59.155,
    "physical_mm": { "width": 150, "height": 94 },
    "pwm": { "used": true, "channel": 0, "freq_hz": 50000,
             "polarity": 1, "max_limit": 255, "backlight": 50 }
  }
}
```

派生值说明（由 DTB 原始属性计算，非测量值）：

- `h_front_porch = lcd_ht - lcd_x - lcd_hbp - lcd_hspw = 820 - 720 - 46 - 2 = 52`
- `v_front_porch = lcd_vt - lcd_y - lcd_vbp - lcd_vspw = 536 - 480 - 20 - 5 = 31`
- `nominal_refresh_hz = 26 MHz / (820 × 536) = 59.155 Hz`

注意：18:44/18:45 两轮 `display-state.txt` 记录的驱动侧 `fps:60.2` 与本处
`59.155 Hz` 不一致。DTB 只给出标称值，驱动实际 dclk 或 disp 内部计数口径可能不同，
**未测量**实际刷新率，此差异保留为未解释项，不作推断。

`interface` 取 `lcd_if=0`（HV/RGB）叠加 `lcd_hv_if=0`，映射为 `rgb666`；
原始寄存器字段保留在 `panels[0].interface_props`。`lcd_hv_sync_polarity=3`（HS/VS 均反相）、
`lcd_hv_clk_phase=1`。

## 与 18:44 那轮 FDT 的差异（跨启动周期）

本次拉取的 DTB 与 `20261125-184406-ui/live-fdt.dtb` 字节数相同（143936）但哈希不同
（`65801950…` vs `6d1301e6…`）。结构化比对后，**整棵树只有 `/chosen` 下 3 个属性不同**，
其余 573 个节点（含 `lcd0@01c0c000` 全部属性）逐字节一致：

| 属性 | 18:44 那轮 | 本次 |
| --- | --- | --- |
| `bootargs` | `…bootreason=button` | `…bootreason=unknow` |
| `baseos,native-timing` | 一组启动时间戳 | 另一组启动时间戳 |
| `baseos,boot-display` | `DSP1…` 含 `0x20d1f498` | `DSP1…` 含 `0x6b510fed` |

结合采集时 `uptime` 仅约 350 s，可判定设备在 18:45 之后**已经重启过**，本次是新的启动周期；
上述 3 个差异全部属于启动期记录，与显示/面板配置无关。面板 timing 结论不受影响。

## 设备侧文件哈希

```text
8f0fd40da4ac161e5fbb9a56a99c254acf8b2f31806e2be6bdaa929b5ee28505  /mnt/sdcard/System/slot            18718728 bytes
4ebae098cc4848fb459fc18299c39acbdbbbeb732db5c3f66d1ff3f6e142e889  /mnt/sdcard/SYSTEM/armgba_libretro.so  2021992 bytes
```

这两个哈希在本仓库此前任何报告中都没有出现过，属于首次记录，无法与历史版本对比。

## 未覆盖 / 未测量

- 本目录不含实际 dclk：DTB 只给标称 26 MHz。TCON DCLK 分频寄存器与视频 PLL 寄存器的只读
  readback 见同级目录 `../tcon-readback/`（`sunxi_dump` 的 dump 请求节点，未写 `write` 节点）。
- 没有读取 `lcd_pwm_*` 之外的其他 PWM/背光寄存器，背光实际占空比未测。
- 没有确认 `rg34xx_v1` 面板驱动代码与 DTB 参数是否逐项一致（驱动源码未在本仓库）。
- 未做 `adb root`，本次可读是因为当前 rootfs 本身以 `uid=0` 提供 shell；
  `getprop` 在该 rootfs 中不存在，因此 `ro.boot.bootreason` 只能用 `bootargs` 里的
  `bootreason=` 字段间接佐证。

## 历史遗留（未撤回的旧提交）

`reports` 分支上仍存在两次在脱敏前推送的提交：

| 提交 | 含什么 |
| --- | --- |
| `cb1b87b` | 首次发布：原始 `live-fdt.dtb`（bootargs 含序列号）+ 未脱敏 `live-fdt.json` |
| `c528833` | 补哈希与 `.gitattributes`，同样携带上述两个未脱敏 blob |

工作树（分支最新提交）已不含序列号与原始 DTB，但 `git log`/`git show` 仍能取到旧 blob
（blob `114d69ed…` 为原始 DTB，`afbece5e…` 为未脱敏 JSON）。要彻底撤回，必须重写
`reports` 分支历史并从远端强推，同时让 GitHub 侧旧对象失去引用；在此之前，任何拿到仓库历史的人
都能恢复序列号。是否执行重写需由仓库所有者决定。
