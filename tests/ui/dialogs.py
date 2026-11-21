import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from PIL import ImageGrab

import smoke_lib as lib


def update_check() -> bool:
    exe = lib.stage(lib.default_config(auto_update="'notice'"))
    lib.stop_apps()
    mark = lib.mark_log()
    lib.start(exe, "--check-update")
    checked = lib.wait_log("started to check for updates", mark, 10)
    result = lib.wait_log("update check", mark, 40)
    window = lib.wait_window("RustShot", 40)
    if window:
        ImageGrab.grab(bbox=window[2]).save(lib.SHOTS / "dialogs-update.png")
    lib.stop_apps()
    return lib.report(
        "update dialog hosted by panel",
        checked and result and window is not None,
        f"checked={checked} result={result} panel={bool(window)}",
    )


def main(argv: list[str]) -> int:
    cases = {"update": update_check}
    if not argv:
        print(" | ".join(cases))
        return lib.finish([update_check()])
    return lib.finish([cases[argv[0]]()])


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
