# RG34XX PI12 电源灯生命周期

## 设计目标与当前实际默认状态

设计目标是在未来通过 handoff/clock/ownership 审核的 opt-in LK build 中实现：

| 阶段（opt-in build） | 目标 PI12 状态 | 说明 |
|---|---|---|
| LK 早期启动钩子 | 点亮 | 未来用于提示越过早期初始化 |
| 前端 / 菜单 | 点亮 | 默认常亮 |
| 游戏运行 `Phase::Playing` | 熄灭 | 只在状态变化时请求 C ABI |
| 游戏成就拍立得页 `Phase::Polaroids` | 熄灭 | 仍属于当前游戏会话 |
| 游戏退出、退卡动画、前端菜单 | 点亮 | 离开 Playing/Polaroids 后恢复 |
| 睡眠 / 唤醒 | 尚无独立策略 | 后续按 suspend/resume 生命周期补充 |

**当前 production build 并没有启用上述物理行为。** `H700_RG34XX_PI12_EARLY_ACCESS_ENABLE` 默认值为 0，且 production `platform/h700/rules.mk` 不定义它；因此 `target_early_init()` / `target_init()` 不访问 PIO，Linkslot C ABI 返回 `ERR_NOT_READY`。这会让硬件保持 bootloader/firmware 留下的状态未知，不应写成“默认点亮”或“默认熄灭”。

保留“玩游戏熄灯”作为未来 opt-in build 的默认策略，无前端用户设置项。真实 LED 极性、灯亮时间和是否足以作为松开电源键提示，仍需在安全门槛关闭后做 RG34XX 冷启动验证。

## 2026-11-27：无拆机首启信号定位

RG34XX 首轮 BL33 bring-up 将 PI12 工作灯列为首选待资格化的外部可视信号。目标非 SP boot DTS 已把 PMU work_led 映射到 bank I / PI12、low_pwr_led 映射到 PI11；现有服务只修改 PI12 的 DATA/MUX 位并保留 PI11。该路线不需要拆机或连接 UART 焊盘，UART0 保留为备用。

这项路线调整没有启用生产硬件 wrapper：H700_RG34XX_PI12_EARLY_ACCESS_ENABLE 仍默认 0。BL33 入口时 APB1/PIO clock、non-secure access/firewall 与 Data Abort containment 仍需资格化。首个实机试验必须使用独立 TF1 测试卡，payload 限于经 map 审计的 BL33-slot-only 布局；全局 H700_MEMORY_LAYOUT_APPROVED 继续关闭。完整策略见 [无拆机 TF bring-up 方案](no-disassembly-tf-bringup-plan.md)。

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
5. 所有页表描述符和 TLBI 操作完成后执行 `DSB SY`，再调用 `arch_enable_mmu()` 设置 `SCTLR.M`，随后执行 `ISB SY`；之后 LK 完成 cache setup，才进入 `target_early_init()`。
6. `target_early_init()` 只有在显式定义 `H700_RG34XX_PI12_EARLY_ACCESS_ENABLE=1` 时才调用物理 PI12 服务；默认 production build 未定义此宏，不触发 PIO MMIO。
7. opt-in build 中，GPIO data latch 先置高，再切 PI12 mux 为 GPIO output，最后读回验证；`target_init()` 仅在同一 opt-in build 中重试。Linkslot C ABI 在默认 build 中返回 `ERR_NOT_READY`，不会自行探测 PIO。

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
- `Session::feed()` 在处理动作后同步一次；`Session::update()` 在 App 动画更新后同步一次。状态 latch 只在状态转换时调用 C ABI，不会逐帧请求硬件；production 默认 wrapper 返回 `ERR_NOT_READY`，只有经批准的 opt-in build 才会真正访问 GPIO。
- opt-in C service 只改 PI12 bit 12，使用 read-modify-write 保留 PI11 和同 Bank 其他位。开始游戏拉低熄灭，退出游戏拉高点亮；默认 production build 不会执行这些 MMIO。
- FFI 失败会输出一次 transition error，不做逐帧重试；后续状态变化会再次请求。早期 target hook 与 target_init retry 只有在显式 PI12 opt-in build 中启用。

`upstream/linkslot` 保持不可变，所有 Session Rust 补丁由 `scripts/apply-linkslot-h700-frontend-core.py` 在隔离工作树生成。

## 测试与未完成项

已加入 fake PIO C host test，覆盖：
- 初始化先置 output latch，再切 pinmux；
- 保留 PI11 和 Port I 其他位；
- 游戏开始/结束切换和重复请求幂等；
- 无效 ABI、未初始化、pinmux 丢失和读回失败；另有 default-off wrapper test，确认未 opt-in 时 early init/game-state C ABI 返回 `ERR_NOT_READY`。

Rust no_std state tests 覆盖初始前端态、开始/退出游戏各一次请求，以及 600 次重复状态采样不会重复发出硬件请求。H700 AArch32/AArch64 compile check [#45526769732](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45526769732) 已通过，包括 AArch64、AArch32 ARM、Thumb-2 对象编译和 MMU/PIO fake-register tests；Linkslot runtime slices [#45524785105](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45524785105) 已通过 no_std state tests 与完整 Linkslot 1.0.45 Frontend overlay 编译。

仍需完成：
- 后续完整 LK target link / 固件构建（当前仍受独立内存批准 gate 限制）；
- RG34XX 冷启动验证灯亮时机是否足够作为松开电源键提示；
- 实机验证游戏时熄灭、退出后点亮，并确认 PI11 低电量灯不受影响；
- 验证休眠唤醒是否会重置 pinmux，必要时在 resume hook 恢复；
- 后续前端设置开关规划。


### MMU enable ordering hardening

The H700 mapping hook now performs an architectural DSB after all identity
section descriptors and TLB invalidations have been written, calls LK's
`arch_enable_mmu()`, then executes ISB before returning to the remainder of
`arch_early_init()`. Host regression checks the barrier mnemonics are present;
AArch32/AArch64 cross-compile validates the architecture guards. This is a
source-level ordering check, not hardware MMU enable evidence.


### 2026-11-26 — real LK early-MMU + PI12 host layout result

Full feasibility [#45843270056](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45843270056) passed 81/81 (0 failed, 0 skipped); artifact [15131735148](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45843270056/artifacts/15131735148). The added minimal layout ELF links the pinned LK's actual `arch_early_init()` / AArch32 section-table MMU implementation and H700's real `platform_init_mmu_mappings()`, DRAM provider, RG34XX `target_early_init()`, and PI12 LED service. The diagnostic entry calls real LK architecture early init before the real target hook.

- Raw image: 7,912 bytes; 32 KiB loader raw-file headroom: 24,856 bytes.
- Real LK translation table: `tt=0x4a008000`, 16 KiB aligned.
- Linked `_end=0x4a00c024`; runtime footprint is separate from raw file length.
- MMU barrier ordering: descriptor/TLBI writes → DSB → `arch_enable_mmu()` → ISB.
- LK arch.c and H700 C services use Thumb-2; FPEXC is accessed with VMRS/VMSR; exception vectors and the fixed SYS_CFG probe remain A32.

This is **not** a bootable image: scheduler/bootstrap thread, vendor `platform_early_init()`, GIC/timer/console, heap and storage are omitted. Early APB1/PIO clock state at BL33 handoff remains unverified; physical runtime memory ownership and secondary A53 hold-off also remain unresolved. No real firmware/SPL pair was generated and no device MMIO, flash or payload execution occurred. Keep PI12 cold-boot and game lifecycle behavior marked unverified.


## PIO fault-safety gate — default hardware access disabled

The Linux inventory's clock summary reports APB1/PIO enabled during normal
Linux operation (APB1 100 MHz; one PIO consumer). This does not establish the
clock/reset/access state at the earlier BL31→A32 BL33 handoff.

A failed register readback is not the same as an external Data Abort. The
current scoped Data Abort recovery intentionally accepts only the single
SYS_CFG `VER_REG` diagnostic load at `0x03000024`; a fault on PIO
`0x0300b124` or `0x0300b130` remains fatal. Therefore the physical PI12
MMIO wrapper is now disabled unless the build explicitly defines
`H700_RG34XX_PI12_EARLY_ACCESS_ENABLE=1`.

The normal H700 platform rules do not define this macro. The real early-MMU /
PI12 layout test sets it only in its disposable compile command to verify
that the actual LK MMU and PI12 objects link together and fit the 32 KiB raw
extent. The generic injected-IO C service remains fully host-testable.
Do not enable the macro in a target firmware until APB1/PIO access at BL33
handoff, physical memory ownership and secondary-core behavior are qualified.


## BaseOS SPL PIO use before BL31 — source audit (not BL33 approval)

The pinned BaseOS-MultiFrontend commit
[`cbe253a43f46`](https://github.com/dengzhan007/BaseOS-MultiFrontend/tree/cbe253a43f46cf551cc9a6c79a84015bde3686cf)
contains the active RG34XX native SPL source in
`experiments/h700-boot-speed/spl/native-boot.c`. Its RG34XX panel contract is
rule=2 (PE6 strap; high=old, low=boe). Before handing off to vendor BL31,
`baseos_vendor_handoff()` starts the panel sequence; the SPL writes
`0x0300b124` / `0x0300b130` for PI15/PI14, then configures and reads PE6
through `0x0300b090`, `0x0300b0ac`, and `0x0300b0a0`. This is positive
source evidence that the native SPL uses the main PIO block before BL31.
It does **not** prove the APB1/PIO clock/reset/access state after BL31 and at
the A32 BL33 entry.

The current Data Abort matcher only recovers the explicitly armed SYS_CFG
read at `0x03000024`; an abort on PI12/PIO remains fatal. Linux runtime
clock-summary data is later in the boot chain and cannot replace a BL33
handoff snapshot. Keep physical PI12 MMIO disabled in production until that
gap and memory/secondary-core ownership gates are closed.


## BL31 PIO/CCU access-candidate scan (2026-11-26)

The exact matched BL31 monitor payload is scanned across the complete
107,216-byte image by `scripts/audit-rg34xx-bl31-full-image-cpu-mmio.py`.
The address classifier now reports two narrower H700 windows before the
broad SYS_CFG/SID range:

- `CPUX_CCU`: `[0x03001000, 0x03002000)`, the 4 KiB CPUX clock/CCU resource
  identified by the pinned `sun50iw9p1-clk.dtsi` source audit. This is a
  source-derived candidate range, not an MMIO readback or access approval.
- `PIO`: `[0x0300b000, 0x0300b400)`, the exact 0x400-byte active PIO resource
  in the sanitized live RG34XX FDT. It includes PI12 configuration/data
  registers and the PI14/PI15 pins touched earlier by the BaseOS SPL.

The broad `SID_SYS` classifier remains for the rest of
`[0x03000000, 0x03100000)`; specific CCU/PIO hits are counted separately.
Host regressions lock the range boundaries and ensure the generic label does
not swallow the specific PIO/CCU classifications.

This scan is deliberately heuristic: it decodes overlapping aligned AArch64
windows and tracks local register constants. A candidate is a lead for
manual disassembly/control-flow review; an absent candidate does not prove
that BL31 never accesses the block through indirect, data-driven or secure
paths. It cannot establish APB1 clock state, PIO access permissions, or
register values at BL33 entry. PI12 physical MMIO therefore remains disabled
in production, and `hardware_applied=0`.


## BL31 PIO finding — conditional low-power code path (2026-11-26)

The first CI run after adding the address classes
([#45851421825](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45851421825))
completed successfully. It initially reported one PIO candidate at
PC `0x480134c8`, a load from `0x0300b100`. Manual disassembly of the exact
matched monitor binary around `0x48013490` shows a coherent register setup
and read-modify-write sequence involving `0x0300b100`, `0x0300b110`, and
`0x0300b118`; the first 16-instruction scan window under-reported this
sequence because later accesses fell outside its local register-tracking
window.

The local call chain includes `0x4800c404 → 0x4800c4d0`, a conditional
`BL` at `0x4800c53c → 0x48013df0`, and `0x48013e40 → 0x48013490`.
The surrounding `0x4800c4d0` path manipulates `SCR_EL3` and executes
`WFI`. However, this function/call chain is not present in the existing
primary-entry direct-control-flow disassembly, and its registration or
invocation point has not been resolved. It is therefore evidence that the
matched BL31 **contains a conditional PIO-access path**, not proof that the
path runs at initial BL33 handoff or that it changes the PIO state during
every boot.

The scanner window is now expanded from 16 to 40 instructions, and its
candidate context preserves the full tracking window. A synthetic regression
separates a register's low address seed from its high-half `MOVK` by 24
instructions, proving the scanner still recognizes the later MMIO load.
Re-run CI and review the complete CCU/PIO rows before drawing any further
conclusion. PI12 remains production default-off.


## BL31 PIO scan clarification — PH10 is not the PI12 LED

The full-image BL31 scan now identifies six PIO read/write candidates. Their
offsets 0x100, 0x110, and 0x118 decode against the live sun50iw9p1-compatible
controller layout as PH10 mux/drive/pull fields; the reference s_cir0_pins_a
DTSI configures PH10 as the IR input. The sequence does not target PI12.
PI12 uses distinct fields at offsets 0x124, 0x130, 0x134, and 0x13c.

This narrows the BL31 finding: it contains a conditional low-power PH10/IR
configuration path, not a discovered PI12 LED operation. It does not prove
that PI12 is accessible from non-secure BL33 after handoff. Keep
H700_RG34XX_PI12_EARLY_ACCESS_ENABLE default-off and retain all existing
memory and secondary-core gates.
