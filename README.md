# RustShot

一个用 Rust 写的 Windows 轻量级截图工具，参照 Snipaste / SnowShot 的交互习惯：
**冻结屏幕 → 框选 → 标注 → 复制 / 保存**，常驻托盘、全局热键随时唤起。

## 功能

**截图**
- 区域截图：拖动框选，四角/四边可再调整，支持拖动移动选区
- 窗口截图：不拖动直接单击，自动拾取光标下的窗口（DWM 真实边框，不含阴影）
- 全屏截图：一键整屏到剪贴板
- 多显示器 / 高 DPI：Per-Monitor V2，按光标所在显示器冻结，1:1 像素还原

**标注**（快捷键 `1`–`7`）
| 键 | 工具 | 说明 |
| --- | --- | --- |
| 1 | 选择 | 移动 / 缩放选区 |
| 2 | 矩形 | 描边矩形 |
| 3 | 箭头 | 带实心箭头 |
| 4 | 画笔 | 自由手绘，圆头 |
| 5 | 文字 | 就地编辑，支持中文（IME） |
| 6 | 马赛克 | 笔刷打码，按局部平均色像素化 |
| 7 | 序号 | 自动递增的圆形编号 |

另有 3 档线宽、6 种颜色、撤销（`Ctrl+Z`）。

**输出**
- `Enter` / `Ctrl+C`：复制到剪贴板（可选同时自动存 PNG）
- `Ctrl+S`：另存为（PNG）
- `Ctrl+A`：全选整屏；`Esc` / 右键：取消

**常驻**
- 关闭主面板（点 ×）默认**最小化到托盘**，不退出
- 托盘左键单击直接截图，右键菜单：区域截图 / 全屏截图 / 显示主面板 / 退出
- 单实例：重复启动会把请求转给已运行的实例

## 全局热键

| 热键 | 动作 |
| --- | --- |
| `F1` | 区域截图 |
| `F2` | 全屏截图到剪贴板 |
| `Ctrl+Alt+S` | 区域截图（备用，避免 F1 被其它软件占用） |
| `Ctrl+Alt+Q` | 退出 |

## 运行

直接运行 `rust_shot.exe`（无需安装、无运行库依赖）。

命令行参数：

```
rust_shot.exe                 启动主面板
rust_shot.exe --capture, -c   立即区域截图（已有实例则转发给它）
rust_shot.exe --full, -f      立即全屏截图到剪贴板
rust_shot.exe --selftest a.png  离屏渲染一张测试图（校核导出管线，不开窗口）
```

截图默认保存在 `%USERPROFILE%\Pictures\RustShot`，可在面板里更改；保存目录和"自动保存"开关会记在 `%LOCALAPPDATA%\RustShot\config.txt`。同名文件自动追加 `_1` / `_2` 后缀，不会覆盖。
诊断日志：`%LOCALAPPDATA%\RustShot\rustshot.log`（仅记录启动、异常与 panic）。

## 从源码构建

```powershell
cargo build --release
# 产物：target\release\rust_shot.exe
```

需要 MSVC 工具链（`rustup default stable-msvc`）。

## 测试

```powershell
cargo test      # 22 项：箭头几何、序号对比色、选区夹取、控制点命中、配置读写、六种标注导出绘制
```

## 设计要点

| 关注点 | 做法 |
| --- | --- |
| 屏幕采集 | GDI `BitBlt` + `CAPTUREBLT`（含分层窗口），`GetDIBits` 取 BGRA，转 RGBA |
| DPI | 进程 `Per-Monitor V2`，所有坐标都是物理像素；覆盖层窗口用 `SetWindowPos` 按物理像素钉住显示器，规避 winit 逻辑坐标换算 |
| 覆盖层 | 冻结图作为背景纹理铺满窗口，选区外做暗化遮罩，所以不需要真透明窗口 |
| 坐标 | 标注几何统一存"帧内物理像素"，预览时除以 `物理宽/窗口宽` 映射到 point，导出时直接用，做到预览与导出像素级对齐 |
| 标注预览 | egui `Painter`（矩形/箭头/画笔/马赛克块），文字用 egui 的 `TextEdit` 以获得输入法支持 |
| 导出合成 | `tiny-skia` 画矢量（抗锯齿），`fontdue` 用系统微软雅黑栅格化文字，合成进裁剪后的 RGBA |
| 热键 / 托盘 / 单实例 | 单独线程跑隐藏消息窗口：`RegisterHotKey`、单实例互斥体 + 注册消息转发 |
| 主面板 | eframe/egui；`logic()` 负责事件（窗口隐藏时也执行），`ui()` 负责绘制与覆盖层视口 |

## 已知限制

- 只支持 Windows（Win32 API 强依赖）
- 未做滚动长截图、取色器、OCR
- 覆盖层依赖主面板窗口处于"非最小化"状态（eframe 限制），从托盘唤起时会先恢复面板一帧，截图结束后自动退回托盘
- 抓取受保护内容（部分带 DRM 的播放器）时可能得到黑图

## 目录结构

```
src/
  main.rs     命令行、单实例转发、自检
  app.rs      主面板 UI、托盘菜单、剪贴板 / 保存分发
  overlay.rs  冻结覆盖层：选区交互、标注绘制、工具栏
  shapes.rs   标注数据模型、egui 预览、tiny-skia/fontdue 导出合成
  capture.rs  GDI 截屏、显示器/窗口几何
  worker.rs   热键线程、单实例 IPC
  config.rs   设置读写（保存目录/自动保存）、同名文件避让
  fonts.rs    系统中文字体加载（egui + fontdue 共用）
  win.rs      窗口定位（物理像素）
  util.rs     时间戳、日志
```
