# 致谢

RustShot 能是一个小巧、无运行库依赖的单文件工具，靠的不是从零造轮子，而是站在别人的肩膀上。这里记下对它真正重要的几件事。

**简体中文** · [English](ACKNOWLEDGEMENTS.en.md)

---

## Rust

整个程序用 Rust 写成。感谢 [Rust](https://www.rust-lang.org/) 语言与社区：

- 所有权与零成本抽象，让一个常驻托盘的 GUI 进程既快又不带 GC 停顿；
- 静态链接后无需任何运行库，exe 拷到 U 盘就能跑；
- `cargo` 与 crates.io 把依赖管理压缩成几行声明。

也同样感谢这些直接依赖的开源 crate，它们各自承担了一整块工作量：

| crate | 在本项目里做什么 |
| --- | --- |
| [`eframe`](https://crates.io/crates/eframe) / [`egui`](https://crates.io/crates/egui) | 窗口、即时模式界面、覆盖层与工具栏的全部绘制 |
| [`image`](https://crates.io/crates/image) | PNG / JPEG 编解码，截图落盘 |
| [`arboard`](https://crates.io/crates/arboard) | 把标注结果写进剪贴板 |
| [`rfd`](https://crates.io/crates/rfd) | 原生「另存为」对话框 |
| [`tiny-skia`](https://crates.io/crates/tiny-skia) | 矩形、箭头、画笔、文字栅格化，预览与导出共用一套几何 |
| [`fontdue`](https://crates.io/crates/fontdue) | 字体度量与文字排版，保证标注文字导出后不糊 |
| [`tray-icon`](https://crates.io/crates/tray-icon) / [`muda`](https://crates.io/crates/muda) | 系统托盘图标与右键菜单 |
| [`windows`](https://crates.io/crates/windows) | 直接调用 Win32：GDI 抓屏、DWM 窗口边框、`RegisterHotKey`、WinHTTP |
| [`winres`](https://crates.io/crates/winres) | 构建期把 ICO 写入 exe 资源 |

## GitHub

代码托管在 [GitHub](https://github.com/i314q159/rust_shot)，每个版本的构建与发布也交给它：推一个 `vX.Y.Z` 标签，[GitHub Actions](https://github.com/features/actions) 就会跑完 版本校验 → `fmt` / `clippy` / `test` → Inno Setup 打包 → 便携版与 SHA-256 → 创建 Release 的全过程。免费的公开仓库与 CI 运行器，让一个人维护的 Windows 工具也能有可复现的构建产物和可回溯的发布记录，不用自己维护一台构建机。

## VS Code

感谢 [Visual Studio Code](https://code.visualstudio.com/) 作为主力编辑器：启动快、占用小，`rust-analyzer` 的补全与跳转、集成终端里的 `cargo fmt` / `clippy` / `test`、以及并排的 diff 视图，基本覆盖了这个项目从改代码到核对改动的全部日常。它开放的扩展接口，也正是 CodeBuddy 这类助手能直接住进编辑器的原因。

## CodeBuddy

感谢 [CodeBuddy](https://www.codebuddy.cn/)（腾讯云的 AI 编程助手）：在编辑器里以插件形态提供补全、多文件改写、仓库级问答与代码审阅，另有独立的 IDE 与 CLI 形态。写 RustShot 时它承担了两件比补全更值钱的事——**代码审阅**：一个功能改完先让它过一遍 diff，挑出漏改的分支、没同步的中英文案、以及只在特定状态才触发的边界；**功能拆分**：把「静默更新」这种一口气做不完的需求拆成能独立编译、独立验证的小步（先做网络层与校验，再做界面开关，最后接安装与重启），每一步都是一个可以回滚的提交，而不是一次改两百行再调试。碰到借用检查器报错时，让它先解释一遍，也比自己盯着生命周期硬啃快得多。

## WorkBuddy

感谢 [WorkBuddy](https://www.codebuddy.cn/work/)（腾讯云的桌面 AI 智能体工作台）。它能在本机直接读写文件、驱动应用，把那些与写代码无关但又躲不掉的琐事接过去：整理截图素材、归档构建产物、按模板起草发版说明、把一堆文件名批量改整齐。一个人维护一个开源项目，真正的瓶颈往往不是代码，而是这些零碎。

## QwenWork

[QwenWork](https://qwenwork.cn/) 是本项目的主力 AI 编程助手。需求拆解、界面与标注逻辑的实现、以及大量真机排错都由它协作完成——例如覆盖层首帧闪黑、托盘菜单点了没反应、在绘制帧里同步调用原生文件对话框导致整屏冻结这几类问题，都是靠它反复复现、定位到根因再改掉的。README、构建发布脚本和这份致谢也由它协同整理。

## DeepSeek

感谢 [DeepSeek](https://www.deepseek.com/) 开放模型权重与推理工具。高质量、可低成本获取的开放模型，是这类「一个人 + AI」的小工具能持续迭代的前提；本项目的代码推理与文档整理过程受益于这类开放模型。

## 还有

- 微软的 Win32 / GDI / DWM 文档——Windows 上做截图，答案几乎都在这几页里。
- [Inno Setup](https://jrsoftware.org/isinfo.php)（Jordan Russell 等）——`C:\Program Files` 安装版与静默安装都靠它。
- 每一个提 Issue、报 Bug、点 Star 的人。像「杀掉进程」「窗口冻结了」这种一句话反馈，往往比长篇描述更快指向真正的 bug。

---

以上项目、产品与商标归各自所有者所有；RustShot 与它们没有隶属或背书关系，只在许可允许的范围内使用。

[返回 README](README.md)