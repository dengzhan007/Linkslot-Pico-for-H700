# Slot 二进制溯源（v1.16.0-pvaibhav.2 / RG34XX）

只读审计，仅 ADB，无源码改动、无硬件写入。设备侧文件与官方发布包**逐字节一致**。

## 0. 结论

| 问题 | 答案 |
| --- | --- |
| 官方包完整性 | ZIP SHA-256 = `1f9bdaa1…fca`，与任务给定值一致 |
| 设备 `slot` 是否官方原版 | 是，SHA-256 与 ZIP 内 `System/slot` 完全相同 |
| 设备 `armgba_libretro.so` 是否官方原版 | 是，SHA-256 与 ZIP 内同名文件完全相同 |
| 追加核对 | 另外三个核心 + 两个脚本也全部一致，mtime 与 ZIP 条目时间戳相同 |
| 源码 commit | `armGBA` @ `df6b2631291f249d54a00dd14f49bc893e438feb` |
| patch SHA-256 | `link-stop-queued-once.patch` = `6ffb6a9f…94db`，实算与 `.meta` 声明一致 |
| 调试符号 | core 与 slot **都保留 `.symtab`**，未 strip |
| 运行时 trace | `SLOT_TRACE_LATENCY` 未设置，`System/trace-latency.on` 不存在 |

## 1. 官方包校验

| 项 | 值 |
| --- | --- |
| URL | `https://github.com/pvaibhav/slot/releases/download/v1.16.0-pvaibhav.2/slot-v1.16.0-pvaibhav.2-h700.zip` |
| 大小 | 17 556 056 B |
| 实测 SHA-256 | `1f9bdaa1110eb4f6006cc9da6a8bb34bca694ea53555c3138dcf656f0dfd1fca` |
| 期望 SHA-256 | `1f9bdaa1110eb4f6006cc9da6a8bb34bca694ea53555c3138dcf656f0dfd1fca` |
| 结果 | **一致** |

ZIP 内 65 个文件（117 个条目含目录），逐条目 SHA-256 见 [`zip-inventory.txt`](zip-inventory.txt)。

## 2. 设备文件身份

设备：`BASEOS 1.5.3` / `rg34xx` / Linux `4.9.170 #19`（serial 已脱敏）。

| 设备路径 | 大小 | mtime (UTC) | SHA-256 | 与 ZIP |
| --- | --- | --- | --- | --- |
| `/mnt/sdcard/System/slot` | 18 718 728 | 2026-11-20 02:16:46 | `8f0fd40da4ac161e5fbb9a56a99c254acf8b2f31806e2be6bdaa929b5ee28505` | **相同** |
| `/mnt/sdcard/SYSTEM/armgba_libretro.so` | 2 021 992 | 2026-11-20 02:16:46 | `4ebae098cc4848fb459fc18299c39acbdbbbeb732db5c3f66d1ff3f6e142e889` | **相同** |

设备端 `sha256sum` 与本地 pull 回来的文件哈希一致，pull 过程未损坏。

追加核对（任务只要求两个文件；其余顺手确认，用于排除手工混装）：

| 文件 | 设备 SHA-256 | ZIP 一致 |
| --- | --- | --- |
| `gpsp_libretro.so` | `056634d6…66c1` | 是 |
| `mgba_libretro.so` | `2a2166cc…8d09` | 是 |
| `gambatte_libretro.so` | `bbb6b96d…d316` | 是 |
| `launch_frontend.sh` | `468b3763…e56a` | 是 |
| `slotlink.sh` | `84537b8c…867f` | 是 |

两个二进制的 mtime `2026-11-20 02:16:46` 与 ZIP 条目的 `date_time` 完全相同，说明是安装时按包内时间戳落盘，不是本地重编译产物。

## 3. ZIP 内的 armgba 文件与 `.meta`

| 文件 | 大小 |
| --- | --- |
| `System/armgba_libretro.so` | 2 021 992 |
| `System/licenses/armgba-df6b2631291f249d54a00dd14f49bc893e438feb.meta` | 352 |
| `System/licenses/armgba-df6b2631291f249d54a00dd14f49bc893e438feb.tar.gz` | 1 480 490 |
| `System/licenses/armgba-link-stop-queued-once.patch` | 293 |
| `System/licenses/armgba-GPL-2.0.txt` | 17 987 |

`.meta` 原文（[`meta/`](meta/)）：

```
commit=df6b2631291f249d54a00dd14f49bc893e438feb
source=https://github.com/MagnominusGit/armGBA/archive/df6b2631291f249d54a00dd14f49bc893e438feb.tar.gz
recipe=cmake -DCMAKE_BUILD_TYPE=Release, target armgba_libretro
device_cflags=-mcpu=cortex-a53
patch=link-stop-queued-once.patch sha256:6ffb6a9f83b6fdaf77350725e74f0bedbd530256bce3e5e4fb0cc4b9e5e794db
```

两处独立验证：

1. patch 实算 SHA-256 = `6ffb6a9f83b6fdaf77350725e74f0bedbd530256bce3e5e4fb0cc4b9e5e794db`，与 `.meta` 声明逐位相同。
2. 源码包顶层目录为 `armGBA-df6b2631291f249d54a00dd14f49bc893e438feb/`，与 `commit=` 一致。

patch 只有一行有效改动，位于 `libretro/link.c`：

```diff
@@ -548,6 +548,7 @@
 		exact.held = true;
 		machine->returnRequested = true;
 		timingInterrupt(machine);
+		timingDeschedule(machine, &exact.stop);
 		timingSchedule(machine, &exact.stop, 0);
 		return;
```

即 link 停止事件先撤销再重排，避免队列中残留。这与刷新率无关。

其余核心的 commit（同样来自各自 `.meta`）：

| 核心 | commit | patch 数 |
| --- | --- | --- |
| gambatte | `77f2567d718b8973d02caa2daa8dce4953faf811` | 1 |
| gpsp | `7b2dd37394e2822110cacef321e9bb291a305b77` | 2 |
| mgba | `e31759b24e7a4e3899285ff720d7b573ac328ae7` | 12 |

## 4. ELF 分析

**工具说明（重要）**：本机与设备都没有 `readelf`（`command -v readelf` 为空，设备侧 binutils/llvm 均不存在）。因此用 `pyelftools 0.33` 复现同样四个视图（`-h`、`-n`、`-S`、`-Ws` / `--dyn-syms`），字段口径与 `readelf` 对齐，原始输出四份附在本目录：

- [`device-armgba.elf.txt`](device-armgba.elf.txt) / [`zip-armgba.elf.txt`](zip-armgba.elf.txt)
- [`device-slot.elf.txt`](device-slot.elf.txt) / [`zip-slot.elf.txt`](zip-slot.elf.txt)

设备与 ZIP 两版 core 的 dump 除首行文件名外**完全一致**（`diff` 仅 1 行差异），与哈希相同互为印证。

### 4.1 头部与 Build ID

| 项 | `slot` | `armgba_libretro.so` |
| --- | --- | --- |
| Class / Data | ELF64 / little endian | ELF64 / little endian |
| Type | `DYN`（PIE 可执行） | `DYN`（共享对象） |
| Machine | AArch64 | AArch64 |
| Entry | `0x7036c` | `0x11880` |
| **Build ID** | `165bb449d732ad848dc0e9b41ec961268abc37c8` | `08309a288967a62cf4155e27d1b10ce13a5e33ef` |
| `.note.gnu.build-id` 与 `PT_NOTE` | 两者一致 | 两者一致 |

### 4.2 符号表存在性

| 项 | `slot` | `armgba_libretro.so` |
| --- | --- | --- |
| `.symtab` | **存在**（37 201 项：37 005 defined / 196 undefined） | **存在**（4 960 项：4 769 defined / 191 undefined） |
| `.dynsym` | 199 项（2 defined / 197 undefined） | 240 项（49 defined / 191 undefined） |
| 是否 strip | 否 | 否 |

两者的 `.symtab` 都保留，意味着可以按符号名做函数级定位，而不必靠启发式扫描。

### 4.3 gba 符号状态

`armgba_libretro.so`：**`.dynsym` 中 0 个**，**`.symtab` 中 47 个**，全部为 `LOCAL`、`DEFAULT` 可见性，无一导出。类型分布为 `FUNC`（`.text`，`ndx=11`）与 `OBJECT`（`.data`/`.bss`，`ndx=26`；少量 `.rodata`，`ndx=13`）。代表项：

| 符号 | 地址 | 大小 | 类型 |
| --- | --- | --- | --- |
| `gbaCreate` | `0x4b180` | 12 932 | FUNC |
| `gbaReset` | `0x4e590` | 3 620 | FUNC |
| `gbaRunPart` | `0x51fe0` | 7 236 | FUNC |
| `gbaLoadRom` | `0x49f20` | 664 | FUNC |
| `gbaShown` | `0x346bf0` | 307 200 | OBJECT |
| `armgba_agb_boot` | `0x1292e0` | 2 304 | OBJECT |
| `gba` | `0x4c16a8` | 8 | OBJECT |

`slot`：`.dynsym` 0 个、`.symtab` 24 个含 `gba` 的名字，但性质不同——都是 Rust 侧对象（`slot_store::gba::field`、`slot_ui::bitmaps::gba_cart1P1`、`slot::root::armgba_boot_crc`，以及一堆 `*_rgba` 图像函数）。**slot 自身不含 GBA 模拟器实现**，模拟全部在 core 内。

### 4.4 libretro 入口

`armgba_libretro.so` 的 24 个 libretro ABI 入口全部 `DEF`，且在 `.dynsym` 与 `.symtab` 中地址一致：

| 入口 | 地址 | 入口 | 地址 |
| --- | --- | --- | --- |
| `retro_init` | `0x1c7b0` | `retro_run` | `0x1f560` |
| `retro_load_game` | `0x1cf60` | `retro_serialize` | `0x21dd0` |
| `retro_get_system_av_info` | `0x1c720` | `retro_unserialize` | `0x221f0` |
| `retro_set_video_refresh` | `0x1c690` | `retro_get_memory_data` | `0x22bc0` |

`slot` 侧这 24 个名字在两张表中**全部 absent**，说明宿主是自己 `dlopen` + 取符号，而不是链接期绑定——与日志中 `slot: launch: core opened` 的记录方式一致。

## 5. 运行时进程状态

按 `cmdline` 中 `/mnt/sdcard/System/slot` 匹配，**PID = 2301**（注意：遍历 `/proc` 时正则也会匹配到执行采集的 shell 自身，已按 comm 与 cmdline 精确排除）。

| 项 | 值 |
| --- | --- |
| `comm` | `ld-linux-aarch6` |
| `cmdline` | `/lib/ld-linux-aarch64.so.1 /mnt/sdcard/System/slot ` |
| `fd/0` | `/dev/console` |
| `fd/1` | `/mnt/sdcard/Logs/slot.log`（`l-wx`） |
| `fd/2` | `/mnt/sdcard/Logs/slot.log`（`l-wx`，与 fd/1 同一文件） |
| 其它 | `/dev/mali0`、`/dev/fb0`、`/dmabuf (deleted)`、`/dev/input/event0-2`、`socket:[4834]` |

`fd/1` 与 `fd/2` 都指向同一个 2 428 字节的日志文件，三条路径（`fd/1`、`fd/2`、`Logs/slot.log`）的整文件 md5 完全相同（`4084bb32865437310e319bec905cb548`）。两者都是只写句柄，本次是以只读方式重新打开同一文件读取，未从任何活动管道取走数据。

日志尾部（游戏名已脱敏）显示本轮会话确实加载的是 armgba：

```
slot: launch: opening <redacted-game>.gba on armgba at 489594.1 ms
slot: core /mnt/sdcard/System/armgba_libretro.so
slot: launch: core opened at 489714.1 ms
slot: emu: ready at 490761.0 ms
slot: launch: first frame shown at 490836.6 ms
...
slot: emu: stopping at 702956.0 ms
slot: emu: stopped at 703032.8 ms
```

`environ` 共 57 项，全部为 `BASEOS_*` / `SLOT_*` 与 `PATH`，与刷新相关的重要事实：

| 检查项 | 结果 |
| --- | --- |
| `SLOT_TRACE_LATENCY` | **不存在**（`grep -c '^SLOT_TRACE_LATENCY='` = 0） |
| `/mnt/sdcard/System/trace-latency.on` | **不存在**（`No such file or directory`） |
| `System/` 下任何 `*trace*` 文件 | 无 |

即延迟跟踪未启用。二进制里确实存在该功能的字符串与分支（`SLOT_TRACE_LATENCY`、`System/trace-latency.on`，以及 `slot: latency: read>pad` / `pad>emu` / `emu>pub` / `pub>take` / `take>swapped` 五个阶段名），但本次**没有创建标记文件**，也没有设置环境变量。

## 6. 附带发现：`high_refresh` 是「设置项 + 能力位门控」

采集 `slot` 符号时命中三个与刷新能力相关的方法，均为 `slot_power::device::DevicePlatform` 的 trait 实现，体积极小，直接离线反汇编（证据：[`high-refresh-gate.txt`](high-refresh-gate.txt)）：

```
Platform::supports_high_refresh @ 0x95664
  ldr  x8, [x0, #0xb8]
  mov  x9, #-0x8000000000000000
  cmp  x8, x9
  cset w0, ne
  ret

Platform::high_refresh_enabled @ 0x95648
  ldr  x9, [x0, #0xb8]
  mov  x8, #-0x8000000000000000
  cmp  x9, x8
  ldrb w8, [x0, #0xe8]
  cset w9, ne
  and  w0, w9, w8
  ret

Platform::set_high_refresh @ 0x4f37d0
  ldr  x8, [x0, #0xb8]!
  mov  x9, #-0x8000000000000000
  cmp  x8, x9
  b.ne #0x4f37e8          ; -> tail call PanelRefresh::set @ 0x4f2cf8
  mov  w0, wzr            ; sentinel 命中：什么都不做，直接返回 0
  ret
```

机制很清楚：`self+0xb8` 这个字段若等于哨兵 `0x8000000000000000`，`supports_high_refresh` 返回 false，`set_high_refresh` **直接返回 0 而不触碰面板**；只有不等于哨兵才会尾调用 `PanelRefresh::set`。`high_refresh_enabled` 还额外要求 `self+0xe8` 字节为真。

`high_refresh=` 这个键在二进制里与 `shader_gba=`、`rewind=`、`gb_palettes=`、`dim=` 等并列，属于设置文件的键值，与 `slot.state` 的观测一致。

这解释了上一轮实测为什么「`high_refresh=1` 但仍是 60 Hz」：设置项只是一个输入，真正生效与否取决于平台能力字段。**本次没有证明该字段在 rg34xx 上到底是哪个分支**（需要 dump 运行时结构体，属禁止的 `/proc/PID/mem` 范畴），只证明了门控存在且不满足时静默返回。

### 6.1 与仓库内 boot-package 路线的关系

仓库另有 [docs/rgxx-boot-package-120hz-review.md](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/porting/linkslot-1.0.45-feasibility/docs/rgxx-boot-package-120hz-review.md)，记录的是一条**完全不同的**路径：改 `boot_package.fex` 后刷到裸盘偏移 `0x1004000` 再重启，由 FDT 固定 DCLK 54 MHz / HT 840 / VT 536（约 119.94 Hz），运行期没有切换点。

两条路径可以用本次读数直接区分：

| 判据 | 120 Hz boot package | 本文档记录的 runtime 60 Hz | 本机实测 |
| --- | --- | --- | --- |
| FDT `lcd_dclk_freq` | 54 MHz | — | **26 MHz** |
| FDT `lcd_ht` / `lcd_vt` | 840 / 536 | — | **820 / 536** |
| PLL video0x4 | — | `0x88001703` | **`0x88001703`** |
| TCON divider | — | `0xf000000b` | 未读到（dump `I/O error`） |

值得注意：那份评审把 pvaibhav/slot 的 **60 Hz** 运行期配置记作 `0x88001703` + `0xf000000b`，与本次实测的 PLL 字逐位相符；divider 因 dump 不可读未能直接验证，但 288 MHz / 11 与 59.57 Hz 的观测一致。

结论：本机既没刷 120 Hz 的 boot package（FDT 仍是 26 MHz / 820×536），运行期也没离开 60 Hz 那一档（PLL 仍是 288 MHz）。两条路都停在 60 Hz 侧，与上一轮 IRQ 59.6 Hz 的实测吻合。

## 7. 合规

- 仅 ADB；`id` 已是 `uid=0(root)`，**未执行 `adb root`**。
- 全程只读：**未**读取 `/proc/PID/mem`，**未**创建 `trace-latency.on`，**未**设置 `SLOT_TRACE_LATENCY`，**未**重启或停止 Slot，**未**修改任何配置，**未**写任何寄存器，**未**注入按键。
- 未修改 Git 仓库源码；本报告目录为新增。
- 未上传二进制本体（ZIP 与两个 ELF 共约 21 MB 留在本地 `build/binary-identity/`），仓库只提交文本证据与哈希。

## 8. 复现

```sh
# 1) 官方包
curl -L -o slot-v1.16.0-pvaibhav.2-h700.zip \
  https://github.com/pvaibhav/slot/releases/download/v1.16.0-pvaibhav.2/slot-v1.16.0-pvaibhav.2-h700.zip
sha256sum slot-v1.16.0-pvaibhav.2-h700.zip

# 2) 设备文件
adb shell sha256sum /mnt/sdcard/System/slot /mnt/sdcard/SYSTEM/armgba_libretro.so
adb pull /mnt/sdcard/System/slot ./slot
adb pull /mnt/sdcard/SYSTEM/armgba_libretro.so ./armgba_libretro.so

# 3) ELF（本机无 readelf，用等价的 pyelftools 脚本）
python build/k21_elf.py ./armgba_libretro.so device-armgba.elf.txt
python build/k21_elf.py ./slot device-slot.elf.txt

# 4) 进程状态
adb shell 'tr "\000" "\n" < /proc/2301/environ | grep -E "^(SLOT_|BASEOS_)"'
adb shell 'readlink /proc/2301/fd/1; readlink /proc/2301/fd/2'
adb shell 'ls -l /mnt/sdcard/System/trace-latency.on'
```

Windows/MSYS 下 `adb push/pull` 需 `export MSYS2_ARG_CONV_EXCL='*'`，否则 `/tmp`、`/mnt` 会被改写成 Windows 路径。


## 2026-11-26 armGBA runtime mapping follow-up

The official-release armGBA core is now mapped while running in-game. The new [runtime-mapping report](slot-refresh-audit/binary-identity/runtime-mapping/SUMMARY.md) derives one load bias from three independent PT_LOAD-backed mappings and distinguishes the two local `gba` objects: `0x4259b8` is the primary libretro GBA pointer; `0x4c16a8` is the link-cable cache. The exact-source `video.frameCounter` offset is 11,948 bytes from the primary GBA object.

This is static/runtime-address closure only: no `/proc/PID/mem` read was performed and no UI/core frame-rate measurement is claimed. See `runtime-mapping/load-bias.json`, `armgba-pt-load.txt`, `armgba-symbol-refs.txt` and `armgba-maps.txt` for the raw evidence. Slot remained running; only the reversible A/Menu input events used to load and unload the libretro core were injected.
