# RG34XX AArch64 探针：实机准入审计（第 5 轮，仅主机侧）

## 0. 结论摘要

| 项目 | 结论 |
| --- | --- |
| framebuffer probe 对象资格 | **具备**：可重复构建、header/入口/重定位/访存/唯一写入路径均通过本轮复核。 |
| FDT 解析安全性 | **已加固并通过 228 个 ASan/UBSan 用例**；包括 header、block 重叠、token、终止符、重复节点/属性、异常深度。 |
| watchdog 恢复链 | **未闭环**：BL31 内存在多组 WDT 写例程，正常启动路径的可达性无法完全排除；RTC pair 跨本机 watchdog reset 的保留性未实测；“whole-system reset” 语义无本机证据。 |
| 二次读出通道 | UART 时钟/divisor 未证明；WDT 只读观察缺少页表 Device mapping 证据。两条通道均未加入。 |
| 实机准入 | **NO-GO**。本轮不建议执行任何探针、复位或写设备操作。 |

本轮未接触设备，未写 SD 卡 / Boot0 / TOC1 / LBA16 / LBA512 / GPT / RTC，未执行任何 ARM payload，
未调用 SMC/HVC，未触发 watchdog reset，未修改 `main`，未构建可安装或可刷写镜像。
所有临时程序、反汇编、JSON 与报告均位于 `build/audit-work/readiness/` 及已有 `build/audit-work/probe-*/` 下。

---

## 1. 基线与证据来源

| 来源 | 版本 / 指纹 | 用途 |
| --- | --- | --- |
| Linkslot 仓库 | `reports` 远端基线 `2d99345` | 上一轮报告基线 |
| BaseOS 源码 | `cbe253a43f46cf551cc9a6c79a84015bde3686cf` | WDT、capsule、SPL、loader 源码 |
| live FDT | `build/audit-work/out/live-fdt-current.dtb` SHA-256 `c0c9c383…`；capture `live-fdt.dtb` SHA-256 `e1560a7f…` | watchdog / UART / FDT header 实测 |
| LBA16 capsule | `boot0-primary.bin` | dispatcher、内嵌 vendor Boot0、native SPL 反汇编 |
| p4 + 40 MiB | `boot-partition.bin@0x02800000`, `0x0283e200` | BL31 / A32 loader 反汇编 |
| TOC1 monitor | SHA-256 `676bc020…` | 与 p4 中 BL31 payload 字节一致 |
| 工具链 | zig 0.17.1 / clang 22.1.8 / lld；capstone 5.0.7 | 构建与反汇编 |

证据等级沿用前几轮：

* **源码证明**：由固定基线的源码和逐字节匹配的二进制共同支持；
* **本机实测**：从当前 RG34XX 的只读抓取中重新计算得到；
* **其他设备实测 / 推断**：BaseOS 实验记录中的硬件结果，但不是这台机器的证明；
* **UNKNOWN**：本机证据不足，明确不写成事实。

---

## 2. Watchdog 寄存器语义与证据来源

### 2.1 来自本机 live FDT 的节点

两个 live FDT 副本给出的节点完全一致：

```
/soc@03000000/watchdog@030090a0
  compatible = "allwinner,sun50i-wdt"
  reg        = 0x030090a0 .. 0x030090c0  (0x20 bytes)
  interrupts = GIC SPI 0x32 (50), level/high
```

这是本机 FDT 的实测结果，不是从其他 H616 设备 DTS 推断的。

### 2.2 对应驱动实现

`allwinner,sun50i-wdt` 在 T507_Kernel 的 `drivers/watchdog/sunxi_wdt.c` 中直接匹配
`sun6i_wdt_reg`（通过 GitHub API 只读取得，文件 SHA `5bf482d3…`）。关键定义：

```
static const struct sunxi_wdt_reg sun6i_wdt_reg = {
    .wdt_ctrl = 0x10, .wdt_cfg = 0x14, .wdt_mode = 0x18,
    .wdt_timeout_shift = 4, .wdt_reset_mask = 0x03, .wdt_reset_val = 0x01,
};
{ .compatible = "allwinner,sun50i-wdt", .data = &sun6i_wdt_reg }
```

配合节点基址 `0x030090a0`，得到任务给出的三个地址：

| 名称 | 地址 | 依据 |
| --- | --- | --- |
| `WDT_CTRL` | `0x030090b0` | base + 0x10 |
| `WDT_CFG` | `0x030090b4` | base + 0x14 |
| `WDT_MODE` | `0x030090b8` | base + 0x18 |

### 2.3 三个写入值的严格解释

| 值 | 解释 | 证据等级 |
| --- | --- | --- |
| `MODE = 0xb1` | bit0 = `WDT_MODE_EN`；bits[7:4] = `0xB`。驱动 `wdt_timeout_map[16] = 0xB`，所以按该驱动应解释为 **16 秒**。 | 源码证明；本机未测时基。不能写成“本机已证明 16 秒”。 |
| `CFG = 1` | 驱动把 `reset_mask=0x03`、`reset_val=0x01` 写入 CFG，注释为 *Set system reset function*。因此可说“选择了驱动定义的 system reset 功能”；**不能**进一步断言这就是文档意义上的 *whole-system reset*。 | 源码证明；“whole-system” 属 UNKNOWN。 |
| `CTRL = 0x14af` | 驱动中 `WDT_CTRL_RELOAD = (1 << 0) | (0x0a57 << 1)`，计算得 `0x14af`；即 reload/触发键。 | 源码证明。 |

“寄存器写入成功并读回正确”与“在本机确实产生复位”是两个不同命题。本机只完成前者；后者 UNKNOWN。

---

## 3. 完整启动路径中的 watchdog 写入

使用 capstone 对固定二进制做常量跟踪与反汇编，脚本为
`build/audit-work/readiness/wdt_scan.py`，输出 `out/wdt-scan.json` 与逐区域反汇编文本。

### 3.1 已定位的写入点

| 阶段 | 位置 | 写入 | 说明 |
| --- | --- | --- | --- |
| capsule dispatcher (A32) | literal `0x2072C` = `0x030090b4`；代码 `0x205A0`, `0x205BC`, `0x205D8` | `CFG=1`, `MODE=0xb1`, `CTRL=0x14af` | 首次布防；每一步都有 DSB + 读回校验（`0x205A8`–`0x205F8`）。 |
| native SPL (A64) | `0x31514`–`0x31538` | `CFG=1`, `MODE=1`, `CTRL=0x14af` 后 `b .` | 形似复位/自检例程；**未证明在正常启动路径上执行**。 |
| native SPL (A64) | `0x35780`–`0x3578C` | `CTRL=0x14af` | 源码对应 `native-boot.c:280`，在 RVBAR/RMR 交接前再次 reload；这是正常 native 路径上可见的**最后一次 WDT 写**。运行时地址约 `0x2478C`（源 `0x31000` → 目的 `0x20000`）。 |
| BL31 (A64) | `0x480072C4`, `0x480139D0`, `0x48013A0C`, `0x48013AC8`, `0x48013B48`, `0x48013B60` 等 | 多组 CFG/MODE/CTRL 写 | 包含 reset-like、start/stop、suspend/resume 例程。见下节可达性说明。 |
| A32 loader (ARM) | 0 处 | — | 无 WDT 写。 |
| Linux userspace `native-health` | `src/native-health.c:334` | `MODE=0` | 正常成功后清除 pending pair 并停止 watchdog；探针替换 kernel，因此这条路径在试跑中不会发生。 |

### 3.2 BL31 可达性：未完全闭环

BL31 的直接调用图中，从入口 `0x48000000` 到第一次 ERET（返回 A32 loader）的 1,843 个可达指令
没有触及上述 WDT 例程；A32 loader 也没有 WDT 写。SMC 向量 `VBAR_EL3+0x600` 的短路径同样未触及。

但是：

1. BL31 入口可达集合内仍有 **5 个间接分支**（`br`/`blr`），静态分析无法解析其全部目标；
2. A32 loader 会通过 `SMC #0` (`r0=0x8000ff04`) 请求 BL31 切换到 kernel，该 SMC 处理路径未被完整枚举；
3. 因此“BL31 在任何情况下都没有关闭 / 重配 / 持续 reload watchdog”不能被写成已证明事实，标记为 **UNKNOWN**。

### 3.3 watchdog reset → BROM → capsule → STOCK 状态机

```
                         +---------------------------------------------+
                         | BROM 读取 LBA16 的 128 KiB raw capsule        |
                         | dispatcher 检查 eGON header 与两个 FNV        |
                         +----------------------+----------------------+
                                                |
                RETENTION=0x070001f8 bit16 ? ----+
                     | yes                             | no
                     v                                 v
        STOCK_RESUME: hash(STOCK)      boot_plan(pair, 0, CAPSULE_COOKIE)
        -> FNV 0x8fb6d562 ?            /        |        \
        -> prepare_copy(STOCK)   empty+cookie  nonzero  cookie==0
                                  NATIVE      STOCK_RECOVERY  INVALID_GENERATION
                                     |             |              |
                              arm WDT + commit     +---- hash(STOCK) ---+
                              pair {cookie,BSP1}        -> prepare_copy(STOCK)
                              -> prepare_copy(NATIVE)
```

* `STOCK` 的 FNV-1a32(0x10000) 实测为 `0x8fb6d562`，与 dispatcher 编译常量一致；
  SHA-256 `b6edbf39…` 与 `compatibility.json@rg34xx.boot0_sha256` 一致。**源码证明 + 本机实测**。
* 非空 pair 与无效 cookie 都先经过 `STOCK_FNV` 校验，再复制内嵌 vendor Boot0。**源码证明**。
* pair 的写入顺序是 cookie 先、marker 后；清除顺序是 cookie 先、marker 后，
  撕裂状态仍落到 STOCK。**源码证明**，并有主机状态表测试佐证。
* watchdog 超时后是否重新进入 BROM / 再次读取 LBA16 raw capsule：BaseOS 设计文档和
  其他设备的恢复实验支持该模型，但**本机本轮没有复位实测**，标记 UNKNOWN。
* RTC GP14/GP15（实际地址 `0x07000138`/`0x0700013C`）以及 retention bit 是否能跨本次
  watchdog reset 保留：同样只有源码和其他设备实验支持，本机 UNKNOWN。

若恢复链不能保证“超时后回到能选择 STOCK 的 BROM/capsule”，那么探针停在 WFI 就等于把机器
交给一个未证明的恢复机制；这正是 NO-GO 的核心原因。

---

## 4. boot_plan / capsule 主机状态表测试

测试源码：`build/audit-work/readiness/boot_plan_test.c`，直接包含 BaseOS 固定基线的
`src/native-attempt.h`；编译与运行命令见 `out/build-commands.txt`。

| case | resume | pair | generation cookie | 期望 | 结果 |
| --- | --- | --- | --- | --- | --- |
| empty pair | 0 | `{0,0}` | `0xf7bed1c8` | NATIVE_ATTEMPT | PASS |
| cookie only | 0 | `{cookie,0}` | `cookie` | STOCK_RECOVERY | PASS |
| marker only | 0 | `{0,BSP1}` | `cookie` | STOCK_RECOVERY | PASS |
| full pending pair | 0 | `{cookie,BSP1}` | `cookie` | STOCK_RECOVERY | PASS |
| resume set, empty | 1 | `{0,0}` | `cookie` | STOCK_RESUME | PASS |
| resume set, pending | 1 | `{cookie,BSP1}` | `cookie` | STOCK_RESUME | PASS |
| invalid cookie | 0 | `{0,0}` | `0` | INVALID_GENERATION | PASS |

补充检查：空 pair 的 `attempt_commit` 依次写入 cookie→marker，共 2 次写、3 次 fence，读回完整
pending pair；非空 pair 的 `attempt_commit` 返回 `IO_REFUSED` 且零写入。全部 PASS。

---

## 5. FDT 解析安全加固与测试

### 5.1 loader arena 的源码依据

`experiments/h700-boot-speed/spl/native-handoff.h`：

```
#define BASEOS_NATIVE_FDT_ADDRESS 0x44000000u
#define BASEOS_NATIVE_FDT_CAPACITY 0x00100000u
```

`native/loader.c` 在 `0x44000000` 调用 `fdt_open_into`，容量上限 1 MiB，并以
`x0=0x44000000` 进入 kernel。live FDT `totalsize=0x23240`（143,936 B），远小于 arena。

### 5.2 probe 的改动

`probe_fb.c` 现在在**首次解引用 x0 之前**检查：

* `x0 ∈ [0x44000000, 0x44100000)`，且 `x0 + totalsize <= arena limit`，否则 `R_FDT_ARENA=14`；
* `magic`, `totalsize >= 64`, `version == 17`, `last_comp_version == 16`；
* `off_rsv/off_struct/off_strings/size_struct/size_strings` 的对齐、非重叠、不越界；
* reservation map 必须以全零 16 字节项终止（live tree 有两项有效 reserve：BL31 与 initrd）。
  **不得**只检查空 map，否则会错误拒绝本机有效树。

`fdt_walk()` 现在强制执行：

* 根节点名为空、只能有一个根、END 只能在根关闭后出现且必须是 struct block 最后一个 token；
* BEGIN_NODE/END_NODE 深度配对，深度上限 16；NOP 只能出现在树内；未知 token 拒绝；
* 每个 PROP 的 `nameoff` 必须在 strings block 内有 NUL 终止符，属性长度与对齐不得越过 struct block；
* 本 probe 消费的 `/chosen`、`/memory@…`、`/soc/…/disp@…` 节点及 `baseos,boot-display`、
  `baseos,native-boot`、`bootargs`、`boot_fb0`、`reg` 属性出现重复即拒绝（`R_FDT_DUPLICATE=15`）。

注意：arena 边界常量是**有意保留**的唯一 DRAM 物理常量，其作用正是防止伪造 x0；它不是
framebuffer 地址。静态扫描器已把 `0x44100000` 记为 `fdt arena limit` 的允许常量，其它 MMIO/DRAM
常量仍为零。

### 5.3 ASan/UBSan 测试矩阵

同一份 `probe_fb.c` 以 `-DPROBE_HOST -fsanitize=address,undefined` 编成 host harness，
对真实 FDT 和畸变体运行。脚本：`build/audit-work/readiness/fdt_safety_test.py`；
结果：`out/fdt-safety-results.json`。

| 类别 | 数量 | 结果 |
| --- | --- | --- |
| 真实 FDT 基线重复运行 | 5 | 全部 `R_OK`，像素数恒为 23,832，写入范围恒为 `0x7BF3565C`–`0x7BFD5EA0` |
| 定向畸形：header/version/size/offset/reserve | 11 | 全部在 framebuffer 写之前拒绝 |
| 定向畸形：token/root/END | 3 | 全部拒绝 |
| 定向畸形：nameoff/len/字符串终止/缺节点 | 6 | 全部拒绝，含此前会漏过的 `prop_nameoff_oob` |
| 定向畸形：重复节点/重复属性/深度 18 | 3 | 全部拒绝（重复 chosen → reason 15；另两项 → reason 12） |
| 确定性随机单 bit 变异 | 200 | 0 次 ASan/UBSan 报错；所有拒绝用例 framebuffer writes = 0 |
| 合计 | **228** | `verdict = pass`，`failures = 0` |

“拒绝时零写入”由 harness 的 `writes` 计数直接检查；接受用例还检查 `writes_outside == 0`。
硬件端 x0 地址无法证明时，本轮选择的是**收紧**校验，而不是放宽校验绕过。

---

## 6. WDT 只读观察与 UART 暂缓

### 6.1 WDT 只读状态显示：未加入

`native-health.c` 在 Linux 用户态通过 `/dev/mem` 以 `PROT_READ` 映射 `0x03009000`，但这不能证明
kernel-entry 时刻 probe 所处的页表已把该页映射为 Device。固定基线的 BL31 页表证据与本机页表
读回都没有覆盖 `0x03009000`。因此不增加 WDT_CFG / WDT_MODE 的只读显示；探针也不显示
“watchdog 启用/复位配置吻合”，避免把未证明状态显示成事实。

### 6.2 UART0 只读源码审计

| 项目 | 结果 |
| --- | --- |
| FDT 节点 | `/soc@03000000/uart@05000000` |
| compatible | `allwinner,sun50i-uart` |
| reg | `0x05000000`，长度 `0x400` |
| clocks | phandle `0x29` → `/clocks/uart0`（`allwinner,periph-clock`） |
| FDT 中的频率/divisor | **没有** `clock-frequency`；父时钟和分频未由 FDT 给出 |
| cmdline | `earlyprintk=sunxi-uart,0x05000000 console=ttyS0,115200` |
| kernel log（已有抓取值） | `uart0: ttyS0 at MMIO 0x5000000 ... base_baud = 1500000` |
| SPL / BL31 / loader | 模拟轨迹确认 SPL 向 `0x05000000` 写字符、读 `0x05000014`；BL31/loader 不改 UART。BaseOS 文档提到 `0x0300190c` 与 `0x05000000` 的 UART setup，但同时明确其 *function/callers and semantics remain unidentified*。 |

115200 只是 cmdline 参数；从现有代码不能推导出 probe 入口时 UART 时钟、_divisor_ 与引脚 mux 的
实际组合。故本轮不向 `0x05000000` 写任何字符，也不增加 UART 输出代码。

---

## 7. 重建、哈希与静态验证

| 产物 | 大小 | SHA-256 | 两次构建 | relocation |
| --- | --- | --- | --- | --- |
| `probe64-image.bin` | 12,288 B | `3cb5c4403b96d287a9aa6dc4629ea7703c7268667fdb9634bfbc7c83e077efa5` | 字节一致 | 0 |
| `probe_fb-image.bin` | 18,432 B | `d77c9ae6fb5f4a7d2296d8a3bd4007a891691844ecb3e611c064e82e15cfc956` | 字节一致 | 0 |

工具链：zig 0.17.1，clang 22.1.8，lld；capstone 5.0.7。完整命令见
`build/audit-work/readiness/out/build-commands.txt`。

| 检查 | probe64 | probe-fb |
| --- | --- | --- |
| header / entry branch / image size / magic | pass | pass |
| flags = 2（little-endian，4 KiB 页） | pass | pass |
| `smc` / `hvc` / `eret` | 0 / 0 / 0 | 0 / 0 / 0 |
| `adrp`/`adr` 目标 | 全部在 image 内 | 全部在 image 内 |
| store base / 栈范围 | 32 次访存，31 次静态解析，1 次动态解析；123 stores / 96 loads | 1,242 条指令；`sp` 位移 `-0x10`…`+0x190` |
| framebuffer 唯一 store | n/a | `fb_store` @ `0x400813A0`，恰好 1 条 |
| `fb_pixel` 守卫 | n/a | `0x40081324`，6 条范围守卫 + 1 次调用 |
| 拒绝路径零写入 | n/a | 9/9 原 mutation + 228 例 FDT 矩阵全部通过 |

`probe64` 与手工编码版的 144 条指令逐条比较结果保持不变：118 条字节相同，26 条差异全部归类
（寄存器别名、movz/movk 位宽、链接器松弛、不可达填充），其中 index 33 是汇编版修正手工版
`image_end` 值。行为解释执行 998 步，步数与手工版相同。

---

## 8. 实机准入结论与最短解除路径

### GO / NO-GO

**NO-GO。** 本轮不授权任何实机试跑，不执行探针，不触发 watchdog，不写设备。

原因不是 probe 对象本身：两个探针都通过了重建、静态与主机行为验证。阻塞的是“探针停住之后
机器如何安全回来”这条链，以及“写对 framebuffer 是否等于看得见”这条观察链。

### 阻塞项

1. watchdog 恢复链未闭环：BL31 间接分支/SMC 路径的 WDT 例程可达性未排除；
2. RTC pair 与 retention bit 是否跨本机 watchdog reset 保留，未在本机实测；
3. watchdog reset 是否重新进入 BROM → LBA16 raw capsule → STOCK，未在本机实测；
4. `CFG=1 = whole-system reset`、`MODE=0xb1 = 本机 16 秒` 只有驱动/源码语义，无本机硬件证明；
5. framebuffer 在 kernel-entry 时刻是否仍被 DE/TCON 扫描，未证明；
6. probe 以 WFI 停驻，没有自身退出路径；依赖上述未证明的 watchdog 恢复；
7. UART divisor/时钟未证明，WDT 只读页表映射未证明，因此没有第二读出通道。

### 最短解除路径（仍按“先主机、后只读、最后才实机”排序）

1. **主机**：完成 BL31 全路径（含间接调用与 SMC 向量）的 WDT 例程可达性分析；只要能证明
   正常 boot→kernel 路径不执行 stop/reconfig/reload，即可关闭阻塞项 1。
2. **文档/源码**：取得 H700/sun50iw9p1 的 WDT CFG/MODE/CTRL 位定义或等价驱动资料，确认
   `CFG=1` 的复位域与 `0xB` 的时钟源；关闭阻塞项 4。
3. **只读实机观察（另行授权）**：用现有 `baseos-regs` 路径读取 GP14/GP15、RTC1f8、WDT
   CFG/MODE，确认正常启动后的初值；这不构成本轮操作。
4. **受控恢复验证（另行授权）**：在保留原始 SD 卡内容的前提下，做一次可观测的 watchdog
   恢复演练，确认 BROM/capsule 再次进入、pair 保留、最终落到 hash 校验过的 STOCK；
   关闭阻塞项 2/3。
5. **显示可见性**：在同一演练中确认 DE/TCON 仍在扫描 `0x7BF24800`；若屏幕全黑，只能记为
   inconclusive，不能记为 probe 失败。
6. **二次通道**：只有先读出 CCU UART 时钟/divisor 并证明 115200，或证明 `0x03009000`
   在 kernel-entry 页表中是 Device mapping，才允许增加 UART 输出或 WDT 只读状态显示。
7. 以上全部关闭后，才重新评估 GO；任何一环仍为 UNKNOWN，结论继续是 NO-GO。

---

## 9. 附录：本轮新增主机侧文件

| 文件 | 用途 |
| --- | --- |
| `build/audit-work/readiness/wdt_scan.py` + `out/wdt-scan*.txt` / `out/wdt-scan.json` | 固件 WDT 写扫描 |
| `build/audit-work/readiness/boot_plan_test.c` + `out/boot_plan_test.exe` | capsule 分支状态表 |
| `build/audit-work/readiness/fdt_safety_test.py` + `out/fdt-cases/` / `out/fdt-safety-results.json` | ASan/UBSan FDT 矩阵 |
| `build/audit-work/readiness/cache/t507-sunxi_wdt.c` | 精确 compatible 对应驱动源码 |
| `build/audit-work/readiness/out/build-commands.txt` | 完整构建与验证命令 |
