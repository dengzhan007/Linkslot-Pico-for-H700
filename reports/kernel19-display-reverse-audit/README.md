# kernel #19 显示路径离线逆向审计

对 `docs/kernel19-display-reverse-task.md` 的响应。全部结论来自**离线** ELF 分析：没有连设备、没有读写任何 MMIO/sysfs/debugfs、没有重导 p4、没有上传 ELF 或原始 DTB。

| 项 | 值 |
| --- | --- |
| 目标 | RG34XX 非 SP / H700 / Linux 4.9.170 **#19** |
| 分析对象 | `build/rg34xx-kernel.elf` |
| ELF SHA-256 | `2f7f36a5f5442fdb9a33dc76f14c53c4626f42ed91edbc4b337d5641c2a9ed44` |
| GNU Build ID | `a1d211f847a5fb69fe9898e15fbccdd3ea8d7b68`（与 reports 记录一致） |
| 符号数 | 89 675（kallsyms，`st_size` 全为 0） |
| 产物 | `symbols.json`、`bounded-disassembly.txt`、`verification.json`、本文件 |

## 0. 身份核验

`.kernel` 段偏移 `0xc8fc60` 处的 note 头解析为 `namesz=4, descsz=20, type=3, name="GNU"`，desc 即上表 Build ID；`rg34xx-inventory-20261125-cpux-v4/kernel-driver-provenance.txt` 里 `/sys/kernel/notes` 的同样的 24 字节与之逐字节相同。`.kernel`+`.kernel2` 是已验证镜像 `39cf8c3e…`(17 461 256 B) 的前缀副本，所以分析对象的身份是闭合的。

**一个必须先说明的限制：** 该 ELF 里所有"指针型"数据字段都是 0（.kernel 与 .kernel2 全是如此），即数据段没有重定位。因此时钟/面板描述符只能通过**标量字段**或**代码里的立即数**读取，无法顺着指针遍历。下文凡是写"无法静态判定"的地方都是这个原因。

## 1. PLL `0x88001703` 的真实含义

代码里的解码函数是 `calc_rate_video0`（`0xffffff800845f3cc`），二进制原文：

```
mov  w2, #0x3600 ; movk w2, #0x16e, lsl #16   -> 0x016e3600 = 24 000 000
ldrh w2, [x1]        ; factors[0] = N
ldrh w1, [x1, #8]    ; factors[4] = D2
add  x1, x1, #1
madd x0, x2, x0, x0  ; parent*(N+1) + ... = (N+1)*parent
udiv x0, x0, x1      ; / (D2+1)
```

字段来自 `sunxi_clk_factors_recalc_rate` 对 `sunxi_clk_factor_pll_video0x4` 描述符的抽取：N 在 **bit[15:8]**（shift 8 / width 8），D2 在 **bit[1]**（shift 1 / width 1）。

于是：

| 字段 | 位 | 值 |
| --- | --- | --- |
| N | [15:8] | 23（`0x17`） |
| D2 | [1] | 1 |
| bit0 | [0] | 1（该 PLL 的解码函数不读） |
| bit27 | [27] | 1（含义未解） |
| bit31 | [31] | 1（按该 CCU 世代惯例是 enable，但 enable 位在描述符里、指针为 0，**未经二进制证实**） |

`24 MHz × 24 / 2 = **288 MHz**`，与同一次启动 `clk_summary` 的 `pll_video0x4 = tcon_lcd = 288 MHz` 吻合。

**一个必须记下来的不一致：** 镜像里的 `factor_pllvideo0x4_tbl`（`0xffffff800903d8d8`）是 `{寄存器字, 速率}` 对，覆盖 288–336 MHz（步进 6 MHz）；但这些字用 `calc_rate_video0` 求值恰好是表里速率的 **2 倍**（如 `0x2f03`：表 288 MHz，算得 576 MHz，9 条全部如此）。所以该表不能当作解码基准，本报告的 PLL 结论只用 `calc_rate_video0` + 实测 `clk_summary`；2 倍关系本身**未解释**，如实列为 open item。

## 2. `tcon_lcd` 的父时钟

实测证据（同一次启动）：`tcon_lcd = pll_video0x4 = 288 MHz`，`pll_video0 = 72 MHz`（后者是 `pll_video0x4` 的 /4 派生节点，不是 PLL 输出）。

代码侧的 mux/parent 选择**无法静态判定**：`tcon_lcd` 的父选择表在 `.kernel2`，其指针字段为 0。可确定的是**驱动不在运行时重选**：写 TCON `base+0x44` 的只有 `disp_al_lcd_cfg`（初始化路径）与 `disp_al_vdevice_cfg`（TV 路径），没有刷新率切换路径。

## 3. divider `0xf000000b` 如何编码

```
tcon0_set_dclk_div(tcon, div):
    val = readl(tcon_base + 0x44)
    bfxil val, div, #0, #7        ; 只替换 bits[6:0]
    writel(val, tcon_base + 0x44)
tcon0_get_dclk_div(tcon):
    return readl(tcon_base + 0x44) & 0x7f
```

写入是**逐位搬运**：没有 `+1`、没有 `-1`、没有 log2 编码，`11` 就是 11。高 4 位 `0xf` 由读-改-写保留，divider 路径不碰它，其含义未解。

**`11` 从哪来（本轮最有价值的发现）：** 它不是按像素时钟算的。`disp_al_lcd_get_clk_info`（`0xffffff8008434f94`）按 `cfg->lcd_type` 索引一张常量表（`0xffffff80089f6070`，5×u32/项，步长 0x14）：

| 表索引 | dclk_div |
| --- | --- |
| 0 | 6 |
| 1 | 12 |
| 2 | 7 |
| 3 | 4 |

随后有若干 override，其中**唯一能产出 11 的分支**是：索引 0 且 `[上下文+0x80] != 0x1a` 时把 divider 强制为 `0x0b`。

也就是说：**DT 里的 `lcd_dclk_freq` 根本没参与这个计算**，divider 是编译期常量 + SoC 版本判断的结果，`26 MHz` 只是声明值。

## 4. `rg34xx_v1 + lcd_type=old` 的初始化路径

- 面板名 `rg34xx_v1` 在镜像里的位置是 `0xffffff8008ffc580`，属于一张定长名字表（`0xffffff8008ffc4c0`，0x60 步长，邻居有 `FX070_DHM18BOEL2_1024X600`、`bp101wx1`）。
- 名字绑定走 `disp_lcd_set_panel_funs`（`0xffffff8008404028`）：先 `disp_sys_script_get_item` 取 `lcd_driver_name`，再绑定回调。
- 时序参数全部来自 DT：`lcd_dclk_freq / lcd_ht / lcd_vt / lcd_hbp / lcd_hspw / lcd_vbp / lcd_vspw` 都是镜像里的查表键。**镜像里没有任何静态时序表**——在 `.kernel2` 中搜不到 720/480 或 820/536 的整数序列，所以时序不可能来自内核。
- 调用顺序（`disp_al_lcd_cfg`）：`tcon_init` → `disp_al_lcd_get_clk_info` → `tcon0_set_dclk_div` → `tcon0_cfg` → `tcon0_cfg_ext` → `tcon0_src_select`。
- `lcd_type=old` 落到索引 0 这条链可由 divider 实测值反推，但"bootarg 字符串 → cfg 索引"的枚举映射是数据驱动的，无法从该 ELF 静态证实。

## 5. 是否存在 120 Hz timing / 模式表

**不存在。** 证据：

1. 镜像里没有 `high_refresh` / `refresh_rate` / `120hz` 之类字符串；
2. 没有第二套时序表（`.kernel2` 里连 720×480 / 820×536 的序列都没有）；
3. 唯一写 dclk divider 的路径是初始化，没有运行时切换函数；
4. PLL 侧可用速率与实测 288 MHz 都锁在 60 Hz 这一档。

要支持 120 Hz，至少缺：`rg34xx_v1` 的第二套 panel timing、受支持的父 PLL 速率、该速率下 SoC 接受的 divider、TE/VBlank 边界处理与回滚路径、以及面板能否接受倍频的独立依据。**结论：未验证 / 不支持启用。** 本报告不把"能算出约 120 Hz"当作面板支持证据。

## 6. 与 live 基线的对齐

| 项 | 值 |
| --- | --- |
| 父时钟 | 288 MHz（实测 `clk_summary`） |
| divider | 11 |
| 推算 DCLK | 26 181 818 Hz |
| HT×VT | 820×536 = 439 520 |
| 推算刷新 | **59.569 Hz** |
| DTB 声明 | 26 MHz → 59.155 Hz |
| IRQ 斜率 | 59.61–59.65 Hz（仅 scanout/vblank proxy，不是游戏 FPS，也不是物理刷新率） |

代码推导与实测在 288/11 这条链上一致；DTB 的 26 MHz 是取整声明值，与实际分频链差约 0.7%。

## 7. 未解决项

- PLL `bit27` 与 TCON 高 4 位的含义；
- 触发 `=11` override 的那个 SoC/版本字段具体是什么（上下文 `0xffffff800918c000` 指针为 0）；
- `tcon_lcd` 的 mux/parent 代码路径（目前只有运行时 `clk_summary` 证据）；
- `factor_pllvideo0x4_tbl` 与 `calc_rate_video0` 的 2 倍不一致；
- `lcd_type=old` → 索引 0 的枚举映射；
- DT `lcd_dclk_freq` 是否在别处被使用（本路径未使用）。

## 8. 复现

```sh
# 身份
python -c "from k19rev import Kernel; ..."   # 见 build/gen_k19_report.py
# 反汇编与符号
python build/gen_k19_report.py               # 重新生成 symbols.json / bounded-disassembly.txt
```

本目录四个文件（本文件、`symbols.json`、`bounded-disassembly.txt`、`verification.json`）的 SHA-256 见 `SHA256SUMS.txt`，校验命令：

```sh
sha256sum -c SHA256SUMS.txt
```

本报告只做归档与来源对照，不含可刷写固件、不含硬件写入代码，也不代表任何面板能力承诺。
