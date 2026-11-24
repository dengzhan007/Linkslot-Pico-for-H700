# armGBA（GBA 第三核心，可选）

- 上游：https://github.com/MagnominusGit/armGBA
- 固定 commit：`dda6df08cf06509a85c52aca92c24e9abc9b0a3f`
- 许可证：GPL-2.0（含 Gambatte；另有 MPL-2.0/LGPL/MIT/Expat 片段，见
  `licenses/armgba-*`）
- 交付方式：LSP 扩展包，不随基础卡包分发；id 为 `armgba`，芯片丝印 `armGBA`。
- 联机：不支持。它的联机是 RetroArch netplay 自己的一套，linkSlot 的
  电缆/netpacket 会话不接；要联机请用 mGBA / gpSP。

## 构建（bullseye arm64 容器）

```bash
docker run --rm --platform linux/arm64 \
  -v "$PWD:/src" -w /src rust:1-bullseye \
  sh -lc 'sh cores/armgba/build.sh build \
    dda6df08cf06509a85c52aca92c24e9abc9b0a3f \
    /src/target/cores/armgba-work \
    /src/target/cores/armgba_libretro.so'
```

首次运行会在容器里装 `cmake`（`g++`、`zlib1g-dev` 通常已随镜像存在）。产物：

- `target/cores/armgba_libretro.so`（aarch64，GLIBC 2.29）
- `target/cores/armgba-work/armGBA-<commit>.tar.gz`（GPL 对应源码）

如果 Docker 访问不了 GitHub，可以先把源码树放到
`target/cores/armgba-work/armGBA/`（带不带 `.git` 都可以），脚本会直接用它。
源码里的 `armulators` 子模块不需要：`ARMGBA_SDL=OFF` 只编 libretro 核心。

## 打包 LSP

```bash
python3 build/pack_lsp.py \
  --meta packs/armgba/pack.json \
  --payload System/linkSlot/cores/armgba/armgba_libretro.so=target/cores/armgba_libretro.so \
  --out dist/packs/linkslot-pack-armgba-v1.0.0.lsp
```
