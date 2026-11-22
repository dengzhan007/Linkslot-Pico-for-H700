# H700 AArch64 构建基础（第一阶段）

## 当前交付

本阶段建立可重复的 AArch64 freestanding 编译检查：

- 目标 triple：`aarch64-none-elf`
- CPU：Cortex-A53（Allwinner H700 / RG34XX）
- ABI：AArch64 默认 LP64
- 编译 C11、C++17 和 AArch64 汇编探针
- 编译约束：`-ffreestanding`、`-fno-builtin`、`-fno-stack-protector`，早期代码使用 `-mgeneral-regs-only`
- C++ 探针关闭异常、RTTI 和静态对象线程安全初始化；不链接 C++ runtime
- 每个目标文件均检查 ELF object architecture 为 AArch64
- GitHub Actions 在相关源文件或脚本变化时自动运行

本地运行：

```sh
sudo apt install clang llvm
bash scripts/build-h700-objects.sh
```

输出位于 `build/h700-arch-check/`，该目录不应提交到 Git。

## 重要限制

这不是可启动固件，也不是 payload：

- 没有链接器脚本、VMA/PMA、加载地址或入口点；
- 没有假设 U-Boot 会提供某个特定寄存器、栈指针或 DTB 参数；
- 没有设置 MMU、cache、异常向量或中断控制器；
- 没有访问 UART/GPIO/DRAM/SD/显示/音频寄存器；
- 没有链接 libc、libc++、C++ ABI runtime、异常 unwinder 或 Rust runtime；
- 没有证明目标设备可以执行该对象文件。

这一步只证明当前工具链可以为 Cortex-A53 生成 freestanding AArch64 C、C++ 和汇编目标文件。它没有证明 Gambatte、mGBA 或 gpSP 已经能够构建，也没有证明 LK 与 Linkslot Rust 代码可以链接。

这些事项依赖目标掌机实际使用的 U-Boot 版本、内存布局、板级硬件以及最终运行时方案。取得只读启动链日志和板型资料后，才能确定下一阶段的 payload linker script、加载地址和入口约定。

目标机型和已知硬件边界见 [RG34XX 硬件基线](rg34xx-hardware-profile.md)。

## 后续扩展顺序

1. 固定第一台实机型号与板级版本。
2. 获取 U-Boot 版本、启动环境、SD 读取命令和 DRAM map。
3. 依据实际 handoff ABI 增加 startup/entry 和 linker script。
4. 验证 C/C++ 所需的 freestanding runtime、内存分配、异常/静态构造策略；核心实际需要的功能再逐步启用。
5. 先完成串口/异常/内存最小验证，再逐项增加 MMU、定时器和 GPIO。
6. 把每个硬件功能放在独立驱动与可读诊断结果中，避免把未验证的寄存器假设带入上层。

该编译检查刻意不依赖 Linkslot 上游，因此 Linkslot 即将发布的大版本不会使本阶段成果过时。
