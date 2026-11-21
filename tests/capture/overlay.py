import io
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from PIL import Image, ImageChops, ImageGrab

import probe
import smoke_lib as lib

REGION = (300, 200, 800, 600)
BASELINE: bytes | None = None


def run(steps: list[tuple[str, object]]) -> bytes | None:
    lib.clean_saved()
    exe = lib.stage(lib.default_config())
    lib.start(exe, "--capture")
    if lib.wait_window("RustShot Overlay") is None:
        lib.stop_apps()
        return None
    probe.drag(*REGION)
    time.sleep(0.3)
    for action, value in steps:
        if action == "key":
            probe.key(value)
        elif action == "drag":
            probe.drag(*value)
        elif action == "click":
            probe.click(*value)
        time.sleep(0.25)
    probe.key("ENTER")
    png = lib.wait_file(lib.SAVED)
    lib.stop_apps()
    return png.read_bytes() if png else None


def baseline() -> bytes | None:
    global BASELINE
    if BASELINE is None:
        BASELINE = run([])
    return BASELINE


def changed(a: bytes, b: bytes) -> float:
    left = Image.open(io.BytesIO(a))
    right = Image.open(io.BytesIO(b))
    return _ratio(left, right)


def _ratio(left: Image.Image, right: Image.Image) -> float:
    if left.size != right.size:
        return 1.0
    diff = ImageChops.difference(left.convert("RGB"), right.convert("RGB")).convert("L")
    pixels = left.size[0] * left.size[1]
    return sum(1 for value in diff.tobytes() if value > 8) / pixels


def undo() -> bool:
    clean = baseline()
    marked = run(
        [
            ("key", "2"),
            ("drag", (360, 260, 700, 520)),
            ("key", "CTRL+Z"),
        ]
    )
    if clean is None or marked is None:
        return lib.report("ctrl+z undoes the shape", False, "no png")
    delta = changed(clean, marked)
    return lib.report("ctrl+z undoes the shape", delta < 0.002, f"delta={delta:.4f}")


def tools() -> bool:
    clean = baseline()
    marked = run(
        [
            ("key", "3"),
            ("drag", (360, 260, 700, 340)),
            ("key", "4"),
            ("drag", (360, 380, 620, 520)),
            ("key", "7"),
            ("click", (520, 450)),
        ]
    )
    if clean is None or marked is None:
        return lib.report("arrow + pen + number", False, "no png")
    delta = changed(clean, marked)
    return lib.report("arrow + pen + number change pixels", delta > 0.01, f"delta={delta:.4f}")


def mosaic() -> bool:
    clean = baseline()
    marked = run(
        [
            ("key", "6"),
            ("drag", (360, 260, 700, 520)),
        ]
    )
    if clean is None or marked is None:
        return lib.report("mosaic", False, "no png")
    delta = changed(clean, marked)
    return lib.report("mosaic changes pixels", delta > 0.01, f"delta={delta:.4f}")


def text() -> bool:
    clean = baseline()
    lib.clean_saved()
    exe = lib.stage(lib.default_config())
    lib.start(exe, "--capture")
    if lib.wait_window("RustShot Overlay") is None:
        lib.stop_apps()
        return lib.report("text editor opens and esc drops it", False, "no overlay")
    probe.drag(*REGION)
    time.sleep(0.3)
    probe.key("5")
    time.sleep(0.3)
    before = ImageGrab.grab(bbox=(380, 250, 660, 370))
    probe.click(420, 300)
    time.sleep(0.8)
    after = ImageGrab.grab(bbox=(380, 250, 660, 370))
    editor = _ratio(before, after) > 0.01
    probe.key("ESC")
    time.sleep(0.4)
    probe.key("ENTER")
    png = lib.wait_file(lib.SAVED)
    lib.stop_apps()
    if clean is None or png is None:
        return lib.report("text editor opens and esc drops it", False, f"editor={editor} png={bool(png)}")
    delta = changed(clean, png.read_bytes())
    return lib.report(
        "text editor opens and esc drops it",
        editor and delta < 0.002,
        f"editor={editor} delta={delta:.4f}",
    )


def main(argv: list[str]) -> int:
    cases = {"undo": undo, "tools": tools, "mosaic": mosaic, "text": text}
    if not argv:
        print(" | ".join(cases))
        return lib.finish([undo(), tools(), mosaic(), text()])
    return lib.finish([cases[argv[0]]()])


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
