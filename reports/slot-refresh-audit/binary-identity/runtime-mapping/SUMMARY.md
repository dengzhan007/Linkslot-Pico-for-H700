# armGBA 运行时映射与主指针裁决（rg34xx / Slot v1.16.0-pvaibhav.2）

只读审计，仅 ADB。**未**读取 `/proc/PID/mem`，**未**创建 `trace-latency.on`，**未**设置 `SLOT_TRACE_LATENCY`，
**未**重启或停止 Slot，**未**修改任何配置，**未**写任何硬件寄存器。
二进制 SHA-256、面板参数、IRQ 293 数据已在本目录上级 `REPORT.md` 给定，此处不重复。

## 0. 结论速览

| 问题 | 结论 |
| --- | --- |
| Slot PID | **2301**，由 `/proc/*/cmdline` 匹配 `/System/slot` 得出，未沿用旧值（旧值相同属巧合，设备已重启） |
| `/proc/PID/exe` | `ld-linux-aarch64.so.1`（Slot 用显式解释器启动，`exe` 是解释器，不是主程序） |
| `armgba_libretro.so` 映射 | **本次未取得**：采集时 Slot 在前端 UI，核心尚未 `dlopen`（`grep -c armgba maps` = 0） |
| PT_LOAD | 2 个：`#0 RX p_vaddr=0`、`#1 RW p_vaddr=0x1857a0 / p_offset=0x1757a0` |
| load bias | 方法已固定（见 §3），数值待游戏运行中采集后填；`load-bias.json` 的 `captured` 目前为 `null` |
| 主 GBA 指针 | **`gba @ 0x4259b8`**（libretro 前端单元）；`gba @ 0x4c16a8` 是联动模块的缓存副本 |

## 1. 采集范围与状态

设备：`rg34xx` / `sun50iw9` / Linux `4.9.170 #19`（serial 已脱敏），`uptime` ≈ 3699 s。
`adb devices -l` 仅一台设备。

| 采集项 | 结果 |
| --- | --- |
| cmdline 匹配到的 Slot 进程 | `2301` |
| `/proc/2301/exe` | `/usr/lib/aarch64-linux-gnu/ld-linux-aarch64.so.1` |
| `/proc/2301/maps` 总行数 | 414 |
| 其中 `armgba` 行数 | **0** |
| 已加载 `.so` 数量 | 12（无任何 libretro 核心） |

结论：前端 UI 态下核心未被加载，`maps` 里没有 `armgba_libretro.so`。这在预期之内 ——
核心只在游戏运行时 `dlopen`、退出即 `dlclose`。要把 bias 落成具体数值，必须在游戏画面中重跑
`armgba-maps.txt` §4 的 4 条命令。详见该文件 §5 的预期行形状与 §6 的待填表。

## 2. PT_LOAD 与地址换算

`armgba-pt-load.txt` 全文在此不复述，只列决定 bias 的两行：

| idx | flags | p_offset | p_vaddr | p_filesz | p_memsz |
| --- | --- | --- | --- | --- | --- |
| 0 | RX | `0x0` | `0x0` | `0x1756c6` | `0x1756c6` |
| 1 | RW | `0x1757a0` | `0x1857a0` | `0x34ab0` | `0x4d0810` |

- `#0` 的 `p_vaddr == p_offset == 0`，是解 bias 的主尺：`bias = maps 起始地址 − 0`。
- `#1` 比 `#0` 在内存里高 `0x10000`（`p_vaddr − p_offset = 0x10000`），用于交叉验证。
- `#1` 的 `p_memsz − p_filesz = 0x49bd60 ≈ 4.6 MiB` 是 `.bss` 零页；两个 `gba`（`0x4c16a8`、`0x4259b8`）都在其中。

一旦拿到 maps：

```text
bias = start(r-xp, offset 0x000000)                        # 主解
bias = start(任意 PT_LOAD#1 行) − 0x185000                  # 交叉验证
runtime_addr(sym) = bias + sym.st_value                    # 反查任意符号
```

本审计后续要用到的三个换算式（游戏运行时才有意义）：

```text
retro_run          = bias + 0x1f560
gba（主指针变量）   = bias + 0x4259b8      →  *(u64) 得到 GBASystem* 堆地址
gba（联动缓存）     = bias + 0x4c16a8
```

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
| `0x4259b8` | `str x2, [x22]` @ `0x1a9ac` | `loadAdvance.part.0` | `startAdvance()` 的返回值（新建 `GBASystem*`），与 `gbas[0]` 同源同值 |
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

## 6. 未完成项

1. `armgba_libretro.so` 的运行时 `maps` 行与 `bias` 数值 —— 需在游戏画面中按 `armgba-maps.txt` §4 采集，
   然后 `python build/k22_bias.py <label>` 重新生成 `load-bias.json`（脚本会自动做多行交叉验证）。
2. 未验证运行时 `*bias+0x4259b8` 与 `*bias+0x4c16a8` 是否确实相等 —— 读这两个值需要 `/proc/PID/mem`，
   属禁止项，只能由反汇编链条推定。

## 7. 文件清单

| 文件 | 内容 |
| --- | --- |
| `armgba-maps.txt` | PID 定位、`exe`、UI 态负结果、待采集命令、预期行形状 |
| `armgba-pt-load.txt` | 2 个 PT_LOAD 全字段、页对齐展开、bias 公式 |
| `armgba-symbol-refs.txt` | 符号表、GOT 槽、125 条引用索引、三个函数全文反汇编 |
| `load-bias.json` | bias 方法、预测行、实测（当前 `pending-capture`） |
| `SUMMARY.md` | 本文件 |
