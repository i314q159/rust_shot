# Acknowledgements

RustShot is a 7 MB, dependency-free single exe not because everything was written from scratch, but because it stands on other people's work. These are the ones that matter.

[简体中文](ACKNOWLEDGEMENTS.md) · **English**

---

## Rust

The whole program is written in Rust. Thanks to the [Rust](https://www.rust-lang.org/) language and community:

- Ownership and zero-cost abstractions keep a tray-resident GUI fast and free of GC pauses;
- A statically linked binary needs no runtime — copy the exe to a USB stick and it runs;
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

## GitHub

The code lives on [GitHub](https://github.com/i314q159/rust_shot), and so does every release build: push a `vX.Y.Z` tag and [GitHub Actions](https://github.com/features/actions) runs version check → `fmt` / `clippy` / `test` → Inno Setup packaging → portable zip and SHA-256 → publishing the GitHub Release. Free public repositories and CI runners mean a single-maintainer Windows tool can still have reproducible artifacts and an auditable release history — no build machine to babysit.

## VS Code

Thanks to [Visual Studio Code](https://code.visualstudio.com/) as the main editor: quick to start and light on memory, with `rust-analyzer` completion and navigation, `cargo fmt` / `clippy` / `test` in the integrated terminal, and side-by-side diffs — between them they cover the whole daily loop from editing a file to checking what changed. Its open extension API is also why assistants like CodeBuddy can move straight into the editor.

## CodeBuddy

Thanks to [CodeBuddy](https://www.codebuddy.cn/) (Tencent's AI coding assistant), which offers completion, multi-file edits and repo-level Q&A as an editor extension, plus a standalone IDE and CLI. While building RustShot, completion and "change these files together" got the most use; when the borrow checker complains, having it explain the error first is usually faster than staring at lifetimes alone.

## QwenWork

[QwenWork](https://qwenwork.cn/) is this project's main AI pair programmer. It helped with breaking down requirements, implementing the UI and annotation logic, and a long list of on-device debugging sessions — the overlay flashing black on its first frame, tray menu clicks doing nothing, and a synchronous native file dialog inside the paint callback freezing the whole screen were all reproduced, traced to their root cause and fixed with its help. The READMEs, the build/release scripts and this page were put together with it too.

## DeepSeek

Thanks to [DeepSeek](https://www.deepseek.com/) for opening up model weights and inference tooling. Cheap, high-quality open models are what make a one-person-plus-AI tool like this iterable at all; the code reasoning and documentation work behind RustShot benefited from them.

## And also

- Microsoft's Win32 / GDI / DWM documentation — on Windows, most screenshot answers live in those pages.
- [Inno Setup](https://jrsoftware.org/isinfo.php) (Jordan Russell and contributors) — both the `C:\Program Files` installer and silent installs run on it.
- Everyone who filed an issue, reported a bug or starred the repo. A three-word report like "it froze" often points at the real bug faster than a long write-up.

---

All projects, products and trademarks above belong to their respective owners. RustShot is not affiliated with or endorsed by them, and only uses them within the terms of their licences.

[Back to README](README.en.md)