# Linkslot 上游同步与移植边界

## 当前状态：等待即将发布的大版本

Linkslot 上游即将有一版较大的更新，因此当前**暂不导入 subtree、不对现有旧版逐 crate 做完整复用审计，也不开始针对旧版做大规模适配**。当前研究快照只用于理解项目结构。等新版本正式发布后，以新 release 对应的 commit SHA 重新执行审计和导入，避免重复工作。

等待期间优先推进独立于 Linkslot API 的 H700 AArch64 工具链、SD 启动链、板级硬件识别、基础 HAL 探测和模拟器核心构建研究。

## 目标

本项目只有一个前端：Linkslot。当前不建设多前端框架、前端插件加载器或通用 Frontend ABI。

本项目需要解决的是：Linkslot 上游持续更新时，尽可能保留上游代码原貌，让 H700 裸机相关差异集中在少量、可审查、可重复应用的适配代码中。上游版本更新后，应主要执行同步、检查和重新构建，不应每次重新移植 UI、存档、云同步和 MOD。

## 1. 推荐仓库结构

采用 Git subtree 将 Linkslot 上游源码保留在仓库的固定子目录；H700 / RG34XX 工程代码放在 subtree 之外：

```text
third_party/linkslot/       # Linkslot 上游镜像，由 subtree 同步
platform/h700/              # Allwinner H700 / RG34XX 专属 HAL 与板级驱动
runtime/lk/                 # LK 运行时适配：任务、锁、时钟、文件、网络等
integration/linkslot/       # Linkslot 与 H700 的集成入口、配置和适配器
cores/                      # Gambatte / mGBA / gpSP 等核心及其适配
patches/linkslot/           # 只有无法通过外部适配解决时才保留的补丁
docs/                       # 架构、上游基线和回归记录
```

选择 subtree 而非要求每次构建都初始化 Git submodule，主要是为了让仓库检出、CI 和发行源码包更简单；上游原始历史通过 subtree 同步保留，构建时不依赖浮动的远程分支。

不要把 H700 驱动、设备路径和板级条件散落写进 Linkslot 的业务模块。设备差异应由 `platform/h700` 与 `integration/linkslot` 提供。

## 2. 上游代码分级

同步 Linkslot 时，先按职责分类，不按目录名盲目复制。

| 类别 | 典型内容 | 处理方式 |
|---|---|---|
| A：优先原样复用 | 数据模型、游戏元数据、排序/筛选、UI 素材、文案、配置语义、云同步仲裁规则、MOD 格式和协议定义 | 尽量保持上游原样；用兼容测试锁定外部行为 |
| B：保留逻辑、替换后端 | 文件操作、时钟、任务/线程、输入设备、音频输出、显示提交、网络传输、持久化 | 将平台调用集中到适配接口；H700 实现自己的后端 |
| C：不直接移植 | Linux 启动入口、进程调用、shell 脚本、Linux 专用设备探测、`std::net::TcpListener` 服务端、依赖 Linux 动态库和窗口系统的部分 | 用 LK/H700 实现替代；只保留需要的协议、资源和用户行为 |

以上分类是审查原则，不代表当前所有 Linkslot crate 已经具备可直接复用的接口。必须通过逐 crate 编译探针和依赖图确认后，才能把某个 crate 标记为 A 类。

尤其注意：Linkslot 目前是 Rust 工作区，部分实现依赖 Rust `std`、线程、文件系统、网络和 Linux 进程接口；GammaOS Pico/LK 不是 Linux 运行环境。不能假设把 Linkslot 的 Cargo crate 加到 LK 工程就能直接运行。首要技术验证是确认所需 Rust 代码能否在选定的 LK/Rust 运行时方案下编译，以及需要提供多少运行时服务。若这部分成本过高，应缩小原样复用范围，优先复用协议、数据模型、资源和可独立测试的纯逻辑，而不是把整个 Linux 运行时搬进裸机系统。

## 3. 适配边界

H700 侧提供稳定的适配接口，不让 Linkslot 业务模块直接访问 SoC 寄存器或 LK 私有 API。至少需要逐步收敛以下能力：

- 文件与目录：枚举、读写、删除、重命名、原子替换、flush/barrier；
- 时间：单调时钟、墙上时钟、RTC；
- 并发：任务创建、互斥锁、事件/队列、取消与超时；
- 显示：提交帧缓冲、纹理/图像上传、Overlay 合成；
- 输入：按键状态、边沿、长按与组合键；
- 音频：PCM 提交、音量、静音和设备关闭；
- 网络：Wi-Fi 状态、TCP/TLS 客户端、HTTP 服务端、DNS；
- 电源：电池/充电状态、即时状态保存请求和完全断电。

接口优先按 Linkslot 现有代码实际需要设计，不提前建设完整 POSIX 模拟层。不要为“可能有用”实现庞大的 Linux syscall 兼容层。

## 4. Cargo 与源码组织规则

1. 不把 Linkslot 全部 crate 无差别合并为 H700 根 workspace 的成员。
2. 保留上游 Cargo workspace 结构；H700 根工程只通过 path dependency 引入确实需要的 crate，或通过薄封装 crate 组合它们。
3. 先尝试上游已有 feature、trait 或模块边界；确实缺少平台切换点时，优先增加小而通用的上游适配 seam。
4. 使用 `default-features = false` 或窄 feature 组合，避免把 Linux 主机窗口、桌面音频、游戏手柄和进程相关依赖带进 H700 构建。
5. 不复制一份长期分叉的 `slot-ui`、`slot-store` 或 `linkslot` 业务代码。确实无法复用的模块才在 H700 工程中重写，并记录原因。
6. 如果必须修改 subtree 内上游文件，修改应局部化、附带 `H700 adaptation` 注释，并在 `patches/linkslot/README.md` 记录原始路径、上游 commit、修改原因、测试和是否可向上游提交。能在上游解决的通用问题优先提 PR。

## 5. 上游更新流程

等大版本发布并选定正式基线后，再将 Linkslot 上游 remote 命名为 `linkslot-upstream`，首次导入到 `third_party/linkslot`。以下为届时使用的流程示意，当前不要执行：

```sh
git remote add linkslot-upstream https://github.com/luguiyi/linkslot.git
git fetch linkslot-upstream --tags

# 首次导入：使用确认过的 release tag，而不是浮动的 main
# 将 <release-tag> 替换为实际版本标签
git subtree add --prefix=third_party/linkslot linkslot-upstream <release-tag> --squash

# 后续更新：先确认新 tag 对应的 commit SHA，再用固定 SHA 同步
git fetch linkslot-upstream --tags
git subtree pull --prefix=third_party/linkslot linkslot-upstream <new-commit-sha> --squash
```

首次导入前确认目标目录不存在或为空，不得覆盖本项目工程文件。每次同步均记录新旧上游 commit SHA 和 tag；正式构建不直接追踪浮动的 `main`。

每次同步必须经过：

1. 上游差异摘要：新增/删除 crate、公共 API、Cargo feature、依赖、数据格式、目录和用户功能变化；
2. 适配差异审查：确认 H700 是否需要修改接口或局部补丁；
3. 编译探针：上游复用部分在主机测试目标与 H700 目标上分别编译；
4. 行为回归：游戏库、配置、存档、即时状态、云同步冲突处理、Web API、Lua MOD；
5. 核心与实机回归：GB/GBC/GBA 启动、音视频、按键、存档；
6. 更新 `docs/upstream-baselines.md`，附上新 SHA、差异摘要、未移植功能和测试结果。

上游更新不能自动升级用户 SD 卡上的数据；任何数据格式迁移必须独立、可验证并保留备份。

## 6. 自动化应承担的工作

后续 CI 应至少包括：

- 检查当前 subtree 对应的上游 SHA 是否记录在 `docs/upstream-baselines.md`；
- 输出 subtree 相对上游的差异文件清单，提示意外扩大本地补丁面；
- 对复用的业务逻辑运行上游单元测试或等价的兼容性测试；
- 编译 Linkslot 集成目标和 H700 目标；
- 检查禁止进入 H700 构建的依赖（窗口系统、桌面输入/音频、Linux 进程调用等）；
- 生成每次上游同步的机器可读摘要和人工审查报告。

初期可以先使用脚本和 GitHub Actions 生成报告，不必一开始就自动解决冲突或自动合并上游。

## 7. 第一个技术验证：上游复用审计

在大规模编写 HAL 前，先完成一份逐 crate 清单：

- crate 名称、用途和依赖图；
- 是否使用 `std`、线程、文件系统、网络、进程、动态库；
- 是否能在当前 H700/LK 工具链编译；
- 可直接复用的代码比例与必须替换的后端；
- 每个替换点对应的 H700 adapter；
- 跟进 Linkslot 下一次更新时最可能产生冲突的文件。

验收标准不是“整个 Linkslot 已经能编译”，而是确定一个可维护的最小复用集合、可自动验证的接口边界，以及不会随 Linkslot UI 更新而反复修改的 H700 代码区域。
