import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from PIL import ImageGrab

import smoke_lib as lib


def opens() -> bool:
    exe = lib.stage(lib.default_config())
    lib.stop_apps()
    lib.start(exe, "--panel")
    window = lib.wait_window("RustShot")
    if window is None:
        return lib.report("panel opens", False, "no window")
    left, top, right, bottom = window[2]
    width, height = right - left, bottom - top
    ImageGrab.grab(bbox=window[2]).save(lib.SHOTS / "panel-open.png")
    lib.stop_apps()
    ok = 440 <= width <= 560 and 440 <= height <= 580
    return lib.report("panel opens at startup size", ok, f"{width}x{height}")


def title_matches() -> bool:
    exe = lib.stage(lib.default_config())
    lib.stop_apps()
    lib.start(exe, "--panel")
    window = lib.wait_window("RustShot")
    lib.stop_apps()
    if window is None:
        return lib.report("panel title", False, "no window")
    return lib.report("panel title", "RustShot" in window[1], window[1])


def main(argv: list[str]) -> int:
    cases = {"open": opens, "title": title_matches}
    if not argv:
        print(" | ".join(cases))
        return lib.finish([opens(), title_matches()])
    return lib.finish([cases[argv[0]]()])


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
