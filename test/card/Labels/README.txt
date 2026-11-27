# Labels（卡带标签）

把 `.png` 放进 `GBA/`、`GB/`、`GBC/`、`FC/`、`SFC/` 或 `MD/`，文件名与 ROM 相同。
标签在所有设备上通用。

  Games/GBA/Metroid Fusion.gba  ->  Labels/GBA/Metroid Fusion.png

  卡带              标签窗口     比例      建议尺寸
  GBA               246 x 129    1.91:1    984 x 516
  Game Boy          186 x 166    1.12:1    744 x 664
  Game Boy Color    186 x 166    1.12:1    744 x 664
  Famicom           296 x 202    1.47:1    1184 x 808
  NES 卡带          121 x 200    0.61:1    484 x 800
  Super Famicom     305 x 106    2.88:1    1220 x 424
  Mega Drive        256 x 206    1.24:1    1024 x 824

建议尺寸 = 标签窗口 x4，比例必须和标签窗口一致。直接拷进卡的图会等比装入标签窗口、
不裁切（比例不符时四周留透明边，不会压扁）；用 Web 上传时按标签窗口裁满，并留下
标签窗口 2 倍的 PNG（GBA 492 x 258）。

FC 机种有两种卡带模型：Famicom 是横版标签，NES 卡带是竖版 121 x 200；NES 游戏建议
单独放一张竖版图。

# Shells（外壳）

`cart_shell.txt` 按每行一个游戏设置外壳：

  Metroid Fusion = auto 6a3fa0 solid

  Outline    auto、notched 或 rounded
  Colour     十六进制 RGB
  Finish     solid、clear 或 glitter
