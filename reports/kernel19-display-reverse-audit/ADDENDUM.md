# kernel #19 显示路径逆向审计 · 补充报告（ADDENDUM）

对 `docs/kernel19-display-reverse-followup-task.md` 的响应。本文**不覆盖**首轮证据：`README.md`、`symbols.json`、`bounded-disassembly.txt` 逐字节未改（哈希与首轮一致），更正以"首轮写法 → 更正"逐条列在 §11。

全部结论仍为**离线**静态分析：没有连设备、没有读写 MMIO/sysfs/debugfs、没有 `devmem`、没有 p4 重导、没有上传 ELF 或原始 DTB。

| 项 | 值 |
| --- | --- |
| 分析对象 | `build/rg34xx-kernel.elf`（21 534 197 B） |
| ELF SHA-256 | `2f7f36a5f5442fdb9a33dc76f14c53c4626f42ed91edbc4b337d5641c2a9ed44` |
| GNU Build ID | `a1d211f847a5fb69fe9898e15fbccdd3ea8d7b68`（命中，见 §1.1） |
| 时序基线 | `live-fdt.dtb` sha256 `cbb0bf254f366a864a34c562602674813c6432832626f5005bab998494c1f7cc`（cpux-v4 同批采集） |
| 新增工具 | `build/k19f_*.py`（镜像视图、反汇编、引用扫描、DT 键解析、面板表、自检） |

**一句话结论：首轮 §6 的对账数值全部成立，但它给出的机制有三处错——`[x19+0x80]` 不是 SoC 版本字而是 `lcd_dclk_freq`，索引字段是 `lcd_if` 不是 `lcd_type`，override 的方向被写反了。改对之后，"26 MHz → divider 11 → 288 MHz → 59.57 Hz"这条链第一次真正闭合，且 PLL 为何恰好落在 288 MHz 也解释清楚了。**

---

## 1. 方法学更正：镜像处于"未重定位"状态

首轮最关键的限制说明是错的，而这条错误限制了首轮全部结论的深度。

**首轮写法（§0）：** "该 ELF 里所有'指针型'数据字段都是 0，即数据段没有重定位。因此时钟/面板描述符只能通过标量字段或代码里的立即数读取，无法顺着指针遍历。"

**实际：指针值不在数据段里，而在 `.rela.dyn` 里。**

| 证据 | 数值 |
| --- | --- |
| `.rela.dyn` 条目数 | **91 760**，全部 `R_AARCH64_RELATIVE`（1027），`r_info` 符号索引全为 0 |
| 这 91 760 个 `r_offset` 处的现场 8 字节 | **100% 为 0**（未应用） |
| addend 落在 `.kernel`/`.kernel2` 内 | 81 874 / 91 760 = **89.2%** |
| addend 精确等于某符号起始地址 | 44 247 / 91 760 = **48.2%** |
| 抽样 | `0xffffff8008cba068 → saved_command_line`、`0x…cb6878 → loops_per_jiffy`、`0x…978338 → _cpu_resume` |

这是一个**链接完成、重定位未应用**的镜像。按 ELF 装载语义把 `r_addend` 覆盖到 `r_offset`，指针表就全部可读。本报告的新增结论都建立在这个覆盖视图上（`build/k19f_img.py` 的 `p64()`；**不落盘任何改写后的镜像**）。

> 首轮"无法顺着指针遍历"不是保守，是误判；它直接导致 §6.2（面板回调表）和 §7.3（时钟父表）被判为不可解。

### 1.1 Build ID 的精确坐标（首轮偏移量不准）

| 项 | 值 |
| --- | --- |
| note 头文件偏移 | `0x00c8fe90` |
| desc 文件偏移 | `0x00c8fea0` |
| note VA | `0xffffff8008d0fc50`（`.kernel`） |
| `namesz/descsz/type` | 4 / 20 / 3 |
| desc | `a1d211f847a5fb69fe9898e15fbccdd3ea8d7b68` ✔ |

首轮写的段偏移 `0xc8fc60` 差 0x230，无法复现；Build ID 值本身不变。另有第二个 note（文件偏移 `0x009084f8`，desc `fe7e68cd…`），与本机无关。

---

## 2. 更正 1：`[x19+0x80]` 是 `panel_info.lcd_dclk_freq`

首轮把它叫"SoC/版本字"，理由是"比较常量是 `0x1a`"。这个推理不成立。现用**结构体构造侧**直接证实。

### 2.1 第三参数确实是 `panel_info`

```
disp_al_lcd_get_clk_info(sel, struct disp_lcd_clk_info *clk, struct disp_panel_para *cfg)
0xffffff8008434fdc  mov  x19, x2        ; 之后所有 [x19+…] 都指向第三参数
```

三条独立证据闭合：

1. **访问器** `disp_lcd_get_panel_info`（`0x8406820`）：`__memcpy(out, priv + 0xf0, 0x120)`。
2. **构造侧** `disp_lcd_init`（`0x8404214`）：`0x84046e0` 处 `add x21, x19, 0xf0`（`x19 = disp_lcd_get_priv()`），随后 `__memset(x21, 0, 0x120)`，再从 DT 逐键填入。
3. **调用侧**：`disp_lcd_enable` 在 `0x840792c`、`disp_lcd_fake_enable` 在 `0x840763c` 都是 `add x1, x19, 0xf0` 后调 `disp_al_lcd_cfg`；`disp_al_lcd_cfg` 在 `0x8435140` 用 `mov x2, x23`（自己的第 1 参）传给 `disp_al_lcd_get_clk_info`。

所以第三参数 = `struct disp_panel_para`（位于 `disp_lcd_private + 0xf0`，大小 `0x120`），不是任何 SoC 寄存器或版本对象。

### 2.2 DT 键 → `disp_panel_para` 偏移（完整映射，从二进制恢复）

`disp_lcd_init` 的解析模式固定为 `adrp/add x1,<键串>` → `disp_sys_script_get_item(sel, 键, &tmp, 1)` → `cmp w0,#1` → `str w0,[panel_info,#偏移]`。逐条提取，共 53 条：

| DT 键 | 偏移 | DT 键 | 偏移 |
| --- | --- | --- | --- |
| `lcd_if` | `0x00` | `lcd_tcon_mode` | **`0x30`** |
| `lcd_hv_if` | `0x04` | `lcd_slave_stop_pos` | `0x34` |
| `lcd_hv_srgb_seq` | `0x08` | `lcd_sync_pixel_num` | `0x38` |
| `lcd_hv_syuv_seq` | `0x0c` | `lcd_sync_line_num` | `0x3c` |
| `lcd_hv_syuv_fdly` | `0x10` | `lcd_slave_tcon_num` | `0x40` |
| `lcd_lvds_if` | `0x14` | `lcd_cpu_mode` | `0x44` |
| `lcd_lvds_colordepth` | `0x18` | `lcd_dsi_if` | **`0x48`** |
| `lcd_lvds_mode` | `0x1c` | `lcd_dsi_lane` | `0x4c` |
| `lcd_lvds_io_polarity` | `0x20` | `lcd_dsi_format` | `0x50` |
| `lcd_cpu_if` | `0x24` | `lcd_dsi_eotp` | `0x54` |
| `lcd_cpu_te` | `0x28` | `lcd_dsi_te` | `0x5c` |
| `lcd_dsi_port_num` | `0x2c` | `lcd_tcon_en_odd_even_div` | `0x60` |
| `lcd_fsync_en` | `0x70` | `lcd_dclk_freq` | **`0x80`** |
| `lcd_fsync_act_time` | `0x74` | `lcd_x` | `0x84` |
| `lcd_fsync_dis_time` | `0x78` | `lcd_y` | `0x88` |
| `lcd_fsync_pol` | `0x7c` | `lcd_width` | `0x8c` |
| `lcd_height` | `0x90` | `lcd_xtal_freq` | `0x94` |
| `lcd_pwm_used` | `0x98` | `lcd_pwm_ch` | `0x9c` |
| `lcd_pwm_freq` | `0xa0` | `lcd_pwm_pol` | `0xa4` |
| `lcd_rb_swap` | `0xa8` | `lcd_vt` | `0xb0` |
| `lcd_ht` | `0xb4` | `lcd_vbp` | `0xb8` |
| `lcd_hbp` | `0xbc` | `lcd_vspw` | `0xc0` |
| `lcd_hspw` | `0xc4` | `lcd_hv_clk_phase` | `0xcc` |
| `lcd_hv_sync_polarity` | `0xd0` | `lcd_frm` | `0xd4` |
| `lcd_gamma_en` | `0xd8` | `lcd_cmap_en` | `0xdc` |

**关键点：`0x80` 紧跟 `lcd_dclk_freq` 的取键调用，中间没有其它键**——`0x84047a8` 取键、`0x84047cc` 执行 `str w0,[x21,#0x80]`。所以 `panel_info+0x80` 就是 `lcd_dclk_freq`（MHz），`0x1a` 就是 26。

### 2.3 四个被点名的偏移

| 偏移 | 身份 | 依据 |
| --- | --- | --- |
| `0x30` | **`lcd_tcon_mode`** | `0x8404c08` 的 `str w0,[x21,#0x30]` 紧跟该键 |
| `0x48` | **`lcd_dsi_if`** | `0x8404b14` 的 `str w0,[x21,#0x48]` 紧跟该键 |
| `0x80` | **`lcd_dclk_freq`** | 见上 |
| `0x118` | **未解（本 profile 的 dead field）** | 见 §2.4 |

### 2.4 `0x118` 为什么是未解

结构体大小恰为 `0x120`，`0x118` 是最后一个 u32。全镜像检索：

- `disp_lcd_init` 的 53 条键没有一条写 `0x118`；镜像里也不存在 `lcd_dclk_div` 之类的键（`lcd_dclk` 前缀只有 `lcd_dclk_freq` 一处）。
- 全 `.kernel`/`.kernel2` 中以 `#0x118` 为位移、落在显示代码里的存储点全部是 `ldr`；`#0x208`（= `priv+0xf0+0x118`）的存储点只有 `disp_init_hdmi` 与普通栈访问。

它唯一的读者是 override 1：

```
0x5068  mov  x0, 0xc00000000
0x506c  ldr  x5, [x19, #0]        ; 64 位读 panel_info[0..7] = {lcd_if, lcd_hv_if}
0x5070  cmp  x5, x0               ; 要求 lcd_if == 0 且 lcd_hv_if == 12
0x507c  ldr  w5, [x19, #280]      ; panel_info[0x118]
0x5080  cbz  w5, …                ; 为 0 则跳过
0x5084  mov  w1, w5               ; 否则把它直接当 divider
```

基线 DT `lcd_if=0`、`lcd_hv_if=0`，64 位值 ≠ `0xc00000000`，**本机不可达**。故 `0x118` 既无写入者、也不参与运算，语义标记为未解（dead field），不再猜测。

---

## 3. `disp_al_lcd_get_clk_info` 的准确语义

函数体逐句还原（`0xffffff8008434f94`）：

```
clk->[0x00..0x0f] = 0; clk->[0x10] = 0
idx = 0/1/2/3  for cfg->lcd_if == 0/1/3/4
      else  ->  w1=6, w2=4, w3=0, w4=1 (并打印 "lcd if is not …")
entry = TAB + idx * 0x14
w1 = entry[+4]   // dclk divider
w4 = entry[+8]   // 恒 1
w2 = entry[+12]  // 恒 1
w3 = entry[+16]  // 恒 0

if (cfg->lcd_if == 0 && cfg->lcd_hv_if == 12 && cfg->[0x118] != 0)
        w1 = cfg->[0x118];                                  // override 1（本机不可达）
else if (cfg->lcd_tcon_mode == 4 && cfg->lcd_if == 4)       // override 2
        { w1 /= 2; w2 /= 2; }
if (cfg->lcd_if == 4) {
        if (cfg->lcd_dsi_if == 1) w1 = w2 = 6;              // override 3
} else if (cfg->lcd_if == 0) {
        if (cfg->lcd_dclk_freq == 26) w1 = 11;              // override 4（关键）
}
clk->[+4] = w1; clk->[+8] = w4; clk->[+12] = w2; clk->[+16] = w3;
```

override 4 的机器码（`0xffffff80084350d4`–`50e0`）：

```
ldr  w0, [x19, #0x80]      ; w0 = lcd_dclk_freq
cmp  w0, 0x1a              ; 与 26 比较
mov  w0, 0xb               ; 常量 11
csel w1, w1, w0, ne        ; NE（freq != 26）→ 保留表值；EQ（freq == 26）→ 取 11
```

`csel` 的方向是首轮**写反**的地方（首轮写"`!= 0x1a` 时强制 0x0b"）。正确读法是：**恰好 26 MHz 才取 11，其它频率用表值。**

三处与首轮不同：

1. 索引的是 **`lcd_if`**（`panel_info+0x00`），不是 `lcd_type`。表项第 0 个字段正好等于它自己服务的 `lcd_if` 码（0/1/3/4），可作自校验。
2. override 4 的条件是 **`lcd_dclk_freq == 26`**，不是"SoC 版本判断"。
3. `csel` 条件为 `ne`，方向与首轮相反。

常量表 `0xffffff80089f6070`（stride `0x14`，5×u32）实测：

| idx | 自描述码 | divider | [8] | [12] | [16] |
| --- | --- | --- | --- | --- | --- |
| 0 | 0 | **6** | 1 | 1 | 0 |
| 1 | 1 | **12** | 1 | 1 | 0 |
| 2 | 3 | **7** | 1 | 1 | 0 |
| 3 | 4 | **4** | 1 | 4 | 0 |

### 3.1 代入基线 DT：divider = 11，整条链闭合

基线 DT：`lcd_if=0`、`lcd_hv_if=0`、`lcd_dclk_freq=26`、无 `lcd_tcon_mode`、无 `lcd_dsi_if`。

```
idx = 0                      → w1 = 6
override 1: lcd_hv_if != 12  → 跳过
override 2/3: lcd_if == 0    → 跳过
override 4: 26 == 26         → w1 = 11
⇒ divider = 11
```

由此，四个独立数字全部对上，且互相锁定：

| 环节 | 计算 | 结果 | 与实测比对 |
| --- | --- | --- | --- |
| TCON `base+0x44` | 低 7 位 = 11（本函数）；高 4 位 = `0xf`（`tcon0_open`，见 §5） | `0xf000000b` | **与首轮寄存器原值逐位一致** |
| PLL 目标速率 | `lcd_dclk_freq × divider` = 26 × 11 | **286 MHz** | `factor_pllvideo0x4_tbl` 最低项是 288 MHz（步进 12），上取整到 288 → **与 `clk_summary` 的 288 MHz 一致** |
| 实际 DCLK | `clk_get_rate(tcon_clk) / divider` = 288 / 11 | **26.1818 MHz** | — |
| 刷新率 | `ht × vt × 1e9 / dclk` = 820 × 536 × 1e9 / 26.1818e6 | **59.5691 Hz** | IRQ 扫描斜率 59.61–59.65（偏差 0.14%），与首轮一致 |

**所以首轮 §6 的对账表是对的，只是理由错了。** 现在这条链不再有"为什么是 11"的空白：11 是内核为 26 MHz 这一档硬编码的特例，因为 PLL 地板是 288 MHz，288/11 = 26.18 是可达的最接近值；而"288 而不是 286"则由 PLL 速率表的 12 MHz 步进决定。

一个附带的强证据：把 PLL 速率表在 288 MHz 那一档的配置字 `0x00001702` 与实际读到的 `0x88001703` 相减，差值恰好是 `0x88000001` = **bit31 + bit27 + bit0**。即：

- bit31 = 使能（与 CCU 惯例一致，且是差值里唯一可能是使能的位）；
- bit27 = 描述符 `+0x0f = 27` 声明的那个位号（见 §8），属状态/lock 类，不参与分频；
- bit0 = 仍未解释。

---

## 4. 更正 2：`factor_pllvideo0x4_tbl` 的"2 倍不一致"不存在

首轮 §1 记了一条 open item：表里 `{寄存器字, 速率}` 对，用 `calc_rate_video0` 求值恰好是表速率的 2 倍，9 条全部如此。

直接读表（`0xffffff800903d8d8`）：

| idx | 寄存器字 | 表速率 | 代入 `24 MHz × (N+1)/(D2+1)` |
| --- | --- | --- | --- |
| 0 | `0x00001702` | 288 000 000 | N=23, D2=1 → **288 MHz** ✔ |
| 1 | `0x00001802` | 300 000 000 | N=24, D2=1 → **300 MHz** ✔ |
| 2 | `0x00001902` | 312 000 000 | N=25, D2=1 → **312 MHz** ✔ |
| … | 步进 12 MHz | | |
| 186 | `0x00006800` | 2 520 000 000 | N=104, D2=0 → **2520 MHz** ✔ |

> **2026-11-25 更正**：本行原写作"尾部 `0x6400`：N=100, D2=0 → 2424 MHz ✔"，与同一段落"末项 2520 MHz"自相矛盾。第三轮逐行导出后确认 `0x00006400` 是**索引 178 的中间项**（2424 MHz），真正的末项是索引 186 的 `0x00006800`。详见 `PLL-TABLE.md`。

**表与解码函数逐条自洽，不存在 2 倍关系。** 表实际范围 288–2520 MHz、步进 12 MHz、共 187 项（`(0x903deb0 − 0x903d8d8)/8`），不是首轮写的"288–336 MHz、9 条"。

首轮举的例子 `0x2f03`（N=47 → 576 MHz）不在这张表里：本表 bit0 恒 0；`D2` 位在前 14 行恒为 1、之后交替（共 87 行 `D2=0`），所以低半字节有 `0x02`/`0x00` 两种形态，不能只看一种。

结论：**2 倍之谜撤销**，该表可以当解码基准使用。

---

## 5. 更正 3：TCON `base+0x44` 的高 4 位 `0xf` 有明确来源

首轮把 `0xf000000b` 的高半字节列为"读-改-写保留，含义未解"，并称"写 `base+0x44` 的只有 `disp_al_lcd_cfg` 与 `disp_al_vdevice_cfg`"。

两条都不准。`tcon0_open(sel, panel_info)`（`0x8436fd8`）：

```
0x8437024  ldr  w1, [x0, #0x44]      ; x0 = *(0xffffff800918c838) = tcon 基址
0x8437028  orr  w1, w1, 0xf0000000   ; 置高 4 位
0x843702c  str  w1, [x0, #0x44]      ; 写回
0x8437030  ldr  w1, [x0, #0x40]
0x8437034  orr  w1, w1, 0x80000000   ; TCON 使能
0x8437060  bl   tcon_irq_enable
```

即 `0xf` 是 `tcon0_open` 主动 OR 进去的。写 `+0x44` 的函数共 5 处：`tcon0_set_dclk_div`（只动 bits[6:0]）、`tcon0_open`（×2，只动 bits[31:28]）、`tcon0_close`、`tcon_exit`。

因此 `0xf000000b` 的构成是：高半字节来自 `tcon0_open`，低 7 位的 `0xb` 来自 `tcon0_set_dclk_div`。**两个来源都已定位，不是"含义未解"。**

---

## 6. `rg34xx_v1 + lcd_type=old` 的完整面板生命周期

### 6.1 面板驱动是怎么被选中的

`lcd_init`（`0x841a468`）遍历指针数组 `0xffffff8008ffbef0`，对每项调 `sunxi_lcd_set_panel_funs(panel, panel + 0x20)`。共 17 项：

| idx | 符号 | `.name` |
| --- | --- | --- |
| 0 | `default_panel` | `default_lcd` |
| 1 | `lt070me05000_panel` | `lt070me05000` |
| 2 | `wtq05027d01_panel` | `wtq05027d01` |
| 3 | `t27p06_panel` | `t27p06` |
| 4 | `dx0960be40a1_panel` | `dx0960be40a1` |
| 5 | `tft720x1280_panel` | `tft720x1280` |
| 6 | `S6D7AA0X01_panel` | `S6D7AA0X01` |
| 7 | `gg1p4062utsw_panel` | `gg1p4062utsw` |
| 8 | `ls029b3sx02_panel` | `ls029b3sx02` |
| 9 | `he0801a068_panel` | `he0801a068` |
| 10 | `inet_dsi_panel` | `inet_dsi_panel` |
| 11 | `lq101r1sx03_panel` | `lq101r1sx03` |
| 12 | `WilliamLcd_panel` | `WilliamLcd` |
| 13 | `fx070_panel` | `fx070` |
| 14 | `FX070_DHM18BOEL2_1024X600_mipi_panel` | `FX070_DHM18BOEL2_1024X600` |
| 15 | `bp101wx1_panel` | `bp101wx1` |
| **16** | **`fog_fj035fhd05_v1_panel`** | **`rg34xx_v1`** |

**要点：`rg34xx_v1` 的符号名是 `fog_fj035fhd05_v1_panel`。** 这是把 vendor 的 `fog_fj035fhd05_v1` 驱动改名复用——`.name` 改成 `rg34xx_v1`，函数体保留原符号名。这解释了镜像里成对出现的 `LCD_*` 实现（`0x841bxxx` 与 `0x8424xxx` 两族），也说明 `rg34xx_v1` 不是独立编写的面板驱动。

选择机制在 `disp_lcd_set_panel_funs`（`0x8404028`）：`sprintf("lcd%d", sel)` → `disp_sys_script_get_item(…, "lcd_driver_name", …)` → 对 17 项逐个 `strcmp` → 命中后 `__memcpy` 把 `panel+0x20` 起的回调块拷进 disp 全局。

### 6.2 `rg34xx_v1` 的回调块（`0x8ffc580`，struct stride `0x60`）

| 偏移 | 内容 | 目标 |
| --- | --- | --- |
| `+0x00` | `.name` | `"rg34xx_v1"` |
| `+0x10`/`+0x18` | 无重定位，值 0 | — |
| `+0x20` | `cfg_panel_info` | `0x8424294` |
| `+0x28` | `open_flow` | `0x84240b0` |
| `+0x30` | `close_flow` | `0x8424018` |
| `+0x38` | `user_defined_func` | `0x84220e0` |
| `+0x40` 起 | 无（无 `set_bright`） | — |

（这些槽在文件里读出来全是 0——只有重定位表里有值。这就是首轮"指针全为 0"的表象来源。）

### 6.3 上电顺序（`open_flow`，`0x84240b0`）

`open_flow` 只做一件事：用 `LCD_OPEN_FUNC(sel, fn, delay_ms)` 注册 4 个步骤。

| # | 回调 | delay | 实际动作 |
| --- | --- | --- | --- |
| 1 | `LCD_power_on` `0x8424144` | 30 ms | `ioremap(0x0300b124)`/`ioremap(0x0300b130)` 后直接写 PIO 寄存器（`orr 0x111` / `orr 0x8000`），`sunxi_lcd_power_enable(sel,0)`，`sunxi_lcd_pin_cfg(sel,1)` |
| 2 | `LCD_panel_init` `0x8422bb8` | 50 ms | 1294 条指令的固定序列：重复 `ioremap(0x0300b130)` + 读改写 PIO + `sunxi_lcd_delay_ms(15)`，末段 `iounmap`。**纯 GPIO 位操作，不含任何时序参数** |
| 3 | `sunxi_lcd_tcon_enable` `0x841a530` | 100 ms | TCON 输出使能 |
| 4 | `LCD_bl_open` `0x842411c` | 0 | `sunxi_lcd_pwm_enable(sel)` → `sunxi_lcd_backlight_enable(sel)` |

`LCD_power_on` 里两个 `ioremap` 的 pin 配置变体由 `cpu_hwcaps & 0x10000` 选择（`0xe80707` / `0xe80f07`）；两个映射在 `close_flow` 末尾 `iounmap`。

### 6.4 关电顺序（`close_flow`，`0x8424018`）

| # | 回调 | delay |
| --- | --- | --- |
| 1 | `LCD_bl_close` `0x8423ff0`（`sunxi_lcd_backlight_disable` → `sunxi_lcd_pwm_disable`） | 0 |
| 2 | `LCD_panel_exit` `0x8422654` | 200 ms |
| 3 | `sunxi_lcd_tcon_disable` `0x841a578` | 0 |
| 4 | `LCD_power_off` `0x84220e8` | 500 ms |

之后 `iounmap` 两个 GPIO 映射并清零槽位。

### 6.5 软件使能主链（`disp_lcd_enable`，`0x8407740`）

```
disp_lcd_get_priv
mutex_lock
disp_lcd_is_enabled              ← 已开则早退
disp_lcd_cal_fps
disp_sys_power_enable
disp_sys_register_irq → disp_sys_enable_irq
disp_lcd_gpio_init
lcd_clk_enable                   ← 父时钟 + 速率（§7）
cal_real_frame_period            ← 帧周期（§9）
disp_sys_pwm_set_polarity
disp_al_lcd_cfg                  ← tcon_init → get_clk_info → set_dclk_div → tcon0_cfg → tcon0_cfg_ext → tcon0_src_select
lcd_calc_judge_line
disp_delay_ms
disp_lcd_get_bright → disp_lcd_set_bright
mutex_unlock
```

`disp_lcd_enable` / `disp_lcd_fake_enable` 没有直接 BL 调用者，由 disp ops 表间接分派；`fake_enable` 的差异仅在跳过 `lcd_clk_enable`/`cal_real_frame_period`，两者传给 `disp_al_lcd_cfg` 的仍是同一个 `priv+0xf0`。

### 6.6 调用图里缺失的边（明确标为未解）

| 缺失边 | 原因 |
| --- | --- |
| `open_flow` 注册的 4 步由谁执行 | `LCD_OPEN_FUNC` 只把 `{fn, delay}` 追加进每 disp 的步进表；执行者是间接调用（`blr`），目标来自运行时填好的表 |
| `priv+0x1978/0x1980/0x1988/0x1990` 四个 `struct clk *` 从哪来 | 全镜像没有对这四个位移的 `str`；也不是按名字 `clk_get("tcon_lcd")`（该字符串在代码里 0 处引用），只能是 `clk_get(dev, clock-names)` 之类的 DT 绑定 |
| `LCD_power_on` 里 `0xffffff800912b4d0` 的决定位 | 该地址在 `.bss`（`cpu_hwcaps`），运行时值 |

---

## 7. 时钟：Linux 继承父时钟，只重设速率

### 7.1 父时钟是"抓下来再原样写回"

`disp_lcd_init` 尾部（`0x8405128`）：

```
x0 = priv[0x1978]; priv[0x1998] = clk_get_parent(x0)
x0 = priv[0x1980]; priv[0x19b0] = clk_get_parent(x0)
x0 = priv[0x1988]; priv[0x19a0] = clk_get_parent(x0)
x0 = priv[0x1990]; priv[0x19a8] = clk_get_parent(x0)
```

`lcd_clk_enable`（`0x84055dc`）随后把这几对原样用回去：

```
clk_set_parent(priv[0x1978], priv[0x1998])
clk_set_parent(priv[0x1980], priv[0x19b0])
clk_set_parent(priv[0x1988], priv[0x19a0])
clk_set_parent(priv[0x1990], priv[0x19a8])
```

**结论（可证）：父时钟是 bootloader/DT 选定的那一个；Linux 在 init 时用 `clk_get_parent` 记下，在 enable 时用 `clk_set_parent` 写回同一个值。** 既不是"运行时重新选父时钟"，也不是"完全不管父时钟"。

### 7.2 速率由驱动算出并设下去

同函数内（`0x56b4`–`0x577c`）：

```
dclk_hz = priv[0x170] * 1000000            // 0x170 = panel_info+0x80 = lcd_dclk_freq
x21     = clk_info[+8] * (clk_info[+4] * dclk_hz)     // = 1 * (divider × dclk)
clk_set_rate(priv[0x1998], x21)            // 设的是“父时钟”（PLL）
x23     = clk_get_rate(priv[0x1998])
if (clk_info[+8]) clk_set_rate(priv[0x1978], x23 / clk_info[+8])
clk_get_rate(priv[0x1978])
```

基线代入：26 MHz × 11 = **286 MHz**；`factor_pllvideo0x4_tbl` 最低 288 MHz（步进 12），于是上取整到 288——这正是 `clk_summary` 里的 288 MHz。同函数 `0x588c` 起还有"目标速率 ≠ 实际速率"的告警分支（比较 `x21` 与 `x23`），说明驱动本来就预期会被取整。

（上一版草稿曾把这里算成 26 × 6 = 156 MHz，是因为把 `csel` 的方向读反了；修正后与实测一致。）

### 7.3 `tcon_lcd` 的 mux 候选表（已恢复）

`tcon_lcd_parents`（`0xffffff8009047508`）：

| idx | 候选 |
| --- | --- |
| 0 | `pll_video0` |
| 1 | `pll_video0x4` |
| 2 | `pll_video1` |
| 3 | `pll_video1x4` |
| 4 | `losc` |
| 5 | `periph32k` |

（紧随其后是第二张 7 项表，属 `tcon_lcd1`：`pll_video0 / pll_video0x4 / pll_video2 / pll_video2x4 / osc48md4 / hoscd2 / losc`。）

配合同一次启动的 `clk_summary`（`tcon_lcd = pll_video0x4 = 288 MHz`），可定位为索引 1。但**"当前选的是索引 1"仍只有运行时证据**——代码侧只能证明"选择来自 `clk_get_parent` 的返回值"。

---

## 8. PLL 因子描述符（`sunxi_clk_factor_pll_video0x4` @ `0x903deb0`）

前 `0x38` 字节（next symbol `factor_pllgpu_tbl` 定界）：

| 偏移 | 值 | 字段 |
| --- | --- | --- |
| `+0x00` | 8 | `nshift` |
| `+0x01` | 8 | `nwidth` |
| `+0x02` | 0 | `mshift` |
| `+0x03` | 0 | `mwidth` |
| `+0x04` | 0 | `pshift` |
| `+0x05` | 0 | `pwidth` |
| `+0x06` | 0 | `d1shift` |
| `+0x07` | 0 | `d1width` |
| `+0x08` | 1 | `d2shift` |
| `+0x09` | 1 | `d2width` |
| `+0x0a` | 0 | `fshift` |
| `+0x0b` | 0 | `fwidth` |
| `+0x0c` | 0 | 未解 |
| `+0x0e` | 0 | 未解 |
| `+0x0f` | **27** | 位号（见下） |
| `+0x10` | `0x1800` | 未解 |
| `+0x18` | `0x140` | 未解 |

字段顺序不是按惯例猜的：`sunxi_clk_factors_recalc_rate`（`0x845b6f0`）逐字段 `ldrb w1,[x20,#2k+1]`（width）/`ldrb w2,[x20,#2k]`（shift）取值，顺序正是 `+0/+1`（N）、`+2/+3`（M）、`+4/+5`（P）、`+6/+7`（D1）、`+8/+9`（D2）、`+10/+11`（f）。

要点：

- **M / P / D1 的 width 全为 0**，rate 只由 N 和 D2 决定——与 `calc_rate_video0` 只读这两个字段吻合。
- **fractional 关闭**（`fwidth = 0`），没有小数分频路径。
- `+0x0f = 27` 是该描述符里唯一出现 27 的地方，`recalc_rate` 不读它。结合 §3.1 的"配置字与现场值的差恰好是 bit31+bit27+bit0"，bit27 最可能就是它（lock/状态位）。但**没有任何代码在本镜像的 rate 路径里读它**，所以只写成"描述符声明了 27 这个位号"，不写成结论。
- `+0x10`/`+0x18` 无可用证据，保留未解。

---

## 9. 120 Hz 复核

结论**不变**：未验证 / 不支持启用。补充三条可直接引用的证据：

1. **面板驱动是改名复用的官方驱动**（`fog_fj035fhd05_v1_panel`），`LCD_panel_init` 是固定命令流；时序全部来自 DT，驱动内没有第二套 timing。
2. **DT 只有一套时序**：`lcd0@01c0c000` 的 `lcd_ht=820`、`lcd_vt=536`、`lcd_dclk_freq=26`、`lcd_if=0`、`lcd_hv_if=0`，无 `lcd_tcon_mode`、无 120 Hz 相关属性，也没有 `clk_parent`/`clk_freq`。
3. **内核自己的帧周期算法**（`cal_real_frame_period`，`0x8405ac4`）只有一个公式，无模式分支：
   ```
   dclk      = clk_get_rate(tcon_clk) / clk_info[+4]      // lcd_if==4 时用 [+12]
   period_ns = panel_info.vt * panel_info.ht * 1e9 / dclk
   ```

要支持 120 Hz，仍缺：第二套 panel timing、该速率下被接受的父 PLL 速率、该速率下的 divider、TE/VBlank 边界与回滚路径、面板侧独立依据。**好消息是 PLL 不再是硬瓶颈**：`factor_pllvideo0x4_tbl` 覆盖 288–2520 MHz、步进 12 MHz，120 Hz 所需约 52 MHz 像素时钟乘 11 得 572 MHz，表内可达（576 MHz）。所以真正的缺口回到面板/时序与 TCON 侧，而不是时钟源。

（顺带更正首轮 §5 第 4 条："factor table 的 288..336 MHz 范围"实际是 288–2520 MHz；这不改变结论，但原表述是错的。）

---

## 10. 仍未解决的问题

1. **`tcon_lcd` 的当前父时钟与实测速率**（§7.3）。候选表已恢复，选中项只有 `clk_summary` 证据。
2. **四个 `struct clk *`（`priv+0x1978…`）的取得点**（§6.6）。不是按名字 `clk_get`，取值在运行时。
3. **`open_flow` 步骤表的执行者**（`blr`，目标运行时填充）。
4. **`panel_info+0x118` 的语义**（§2.4）。无写入者、本机不可达，标记为 dead field。
5. **PLL 描述符 `+0x0c` / `+0x0e` / `+0x0f` / `+0x10` / `+0x18`** 的语义，以及现场值里 bit0 的作用（§8、§3.1）。
6. **`lcd_type=old` → 面板/接口选择的实际作用点**。`lcd_if` 来自 DT（0），bootarg `lcd_type=old` 如何影响面板或接口选择，仍无静态证据。

（首轮的第 1、4 项已闭合；第 6 项"DT `lcd_dclk_freq` 是否在别处被使用"已由 §7.2 回答。）

---

## 11. 更正与撤回清单

| # | 首轮位置 | 首轮写法 | 更正 |
| --- | --- | --- | --- |
| 1 | §0 限制说明 | "所有指针字段为 0，数据段没有重定位，无法顺着指针遍历" | 镜像是未重定位的；`.rela.dyn` 有 91 760 条 `R_AARCH64_RELATIVE`，覆盖后指针表全可读（§1） |
| 2 | §0 身份核验 | note 头在段偏移 `0xc8fc60` | 文件偏移 `0xc8fe90`（desc `0xc8fea0`，VA `0xffffff8008d0fc50`）；Build ID 值不变（§1.1） |
| 3 | §1 | `factor_pllvideo0x4_tbl` 覆盖 288–336 MHz、9 条 | 288–2520 MHz、步进 12 MHz、187 条（§4） |
| 4 | §1 | 表字求值是表速率的 2 倍，未解 | 逐条自洽，无 2 倍；该 open item 撤销（§4） |
| 5 | §1 | bit27 含义未解 | 描述符 `+0x0f` 声明位号 27；且现场值 = 表配置字 + bit31 + bit27 + bit0，bit27 属状态类（§3.1、§8） |
| 6 | §2 | `tcon_lcd` 父选择表指针为 0，无法静态判定 | `tcon_lcd_parents` 已恢复 6 个候选；选中项仍靠运行时（§7.3） |
| 7 | §2 / §5 | 写 TCON `+0x44` 的只有 `disp_al_lcd_cfg` 与 `disp_al_vdevice_cfg` | 还有 `tcon0_open`（×2）、`tcon0_close`、`tcon_exit`（§5） |
| 8 | §3 | `[上下文+0x80]` 是 SoC/版本字 | `+0x80` 是 `lcd_dclk_freq`；`0x1a` 就是 26 MHz（§2） |
| 9 | §3 | 表索引按 `cfg->lcd_type` | 按 `cfg->lcd_if`（`panel_info+0x00`）；表项首字段即其服务的 `lcd_if` 码（§3） |
| 10 | §3 | "`!= 0x1a` 时强制 0x0b" | 方向写反：`csel …, ne` 表示 **`== 26` 时取 11**，其它频率用表值（§3） |
| 11 | §3 | DT `lcd_dclk_freq` 完全不参与计算 | 它决定 divider 取 11 还是表值，并且是 `lcd_clk_enable` 里 PLL 目标速率的乘数（§7.2） |
| 12 | §3 | 高 4 位 `0xf` 由读-改-写保留，含义未解 | `tcon0_open` 主动 `orr 0xf0000000`（§5） |
| 13 | §4 | 面板名字表 `0x8ffc4c0`，stride `0x60` | 名称在 `0x8ffc580`；驱动表是 `0x8ffbef0` 的 17 项指针数组；该符号名是 `fog_fj035fhd05_v1_panel`（§6.1） |
| 14 | §4 | `lcd_type=old` 落到索引 0 | 索引来自 DT `lcd_if=0`；bootarg 的作用点仍无静态证据（§10.6） |
| 15 | §6 | 对账表（288 MHz / 11 / 26.18 MHz / 59.569 Hz） | **数值全部保留**，理由改为"override 4 对 26 MHz 取 11"，并补上"286 → 288 的 PLL 步进取整"这一环（§3.1） |
| 16 | §7 | 未解决项 1/2/4/6 | 1 见 §5（已解）；2 见 §2.4（dead field）；4 见 §4（撤销）；6 见 §7.2（已答） |

首轮未改动、本轮复核仍成立：PLL 解码函数与 `24 MHz × (N+1)/(D2+1)` 公式、`0x88001703 → N=23/D2=1 → 288 MHz`、`tcon0_set_dclk_div` 的 `bfxil` 逐位写入无 ±1、`tcon0_get_dclk_div` 的 `& 0x7f`、divider 11、"288/11 → 26.18 MHz → 59.57 Hz"、时序全部来自 DT、120 Hz 结论。

---

## 12. 复现与自检

```sh
# 1. 未重定位视图 + 指针覆盖（核心前提）
python build/k19f_img.py            # 段布局、符号数、重定位条数
python build/k19f_rela_real.py      # 91 760 条、现场 100% 为 0
python build/k19f_rela_sym.py       # addend 与符号表交叉验证

# 2. DT 键 -> panel_info 偏移
python build/k19f_dtkeys.py

# 3. 面板驱动表与 rg34xx_v1 条目
python build/k19f_panelarr.py
python build/k19f_panels.py

# 4. divider 表 / PLL 表 / 描述符
python build/k19f_clktbl.py
python build/k19f_tbl.py factor_pllvideo0x4_tbl 12
python build/k19f_bytes.py sunxi_clk_factor_pll_video0x4 0x38
python build/k19f_strarr.py tcon_lcd_parents 14

# 5. 调用关系与单函数反汇编
python build/k19f_callers.py disp_al_lcd_cfg lcd_clk_enable
python build/k19f_calls.py disp_lcd_enable
python build/k19f_dis.py disp_al_lcd_get_clk_info

# 6. 本报告的自洽性检查（82 项，全部通过才允许发布）
python build/k19f_selfcheck.py
```

自检覆盖：Build ID 值与两处偏移、重定位 91 760 条且现场全 0、divider 表 4 项、PLL 表 187 项与首末项、描述符 8 个字段、面板表 17 项与 `rg34xx_v1` 条目、`tcon_lcd_parents` 6 项、19 个函数地址与符号名一一对应、`cmp/csel/orr/bfxil/str` 五处指令级断言、五个交付文件的 SHA-256 覆盖关系，以及首轮三个文件哈希未变。

**这份自检确实抓到过我自己的一处错误**：草稿里按 `csel …, eq` 读，推出 divider = 6，并据此准备撤回首轮对账表；检查脚本逐条断言 `csel` 的条件字段时发现是 `ne`，方向相反，真实结论是 11。修正后全链闭合。这条经历本身就是"不要凭阅读顺序猜条件码"的证据，故保留在报告里。

本补充报告同样只做归档与来源对照，不含可刷写固件、不含硬件写入代码，也不代表任何面板能力承诺。

## 13. 第三轮增量：PLL 因子表完整导出与选频行为

本节记录 `docs/kernel19-clock-factor-table-audit-task.md` 的结果。完整数据与逐行表格在
[`PLL-TABLE.md`](PLL-TABLE.md) 与 [`pll-factor-table.json`](pll-factor-table.json)，此处只保留结论与更正。

### 13.1 表结构与矛盾裁决

三条互相独立的证据都给出 **187 项**：符号间距 `0x903deb0 − 0x903d8d8 = 0x5D8 = 187×8`、
`get_factors_pll_video0` 里的 `mov x4, #0xbb`、以及兄弟表 `factor_pllvideo1_tbl` 同样 1496 字节。

187 行逐行验证：**288 MHz 到 2520 MHz，全部 12 MHz 步进，`24 MHz × (N+1)/(D2+1)` 与表内速率逐行相等，
0 处偏差、0 处重复、0 处非 12 MHz 跳变。** 描述符 `0x903deb0` 不计入表项。

因此 §4 的两句话里"187 项、288–2520、步进 12"成立，而"末项 `0x00006400`、2424 MHz"是笔误：
`0x00006400` 在索引 178，末项是索引 186 的 `0x00006800`（N=104、D2=0、2520 MHz）。上方 §4 表格已就地标注。

另一个必须记住的结构事实：**N 与 D2 在表内不单调**。前 14 行只用 `D2=1`；从索引 14 起交替使用
`D2=0`（共 87 行），两种编码给出同一速率阶梯。逐行按位域解码是唯一可靠方式。

### 13.2 选频不是 floor 也不是 ceil

链路是 `clk_set_rate` → `sunxi_clk_factors_round_rate` → `get_factors_pll_video0`
→ `sunxi_clk_com_ftr_sr` → `sunxi_clk_freq_search`：

1. `pllvideo0x4_max` 先做上界夹取（`csel …, ls` 取 `min`）。该变量在文件镜像里是 0，
   真值由 `sunxi_clk_factor_initlimits` 从表末项 rate 字段装入，即 **2 520 000 000**。
2. `udiv` 把请求截断到整 MHz。
3. 二分查找返回精确值或 floor；若请求低于表首项，`hi = −1` 时函数返回 0，于是映射到 288 MHz。
4. 再比较该行与下一行的距离，`cinc w0, w0, gt` 只在“当前差值严格更大”时进位 → **等距取低**。

结论：**夹取 + 最近邻 + 等距取低**。

| 场景 | PLL 请求 | divider | 选中 | PLL 速率 | DCLK / 刷新率 |
| --- | ---: | ---: | --- | ---: | --- |
| 基线 stock：26 MHz | 286 MHz | 11（26-MHz 特例） | row 0 `0x1702` | 288 MHz | 26.1818 MHz / 59.5691 Hz |
| Stock path：`lcd_dclk_freq=52` | 312 MHz | **6**（52 不触发 26-MHz 特例） | row 2 `0x1902` | 312 MHz | 52.0000 MHz / 118.3109 Hz |
| 外部强制 divider 11 的反事实 | 572 MHz | 11（强制） | row 24 `0x1700` | 576 MHz | 52.3636 MHz / 119.1382 Hz |
| 外部强制 divider 10 的候选 | 528 MHz | 10（强制） | row 20 `0x1500` | 528 MHz | 52.8000 MHz / 120.1311 Hz |

**关键路径约束：** `disp_al_lcd_get_clk_info` 对 `lcd_if=0` 先取 divider 表值 6，只有
`lcd_dclk_freq == 26` 才覆盖为 11。因此，“52 MHz × 11 = 572 MHz”仅是保持 divider=11 的
反事实选频测试，不是 stock 驱动把 `lcd_dclk_freq` 改成 52 MHz 后会执行的路径；stock 路径实际是
52 MHz × 6 = 312 MHz。divider 10 / PLL 528 MHz 是表内可达的手动候选，但当前 stock 路径不会生成
divider 10。

### 13.3 与显示时序的关系及口径修正

- `lcd_clk_enable` 确实调用 `clk_set_rate`（3 处）与 `clk_set_parent`（4 处）。`disp_lcd_init`
  用 `clk_get_parent` 捕获，`lcd_clk_enable` 回写同一父钟，然后按
  `clk_info.w4 × divider × lcd_dclk_freq` 下发 PLL 请求。
- 26 MHz 基线仍是：特例 divider 11 → PLL 请求 286 MHz → table 选 288 MHz → DCLK 26.1818 MHz
  → 59.5691 Hz，与 live baseline 一致。
- 若 stock 配置把 `lcd_dclk_freq` 改为 52 MHz，divider 会改回 6；PLL 请求 312 MHz 精确命中
  row 2，DCLK 52 MHz，820×536 totals 下约 118.3109 Hz。
- 576/11 的 119.1382 Hz 仅属于外部强制 divider=11 场景；528/10 的 120.1311 Hz 是另一个
  外部强制候选。两者都不是 stock LCD init 的配置输出，也没有面板侧验证。

以上结果不改变 verdict：**120 Hz 未验证 / 不支持启用**。仍缺第二套 panel timing、
该分频组合的面板接受证据、TE/VBlank 边界、rollback 路径。所有硬件 writer 保持关闭。

### 13.4 未解

描述符侧字段 `+0x0f/+0x10/+0x18/+0x20` 只按字节位置报告，不按惯例推断位号含义；
`clk_factors_ops` 在镜像里 `recalc/round/set_rate` 槽为 NULL，填充点未逐指令追；
`pllvideo0x4_max` 只有静态证明（表末项），无运行时回读；`lcd_if == 4` 分支未展开。
