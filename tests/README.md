# tests

RustShot 的 Python 冒烟测试。目录与 `src` 一一对应，一个模块一个脚本，用合成鼠标 / 键盘输入、截图取证、读应用日志来断言。

```
tests/
├── probe.py          合成输入与截图原语（一次一条命令）
├── smoke_lib.py      启动 / 环境 / 日志 / 断言等共用工具
├── checks.py         逐场景手工校验，需要人眼看截图
├── requirements.txt  pillow、pywinauto
├── app/app.py        src/app
├── app/files.py          src/app/files.rs
├── capture/capture.py    src/capture
├── capture/overlay.py    src/capture/overlay
├── config/config.py      src/config
├── config/i18n.py        src/config/i18n
├── platform/platform.py  src/platform
├── theme/theme.py        src/theme
├── update/update.py      src/update
├── update/watch_download.py  下载进度快照
└── ui/
    ├── panel.py      src/ui/panel
    ├── dialogs.py    src/ui/dialogs
    └── tray.py       src/ui/tray
```

## 依赖

```powershell
python -m venv .venv
.\.venv\Scripts\python.exe -m pip install -r tests\requirements.txt
```

需要代理时加 `--proxy http://127.0.0.1:12334`。

| 包 | 用途 |
| --- | --- |
| `pillow` | 截屏（`ImageGrab`）、剪贴板取证、裁剪放大 |
| `pywinauto` | UIAutomation 定位任务栏托盘按钮 |

鼠标键盘输入只用 `ctypes` 调 `user32`。

## 前置条件

- 先 `cargo build --release`，脚本把 `target\release\rust_shot.exe` 复制到 `%TEMP%\rustshot-verify\app\`，每次都以干净状态启动。
- 运行期间会接管鼠标键盘，**机器必须空闲**；接管鼠标的只有 `capture`。
- 应用日志在 `%LOCALAPPDATA%\RustShot\rustshot.log`，断言基本都读它。
- 截图落在 `%TEMP%\rustshot-verify\shots\`。
- 脚本结束会 `taskkill /F /IM rust_shot.exe`，**会连托盘里正常运行的实例一起结束**。

## 冒烟用例

| 脚本 | 覆盖 |
| --- | --- |
| `app/app.py` | `--tray` 不弹窗、`--panel` 转发给运行中实例、关窗后隐藏到托盘 |
| `app/files.py` | `auto_save = false` 时只进剪贴板不落盘、`auto_save = true` 时落盘 |
| `capture/capture.py` | 区域截图 500×400 + 剪贴板、全屏截图、Esc 不落盘、标注后像素变化 |
| `capture/overlay.py` | Ctrl+Z 撤销、箭头 / 画笔 / 序号、马赛克、文字框打开与 Esc 丢弃 |
| `config/config.py` | `config.toml` 的 save_dir 生效、首次运行自动生成配置、旧 `config.txt` 不再迁移 |
| `config/i18n.py` | `--help` 弹窗随 `lang` 切中英文 |
| `platform/platform.py` | 日志写在 `%LOCALAPPDATA%\RustShot`、单实例互斥 |
| `theme/theme.py` | dark / light / system 三种偏好启动后的主题日志 |
| `update/update.py` | 非安装副本调用 `--apply-update` 时跳过安装（不打 UAC）、可选联网检查 |
| `ui/panel.py` | 主面板启动尺寸与标题 |
| `ui/dialogs.py` | 检查更新时面板回显并承载更新弹窗 |
| `ui/tray.py` | 托盘进程、无面板、托盘图标注册（`[tray_icon_app]`）；`menu x y` 手动右键验证菜单 |

```powershell
python tests\app\app.py
python tests\app\files.py
python tests\capture\capture.py
python tests\capture\overlay.py
python tests\config\config.py
python tests\config\i18n.py
python tests\platform\platform.py
python tests\theme\theme.py
python tests\update\update.py
python tests\ui\panel.py
python tests\ui\dialogs.py
python tests\ui\tray.py smoke
```

不带参数跑该脚本的全部用例；带参数只跑单个，例如 `python tests\capture\capture.py region`、`python tests\update\update.py online`。

退出码 0 全通过，1 有失败；每个用例打印 `PASS` / `FAIL`。

## 手动原语

```powershell
python tests\probe.py shot out.png
python tests\probe.py drag 300 200 800 600
python tests\probe.py key CTRL+Z
python tests\probe.py wins
```

## 已知限制

- `ui/dialogs.py` 的更新弹窗依赖 GitHub 网络；离线时走失败分支，日志断言仍成立但弹窗内容不同。
- `checks.py` 的 `uac-cancel` 需要系统 UAC 处于默认提示级别；`ConsentPromptBehaviorAdmin=0`（从不通知）时不会出现 UAC 窗口。
- DPI 相关行为需在 100% / 125% / 150% 下各跑一次。
