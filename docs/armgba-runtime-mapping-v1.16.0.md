# armGBA v1.16.0 运行时映射与主指针裁决（RG34XX）

## 范围与安全边界

本记录对应设备上与官方 H700 v1.16.0-pvaibhav.2 ZIP 逐字节一致的 `armgba_libretro.so`。官方发布元数据将源码 pin 到 `MagnominusGit/armGBA@df6b2631291f249d54a00dd14f49bc893e438feb`；布局探针、运行时 ELF 和反汇编均以此版本为准。

本轮只读检查了进程 maps、ELF PT_LOAD、符号表和反汇编。为让核心在游戏态加载，按键注入触发进入游戏，再长按 Menu 退出；没有读取 `/proc/PID/mem`，没有建立 tracing marker、没有设置 `SLOT_TRACE_LATENCY`、没有重启或停止 Slot、没有修改配置、没有写硬件寄存器。报告数据与完整按键采集方法见 [reports/runtime-mapping](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/slot-refresh-audit/binary-identity/runtime-mapping)。

## 1. 精确二进制与源码布局

- 设备和官方 ZIP 内核心 SHA-256：`4ebae098cc4848fb459fc18299c39acbdbbbeb732db5c3f66d1ff3f6e142e889`。
- Build ID：`08309a288967a62cf4155e27d1b10ce13a5e33ef`；ELF AArch64 ET_DYN，保留完整 `.symtab`（4960 项），但 `.dynsym` 不导出 `gba`。
- 上游官方 source pin：`df6b2631291f249d54a00dd14f49bc893e438feb`。
- exact-source cross-ABI layout probe：`sizeof(GBA)=788712`、`offsetof(GBA,video)=11944`、`offsetof(Video,frameCounter)=4`，所以 `gba->video.frameCounter` 相对 `GBASystem*` 的偏移为 **11948 bytes**。Host、AArch32 ARM、AArch32 Thumb-2、AArch64 静态断言均通过；专用 CI step 通过，不将后续取消的整个 workflow 记作通过。

## 2. load bias 交叉验证

Slot 运行 PID 本次为 2301；核心只在游戏态 `dlopen`，退出游戏后 `dlclose`，进程本身不重启。ELF 有两个 PT_LOAD：

| PT_LOAD | flags | p_offset | p_vaddr | p_filesz | p_memsz |
| --- | --- | --- | --- | --- | --- |
| #0 | RX | `0x0` | `0x0` | `0x1756c6` | `0x1756c6` |
| #1 | RW | `0x1757a0` | `0x1857a0` | `0x34ab0` | `0x4d0810` |

按 Linux 4-KiB page 对齐后，对三条文件映射独立计算：

| maps 行 | offset | maps 起始地址 | 解出的 bias |
| --- | --- | --- | --- |
| `r-xp` | `0x000000` | `0x7f64070000` | `0x7f64070000` |
| `r--p` | `0x175000` | `0x7f641f5000` | `0x7f64070000` |
| `rw-p` | `0x178000` | `0x7f641f8000` | `0x7f64070000` |

三行独立结果完全一致，故本次装载的 load bias 为 **`0x7f64070000`**。PROT_NONE loader gap 不参与计算。该值会随进程重启或重新 dlopen/ASLR 变化，不能硬编码。

## 3. 两个同名 `gba` 符号的区分

完整符号表有两个 8-byte LOCAL OBJECT：

| ELF vaddr | 本次运行时地址 | 含义 |
| --- | --- | --- |
| `0x4259b8` | `0x7f644959b8` | **主 GBA 指针**，来自 `libretro/libretro.c` |
| `0x4c16a8` | `0x7f645316a8` | link cable 模块的文件局部缓存指针，来自 `libretro/link.c` |

裁决依据：

1. `loadAdvance.part.0` 将 `startAdvance()` 返回的新 `GBASystem*` 同时写入 `gbas[0]` 和 `gba@0x4259b8`。后者是活动核心指针。
2. `retro_run` 中 21 处引用通过 GOT 槽取出 `gba@0x4259b8`，随后将该指针交给 `gbaSetKeys`、`linkGbaFrame`、`dolphinGbaFrame` 和帧推进逻辑。
3. `linkSetGba` 是 `0x4c16a8` 的缓存写入者，其参数来自主指针；该地址的访问主要局限于联动链路。
4. `retro_unload_game` 先调用 `linkSetGba(NULL)` 清缓存，随后将主 `gba` 清零，符合下游缓存依附于主指针的生命周期。

因此，后续若进行任何有界读取，正确的指针变量地址是本次装载时的 `bias + 0x4259b8`，不是 `bias + 0x4c16a8`。

## 4. frameCounter 的精确语义

同一官方源码 commit 中：

- `src/video/video.c` 在 VBlank 的 `VISIBLE_LINES` 分支调用 `gbaFrameEnded(gba)` 后执行 `++video->frameCounter`。
- `libretro/libretro.c` 的 GBA 运行路径读取 `gba->video.frameCounter`，执行 `gbaRunPart(gba)`，直到计数器变化或联机暂停条件成立才离开循环。
- 所以该字段是 armGBA 核心已提交/完成的模拟器帧计数器，适合用相邻采样差分计算 committed frames/s；它不是 Slot UI presents/s，也不是 LCD IRQ/s。

精确源码：
- [video.c](https://github.com/MagnominusGit/armGBA/blob/df6b2631291f249d54a00dd14f49bc893e438feb/src/video/video.c#L119-L135)
- [gba.h](https://github.com/MagnominusGit/armGBA/blob/df6b2631291f249d54a00dd14f49bc893e438feb/src/gba.h#L276-L280)
- [libretro.c](https://github.com/MagnominusGit/armGBA/blob/df6b2631291f249d54a00dd14f49bc893e438feb/libretro/libretro.c#L4879-L4889)
- [link.T](https://github.com/MagnominusGit/armGBA/blob/df6b2631291f249d54a00dd14f49bc893e438feb/libretro/link.T#L1-L4)

## 5. 未完成的动态测量

本轮**没有**解引用 `0x7f644959b8`，也没有读取其后 11948-byte 的计数器位置。两个指针变量在运行时是否数值相等仍未直接读取验证；这不影响静态调用图已确定主指针身份。

下一阶段仍需用户明确授权以下设备操作后才能进行：

- UI presents：在 Slot 启动前启用 `System/trace-latency.on`（或环境变量），并重启 Slot 以让 trace gate 初始化；日志目标已确认是 `/mnt/sdcard/Logs/slot.log`。
- armGBA frames：在游戏态对当前 PID 进行严格有界、只读的指针/计数器采样。每次都须重新取当前 PID 与 maps 并重算 load bias，按主符号解引用一次，再加 11948 bytes；不进行扫描、不写入进程内存。

这些操作尚未执行。所有硬件刷新率 writer 仍禁用，`hardware_applied=0`。指标必须单独报告：`slot_presents_per_s`、`armgba_committed_frames_per_s`、`lcd_irq_per_s`。
