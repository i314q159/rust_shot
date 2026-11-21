import ctypes
import ctypes.wintypes as wintypes
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from PIL import ImageGrab

import probe
import smoke_lib as lib

user32 = ctypes.windll.user32


def dialog() -> int | None:
    deadline = time.time() + 10
    while time.time() < deadline:
        for hwnd, title, _rect, visible in probe.windows_all():
            if visible and "#32770" in title:
                return hwnd
        time.sleep(0.15)
    return None


def dialog_text(hwnd: int) -> str:
    parts: list[str] = []
    proc = ctypes.WINFUNCTYPE(ctypes.c_bool, wintypes.HWND, wintypes.LPARAM)

    def collect(child, _):
        length = user32.GetWindowTextLengthW(child)
        if length:
            buffer = ctypes.create_unicode_buffer(length + 1)
            user32.GetWindowTextW(child, buffer, length + 1)
            parts.append(buffer.value)
        return True

    user32.EnumChildWindows(hwnd, proc(collect), 0)
    return "\n".join(parts)


def help_text(lang: str) -> str | None:
    exe = lib.stage(lib.default_config(lang=f"'{lang}'"))
    lib.start(exe, "--help")
    hwnd = dialog()
    if hwnd is None:
        lib.stop_apps()
        return None
    text = dialog_text(hwnd)
    ImageGrab.grab(bbox=lib.window_rect(hwnd)).save(lib.SHOTS / f"i18n-help-{lang}.png")
    lib.close_window(hwnd)
    time.sleep(0.4)
    lib.stop_apps()
    return text


def summary(text: str | None) -> str:
    if text is None:
        return "no dialog"
    flat = " ".join(text.split())
    return flat[:60]


def chinese() -> bool:
    text = help_text("zh")
    ok = text is not None and "--panel" in text and "全屏截图" in text
    return lib.report("help dialog in chinese", ok, summary(text))


def english() -> bool:
    text = help_text("en")
    ok = text is not None and "--panel" in text and "full screen" in text.lower()
    return lib.report("help dialog in english", ok, summary(text))


def main(argv: list[str]) -> int:
    cases = {"zh": chinese, "en": english}
    if not argv:
        print(" | ".join(cases))
        return lib.finish([chinese(), english()])
    return lib.finish([cases[argv[0]]()])


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
