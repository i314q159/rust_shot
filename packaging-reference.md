# RustShot 可分发应用打包参考

本文蒸馏了 `rust_shot` 项目把 Rust 桌面程序打包成 Windows 可分发应用的完整链路，可作为同类项目的复用骨架。

## 结论

单 exe + Inno Setup 安装包 + GitHub Release 分发 + 应用内自更新。版本、命名和校验规则全部收敛成硬契约，任何一处不一致都会让构建或更新链路直接失败。

## 产物边界

| 产物 | 来源 | 是否发布 |
| --- | --- | --- |
| `rust_shot.exe` | `cargo build --release` | 否，仅作为中间产物复制到 `outputs/` |
| `RustShot-Setup-<version>.exe` | ISCC 编译 `installer/rust_shot.iss` | 是 |
| `SHA256SUMS.txt` | CI 对安装包做 SHA-256 | 是 |
| `release-notes.md` | CI 根据 tag 间 git log 生成 | 作为 Release body |

不提供便携包；裸 exe 只用于本地验证和更新器 staging。

## 关键文件职责

| 文件 | 作用 |
| --- | --- |
| `Cargo.toml` | 唯一版本源；`[profile.release]` 开 `opt-level=3`、thin LTO、`codegen-units=1`、`strip=true` |
| `build.rs` | 用 `winres` 给 exe 嵌入图标、产品名、文件描述、语言 `0x0804`；失败只 warning 不阻断 |
| `installer/build.ps1` | 本地/共用的打包入口：读版本、release 构建、复制 exe、找 `ISCC.exe`、编安装包、可选静默安装并校验注册表版本 |
| `installer/rust_shot.iss` | Inno Setup 脚本：安装目录、权限、架构、语言、任务、文件、快捷方式、注册表、PATH 修改与卸载清理 |
| `installer/lang/*.isl` | 英文 / 简体中文安装界面文案 |
| `.github/workflows/release.yml` | tag `v*` 触发：版本一致性、质量门、安装 Inno Setup、复用 `build.ps1 -NoInstall`、SHA-256、发布说明、GitHub Release |
| `src/update/*` | 应用内更新：GitHub API/页面探测、下载、SHA-256 校验、helper 提权静默安装、重启 |

## 两条执行路径

### 本地

```powershell
installer\build.ps1 -NoInstall
installer\build.ps1
installer\build.ps1 -Interactive
```

流程：解析 `Cargo.toml` 版本，执行 `cargo build --release`，复制 `target\release\rust_shot.exe` 到 `outputs\`，按 PATH / LOCALAPPDATA / ProgramFiles(x86) / ProgramFiles 顺序找 `ISCC.exe`，执行 `ISCC /DMyAppVersion=$version`，生成 `outputs\RustShot-Setup-$version.exe`。默认还会静默安装、查注册表 `DisplayVersion` 校验、启动 `--tray`。

`-NoInstall` 只打包不安装；`-Interactive` 把 `/VERYSILENT` 换成 `/SILENT`；`-KeepAppRunning` 在程序仍运行时直接报错而不是强杀进程。

### CI（tag 触发）

1. 校验 `vX.Y.Z` 与 `Cargo.toml` 的 `version` 完全一致，不一致立即失败。
2. 质量门：`cargo fmt --check`，`cargo clippy --all-targets -- -D warnings`，`cargo test`。
3. `choco install innosetup`，确认 `ISCC.exe` 存在。
4. `pwsh -NoProfile -File installer\build.ps1 -NoInstall`。
5. 生成 `SHA256SUMS.txt`，格式为 hash + 两个空格 + 文件名。
6. 生成发布说明：下载表格、静默安装命令、快捷键、自上一个 tag 起的提交列表。
7. `softprops/action-gh-release` 上传安装包和校验文件；tag 含 `-` 自动标记预发布。

## 七条不能破的契约

1. 版本号唯一来源是 `Cargo.toml` 的 `[package] version`。`build.ps1` 和 CI 都用限定 `[package]` 段的正则提取，不读任何留档文件。
2. tag 规则：`v0.1.10` 对应 `version = "0.1.10"`，必须严格一致。
3. exe 名固定 `rust_shot.exe`，安装包、更新器、注册表、互斥量都依赖它。
4. 产物命名固定：`RustShot-Setup-<version>.exe`、`SHA256SUMS.txt`；更新器按此拼 URL。
5. `SHA256SUMS.txt` 必须 hash + 两个空格 + 文件名。`checksum_for` 用 `split_once("  ")` 解析，单空格会导致更新校验永远失败。
6. AppId / GUID 固定：`{B7E1C0F2-6B4A-4E1D-9A3C-2F5D8C1A7E64}`，卸载注册表查询依赖它。
7. 单实例互斥：`AppMutex=RustShotSingleInstance`，与代码里的 mutex 转发逻辑对应。

## 安装包设计要点

- `x64compatible`，最低 Windows 10，`PrivilegesRequired=admin`，默认装到 `{autopf}\RustShot`。
- `lzma2/max` + `SolidCompression=yes`，安装包体积小。
- 双语安装界面：`ShowLanguageDialog=yes`，`UsePreviousLanguage=no`。
- 三个可选任务：桌面图标、开机自启（写 `Run` 注册表带 `--tray`）、加入 PATH。
- PATH 修改用 Pascal Code 直接写 HKLM/HKCU 的 `Environment\Path`，卸载时反向删除；失败弹窗提示手动添加。
- `[InstallDelete]` 清理旧快捷方式，避免历史名称残留。
- `[Files]` 除 exe 外带上 `README.md` 和 `LICENSE-MIT.md`。

## 更新闭环

应用内检测 GitHub latest release（API 失败回退 releases 页面），解析 tag 比较版本，下载 `RustShot-Setup-<version>.exe`，从 `SHA256SUMS.txt` 取期望 hash 并比对，复制自身到 `%TEMP%\RustShot\update\RustShot-Update.exe`，等待父进程退出和 mutex 释放，以 `runas` 静默执行安装包，最后重启 `--panel`。

静默安装参数：`/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /NOCLOSEAPPLICATIONS`。网络层走 WinHTTP，支持配置代理 `127.0.0.1:12334`。

## 可移植到新项目的最小清单

1. 选一个版本唯一来源，所有脚本提取它。
2. 固定 exe 名、安装包名、校验文件名、GUID、mutex。
3. 写一个 `build.ps1` 同时服务本地和 CI，用 `-NoInstall` 区分是否安装。
4. 用 Inno Setup 管安装目录、快捷方式、自启、PATH、卸载清理。
5. CI 在 tag 上做版本一致性校验，再复用同一个打包脚本，避免两套逻辑漂移。
6. 校验文件格式和解析代码必须成对设计。
7. 如果做自更新，发布产物命名必须被更新器当作协议对待。

## 已知坑

- Rust 侧目前零单测，`cargo test` 是空跑；真正行为保障是 `tests/` 下的 Python 冒烟测试。
- cargo 增量可能误判 fresh，改版本号后必要时用 `(Get-Item src\main.rs).LastWriteTime = Get-Date` 触发重建。
- PowerShell 5.1 不要对原生命令用 `2>&1 |`，会抛 `NativeCommandError`，而 `build.ps1` 是 `ErrorActionPreference = 'Stop'`，会直接中断。
- 应用更新失败大概率是权限或占用：安装目录在 `C:\Program Files`，替换需要 UAC；上一版进程没退干净会卡在安装阶段。
