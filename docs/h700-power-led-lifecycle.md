# RG34XX PI12 电源灯生命周期

## 默认行为

| 阶段 | PI12 工作/电源灯 | 说明 |
|---|---|---|
| LK 早期启动钩子 | 点亮 | 目标是灯亮即可表示已越过早期初始化，用户可松开电源键 |
| 前端 / 菜单 | 点亮 | 默认常亮 |
| 游戏运行 `Phase::Playing` | 熄灭 | 只在状态变化时调用 C 服务 |
| 游戏成就拍立得页 `Phase::Polaroids` | 熄灭 | 仍属于当前游戏会话 |
| 游戏退出、退卡动画、前端菜单 | 点亮 | 离开 Playing/Polaroids 后恢复 |
| 睡眠 / 唤醒 | 暂无独立策略 | 后续按 suspend/resume 生命周期补充 |

当前默认启用“玩游戏熄灯”，没有用户设置项。前端设置开关等后续前端选项规划时再增加，未来默认仍为开启。

**实机行为尚未验证。** 目前代码按高电平点亮实现；实际极性、灯亮时间和用户松开电源键的可操作时机，仍需 RG34XX 冷启动验证。

## 板级证据与寄存器

RG34XX（非 SP）KNULLI 启动设备树（固定提交
[57afa662009641d181f57241bcc480119b131530](https://github.com/knulli-cfw/knulli-linux/blob/57afa662009641d181f57241bcc480119b131530/package/boot/uboot-h700/rg34xx/boot_package/rg34xx.dts)）
在 PMU 节点明确给出：

- `work_led = <0x4b 0x08 0x0c 0x01 0x01 0xffffffff 0x00>` → PIO phandle `0x4b` / bank I / PI12；
- `low_pwr_led = <0x4b 0x08 0x0b 0x01 0x01 0xffffffff 0x00>` → 同一 PIO / bank I / PI11。

同一提交的 `pinctrl@0300b000` 定义 phandle `0x4b`。其 GPIO provider 使用六个参数；Linux 4.9 `pinctrl-sunxi` 的 `of_xlate` 对应解释为 bank、pin、mux、pull、drive、data。因此上面 PI12 的 mux=1 表示 GPIO output，最后 data=0 是初始输出低电平，**不是 active-low 极性标志**。MustardOS 的 [RG34XX-H DTS](https://github.com/MustardOS/internal/blob/aef38ed9a870419a0fe4f351dac1bac58a84ba5e/device/rg34xx-h/package/sunxi.dts) 也有相同 PI12/PI11 映射，作为 H 系列交叉参考。

KNULLI DTS 同时把 UART3 控制器（`uart@05000c00`）设为 `disabled`，UART0（`uart@05000000`）为 `okay` 且是 `serial0`。UART3 pinctrl 组虽然列出 PI9–PI12，但当前启动配置未启用 UART3；PI12 GPIO 不占用 UART0 控制台。PI11 不属于本功能，不会被修改。

**极性证据要分开看：** 主线 H700 RG35XX-2024 DTS 将 PI12 定义为 `GPIO_ACTIVE_HIGH` 并默认点亮；这是同 SoC 掌机的 active-high 旁证，不等于 RG34XX 本机的 LED 电气读回。本实现暂采用高电平点亮，冷启动实测仍是验收项。

H700 PIO 字段：

- PIO base：`0x0300b000`；
- Port I bank index：8，bank stride：`0x24`；
- PI12 CFG1/MUX1：偏移 `0x124`，pin12 的 4-bit mux 位域在 bits 19:16，function 1 = GPIO output；
- PI12 DATA：偏移 `0x130`，bit 12；
- PI11 是同一 DATA 寄存器 bit 11，使用 read-modify-write 保留此位和其他 GPIO。

主线 H700 RG35XX-2024 DTS 对同一 PI12 使用 active-high LED 配置，仅作为 H700 同族极性旁证，不替代 RG34XX 本机电气验证：
[mainline RG35XX-2024 DTS](https://github.com/torvalds/linux/blob/master/arch/arm64/boot/dts/allwinner/sun50i-h700-anbernic-rg35xx-2024.dts)。

## 启动时序

1. GammaOS LK 的 `arch_early_init()` 在 `target_early_init()` 之前调用 `platform_init_mmu_mappings()`。
2. H700 hook 调用 `arm_mmu_init()` 初始化 LK 4096-entry AArch32 section table / TTBR。
3. 将现场确认的 DRAM `[0x40000000, 0x80000000)` 以 normal write-back / read-write 属性建立 1024 个 1 MiB identity sections。
4. 将 `[0x03000000, 0x03100000)` 以 shared Device / read-write / XN 属性映射，覆盖 CCU 与 PIO@0x0300b000。
5. 调用 `arch_enable_mmu()`；之后 LK 完成 cache setup，才进入 `target_early_init()`。
6. `target_early_init()` 调用 `h700_rg34xx_power_led_early_init()`。GPIO data latch 先置高，再切 PI12 mux 为 GPIO output，最后读回验证。
7. 若早期访问失败，`target_init()` 在 `platform_init()` 后重试；Linkslot C ABI 首次请求时也会在必要时尝试初始化。

PIO MMU 映射只更新已有 ARM section translation table 的一个外设条目，不分配 DRAM，不改变 LK heap/stack/BSS/BL31 保留区，也不写 CCU/APB1、PMIC 或 CPU PLL。v2 Linux 运行态报告中 PIO/APB1 时钟为 100 MHz；LED driver 不改变这个时钟树。

## PIO bus-clock dependency

The H700/H616-family PIO binding in the reference
[`sun50i-h616.dtsi`](https://github.com/torvalds/linux/blob/master/arch/arm64/boot/dts/allwinner/sun50i-h616.dtsi)
connects the main pinctrl block to `CLK_APB1` and does not declare a separate
PIO reset line. The reference
[`ccu-sun50i-h616.c`](https://github.com/torvalds/linux/blob/master/drivers/clk/sunxi-ng/ccu-sun50i-h616.c)
models APB1 as a mux/divider clock at CCU offset `0x520`, without a distinct
PIO gate bit. The pinned vendor-equivalent H616 family uses the same
`allwinner,sun50i-h616-pinctrl` binding; the v2 RG34XX Linux clock summary
also reads `apb1=100 MHz` with a PIO consumer.

This supports using the already configured APB1 path without adding a
speculative PIO gate/reset sequence. It does **not** prove the exact APB1
parent/rate at the earlier BL33 handoff. The PI12 service deliberately does
not write CCU registers or alter APB1. Early PIO access still needs cold-boot
validation; if the board's handoff configuration has APB1 unavailable, keep
the LED function fail-closed rather than enabling or reparenting clocks based
on an unverified assumption.

## Linkslot 游戏生命周期

- `slot-frontend-state::GamePowerLedState` 是 no_std transition latch：首次同步请求前端态（`game_active=false`），之后仅在 `false ↔ true` 变化时发出请求。
- 临时 overlay 修改 Linkslot 1.0.45 的 `crates/slot/src/session.rs`。Session 读取 `App::phase()`；Playing/Polaroids 代表游戏活动，其余阶段视为前端态。
- `Session::feed()` 在处理动作后同步一次；`Session::update()` 在 App 动画更新后同步一次。状态 latch 让真实 C ABI/GPIO 访问只发生在状态转换时，不会逐帧写寄存器。
- C service 只改 PI12 bit 12，使用 read-modify-write 保留 PI11 和同 Bank 其他位。开始游戏拉低熄灭，退出游戏拉高点亮。
- FFI 失败会输出一次 transition error，不做逐帧重试；后续状态变化会再次请求。早期 target hook 与 target_init retry 负责常规启动时初始化。

`upstream/linkslot` 保持不可变，所有 Session Rust 补丁由 `scripts/apply-linkslot-h700-frontend-core.py` 在隔离工作树生成。

## 测试与未完成项

已加入 fake PIO C host test，覆盖：
- 初始化先置 output latch，再切 pinmux；
- 保留 PI11 和 Port I 其他位；
- 游戏开始/结束切换和重复请求幂等；
- 无效 ABI、未初始化、pinmux 丢失和读回失败。

Rust no_std state tests 覆盖初始前端态、开始/退出游戏各一次请求，以及 600 次重复状态采样不会重复发出硬件请求。H700 AArch32/AArch64 compile check [#45526769732](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45526769732) 已通过，包括 AArch64、AArch32 ARM、Thumb-2 对象编译和 MMU/PIO fake-register tests；Linkslot runtime slices [#45524785105](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45524785105) 已通过 no_std state tests 与完整 Linkslot 1.0.45 Frontend overlay 编译。

仍需完成：
- 后续完整 LK target link / 固件构建（当前仍受独立内存批准 gate 限制）；
- RG34XX 冷启动验证灯亮时机是否足够作为松开电源键提示；
- 实机验证游戏时熄灭、退出后点亮，并确认 PI11 低电量灯不受影响；
- 验证休眠唤醒是否会重置 pinmux，必要时在 resume hook 恢复；
- 后续前端设置开关规划。
