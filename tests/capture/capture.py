import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from PIL import Image, ImageGrab

import probe
import smoke_lib as lib


def open_overlay(exe, *flags) -> int | None:
    lib.start(exe, *flags)
    window = lib.wait_window("RustShot Overlay")
    return window[0] if window else None


def overlay_gone(timeout: float = 8.0) -> bool:
    deadline = time.time() + timeout
    while time.time() < deadline:
        if not any("RustShot Overlay" in w[1] for w in probe.windows()):
            return True
        time.sleep(0.2)
    return False


def region() -> bool:
    lib.clean_saved()
    exe = lib.stage(lib.default_config())
    if open_overlay(exe, "--capture") is None:
        return lib.report("region capture", False, "overlay did not open")
    probe.drag(300, 200, 800, 600)
    time.sleep(0.3)
    probe.key("ENTER")
    png = lib.wait_file(lib.SAVED)
    clip = lib.clipboard_image()
    lib.stop_apps()
    size = Image.open(png).size if png else None
    clip_size = getattr(clip, "size", None)
    ok = size == (500, 400) and clip_size == (500, 400)
    return lib.report("region capture 500x400 + clipboard", ok, f"png={size} clip={clip_size}")


def full() -> bool:
    lib.clean_saved()
    exe = lib.stage(lib.default_config())
    screen = ImageGrab.grab().size
    lib.start(exe, "--full")
    clip = None
    deadline = time.time() + 15
    while time.time() < deadline:
        current = lib.clipboard_image()
        if getattr(current, "size", None) == screen:
            clip = current
            break
        time.sleep(0.3)
    png = lib.wait_file(lib.SAVED, timeout=4)
    lib.stop_apps()
    size = Image.open(png).size if png else None
    return lib.report(
        "full screen to clipboard + auto save",
        clip is not None and size == screen,
        f"png={size}",
    )


def cancel() -> bool:
    lib.clean_saved()
    exe = lib.stage(lib.default_config())
    if open_overlay(exe, "--capture") is None:
        return lib.report("esc cancels", False, "overlay did not open")
    probe.drag(300, 200, 800, 600)
    time.sleep(0.3)
    probe.key("ESC")
    closed = overlay_gone()
    files = list(lib.SAVED.glob("*.png"))
    lib.stop_apps()
    return lib.report("esc cancels without saving", closed and not files, f"png={len(files)} closed={closed}")


def annotate() -> bool:
    lib.clean_saved()
    plain = lib.stage(lib.default_config())
    if open_overlay(plain, "--capture") is None:
        return lib.report("annotation render", False, "overlay did not open")
    probe.drag(300, 200, 800, 600)
    time.sleep(0.3)
    probe.key("ENTER")
    first = lib.wait_file(lib.SAVED)
    lib.stop_apps()
    if first is None:
        return lib.report("annotation render", False, "no baseline png")
    baseline = first.read_bytes()

    lib.clean_saved()
    exe = lib.stage(lib.default_config())
    if open_overlay(exe, "--capture") is None:
        return lib.report("annotation render", False, "overlay did not reopen")
    probe.drag(300, 200, 800, 600)
    time.sleep(0.3)
    probe.key("2")
    probe.drag(360, 260, 700, 520)
    time.sleep(0.4)
    probe.key("ENTER")
    second = lib.wait_file(lib.SAVED)
    lib.stop_apps()
    if second is None:
        return lib.report("annotation render", False, "no annotated png")
    return lib.report("annotation render changes pixels", baseline != second.read_bytes())


def main(argv: list[str]) -> int:
    cases = {"region": region, "full": full, "cancel": cancel, "annotate": annotate}
    if not argv:
        print(" | ".join(cases))
        return lib.finish([region(), full(), cancel(), annotate()])
    return lib.finish([cases[argv[0]]()])


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
