# RustShot

**7 MB 的 Windows 截图 / 标注工具：`F1` 冻结屏幕、拖选、加几笔、回车即复制。**
单文件绿色版，或用安装包装进 `C:\Program Files`；无运行库依赖，配置跟着 exe 走。

> RustShot is a tiny screenshot & annotation tool for Windows. No runtime, no telemetry — one 7 MB exe for a USB stick, or a Setup.exe that installs machine-wide.

**简体中文** · [English](README.en.md)

[![Platform](https://img.shields.io/badge/platform-Windows%2010%20%7C%2011-0078D6?style=flat-square&logo=windows&logoColor=white)](https://github.com/i314q159/rust_shot)
[![Rust](https://img.shields.io/badge/rust-edition%202024-CE412B?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-3DA639?style=flat-square)](LICENSE-MIT.md)
[![Size](https://img.shields.io/badge/exe-7.5%20MB-lightgrey?style=flat-square)](#下载)
[![Runtime](https://img.shields.io/badge/runtime-none-success?style=flat-square)](#下载)
[![Offline](https://img.shields.io/badge/network-not%20required-success?style=flat-square)](#已知限制)

<p align="center">
  <img src="assets/screenshot-overlay.png" width="720" alt="框选后：暗化遮罩、尺寸提示与标注工具栏">
</p>

<p align="center"><sub>框选后：选区外的暗化遮罩、选区尺寸提示、工具栏（`1`–`7` 切工具 · 3 档线宽 · 6 色 · 撤销 / 复制 / 保存）</sub></p>

<p align="center">
  <img src="assets/screenshot-panel.png" width="300" alt="RustShot 主面板">
</p>

<p align="center"><sub>主面板：托盘常驻、保存目录、快捷键提示；整体浅色，压在原图上也不糊</sub></p>

---

## 目录

- [特性](#特性)
- [工作流](#工作流)
- [使用案例](#使用案例)
- [快捷键](#快捷键)
- [下载](#下载)
- [命令行](#命令行)
- [配置](#配置)
- [从源码构建](#从源码构建)
- [实现要点](#实现要点)
- [项目结构](#项目结构)
- [已知限制 / FAQ](#已知限制--faq)
- [许可证](#许可证)

---

## 特性

**截图** — `F1` 拖选，松手后仍可拖四角 / 四边微调或整体移动；不拖动直接单击则拾取光标下的窗口（DWM 真实边框，不含阴影）；`F2` 整屏截图并直接进入同一套标注工具（默认已选整屏，回车即复制，也可拖选缩小范围）。Per-Monitor V2，多显示器 / 高 DPI 下按光标所在屏幕冻结，1:1 物理像素。

**标注** — 快捷键 `1`–`7` 切工具，预览和导出共用同一套几何，所见即所得。

| 键 | 工具 | 说明 |
| --- | --- | --- |
| `1` | 选择 | 移动 / 缩放选区 |
| `2` | 矩形 | 描边矩形 |
| `3` | 箭头 | 带实心箭头的直线 |
| `4` | 画笔 | 自由手绘，圆头笔迹 |
| `5` | 文字 | 就地编辑，支持中文输入法 |
| `6` | 马赛克 | 按局部平均色像素化，笔刷打码 |
| `7` | 序号 | 自动递增的圆形编号 |

3 档线宽、6 种颜色、`Ctrl+Z` 逐笔撤销。

**输出** — `Enter` / `Ctrl+C` 复制到剪贴板（可选同时存 PNG）；`Ctrl+S` 另存为，同名文件自动追加 `_1` / `_2` 不覆盖；`Esc` / 右键取消。

**常驻** — 直接启动（双击 / 开始菜单 / 开机自启）就是托盘应用：窗口在第一帧之前就移出屏幕，桌面不会闪出面板，F1 / F2 随时可用；要看面板用 `--panel` 或托盘右键「显示主面板」。点 × 也是隐藏到托盘（窗口移到屏幕外并退出任务栏，不会残留在 Alt+Tab 里）；托盘左键直接截图，右键菜单切换区域 / 全屏 / 主面板 / 退出；单实例，重复启动不会重复弹窗。

**检查更新** — 顶部 `帮助 → 检查更新` 读取 GitHub 的 release 发布页，和当前版本号比较：发现新版本时弹窗列出发布说明，一键打开发布页下载；已是最新则直接提示。菜单栏右上角会出现 `可更新 vX.Y.Z` 标记，点开即可回看。只在点击时联网，不后台轮询；`api.github.com` 访问不了时自动改用发布页重定向兜底。

**主题** — 顶部 `主题` 菜单切换 `跟随系统 / 浅色 / 深色`，默认跟随 Windows 的浅色/深色设置：改系统设置时面板和截图工具栏一起变色，不用重启。手动选定的主题会写进 `config.txt`，固定使用、不再跟随系统。

**语言** — 界面中英双语，默认跟随 Windows 的显示语言（中文系统装中文，其它装英文）。顶部 `语言` 菜单可切 `跟随系统 / 中文 / English`，选定后写进 `config.txt`；切换即时生效，窗口标题、托盘菜单、截图工具栏、安装包界面一起换。

**为什么不用 Snipaste / 系统截图**

| | RustShot | 系统截图 | Snipaste |
| --- | --- | --- | --- |
| 体积 | 单文件 7.5 MB | 内置 | 安装包 + 依赖 |
| 标注 | 6 种 + 序号 + 马赛克 | 基础画笔 | 齐全（含贴图、取色） |
| 便携配置 | `config.txt` 跟 exe 走 | — | 部分可配 |
| 贴图 / OCR / 滚动长截图 | 不做 | 不做 | 贴图有，OCR 无 |

取舍很明确：只把「冻结 → 圈选 → 标注 → 交付」这一条路做到没有多余步骤。

## 工作流

```
        F1                    拖选 / 点选                       1-7
  ┌───────────┐   ┌──────────────────────────┐   ┌────────────────────┐
  │ 冻结屏幕  │ → │ 选区（可再调整）+ 工具栏 │ → │ 矩形/箭头/文字/… │
  └───────────┘   └──────────────────────────┘   └─────────┬──────────┘
                                                            │
                              ┌─────────────────────────────┴───────────────┐
                              │  Enter 复制   ·   Ctrl+S 另存为   ·   Esc 取消 │
                              └──────────────────────────────────────────────┘
```

按下 `F1` 的瞬间屏幕已被冻结（GDI `BitBlt` + `CAPTUREBLT`）：背后的窗口继续滚动、视频继续播放，你圈到的仍是那一帧。

## 使用案例

四个最常见的用法，都是「按一个键 → 圈一下 → 回车」三步内结束。

### 1. 提 issue 前，把报错指出来

```
F1 → 框选报错窗口 → 2 画红框圈住报错行 → 7 点一个序号 → Enter
```

圈完直接粘进 GitHub issue / Jira。上面那张演示图就是这条路：红框圈住正文、箭头指向右侧图表、序号 `1` 标出要讨论的位置。

### 2. 截图里带着自己的判断

```
F1 → 框选 → 3 箭头从结论指向数据 → 5 在空白处写一句"这里对不上" → Enter
```

文字是就地编辑，直接敲中文（支持输入法）；`Ctrl+Z` 逐笔撤销，画歪了不用重来。

### 3. 发给别人之前先打码

```
F1 → 框选 → 6 用马赛克笔刷涂掉姓名 / 手机号 / 内网地址 → Ctrl+S 另存
```

马赛克按局部平均色像素化，涂在导出的 PNG 里是真实像素，不是盖一层半透明色块。

### 4. 给文档配一整套过程图

```
逐张截图 → Ctrl+S（同名自动 _1 / _2，不覆盖）→ 直接拖进 Word / 飞书文档
```

开启「复制到剪贴板时同时保存 PNG」后，`Enter` 一次就同时进剪贴板、落到保存目录，写文档时不用来回切换。

| 场景 | 主要用到 | 产物 |
| --- | --- | --- |
| 报 bug / 提 issue | `F1` + 矩形 + 序号 | 剪贴板图片 |
| 评审、批注、讲结论 | `F1` + 箭头 + 文字 | 剪贴板图片 |
| 分享含隐私的界面 | `F1` + 马赛克 | PNG 文件 |
| 写文档、做教程 | `F1` / `F2` + 自动保存 | PNG 序列 |

## 快捷键

| 热键 | 动作 |
| --- | --- |
| `F1` | 区域截图 |
| `F2` | 整屏截图并进入标注（默认全选，回车复制） |
| `Ctrl+Alt+S` | 区域截图（备用，避免 `F1` 被其它软件占用） |
| `Ctrl+Alt+Q` | 退出 |

截图过程中：`1`–`7` 切工具、`Ctrl+Z` 撤销、`Ctrl+A` 全选、`Enter` / `Ctrl+C` 复制、`Ctrl+S` 保存、`Esc` / 右键取消。

## 下载

两种用法，二选一。

### 安装版（推荐）

到 [Releases](https://github.com/i314q159/rust_shot/releases) 下载 `RustShot-Setup-<版本>.exe`，双击安装，需要一次 UAC 确认。

| 项目 | 位置 |
| --- | --- |
| 程序 | `C:\Program Files\RustShot\`（所有用户可用） |
| 开始菜单 | `RustShot`、`卸载 RustShot` |
| 桌面图标 | 安装时勾选 |
| 开机自启 | 安装时勾选，写 `HKLM\…\Run`，参数 `--tray`（静默常驻托盘，不弹面板；与双击启动行为一致） |
| 命令行 | 安装时勾选，安装目录加入系统 `PATH`，任意目录可 `rust_shot` |
| 卸载 | 设置 → 应用 → RustShot，或开始菜单「卸载 RustShot」 |

安装包只写 `C:\Program Files\RustShot`，不会装到用户目录。卸载会清掉快捷方式、自启项和 PATH 条目（PATH 其它内容原样保留），截图文件与 `%LOCALAPPDATA%\RustShot` 里的设置不动。

### 便携版

下载 `rust_shot.exe`，双击即用：

1. 首次启动会在 **exe 同目录**生成 `config.txt`（便携配置）；
2. 放在 `Program Files` 等只读目录时，配置自动回退到 `%LOCALAPPDATA%\RustShot\`；
3. 想换默认保存位置：面板里点 `更改`，或直接编辑 `config.txt`。

空闲时约 65 MB 私有内存（工作集约 100 MB），无后台服务、无联网。

## 命令行

```powershell
rust_shot.exe                 启动后常驻托盘，不弹面板（已在运行则静默退出）
rust_shot.exe --panel, --show  启动并显示主面板（已有实例则唤出它的面板）
rust_shot.exe --capture, -c   立即区域截图（已有实例则转发给它）
rust_shot.exe --full, -f      立即全屏截图到剪贴板（不进标注界面，便于脚本调用）
rust_shot.exe --tray, --silent  与无参数相同，开机自启用
rust_shot.exe --selftest a.png  离屏渲染一张测试图并退出（校核导出管线）
rust_shot.exe --help, -h      用法说明
```

## 配置

设置写在**程序同目录的 `config.txt`**，四行键值对，用记事本就能改：

```ini
# RustShot 配置（保存后重启生效；save_dir 支持 %USERPROFILE% 这类环境变量）
save_dir=%USERPROFILE%\Pictures\RustShot
auto_save=true
theme=system
lang=system
```

| 键 | 说明 |
| --- | --- |
| `save_dir` | 保存目录，支持 `%USERPROFILE%`、`%TEMP%` 等环境变量 |
| `auto_save` | 复制到剪贴板时是否同时存 PNG（`true` / `false`） |
| `theme` | 界面主题：`system` 跟随 Windows / `light` 浅色 / `dark` 深色，和顶部「主题」菜单是同一个开关 |
| `lang` | 界面语言：`system` 跟随 Windows / `zh` 中文 / `en` English，和顶部「语言」菜单是同一个开关 |

面板上的 `更改` / `重置` 与顶部「主题」菜单会立刻写回配置文件；旧版本存放在 `%LOCALAPPDATA%\RustShot\config.txt` 的设置，首次启动时自动迁移。

诊断日志：`%LOCALAPPDATA%\RustShot\rustshot.log`（只记启动、异常和 panic）。

## 从源码构建

```powershell
git clone https://github.com/i314q159/rust_shot.git
cd rust_shot
cargo build --release
# 产物：target\release\rust_shot.exe
```

需要 MSVC 工具链（`rustup default stable-msvc`）。构建时 `build.rs` 会把 `assets/icon/rustshot.ico` 写进 exe 资源；机器上没有 `rc.exe` 只告警、不中断。

### 打安装包

```powershell
winget install JRSoftware.InnoSetup      # 只需一次
pwsh -File installer\build.ps1           # cargo build --release + 复制 exe + 编译安装包 + 静默安装
# 产物：outputs\RustShot-Setup-<版本>.exe，同时刷新 outputs\rust_shot.exe
# 安装日志：outputs\install-<版本>.log
```

安装脚本是 `installer\rust_shot.iss`（Inno Setup 6），版本号从 `Cargo.toml` 读取，不手写第二份。安装包固定装到 `C:\Program Files\RustShot`，界面四页：欢迎 → 附加设置 → 安装 → 完成。安装包自带 English / 简体中文，启动时弹语言选择并按系统语言预选（`installer\lang\*.isl` 放两套文案，`/LANG=english` 可跳过选择）。

`build.ps1` 默认在打包完成后立刻安装这次编出来的 `RustShot-Setup-<版本>.exe`（静默、不重启；非管理员会话弹一次 UAC）。若 RustShot 正在运行，脚本先尝试正常关闭，必要时强制结束，避免被安装包的占用检测拦住；装完再读卸载注册表里的版本号校验。

| 参数 | 作用 |
| --- | --- |
| `-NoInstall` | 只编译打包，不安装 |
| `-Interactive` | 安装时显示进度界面（默认完全静默） |
| `-KeepAppRunning` | 不自动关闭正在运行的 RustShot，脚本直接报错提醒先退出 |

无人值守安装/卸载（例如分发脚本）：

```powershell
RustShot-Setup-0.1.1.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART
"C:\Program Files\RustShot\unins000.exe" /VERYSILENT /SUPPRESSMSGBOXES /NORESTART
```

`build.ps1` 用的就是第一条命令。两者都需要管理员权限；安装包不接受 `/CURRENTUSER`，不会写到 `%LOCALAPPDATA%`。

### 测试

```powershell
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test          # 45 项：箭头几何、序号对比色、选区夹取、控制点命中、配置读写/环境变量、六种标注导出绘制
```

## 实现要点

| 关注点 | 做法 |
| --- | --- |
| 屏幕采集 | GDI `BitBlt` + `CAPTUREBLT`（含分层窗口），`GetDIBits` 取 BGRA 转 RGBA |
| DPI | 进程 Per-Monitor V2，全程物理像素；覆盖层用 `SetWindowPos` 按物理像素钉住显示器 |
| 覆盖层 | 冻结图作为背景纹理铺满窗口 + 选区外暗化遮罩，无需真透明窗口 |
| 坐标 | 标注几何统一存"帧内物理像素"，预览除缩放、导出直接用，做到像素级对齐 |
| 标注预览 | egui `Painter` 画矩形/箭头/画笔/马赛克块，文字用 `TextEdit` 以支持输入法 |
| 导出合成 | `tiny-skia` 画矢量（抗锯齿），`fontdue` 栅格化文字，合成进裁剪后的 RGBA |
| 热键 / 托盘 / 单实例 | 独立线程跑隐藏消息窗口：`RegisterHotKey`、互斥体单实例、托盘菜单事件入队后唤醒 UI |
| 检查更新 | WinHTTP（系统自带，不加 TLS 依赖）后台线程请求 release，`/releases/latest` 的跳转地址兜底；返回的 JSON 用约 100 行扫描器取字段，不引入 serde |
| 主题 | 读 `HKCU\…\Themes\Personalize\AppsUseLightTheme` 判断系统深浅色；隐藏的顶层窗口接 `WM_SETTINGCHANGE` 唤醒界面，浅/深两套 palette 一起注册，面板与截图工具栏共用一个颜色来源 |
| 语言 | 文案在调用处写成 `tr("中文", "English")`，`src/i18n.rs` 只负责当前语言、Windows UI 语言探测与 `{}` 占位替换；切换语言时同步改窗口标题与托盘菜单文字 |
| 字体 | 不打包字体：按固定顺序查找系统字体（微软雅黑 → 黑体 → 宋体 → Segoe UI），界面与导出共用同一套字形 |
| 主面板 | eframe/egui；`logic()` 负责事件（窗口隐藏时也执行），`ui()` 负责绘制与覆盖层视口 |
| 图标 | `assets/icon/` 下多尺寸 PNG/ICO，窗口与托盘内嵌 PNG，`build.rs` 把 ICO 写进 exe 资源 |

## 项目结构

```
rust_shot/
├─ src/
│  ├─ main.rs      命令行、单实例转发、自检
│  ├─ app.rs       主面板 UI、菜单栏、托盘、剪贴板 / 保存分发
│  ├─ overlay.rs   冻结覆盖层：选区交互、标注绘制、工具栏
│  ├─ shapes.rs    标注数据模型、egui 预览、tiny-skia/fontdue 导出合成
│  ├─ capture.rs   GDI 截屏、显示器 / 窗口几何
│  ├─ worker.rs    热键线程、单实例 IPC
│  ├─ update.rs    GitHub 发布页更新检查（WinHTTP + 极简 JSON 扫描）
│  ├─ i18n.rs      中英双语：语言选择、系统语言探测、文案替换
│  ├─ config.rs    便携配置读写、同名文件避让
│  ├─ fonts.rs     系统中文字体加载（egui + fontdue 共用）
│  ├─ win.rs       窗口定位（物理像素）
│  └─ util.rs      时间戳、日志
├─ assets/
│  ├─ icon/        图标源文件（ICO + 16–256 各尺寸 PNG + 预览）
│  ├─ screenshot-panel.png
│  └─ screenshot-overlay.png
├─ installer/
│  ├─ rust_shot.iss   Inno Setup 6 安装脚本（Program Files、快捷方式、PATH、自启、卸载）
│  └─ build.ps1       一键构建、打包并静默安装最新产物
├─ build.rs        把 ICO 写入 exe 资源
├─ v.txt           最近一次构建的版本号（由 build.ps1 写入，来源是 Cargo.toml）
├─ LICENSE-MIT.md
├─ README.md       （简体中文）
└─ README.en.md    （English）
```

## 已知限制 / FAQ

**只支持 Windows 10 / 11**：强依赖 Win32（GDI、DWM、`RegisterHotKey`）。

**截图是黑的？** 抓取受保护内容（部分 DRM 播放器、受保护窗口）时系统返回黑帧，与工具无关。

**隐藏到托盘后按热键，面板会不会跳出来？** 不会。隐藏时主面板不是最小化，而是被移到所有屏幕之外并退出任务栏——eframe 仍能从它身上创建截图覆盖层，所以 F1 / F2 / 托盘截图都不会把面板显示出来，也不会抢走焦点。只有你自己在任务栏上手动最小化了面板时，才需要为覆盖层临时恢复它（截图结束自动退回）。

**检查更新会自己联网吗？** 不会。只有点「检查更新」时请求一次 GitHub，走系统 WinHTTP（沿用系统代理设置），失败只提示不重试；想升级时点弹窗里的「打开发布页」自己下载安装包。

**双击启动后没看到窗口？** 正常，RustShot 是常驻托盘的工具：启动（含开机自启）不弹面板，窗口在第一帧之前就移出屏幕，直接按 F1 / F2 就能截图。要看主面板：托盘图标右键 →「显示主面板」，或运行 `rust_shot --panel`。已在运行时再启动一次是静默退出，不会多开、也不会抢焦点。

**没做**：滚动长截图、取色器、OCR、贴图、自动静默升级。

**想要另一种标注方式？** 开个 Issue 说说场景，比沉默更有用。

## 许可证

[MIT](LICENSE-MIT.md) © 2026 i314q159 —— 软件内也可直接查看：`帮助 → 开源声明（MIT）`。

---

<p align="center"><sub>如果这个小工具帮你省了几次来回，欢迎点个 ⭐ Star。</sub></p>