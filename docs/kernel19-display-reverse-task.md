# Kernel #19 display-path reverse engineering task

**目标设备：** RG34XX 非 SP / H700 / Linux 4.9.170 #19  
**分析对象：** 本地已恢复、已与 live kernel Build ID 精确匹配的 `build/rg34xx-kernel.elf`  
**当前工作分支：** `porting/linkslot-1.0.45-feasibility`  
**数据来源：** reports 分支现有脱敏 FDT / TCON readback / stock kernel provenance

## 0. 硬约束

- 只做离线 ELF/符号/字符串/反汇编分析；不得连接设备，不调用 sysfs/debugfs/vendor getter。
- 不读写 MMIO，不写 `/sys/class/sunxi_dump/write`，不使用 `devmem`，不改 PLL/TCON/CCU/PMIC。
- 不重导出 p4，不重新恢复 ELF；若本地 ELF 不存在或 Build ID 不匹配，停止并报告，不另找猜测内核替代。
- 不上传 kernel ELF、boot p4、raw `live-fdt.dtb` 或任何含序列号的原始文件。只上传有界文本/JSON 和哈希。
- 不把 pvaibhav/slot 或其他面板 profile 的寄存器值移植到 `rg34xx_v1`。

## 1. 输入身份核验

本地 ELF 必须存在：`build/rg34xx-kernel.elf`。

用 `readelf -n` / pyelftools 读取 GNU Build ID，必须精确为：

```text
a1d211f847a5fb69fe9898e15fbccdd3ea8d7b68
```

与 reports 中 `rg34xx-inventory-20261125-cpux-v4/kernel-driver-provenance.txt`、`rg34xx-boot-p4-verified.json` 记录交叉核验。只把 build-id 与本地 SHA 写入审计报告，不上传 ELF。

## 2. 固定现场基线

对照以下已存在的只读证据，不重新采集设备：

- `reports/pvaibhav-slot-runtime/fdt-reparse/live-fdt.json`：panel `rg34xx_v1`、`lcd_type=old`、RGB666、720×480、DCLK property 26 MHz、HT/VT 820×536。
- `reports/pvaibhav-slot-runtime/tcon-readback/tcon-pll-readback.txt`：TCON divider register `0x06511044 = 0xf000000b`（divider 11）；PLL raw register `0x03001040 = 0x88001703`。
- `reports/rg34xx-inventory-20261125-163152/display-runtime.txt`：vendor display snapshot 与时钟树（包括 `tcon_lcd` 288 MHz、`de_rate` 696 MHz）。

这些是同一次设备调查形成的证据；当前 60 Hz 估算只是外部 sun50iw9 公式的交叉校验，尚未被目标 kernel #19 的真实 bitfield/parent 实现确认。

## 3. 需要恢复的代码路径

优先按符号检索；若 symbol 名不同，按字符串引用、函数地址、调用者和寄存器 offset 追踪：

1. panel profile 注册、选择与 `rg34xx_v1` 对应的 panel driver / panel struct。
2. `lcd_type=old` 的解析、分支和具体 panel init callback。
3. `sunxi_clk_factor_pll_video0x4`（如有）、PLL factor/CCU 计算和设置函数。
4. `disp_al_lcd_get_clk_info`、`disp_al_lcd_cfg`、`tcon0_set_dclk_div`（若符号名不同，找到等价路径）。
5. `tcon_lcd` 的 parent clock、mux/source selection、divider 配置、rate propagation。
6. panel power/enable/init → display HAL config → PLL/parent/divider 配置的调用顺序。

输出符号地址、函数边界证据、相关字符串地址、调用者/被调用者关系。对每个函数保留可审查的 bounded disassembly；重要分支应给出基本块/分支目标，而不是只截函数开头。

## 4. 要回答的问题

- `0x88001703` 在目标 kernel #19 中每个字段的真实含义是什么？N/M/D1/D2、gate、enable 和其它位分别如何解释？
- 实际 feed `tcon_lcd` 的 parent 是哪个 clock？parent/mux 是在 bootloader 设定后保持，还是 Linux driver 重新选择？
- divider word `0xf000000b` 中 divider 11 如何由代码编码/解码？是否存在特殊值或除数偏移？
- `rg34xx_v1 + lcd_type=old` 的真实初始化路径使用什么 timing、PLL、DCLK 与 TCON 参数？
- 目标 kernel/板级 profile 中是否存在明确的 120 Hz timing 或模式表？它是否明确绑定 `rg34xx_v1` 而非其它面板？
- 若发现 120-Hz 候选，列出支持它所需的完整 panel timing、PLL parent、divider、TE/VBlank 边界、切换顺序和 rollback 路径；若缺少任一项，明确写“未验证/不支持启用”。
- 对照 live baseline 的寄存器读回和 59.6 Hz IRQ proxy，解释代码推导是否一致；IRQ 只作为 scanout proxy，不得称为 game FPS 或物理面板刷新率。

不得以“能算出约 120 Hz”作为面板支持证据。

## 5. 交付文件

在 `reports` 分支新增：

```text
reports/kernel19-display-reverse-audit/
  README.md
  symbols.json
  bounded-disassembly.txt
  verification.json
  SHA256SUMS.txt
```

`verification.json` 至少列出 ELF Build ID、输入证据路径与 hash、函数地址/边界依据、clock tree 结论、PLL bitfield、divider 编码、panel profile 分支、120 Hz 分支是否存在及未解问题。哈希覆盖报告内全部文件（不自包含哈希文件）。

完成后更新工作分支的 `docs/PROJECT-STATUS.md` / `docs/roadmap.md`，将下一项任务和报告链接写入；不要把分析产物或硬件写入代码混入上游快照。

## 6. 当前明确不做

- 不实现 PLL/TCON writer。
- 不启用 UI/GAME 物理 120/60 Hz 切换。
- 不更改 `rg34xx_v1` 面板 allowlist。
- 不做额外设备采集；先用现有已脱敏数据完成静态闭环。
