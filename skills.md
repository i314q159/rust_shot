# skills.md

给 AI 助手和新贡献者的操作手册。这里只写「怎么动这个仓库」和「动之前必须知道的约束」，不重复 README 的产品介绍。

## 项目快照

- Rust 2024 edition，Windows 专用，单 exe，无运行库依赖。
- GUI 用 `eframe` / `egui`（glow 后端），抓屏走 GDI，托盘用 `tray-icon` + `muda`，更新走 WinHTTP。
- 发行物只有安装包 `RustShot-Setup-<version>.exe` + `SHA256SUMS.txt`，不提供便携包。
- 目标：`x64compatible`，最低 Windows 10；安装需要管理员权限。

## 测试现状（先看这个）

- **Rust 侧零单测。** 源码内嵌测试已在 `c487492` 全部移除，`cargo test` 是空跑：CI 会执行、永远通过、守不住任何逻辑。别把它当保护网。
- **唯一的行为保障是 `tests/` 下的 Python 冒烟测试。** 目录与 `src` 一一对应（一个模块一个脚本），用 ctypes 合成鼠标键盘 + `ImageGrab` 截图取证 + 读 `rustshot.log` 断言。改了哪块就手动跑对应脚本，改完必须跑。

## 常用命令

```powershell
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test                          # 目前零用例，CI 仍会跑
cargo build --release

installer\build.ps1 -NoInstall      # 打包但不装（CI 用法）
installer\build.ps1                 # 打包 + 静默安装 + 以 --tray 启动
installer\build.ps1 -Interactive    # 安装时显示进度条
```

顺序固定：`fmt` → `clippy` → `test`。CI 第一关就是这三条，本地跑不过别提交。真正的行为验证靠 `tests\` 下的 Python 冒烟测试（见「验证」）。

## 模块地图

| 路径 | 职责 |
| --- | --- |
| `src/main.rs` | 参数解析、单实例转发、`eframe` 启动 |
| `src/app/` | `App` 状态与生命周期：`session`（截图会话与导出投递）、`window`（面板显隐、托盘泵、发起截图）、`files`（复制/保存落盘）、`poll`（异步结果回收：选目录、另存、复制、安装）、`theme`（主题/语言切换、代理、开始安装）、`worker`（全局热键线程 + 单实例互斥 + 命令转发） |
| `src/capture/mod.rs` | GDI 抓屏、DPI 感知、光标下窗口命中 |
| `src/capture/overlay/` | 标注界面：`canvas` 绘制、`draw` 形状输入、`input` 键鼠与选区手柄、`text` 文字编辑、`toolbar` 工具栏、`history` 撤销、`export` 导出 |
| `src/capture/shapes/` | 形状模型；`paint` 预览与 `render` 导出共用同一套几何 |
| `src/config/` | `config.toml` 读写、字体探测、i18n、代理 |
| `src/theme/` | 明/暗两套色板，统一走 `theme::bg()` 这类取值函数 |
| `src/ui/` | `tray` 托盘菜单、`panel` 主面板、`dialogs`（license / proxy / release） |
| `src/platform/` | `win.rs` Win32 杂项（离屏停放、任务栏、按标题找窗口）、`util.rs` 日志/路径/时间戳 |
| `src/update/` | 版本比较、GitHub Release 解析、WinHTTP、SHA-256、`apply` 更新助手 |

## 硬规则

- **clean code，不写注释。** 解释靠命名和函数切分，不靠注释。删注释后留下的空行要一并清掉，否则 `cargo fmt --check` 会挂。
- **命令用 PowerShell**，不要 Git Bash。
- **exe 名固定 `rust_shot.exe`**，不要改（安装包、更新器、注册表、互斥量都依赖它）。
- 需要网络时走代理 `127.0.0.1:12334`。
- 配置项、文案、主题色都必须走各自的中枢（见下），不要就地硬编码。
- `target/`、`outputs/`、`release-notes.md`、`config.toml`、`*.log` 是本地产物，已被忽略，不要提交。
- 改完必须跑 `tests/` 下对应模块的冒烟脚本——Rust 侧没有单测兜底。

## 必须保持的契约

改这些地方会连带崩掉别处：

1. **版本号唯一来源是 `Cargo.toml` 的 `[package]` 段 `version`。** `build.ps1` 和 `release.yml` 都用限定 `[package]` 的正则提取，tag 必须是 `vX.Y.Z` 且与之一致，否则 CI 在第一步就失败。
2. **`SHA256SUMS.txt` 用两个空格分隔** hash 和文件名。`update/checksum.rs` 的 `checksum_for` 用 `split_once("  ")` 解析，CI 用 `'{0}  {1}'` 生成。单空格 = 校验永远失败。
3. **发行物命名**：`RustShot-Setup-<version>.exe`、`SHA256SUMS.txt`、安装后的 `rust_shot.exe`。更新器按这些名字拼 URL。
4. **`Tool::all()` 的下标就是数字键 1–7。** 调整工具顺序等于改快捷键语义。
5. **i18n 每条文案必须成对**（`zh` + `en`），占位符统一 `{}`，最多三个（`tr` / `tr1` / `tr2` / `tr3`）。只写一种语言会在切换语言时露馅。
6. **`config.toml` 的键名**：`save_dir` `auto_save` `theme` `lang` `auto_update` `proxy`。写入是 tmp + rename 原子替换；读取同时接受带引号和无引号。
7. **单实例**：互斥量 + 安装包 `AppMutex=RustShotSingleInstance`。已在运行时带 `--panel` / `--capture` / `--full` / `--check-update` 会转发给已有进程；无参数则静默退出。

## 发版流程

```powershell
# 1. 改版本（唯一来源）
#    Cargo.toml: version = "0.1.11"
# 2. 提交
git commit -am "chore: 0.1.11"
# 3. 打 tag 并推送（tag 触发全部 CI）
git tag v0.1.11
git push origin main --tags
```

CI 依次执行：版本校验 → `fmt` / `clippy` / `test`（test 为空跑，等于只过编译）→ `cargo build --release` → ISCC 打包 → 生成 SHA-256 → 建 Release（自动按提交生成发布说明，带 `-` 的 tag 视为预发布）。

发版前本地至少跑一次 `installer\build.ps1 -NoInstall`，确认 ISCC 能编译。

## 验证

冒烟测试的详细说明以 `tests/README.md` 为准，这里只留跑法：

```powershell
python -m venv .venv
.\.venv\Scripts\python.exe -m pip install -r tests\requirements.txt
cargo build --release
python tests\app\app.py            # 不带参数跑该脚本全部用例
python tests\capture\overlay.py undo    # 带参数只跑单个（undo / tools / mosaic / text）
```

- 前置条件：脚本把 `target\release\rust_shot.exe` 复制到 `%TEMP%\rustshot-verify\app\` 干净启动；**运行时会接管鼠标键盘，机器必须空闲**；结束时会 `taskkill /F /IM rust_shot.exe`，会连托盘里正常运行的实例一起杀掉。
- 脚本路径与 `src/` 同构：改 `src/capture/overlay/` → `python tests\capture\overlay.py`，改 `src/ui/tray.rs` → `python tests\ui\tray.py`，以此类推。同一模块下有多个脚本时按关注点分：`app/app.py`（启动/转发/隐藏）+ `app/files.py`（复制与落盘）、`config/config.py`（读写与首启）+ `config/i18n.py`（中英文文案）、`capture/capture.py`（抓屏与导出）+ `capture/overlay.py`（标注工具与撤销）。
- 断言主要读 `%LOCALAPPDATA%\RustShot\rustshot.log`，截图落在 `%TEMP%\rustshot-verify\shots\`，退出码 0 = 全通过。
- 需要代理时给 pip 加 `--proxy http://127.0.0.1:12334`。

更新链路手测（不用专门发一个假版本）：把已安装的 exe 复制到 `%TEMP%\RustShot\update\RustShot-Update.exe`，然后

```powershell
& "$env:TEMP\RustShot\update\RustShot-Update.exe" --apply-update 0 "$env:TEMP\RustShot\update\RustShot-Setup-x.y.z.exe" "C:\Program Files\RustShot\rust_shot.exe"
```

期望日志 `installed=true` → `setup exit code 0` → `ok=true`。第四个参数（真实安装路径）不能省，否则助手会把自己当成非安装副本，直接跳过安装。

## 已知坑

- **cargo 增量会误判 fresh**：`Copy-Item` 保留旧 mtime，改完 `Cargo.toml` 回滚版本号后不会重建。强制重建用 `(Get-Item src\main.rs).LastWriteTime = Get-Date`，`cargo clean -p rust_shot` 对版本已变的包无效。
- **PowerShell 5.1 不要对原生命令用 `2>&1 |`**：会抛 `NativeCommandError`，而 `build.ps1` 是 `ErrorActionPreference = 'Stop'`，直接中断。
- **`rfd` 对话框不能在 paint 回调里同步调用**，会冻住整屏。另存为走独立线程 + channel 回传（`app/session.rs` + `app/poll.rs::poll_save`），选目录同理（`poll.rs::pick_dir`）。
- **窗口"隐藏到托盘"是离屏停放 + `WS_EX_TOOLWINDOW`，不是 `ShowWindow(SW_HIDE)`**，因为 egui 需要窗口保持可见才能继续渲染。动之前先读 `platform/win.rs`。
- **release build 的 dead_code 告警**：只给 debug 用的常量或函数要加 `#[cfg(debug_assertions)]`，否则 `cargo build --release` 会告警；`cargo clippy` 在 dev profile 跑，抓不到这类问题。
- 应用更新失败大概率是权限或占用：安装目录在 `C:\Program Files`，替换需要 UAC；上一版进程没退干净会卡在安装阶段。

## 常见改动食谱

**加一条界面文案**

1. 在 `src/config/i18n/strings/` 下对应分类文件（`capture` / `general` / `net` / `panel` / `update`）追加 `pub const X: Text = Text { zh: "...", en: "..." };`
2. 分类文件已由 `strings/mod.rs` 统一 re-export，直接 `strings::X`。
3. 使用处 `tr(strings::X)`；带参用 `tr1(strings::X, value)`，文案里写 `{}`。

**加一个配置项**

1. `Config` 结构体加字段 + `Default` 值。
2. `parse` 里加键名分支，`render` 里加一行输出。
3. 面板控件改完调 `self.persist()`。
4. 旧配置文件缺这个键时要能落回默认值（`parse` 从 `default()` 起手，天然满足）。

**加一个标注工具**

1. `capture/overlay/mod.rs` 的 `Tool` 枚举加变体，补 `label()` 和 `icon()`。
2. 加进 `Tool::all()`，注意数字键映射。
3. `capture/shapes/` 里加 `Shape` 变体，并在 `paint.rs`（预览）和 `render.rs`（导出）两条路径都实现——只做一条会出现"预览有、导出没有"。
4. 撤销走 `history.rs`，形状构造走 `draw.rs` 的 `start_shape` / `update_shape` / `finish_shape`。

## 相关文档

- `README.md`：面向用户的功能与下载说明。
- `tests/README.md`：冒烟测试的权威说明（用例清单、前置条件、单条运行方式）。
- `todo.txt`：待办，保留跟踪，不要忽略或删除。
- `.github/workflows/release.yml`：发版全部细节（含自动生成的发布说明模板）。
- `installer/build.ps1`：本地打包 + 静默安装 + 版本校验脚本。
