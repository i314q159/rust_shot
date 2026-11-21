# TODO

## 1. 静默升级

**现状**：`src/update.rs` 只做「检查更新」——后台查 `releases/latest`，发现新版本就弹窗并 `explorer` 打开发布页，升级全靠用户手动下载安装。安装版在 `C:\Program Files\RustShot`（`{autopf}`），手工升级要过一遍安装向导。

**目标**：更新对话框多一个「立即更新」，下载 → 校验 → 静默安装 → 自动重启，全程只弹一次 UAC。

**要点**
- [ ] 复用 `update.rs` 的 WinHTTP 会话下载 `RustShot-Setup-<ver>.exe` 到 `%TEMP%\RustShot\update\`；现有 `read_body` 只收 UTF-8 文本，需要新增二进制 + 进度回调的读取路径（`WinHttpQueryDataAvailable` 循环，边下边报百分比）。
- [ ] 校验 SHA256：Release 里已有 `SHA256SUMS.txt`，下载后用 Windows CNG（`bcrypt`）或 `CertGetCertificateChain` 之外的最简方案算摘要比对；不一致直接判失败，不执行。
- [ ] 安装：`Setup /VERYSILENT /SUPPRESSMSGBOXES /NORESTART /CLOSEAPPLICATIONS`（与 `installer\build.ps1` 自测参数一致）。UAC 不可避免（写 `Program Files` + `HKLM\Run`），失败/取消时保留当前版本并退回「打开发布页」。
- [ ] 重启：安装程序结束后由新 exe 以 `--panel` 启动，旧进程退出；注意单实例互斥锁要先释放，否则新进程静默退出。
- [ ] 便携版分支：exe 不在 `Program Files` 且无卸载注册项时，下载 `RustShot-<ver>-portable.zip`，解压替换；运行中的 exe 不能覆盖，先把自己改名成 `.old` 再落新文件，退出后清理。
- [ ] 配置项：`auto_update=notice|silent`（默认 `notice`，只提示不擅自安装），写进 `config.txt` 并补 README 配置表。
- [ ] 版本对比与「已是最新」逻辑保持现有 `is_newer` 不动；升级前记录日志到 `%LOCALAPPDATA%\RustShot\rustshot.log`。

**验收**
- 低版本安装版触发升级 → 静默装成新版，版本号、`config.txt`（save_dir/主题/语言）、`HKLM\Run` 自启、PATH 均保持。
- 取消 UAC：现有安装完好，可继续用旧版本。
- 断网 / 摘要不匹配 / 安装包被 SmartScreen 拦：都有明确提示，不产生半装状态。
- `cargo fmt --check`、`clippy --all-targets -D warnings`、`cargo test` 全绿。

## 2. 代理端口（写进配置文件）

**现状**：`update.rs` 用 `WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY`，完全跟随系统代理设置。本机常开 Clash(7890)/v2ray(10808) 但没开「系统代理」时，检查更新直接失败，用户无法指定端口。

**目标**：`config.txt` 支持显式代理，检查更新与后续升级下载都走它；不配则行为与现在一致。

**配置设计**（`src/config.rs`）

```ini
# 空 = 跟随系统代理（默认）；direct = 直连；host:port 或只写端口 = 显式代理
proxy=
proxy=127.0.0.1:7890
proxy=7890
proxy=direct
```

**要点**
- [ ] `Config` 新增 `proxy: Proxy`（枚举 `System` / `Direct` / `Manual{host,port}`），`load` 解析时只写端口自动补 `127.0.0.1`，非法值忽略并记日志（沿用 BOM/无效键的容错风格）。
- [ ] `http_get` 按配置 `WinHttpOpen`：`System → AUTOMATIC_PROXY`、`Direct → ACCESS_TYPE_NO_PROXY`、`Manual → ACCESS_TYPE_NAMED_PROXY` + `WinHttpSetOption(WINHTTP_OPTION_PROXY, WINHTTP_PROXY_INFO{ lpszProxy: "host:port" })`。
- [ ] 失败文案带上当前代理模式（如「当前代理：127.0.0.1:7890」），方便判断是代理没开还是网络问题。
- [ ] 托盘/主面板「帮助 → 检查更新」对话框里显示代理来源一行，改配置后提示重启生效（与现有「保存后重启生效」一致）。
- [ ] README.md / README.en.md 配置段补 `proxy` 说明与示例。
- [ ] 单元测试：`proxy=` 各种写法的解析、端口越界（1–65535）拒绝、空值回落 System。

**验收**
- 关掉系统代理、仅 `proxy=127.0.0.1:7890`：检查更新成功拉到最新版本号。
- `proxy=direct` + 断网：报错清晰，不卡 12 秒以上（沿用现有超时）。
- 没有 `proxy` 键：行为与当前版本完全一致。
- 上述三条在 release 构建上实测，给出「检查项 | 结果」表。