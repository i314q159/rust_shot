import shutil
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import probe
import smoke_lib as lib

CUSTOM = lib.SHOTS / "custom"


def custom_save_dir() -> bool:
    shutil.rmtree(CUSTOM, ignore_errors=True)
    CUSTOM.mkdir(parents=True, exist_ok=True)
    lib.clean_saved()
    exe = lib.stage(lib.default_config(CUSTOM))
    lib.stop_apps()
    lib.start(exe, "--capture")
    if lib.wait_window("RustShot Overlay") is None:
        return lib.report("config save_dir", False, "overlay did not open")
    probe.drag(300, 200, 800, 600)
    time.sleep(0.3)
    probe.key("ENTER")
    saved = lib.wait_file(CUSTOM)
    wrong = list(lib.SAVED.glob("*.png"))
    lib.stop_apps()
    return lib.report(
        "config save_dir honoured",
        saved is not None and not wrong,
        f"saved={saved.name if saved else None} stray={len(wrong)}",
    )


def created_on_first_run() -> bool:
    exe = lib.stage(None)
    lib.stop_apps()
    lib.start(exe, "--panel", env=lib.isolated_env())
    if lib.wait_window("RustShot") is None:
        return lib.report("config created on first run", False, "panel did not open")
    config = lib.APP_DIR / "config.toml"
    deadline = time.time() + 6
    while time.time() < deadline and not config.exists():
        time.sleep(0.2)
    lib.stop_apps()
    if not config.exists():
        return lib.report("config created on first run", False, "no config.toml")
    text = config.read_text(encoding="utf-8")
    ok = "save_dir" in text and "theme" in text
    return lib.report("config created on first run", ok, f"{len(text)} bytes")


def legacy_ignored() -> bool:
    old_dir = lib.SHOTS / "legacy"
    exe = lib.stage(None)
    (lib.APP_DIR / "config.txt").write_text(
        f"save_dir={old_dir}\r\nauto_save=false\r\ntheme=dark\r\n",
        encoding="utf-8",
    )
    lib.stop_apps()
    lib.start(exe, "--panel", env=lib.isolated_env())
    if lib.wait_window("RustShot") is None:
        return lib.report("legacy config.txt ignored", False, "panel did not open")
    config = lib.APP_DIR / "config.toml"
    deadline = time.time() + 6
    while time.time() < deadline and not config.exists():
        time.sleep(0.2)
    lib.stop_apps()
    if not config.exists():
        return lib.report("legacy config.txt ignored", False, "no config.toml")
    text = config.read_text(encoding="utf-8")
    leaked = str(old_dir) in text or "theme = dark" in text
    return lib.report("legacy config.txt ignored", not leaked)


def main(argv: list[str]) -> int:
    cases = {
        "save-dir": custom_save_dir,
        "first-run": created_on_first_run,
        "legacy": legacy_ignored,
    }
    if not argv:
        print(" | ".join(cases))
        return lib.finish([custom_save_dir(), created_on_first_run(), legacy_ignored()])
    return lib.finish([cases[argv[0]]()])


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
