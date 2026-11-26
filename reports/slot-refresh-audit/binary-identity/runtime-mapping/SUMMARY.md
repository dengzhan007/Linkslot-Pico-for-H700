# armGBA 运行时映射与主指针裁决（rg34xx / Slot v1.16.0-pvaibhav.2）

只读审计，仅 ADB。**未**读取 `/proc/PID/mem`，**未**创建 `trace-latency.on`，**未**设置 `SLOT_TRACE_LATENCY`，
**未**重启或停止 Slot，**未**修改任何配置，**未**写任何硬件寄存器。
二进制 SHA-256、面板参数、IRQ 293 数据已在本目录上级 `REPORT.md` 给定，此处不重复。

状态改变只有两次键注入（均为应要求、可逆）：A 键（`BTN_SOUTH=304`）打开游戏让核心 `dlopen`，
长按 Menu 键（`BTN_TL2=312`）3 秒退出、让核心 `dlclose`；
方法与字节流见 `armgba-maps.txt` §3。

## 0. 结论速览

| 问题 | 结论 |
| --- | --- |
| Slot PID | **2301**，由 `/proc/*/cmdline` 匹配 `/System/slot` 得出，未沿用旧值（旧值相同属巧合，设备已重启） |
| `/proc/PID/exe` | `ld-linux-aarch64.so.1`（Slot 用显式解释器启动，`exe` 是解释器，不是主程序） |
| `armgba_libretro.so` 映射 | **已取得**：进游戏后 4 行（UI 态 0 行），PID 未变 |
| PT_LOAD | 2 个：`#0 RX p_vaddr=0`、`#1 RW p_vaddr=0x1857a0 / p_offset=0x1757a0` |
| load bias | **`0x7f64070000`**，由 `r-xp` / `r--p` / `rw-p` 三行独立解出同值（`load-bias.json` → `captured`） |
| 主 GBA 指针 | **`gba @ 0x4259b8`**（libretro 前端单元）；`gba @ 0x4c16a8` 是联动模块的缓存副本 |

## 1. 采集范围与状态

设备：`rg34xx` / `sun50iw9` / Linux `4.9.170 #19`（serial 已脱敏），`uptime` ≈ 3699 s。
`adb devices -l` 仅一台设备。

| 采集项 | UI 态 | 游戏态 |
| --- | --- | --- |
| cmdline 匹配到的 Slot 进程 | `2301` | `2301`（同一进程，未重启） |
| `/proc/2301/exe` | `ld-linux-aarch64.so.1` | 同左 |
| `/proc/2301/maps` 总行数 | 414 | 430 |
| 其中 `armgba` 行数 | **0** | **4** |
| 已加载 `.so` 数量 | 12（无 libretro 核心） | 13（含 `armgba_libretro.so`） |

核心只在游戏运行时 `dlopen`、退出即 `dlclose`，所以 UI 态取不到映射。两次采集之间由用户要求，
向 `/dev/input/event1` 注入了两次按键：`BTN_SOUTH`(304) 打开游戏、长按 `BTN_TL2`(312) 3 秒退出
（设备无 `input`/`sendevent`，用 `dd` 写 24 字节 `input_event`）；这不属审计禁止项，也未触碰寄存器或配置。
退出后 `armgba` 行数由 4 回到 0、PID 仍为 2301。方法见 `armgba-maps.txt` §3 / §3.1。

## 2. PT_LOAD 与地址换算

`armgba-pt-load.txt` 全文在此不复述，只列决定 bias 的两行：

| idx | flags | p_offset | p_vaddr | p_filesz | p_memsz |
| --- | --- | --- | --- | --- | --- |
| 0 | RX | `0x0` | `0x0` | `0x1756c6` | `0x1756c6` |
| 1 | RW | `0x1757a0` | `0x1857a0` | `0x34ab0` | `0x4d0810` |

- `#0` 的 `p_vaddr == p_offset == 0`，是解 bias 的主尺：`bias = maps 起始地址 − 0`。
- `#1` 比 `#0` 在内存里高 `0x10000`（`p_vaddr − p_offset = 0x10000`），用于交叉验证。
- `#1` 的 `p_memsz − p_filesz = 0x49bd60 ≈ 4.6 MiB` 是 `.bss` 零页；两个 `gba`（`0x4c16a8`、`0x4259b8`）都在其中。

实测（见 `armgba-maps.txt` §5–§6）：

```text
r-xp 00000000 @ 0x7f64070000  → bias = 0x7f64070000 − (0x000000 + 0x00000) = 0x7f64070000
r--p 00175000 @ 0x7f641f5000  → bias = 0x7f641f5000 − (0x175000 + 0x10000) = 0x7f64070000
rw-p 00178000 @ 0x7f641f8000  → bias = 0x7f641f8000 − (0x178000 + 0x10000) = 0x7f64070000
```

三行同值，`bias = 0x7f64070000`。`---p` 空洞行的 offset 列是合成值，不参与解算。

本次实测（bias `0x7f64070000`）下的运行时地址：

```text
retro_run           = 0x7f6408f560
retro_load_game     = 0x7f6408cf60
retro_unload_game   = 0x7f6408d470
gba（主指针变量）    = 0x7f644959b8   → 解引用得 struct GBA * 堆地址（每次装载都变）
gba（联动缓存）      = 0x7f645316a8
```

两个 `gba` 都落在 `.bss` 匿名续行 `7f6422b000-7f646c6000` 内，已核对未越界（`.bss` 末符号 `0x655fb0`
→ `0x7f646c5fb0`）。

## 3. 三个 retro_* 函数的反汇编结论

完整清单见 `armgba-symbol-refs.txt` §7（含 `retro_run` 全部 2566 条指令）。

**`retro_load_game`（`0x1cf60`, 512 B）**
不直接碰 `gba`。它解析 `retro_game_info`，按 ROM 类型分派：`b 0x1ad30`（`loadAdvance`）、
`b 0x1b940`（`loadGameBoy`）、`b 0x1c240`（`loadPairFile`）；返回前通过 `bl 0x1c0c0`
（`loadMegaDuckLaptop`）等路径进入具体装载器。**指针赋值发生在下一层的 `loadAdvance.part.0`**。

**`retro_unload_game`（`0x1d470`, 1224 B）**
收尾时 `bl 0x33160`（`linkSetGba`，参数为 0）**先**断开联动模块的缓存，**再**执行
`0x1d6f0: str xzr, [x1]`（x1 = `GOT[&gba]`）把主指针清零。顺序本身就是“下游 ← 上游”的证据。

**`retro_run`（`0x1f560`, 10264 B）**
每帧固定套路，共 21 处命中：

```text
adrp x21, 0x187000
ldr  x0,  [x21, #0xd58]     ; x0 = &gba   （GOT 槽 0x187d58，R_AARCH64_RELATIVE → 0x4259b8）
ldr  x0,  [x0]              ; x0 = gba
bl   <消费函数>
```

消费函数包括 `gbaSetKeys`(`0x4aed0`)、`linkGbaFrame`(`0x335e0`)、`dolphinGbaFrame`(`0x30cc0`)，
以及帧步进本体 `advanceFrame`(`0x12c10`)。开头还会 `bl 0x33530`（`linkSession`）读联动状态 ——
那是 `gba@0x4c16a8` 那条支线的入口（读的是 `session` 标志，不是指针本身）。

## 4. 两个同名 `gba` 的裁决

两者都是 `STB_LOCAL`、大小 8 字节的指针变量，都在 `.bss`。判据不是名字，而是三条独立证据：

**证据 A — 赋值源**

| 对象 | 写入指令 | 所在函数 | 写入值 |
| --- | --- | --- | --- |
| `0x4259b8` | `str x2, [x22]` @ `0x1a9ac` | `loadAdvance.part.0` | `startAdvance()` 的返回值（新建 `struct GBA *`），与 `gbas[0]` 同源同值 |
| `0x4259b8` | `str xzr, [x1]` @ `0x1d6f0` | `retro_unload_game` | `NULL` |
| `0x4c16a8` | `str x19, [x1, #0x70]` @ `0x3318c` | `linkSetGba` | 形参 `x0` —— 即上游传进来的主指针 |
| `0x4c16a8` | `str xzr, [x1, #0x70]` @ `0x33228` | `linkSetGameBoy` | `NULL`（切 GB 模式） |

`0x4c16a8` 的写入者 `linkSetGba` 只被 **4 个函数、共 7 处**调用：`startOthers`（×2）、`loadAdvance.part.0`（×3）、
`retro_unload_game`、`retro_reset` —— 全在“核心刚建/刚毁”的路径上，是**下游缓存**。

**证据 B — 访问面**

| 对象 | 引用数 | 函数数 | 代表性函数 |
| --- | --- | --- | --- |
| `0x4259b8` | 108 | 19 | `retro_run`(21)、`retro_unserialize`(10)、`retro_unload_game`、`retro_reset`、`retro_serialize` |
| `0x4c16a8` | 17 | 12 | `receive`、`takeRequests`、`linkSetGba`、`cableChanged`、`disconnected` |

前者是整条 libretro 入口面，后者只有联动链路。

**证据 C — 拆卸顺序**
`retro_unload_game` 先 `linkSetGba(NULL)`、后 `gba = NULL`，说明 `0x4c16a8` 依附于 `0x4259b8`。

裁决：**`gba @ 0x4259b8` 是主 GBA 指针**（libretro 前端编译单元持有，`retro_run` 每帧从这里取核心）；
`gba @ 0x4c16a8` 是 link cable 编译单元持有的同值缓存。运行期两者相等，但只有前者是源。

## 5. 与 FPS 议题的衔接

- 主指针这一层是“谁在被跑”。已确认每帧都从 `*bias+0x4259b8` 取核心，没有第二条并行通路。
- `0x4259b8` 是**可换**的：`loadAdvance.part.0` 会整体替换（多卡/换卡时 `gbas[]` 里换人），
  所以不能把游戏期间抓到的堆地址当成常量写进报告；每次都要按 `bias + 0x4259b8` 重新解引用。
- 通过 GOT（`0x187d58`）而不是 `adrp` 直连，是编译单元把该符号当外部可见对象处理的结果；
  这也解释了为什么上一轮纯文本扫描 `adrp`+`offset` 找不到它 —— 必须走 GOT 槽才能命中。
- 帧率相关的判据（面板时序、IRQ 计数）不在本次范围，未重复采集；本文件只提供地址底座，
  供后续把运行期 PC / 指针换算回符号。

## 6. 边界与未做项

1. `load-bias.json` 已从 `pending-capture` 转为 `captured`，三行同值。bias 是**本次装载**的值，
   进程重启或重新 `dlopen` 后会变（ASLR），引用时一律按“现采 maps → 现算 bias”走，不要当常量。
2. 未验证运行时 `*bias+0x4259b8` 与 `*bias+0x4c16a8` 是否确实相等 —— 读这两个值需要 `/proc/PID/mem`，
   属禁止项，只能由反汇编链条推定（三条证据见 §4）。
3. 未测帧率/IRQ：本文件只交付地址底座，面板时序与中断计数不在本轮范围。

## 7. 文件清单

| 文件 | 内容 |
| --- | --- |
| `armgba-maps.txt` | PID 定位、`exe`、UI 态/游戏态两相、进/退游戏的键注入方法、bias 解算表、`.bss` 归属 |
| `armgba-pt-load.txt` | 2 个 PT_LOAD 全字段、页对齐展开、bias 公式 |
| `armgba-symbol-refs.txt` | 符号表、GOT 槽、125 条引用索引、三个函数全文反汇编 |
| `load-bias.json` | bias 方法、实测行、`captured` 结果与运行时符号表 |
| `SUMMARY.md` | 本文件 |
