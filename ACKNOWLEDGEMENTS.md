# Acknowledgements

RustShot is a small, runtime-free single `.exe` not because everything was written from scratch, but because it stands on other people's work. Here are the people, projects and tools that made it possible.

---

## Programming

### Rust

The whole program is written in Rust. Thanks to the [Rust](https://www.rust-lang.org/) language and community:

- Ownership and zero-cost abstractions keep a tray-resident GUI fast and free of GC pauses;
- A statically linked binary ships with no runtime — copy the `.exe` to a USB stick and it runs;
- `cargo` and crates.io reduce dependency management to a few declarations.

Thanks as well to the open-source crates this project depends on directly — each of them carries a whole subsystem:

| crate | What it does here |
| --- | --- |
| [`eframe`](https://crates.io/crates/eframe) / [`egui`](https://crates.io/crates/egui) | Window, immediate-mode UI, every pixel of the overlay and toolbar |
| [`image`](https://crates.io/crates/image) | PNG / JPEG encode and decode for saved shots |
| [`arboard`](https://crates.io/crates/arboard) | Writing the annotated result to the clipboard |
| [`rfd`](https://crates.io/crates/rfd) | The native "Save as" dialog |
| [`tiny-skia`](https://crates.io/crates/tiny-skia) | Rasterising rectangles, arrows, brush strokes and text — preview and export share one geometry |
| [`fontdue`](https://crates.io/crates/fontdue) | Font metrics and text layout, so labels stay crisp in the exported file |
| [`tray-icon`](https://crates.io/crates/tray-icon) / [`muda`](https://crates.io/crates/muda) | Tray icon and its context menu |
| [`windows`](https://crates.io/crates/windows) | Direct Win32 access: GDI capture, DWM window frames, `RegisterHotKey`, WinHTTP |
| [`winres`](https://crates.io/crates/winres) | Embedding the ICO into the exe at build time |

### GitHub

The code and every release build live on [GitHub](https://github.com/i314q159/rust_shot). Push a `vX.Y.Z` tag and [GitHub Actions](https://github.com/features/actions) takes over: version check → `fmt` / `clippy` / `test` → Inno Setup packaging → SHA-256 checksum → GitHub Release. Free public repositories and CI runners give a single-maintainer Windows tool reproducible artifacts and an auditable release history — without a build machine to babysit.

### VS Code

Thanks to [Visual Studio Code](https://code.visualstudio.com/) as the main editor. It starts quickly, stays light on memory, and covers the daily loop: `rust-analyzer` for completion and navigation, `cargo fmt` / `clippy` / `test` in the integrated terminal, and side-by-side diffs for checking what changed. Its open extension API is also why assistants like CodeBuddy can move straight into the editor.

### ToDesk

Thanks to [ToDesk](https://www.todesk.com/) for making this project possible in the first place. RustShot was developed entirely through ToDesk remote sessions into a laptop on the local area network, packed with the AI tools I rely on. It kept that machine close — screen, keyboard and workflow across the LAN — so I could work from my usual desk while the models and tooling ran where they belonged. Without that bridge, this development setup would not have existed.

### Microsoft Documentation

Microsoft's Win32 / GDI / DWM documentation — on Windows, most screenshot answers live in those pages.

### Inno Setup

[Inno Setup](https://jrsoftware.org/isinfo.php) (Jordan Russell and contributors) — both the `C:\Program Files` installer and silent installs run on it.

## AI

### CodeBuddy

Thanks to [CodeBuddy](https://www.codebuddy.cn/) (Tencent's AI coding assistant) for completion, multi-file edits, repo-level Q&A and code review across the editor extension, standalone IDE and CLI. Two things mattered more than completion here. **Code review**: after a feature landed, it read the diff and caught the missed branch, the Chinese and English strings that had drifted apart, and the edge case that only appeared in one state. **Feature breakdown**: it turned work like "silent update" into independently compilable, verifiable steps — network layer and checksum first, then the UI switch, then install and restart — so each step was a rollback-friendly commit instead of two hundred changed lines followed by debugging. And when the borrow checker complained, asking it to explain the error first was still faster than staring at lifetimes alone.

### WorkBuddy

Thanks to [WorkBuddy](https://www.codebuddy.cn/work/), Tencent's desktop AI agent workbench. It reads and writes files and drives applications on the machine itself, taking over the chores that are not coding but cannot be skipped: sorting screenshot assets, archiving build outputs, drafting release notes from a template, batch-renaming files. For a single-maintainer open-source project, the bottleneck is rarely the code — it is all of that.

### QwenWork

[QwenWork](https://qwenwork.cn/) is this project's main AI pair programmer. It helped break down requirements and implement the UI and annotation logic, and sat through a long list of on-device debugging sessions: the overlay flashing black on its first frame, tray menu clicks doing nothing, and a synchronous native file dialog inside the paint callback freezing the whole screen. Each was reproduced, traced to its root cause and fixed with its help. The READMEs, build/release scripts and this page were put together with it too.

### DeepSeek

Thanks to [DeepSeek](https://www.deepseek.com/) for opening up model weights and inference tooling. Cheap, high-quality open models are what make a one-person-plus-AI tool like this iterable at all; the code reasoning and documentation behind RustShot benefited from them.

---

All projects, products and trademarks above belong to their respective owners. RustShot is not affiliated with or endorsed by them, and only uses them within the terms of their licences.

## And finally

Everyone who filed an issue, reported a bug or starred the repo. A three-word report like "it froze" often points at the real bug faster than a long write-up.

[Back to README](README.md)
