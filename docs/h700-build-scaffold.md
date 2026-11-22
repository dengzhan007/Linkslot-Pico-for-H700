# H700 AArch64 构建基础（第一阶段）

## 当前交付

本阶段只建立可重复的 AArch64 freestanding 编译检查：

- 目标 triple：`aarch64-none-elf`
- CPU：Cortex-A55（RK3566/H700 CPU 核心）
- ABI：AArch64 默认 LP64
- 编译约束：`-ffreestanding`、`-fno-builtin`、`-fno-stack-protector`，早期代码使用 `-mgeneral-regs-only`
- C 与汇编各编译一个独立目标文件，并检查 ELF object architecture 为 AArch64
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
- 没有证明目标设备可以执行该对象文件。

这些事项依赖目标掌机实际使用的 U-Boot 版本、内存布局和板级硬件。取得只读启动链日志和板型资料后，才能确定下一阶段的 payload linker script、加载地址和入口约定。

## 后续扩展顺序

1. 固定第一台实机型号与板级版本。
2. 获取 U-Boot 版本、启动环境、SD 读取命令和 DRAM map。
3. 依据实际 handoff ABI 增加 startup/entry 和 linker script。
4. 先完成串口/异常/内存最小验证，再逐项增加 MMU、定时器和 GPIO。
5. 把每个硬件功能放在独立驱动与可读诊断结果中，避免把未验证的寄存器假设带入上层。

该编译检查刻意不依赖 Linkslot 上游，因此 Linkslot 即将发布的大版本不会使本阶段成果过时。
