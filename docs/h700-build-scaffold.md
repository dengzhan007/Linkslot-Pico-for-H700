# H700 AArch64 构建基础（第一阶段）

## 当前交付

本阶段在 compile-only 探针之后，增加一个**只用于静态审查、尚未执行**的 AArch64 Image 入口探针构建：

- 目标 triple：`aarch64-none-elf`
- CPU：Cortex-A53（Allwinner H700 / RG34XX）
- ABI：AArch64 默认 LP64
- 保留 C11、C++17 和 AArch64 汇编目标文件检查
- 新增 `platform/h700/bootprobe/entry.S` 与 `payload.ld`，建立 AArch64 Image header 后的 `_start`、Image 内私有 stack 和 entry record
- 入口代码先保存 x0..x3、原始 SP、CurrentEL、DAIF，再屏蔽异常并读取 `SCTLR_EL1`、TTBR0/1、TCR、MAIR、VBAR、MPIDR；最后写入 Image 内记录区并 `WFE` 停机
- 不调用 C/C++ runtime，不使用 Linux 代码、不访问 MMIO、不调用 SMC、不分配 Image 外 RAM
- `tools/pack_aarch64_image.py` 生成标准 64-byte AArch64 Image header：`code0` 分支指向 `_start`，`text_offset=0x80000`，`image_size` 对齐并受 `0x1138000` 上限约束
- 输出 ELF、map、raw body 和 `entry-probe.Image` 到 `build/h700-arch-check/`；不生成 Android p4、不执行、不刷写
- CI 静态检查 ELF 架构、Image header 与 stock kernel slot 大小上限

本地运行：

```sh
sudo apt install clang llvm
bash scripts/build-h700-objects.sh
```

输出位于 `build/h700-arch-check/`，该目录不应提交到 Git。

## 重要限制

`entry-probe.Image` 是一个 AArch64 Image 格式的静态入口探针构建产物，但**不是经过实机验证的 payload 或可刷写固件**：

- 当前 linker VMA 只用于把代码/私有栈/记录区限制在 `0x40080000` 开始、且不超过原厂 Image `image_size` 的范围内；并不授权刷写或实机执行；
- `x0` 是否确为 FDT、CurrentEL 是否为 EL1、`SCTLR_EL1`/MMU/cache 具体状态，仍待 monitor ERET 实证；
- 探针不修改 MMU/cache/异常向量/GIC，不触碰 UART/GPIO/DRAM/SD/显示/音频寄存器；
- 不建立通用 allocator，不访问 Image 外内存；未知 reserved、CMA、monitor、framebuffer 等区间仍全部排除；
- 不链接 libc、libc++、C++ ABI runtime、异常 unwinder 或 Rust runtime；
- 不调用 Linux 初始化代码，不执行 C/C++ 函数；目前只完成汇编入口构建与 header 检查；
- 没有生成 Android boot image 候选，也没有证明目标设备可以安全执行该 Image。

当前阶段证明：工具链可为 Cortex-A53 生成 AArch64 C/C++/汇编 object，并可将最小入口汇编链接为受尺寸约束的 AArch64 Image 格式文件。它仍未证明 monitor 会以文档中的寄存器状态进入该代码，也没有证明 Gambatte、mGBA、gpSP、LK 或 Linkslot Rust 代码可以链接运行。

这些事项依赖目标掌机实际使用的 U-Boot 版本、内存布局、板级硬件以及最终运行时方案。取得只读启动链日志和板型资料后，才能确定下一阶段的 payload linker script、加载地址和入口约定。

目标机型和已知硬件边界见 [RG34XX 硬件基线](rg34xx-hardware-profile.md)。

## 后续扩展顺序

1. 核实 RG34XX 实机主板 revision 与当前固件版本。
2. 获取 U-Boot 版本、启动环境、SD 读取命令和 DRAM map。
3. 依据实际 handoff ABI 增加 startup/entry 和 linker script。
4. 验证 C/C++ 所需的 freestanding runtime、内存分配、异常/静态构造策略；核心实际需要的功能再逐步启用。
5. 先完成串口/异常/内存最小验证，再逐项增加 MMU、定时器和 GPIO。
6. 把每个硬件功能放在独立驱动与可读诊断结果中，避免把未验证的寄存器假设带入上层。

该编译检查刻意不依赖 Linkslot 上游，因此 Linkslot 即将发布的大版本不会使本阶段成果过时。
