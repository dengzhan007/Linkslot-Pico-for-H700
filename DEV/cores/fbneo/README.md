# FBNeo 核心（Arcade / NeoGeo 平台包）

一份 `fbneo_libretro.so` 同时服务 `arcade` 与 `neogeo` 两个平台。核心**不随卡包
分发**，由 `linkslot-pack-arcade-neogeo-v1.0.0.lsp` 平台包安装（`manifest/cores.toml`
里标 `optional`）。ROM / BIOS 不进包；`neogeo.zip` 由用户放
`BIOS/fbneo/neogeo.zip`（或 `BIOS/neogeo.zip`）。

## 许可证（先读）

FBNeo 是 **Non-commercial**，不是 GPL。`licenses/fbneo-license.txt` 是上游
`src/license.txt` 的逐字副本，条款要点：

- 不得出售/出租/以 FBNeo 牟利；不得在商业产品/活动中使用；
- 必须公开你对源码的修改（本项目不打补丁，无需公开改动）；
- 分发时必须**逐字附带**完整许可证文本；
- 不得随核心分发 ROM；
- 不得为任何使用 FBNeo 源码的项目募捐。

上游 `src/burner/libretro/README.md` 还写明：商业前端若**再分发** FBNeo，属于
不被支持/不合法；只有**用户手动安装**核心才被视为合法。因此：

- 非商业、免费分发的卡包/扩展包：可作为可选 LSP 提供；
- 商业分发（卖卡、卖机器预装、含广告/赞助）：**不要打包**，改为用户手动放
  `System/linkSlot/cores/fbneo/fbneo_libretro.so`。

## 构建

```sh
# 1) 取对应源码（发布时同时作为 GPL/许可义务的源码记录）
curl -fL -o vendor/fbneo-src.tar.gz \
  https://codeload.github.com/libretro/FBNeo/tar.gz/<commit>

# 2) 编译（aarch64 主机直接编；其他主机自动用 bullseye 交叉工具链）
sh cores/fbneo/build.sh build <commit> vendor/fbneo-src.tar.gz \
  target/cores/fbneo vendor/fbneo_libretro.so
```

手工等价命令（在 bullseye 容器里）：

```sh
make -C src/burner/libretro -j16 platform=unix \
  CC=aarch64-linux-gnu-gcc CXX=aarch64-linux-gnu-g++ AR=aarch64-linux-gnu-ar
aarch64-linux-gnu-strip --strip-unneeded fbneo_libretro.so
```

- 默认 `SUBSET=all`：全量驱动集，Arcade + NeoGeo 共用；
- 设备基线 glibc 2.31；交叉工具链也是 bullseye（glibc 2.31）sysroot；
- 交叉只是省时间，产出的仍是 aarch64 ELF。

## 实测（2026-11-23，commit `3f0de7fe`）

| 项 | 值 |
|---|---|
| 上游 commit | `3f0de7fefbb7cc4b46e832647215cbe41eaf37b0`（2026-11-22） |
| 构建环境 | Debian bullseye 11.11，gcc 10.2.1（aarch64 交叉） |
| 全量编译耗时 | 96 s（-j16，交叉） |
| 未 strip | 93,511,096 B（89.2 MiB） |
| strip 后 | **83,492,832 B（79.6 MiB）** |
| sha256 | `79ef304cf02c3c45af60870ae4f0853aa254fe573ec087f150da8bec641842ae` |
| 最高 GLIBC | **2.29**（设备 2.31，通过） |
| NEEDED | libpthread / libstdc++ / libm / libgcc_s / libc |
| `.bss` | ≈ 99.7 MiB（运行时内存，不含堆） |
| `-Os` + 关 7z 的体积下限 | 73,169,768 B（69.8 MiB），仍远超限制 |

libretro 接口事实（实测 + 源码）：

- `retro_api_version() = 1`；
- `need_fullpath = true`、`block_extract = true` → 必须走**路径加载**，
  当前 `slot-retro` 会把整个 zip 读进内存再传 data，街机 set 大时需要改成
  path-only；
- `valid_extensions = zip|7z|cue|ccd|chd|m3u`；
- 内容/BIOS 查找顺序：`<system>/fbneo/patched/` → `<system>/fbneo/` → `<system>/`，
  所以 `BIOS/fbneo/neogeo.zip` 与 `BIOS/neogeo.zip` 都能找到；
- `retro_load_game(NULL)` 返回 false（没有“无 ROM 启动”），无内容时 `retro_run`
  会崩溃——60 帧 smoke 必须用真 ROM。

## 体积门（未通过）

`crates/slot/src/packs.rs` 当前限制是单文件 32 MiB、payload 64 MiB。全量 FBNeo
strip 后 79.6 MiB，过不了任何一条；即使 `-Os` + 关 7z 也只能到 69.8 MiB。

**决定（2026-11-23）**：放宽到单文件 96 MiB、payload 128 MiB，Phase 2 落地
（同时改 Web 上传上限，并把安装改成流式落盘）。详见
`doc/arcade-phase0-fbneo-spike.md`。
