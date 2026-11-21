import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import probe
import smoke_lib as lib

REGION = (300, 200, 800, 600)


def copy_without_save() -> bool:
    lib.clean_saved()
    exe = lib.stage(lib.default_config(auto_save="false"))
    lib.start(exe, "--capture")
    if lib.wait_window("RustShot Overlay") is None:
        lib.stop_apps()
        return lib.report("copy keeps no png when auto_save is off", False, "no overlay")
    probe.drag(*REGION)
    time.sleep(0.3)
    probe.key("ENTER")
    deadline = time.time() + 12
    clip = None
    while time.time() < deadline:
        current = lib.clipboard_image()
        if getattr(current, "size", None) == (500, 400):
            clip = current
            break
        time.sleep(0.3)
    time.sleep(1.2)
    files = list(lib.SAVED.glob("*.png"))
    lib.stop_apps()
    ok = clip is not None and not files
    return lib.report(
        "copy keeps no png when auto_save is off",
        ok,
        f"clip={getattr(clip, 'size', None)} png={len(files)}",
    )


def auto_save_writes() -> bool:
    lib.clean_saved()
    exe = lib.stage(lib.default_config(auto_save="true"))
    lib.start(exe, "--capture")
    if lib.wait_window("RustShot Overlay") is None:
        lib.stop_apps()
        return lib.report("auto_save writes the png", False, "no overlay")
    probe.drag(*REGION)
    time.sleep(0.3)
    probe.key("ENTER")
    png = lib.wait_file(lib.SAVED)
    lib.stop_apps()
    size = None
    if png is not None:
        from PIL import Image

        size = Image.open(png).size
    return lib.report("auto_save writes the png", size == (500, 400), f"png={size}")


def main(argv: list[str]) -> int:
    cases = {"copy": copy_without_save, "save": auto_save_writes}
    if not argv:
        print(" | ".join(cases))
        return lib.finish([copy_without_save(), auto_save_writes()])
    return lib.finish([cases[argv[0]]()])


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
