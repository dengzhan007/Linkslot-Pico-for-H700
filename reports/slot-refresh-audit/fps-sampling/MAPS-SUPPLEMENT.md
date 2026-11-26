# armGBA maps-only 证据补充（RG34XX，2026-11-26）

## 1. 原始 PID 22514 快照：未找回

按任务要求搜索了当前仓库、Agent 工作目录、任务产物与本地采集缓存，关键词包括 PID `22514`、
bias `0x7f55ae0000`、map signature `8f7db48f8bb8ea5f`、`/proc/22514/maps`、`maps-armgba`、`armgba-maps`。
只找到已生成的汇总文件（CSV/JSON/README）与更早的 PID 2301 / PID 2842 maps，**没有**本次采样
PID 22514 的首尾完整 `/proc/PID/maps` 快照。因此没有伪造或改名旧文件；原报告的 bias 与
signature 仍只是摘要声明，缺原始三行支撑。

## 2. 独立 maps-only 补采（新 PID / 新 ASLR）

这不是对旧证据的修复，而是一次新的、明确区分的采集：

| 项 | 值 |
| --- | --- |
| 设备 serial | `2c00107411818711bd2`（`adb devices -l` 只有这一台，`state=device`） |
| 设备身份 | `uname=Linux rg34xx 4.9.170 ... aarch64`；DT model `sun50iw9`；compatible `allwinner,h616 arm,sun50iw9p1`；`BASEOS_TARGET=rg34xx` |
| 新 Slot PID | **8064**（旧采样 PID 为 22514，不可混用） |
| ROM | `/mnt/sdcard/Games/GBA/CT特种部队1.gba` |
| core | `/mnt/sdcard/System/armgba_libretro.so` |
| 第一次快照 | host UTC `2026-11-26T01:41:55.877651900Z` → `2026-11-26T01:42:01.229094800Z` |
| 第二次快照 | host UTC `2026-11-26T01:42:02.366788800Z` → `2026-11-26T01:42:02.601369900Z` |
| 快照差异 | 两份完整 maps 均为 425 行，armgba 四行逐字节相同，load bias 与 signature 相同 |
| 新 bias | **`0x7f339a0000`** |
| 新 map signature | `592b1f67f24c64be`（旧报告 signature `8f7db48f8bb8ea5f` 属于 PID 22514） |

进入游戏使用此前已验证的 A 键 helper（`/tmp/a_press.bin` + `/tmp/a_release.bin`，`BTN_SOUTH=304`），
日志确认 `emu: loading .../CT特种部队1.gba` 与 `launch: core opened`，maps 出现 4 行 armgba 后才采集。

## 3. 三行 PT_LOAD 独立交叉验证

两份文件分别按同样公式计算，结果完全一致：

| 映射行 | offset | 第一次 map_start | 第一次 bias | 第二次 map_start | 第二次 bias |
| --- | --- | --- | --- | --- | --- |
| RX `r-xp` | `0x000000` | `0x7f339a0000` | `0x7f339a0000` | `0x7f339a0000` | `0x7f339a0000` |
| R `r--p` | `0x175000` | `0x7f33b25000` | `0x7f339a0000` | `0x7f33b25000` | `0x7f339a0000` |
| RW `rw-p` | `0x178000` | `0x7f33b28000` | `0x7f339a0000` | `0x7f33b28000` | `0x7f339a0000` |

公式：`bias = map_start`（RX offset 0）、`map_start - 0x185000`（R offset 0x175000）、
`map_start - 0x188000`（RW offset 0x178000）。`---p` 空洞行不参与。新 bias 与旧报告的
`0x7f55ae0000` 不同，这是 ASLR 造成的正常差异，**不能**用来替代或“证明”旧 PID 的映射行。

## 4. 边界与恢复

- 未重采 FPS；未读取 `/proc/PID/mem`；未 ptrace/gdb；未创建 trace marker；未设置
  `SLOT_TRACE_LATENCY`；未重启或杀死 Slot；未改配置、ROM、存档或启动分区。
- 未写 PLL/TCON/CCU/PMIC，未使用 `/dev/mem` 或 `sunxi_dump/write`。`hardware_applied = 0`。
- 采集后用此前验证过的长按 Menu helper（`BTN_TL2=312`，3 秒）退出游戏：
  `2026-11-26T01:43:02Z` 时 PID 8064 仍存活、状态 S、armgba 映射 **0**、marker ABSENT、
  `SLOT_TRACE_LATENCY` absent，Slot 回到 UI。
- `adb shell cat` 在传输层把每行改为 `\r\r\n`；交付的 `.txt` 仅去掉行尾 CR，
  保留所有字段与顺序，原样字节可从采集 raw 复核（未上传 raw）。

## 5. 文件 SHA-256

| 文件 | SHA-256 |
| --- | --- |
| `armgba-maps-supplement-first.txt` | `5b2ff0a9cf21350ef314f7c1d07fddffdcdaedbf42b98a27b6a6095a97ff2e75` |
| `armgba-maps-supplement-last.txt` | `401362c79fe7dcbf3e37cc2502280741155105a5378e8f74317f09e6e593682d` |

两份文件已加入本目录 `SHA256SUMS.txt`；CSV、FPS 结果、JSON 与旧日志均未重新生成或覆盖。
