import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from PIL import ImageGrab
from pywinauto import Desktop

import probe
import smoke_lib as lib

SHOTS = lib.SHOTS


def tray_button(needle: str):
    desktop = Desktop(backend="uia")
    for window in desktop.windows():
        try:
            for button in window.descendants(control_type="Button"):
                if needle.lower() in button.window_text().lower():
                    return button
        except Exception:
            continue
    return None


def tray_window() -> int | None:
    for hwnd, title, _rect, _visible in probe.windows_all():
        if "[tray_icon_app]" in title:
            return hwnd
    return None


def menu_window(timeout: float = 5.0):
    deadline = time.time() + timeout
    while time.time() < deadline:
        for hwnd, title, rect, visible in probe.windows_all():
            if "#32768" in title and visible:
                return hwnd, rect
        time.sleep(0.2)
    return None


def menu(x: int | None = None, y: int | None = None) -> int:
    exe = lib.stage(lib.default_config())
    lib.start(exe, "--tray")
    if not lib.wait_pids():
        return lib.finish([lib.report("tray menu", False, "no process")])
    spot = (x, y) if x is not None and y is not None else None
    if spot is None:
        button = tray_button("RustShot")
        if button is None:
            lib.stop_apps()
            return lib.finish([lib.report("tray menu", False, "tray button not found")])
        rect = button.rectangle()
        spot = ((rect.left + rect.right) // 2, (rect.top + rect.bottom) // 2)
    probe.click(spot[0], spot[1], right=True)
    opened = menu_window()
    if opened is not None:
        ImageGrab.grab().save(SHOTS / "tray-menu.png")
        probe.key("ESC")
    lib.stop_apps()
    return lib.finish([lib.report("tray menu opens", opened is not None, str(spot))])


def smoke() -> int:
    exe = lib.stage(lib.default_config())
    mark = lib.mark_log()
    lib.start(exe, "--tray")
    results = [lib.report("tray process runs", lib.wait_pids(), "")]
    results.append(lib.report("tray start logged", lib.wait_log("started in tray", mark, 10), ""))
    window = tray_window()
    results.append(lib.report("tray icon registered", window is not None, ""))
    onscreen = [w for w in probe.windows() if "RustShot" in w[1] and w[2][0] > -10000]
    results.append(lib.report("no panel on tray start", not onscreen, f"windows={len(onscreen)}"))
    lib.stop_apps()
    return lib.finish(results)


def locate(needle: str) -> int:
    button = tray_button(needle)
    if button is None:
        print("tray button not found")
        return 1
    rect = button.rectangle()
    print(f"found {button.window_text()!r} at {rect.left},{rect.top},{rect.right},{rect.bottom}")
    print("center", (rect.left + rect.right) // 2, (rect.top + rect.bottom) // 2)
    return 0


def click(needle: str) -> int:
    button = tray_button(needle)
    if button is None:
        print("tray button not found")
        return 1
    button.click_input()
    time.sleep(1.2)
    ImageGrab.grab().save(SHOTS / "tray-uia-click.png")
    print("clicked")
    return 0


def main(argv: list[str]) -> int:
    if not argv:
        return locate("RustShot")
    command = argv[0]
    if command == "smoke":
        return smoke()
    if command == "menu":
        if len(argv) >= 3:
            return menu(int(argv[1]), int(argv[2]))
        return menu()
    if command == "click":
        return click(argv[1] if len(argv) > 1 else "RustShot")
    return locate(command)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
