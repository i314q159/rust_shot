import ctypes
import ctypes.wintypes as wintypes
import sys
import time
from pathlib import Path

from PIL import ImageGrab

user32 = ctypes.windll.user32

LEFTDOWN, LEFTUP, RIGHTDOWN, RIGHTUP = 0x0002, 0x0004, 0x0008, 0x0010
KEYUP = 0x0002

VK = {
    "ESC": 0x1B,
    "ENTER": 0x0D,
    "TAB": 0x09,
    "BACK": 0x08,
    "SPACE": 0x20,
    "CTRL": 0x11,
    "SHIFT": 0x10,
    "ALT": 0x12,
    "F1": 0x70,
    "F2": 0x71,
    "DOWN": 0x28,
    "UP": 0x26,
    "LEFT": 0x25,
    "RIGHT": 0x27,
    "MENU": 0x5D,
}


KEYEVENTF_UNICODE = 0x0004
INPUT_KEYBOARD = 1


class POINT(ctypes.Structure):
    _fields_ = [("x", wintypes.LONG), ("y", wintypes.LONG)]


class KEYBDINPUT(ctypes.Structure):
    _fields_ = [
        ("wVk", wintypes.WORD),
        ("wScan", wintypes.WORD),
        ("dwFlags", wintypes.DWORD),
        ("time", wintypes.DWORD),
        ("dwExtraInfo", wintypes.LPVOID),
    ]


class INPUT(ctypes.Structure):
    _fields_ = [
        ("type", wintypes.DWORD),
        ("ki", KEYBDINPUT),
        ("pad", ctypes.c_byte * 8),
    ]


def text(value: str) -> None:
    for char in value:
        for flags in (KEYEVENTF_UNICODE, KEYEVENTF_UNICODE | KEYUP):
            event = INPUT(
                type=INPUT_KEYBOARD,
                ki=KEYBDINPUT(0, ord(char), flags, 0, None),
            )
            user32.SendInput(1, ctypes.byref(event), ctypes.sizeof(INPUT))
            time.sleep(0.02)
        time.sleep(0.02)


def move(x: int, y: int) -> None:
    user32.SetCursorPos(x, y)
    time.sleep(0.05)


def pos() -> tuple[int, int]:
    point = POINT()
    user32.GetCursorPos(ctypes.byref(point))
    return point.x, point.y


def mouse(flag: int) -> None:
    user32.mouse_event(flag, 0, 0, 0, 0)


def click(x: int, y: int, right: bool = False) -> None:
    move(x, y)
    time.sleep(0.12)
    down, up = (RIGHTDOWN, RIGHTUP) if right else (LEFTDOWN, LEFTUP)
    mouse(down)
    time.sleep(0.06)
    mouse(up)
    time.sleep(0.12)


def drag(x1: int, y1: int, x2: int, y2: int, steps: int = 16, settle: float = 0.12) -> None:
    move(x1, y1)
    time.sleep(settle)
    mouse(LEFTDOWN)
    time.sleep(0.08)
    for i in range(1, steps + 1):
        move(x1 + (x2 - x1) * i // steps, y1 + (y2 - y1) * i // steps)
        time.sleep(0.022)
    time.sleep(settle)
    mouse(LEFTUP)
    time.sleep(settle)


def vk(name: str) -> int:
    key = name.upper()
    if key in VK:
        return VK[key]
    if len(key) == 1:
        return ord(key)
    raise SystemExit(f"unknown key {name}")


def tap(code: int) -> None:
    user32.keybd_event(code, 0, 0, 0)
    time.sleep(0.03)
    user32.keybd_event(code, 0, KEYUP, 0)
    time.sleep(0.03)


def key(combo: str) -> None:
    parts = [p for p in combo.split("+") if p]
    mods = [vk(p) for p in parts[:-1]]
    for code in mods:
        user32.keybd_event(code, 0, 0, 0)
        time.sleep(0.03)
    tap(vk(parts[-1]))
    for code in reversed(mods):
        user32.keybd_event(code, 0, KEYUP, 0)
        time.sleep(0.03)


def shot(path: str) -> None:
    target = Path(path)
    target.parent.mkdir(parents=True, exist_ok=True)
    image = ImageGrab.grab()
    image.save(target)
    print(f"shot -> {target} {image.width}x{image.height}")


def windows(pid: int | None = None) -> list[tuple[int, str, tuple[int, int, int, int]]]:
    found = []
    enum_proc = ctypes.WINFUNCTYPE(ctypes.c_bool, wintypes.HWND, wintypes.LPARAM)

    def callback(hwnd, _):
        if not user32.IsWindowVisible(hwnd):
            return True
        length = user32.GetWindowTextLengthW(hwnd)
        if length == 0:
            return True
        buffer = ctypes.create_unicode_buffer(length + 1)
        user32.GetWindowTextW(hwnd, buffer, length + 1)
        class_name = ctypes.create_unicode_buffer(256)
        user32.GetClassNameW(hwnd, class_name, 256)
        rect = wintypes.RECT()
        user32.GetWindowRect(hwnd, ctypes.byref(rect))
        found.append(
            (
                hwnd,
                f"{buffer.value}\t[{class_name.value}]",
                (rect.left, rect.top, rect.right, rect.bottom),
            )
        )
        return True

    user32.EnumWindows(enum_proc(callback), 0)
    return found


def windows_all() -> list[tuple[int, str, tuple[int, int, int, int], bool]]:
    found = []
    enum_proc = ctypes.WINFUNCTYPE(ctypes.c_bool, wintypes.HWND, wintypes.LPARAM)

    def callback(hwnd, _):
        length = user32.GetWindowTextLengthW(hwnd)
        buffer = ctypes.create_unicode_buffer(length + 1)
        user32.GetWindowTextW(hwnd, buffer, length + 1)
        class_name = ctypes.create_unicode_buffer(256)
        user32.GetClassNameW(hwnd, class_name, 256)
        rect = wintypes.RECT()
        user32.GetWindowRect(hwnd, ctypes.byref(rect))
        found.append(
            (
                hwnd,
                f"{buffer.value}\t[{class_name.value}]",
                (rect.left, rect.top, rect.right, rect.bottom),
                bool(user32.IsWindowVisible(hwnd)),
            )
        )
        return True

    user32.EnumWindows(enum_proc(callback), 0)
    return found


def main(argv: list[str]) -> None:
    if not argv:
        print("shot | move x y | click x y | rclick x y | drag x1 y1 x2 y2 | key COMBO | pos | wins | winsall")
        return
    command, rest = argv[0], argv[1:]
    if command == "shot":
        shot(rest[0])
    elif command == "move":
        move(int(rest[0]), int(rest[1]))
    elif command == "click":
        click(int(rest[0]), int(rest[1]))
    elif command == "rclick":
        click(int(rest[0]), int(rest[1]), right=True)
    elif command == "drag":
        drag(*(int(v) for v in rest[:4]))
    elif command == "key":
        key(rest[0])
    elif command == "pos":
        print("pos", *pos())
    elif command == "wins":
        for hwnd, title, rect in windows():
            print(f"{hwnd}\t{rect}\t{title}")
    elif command == "winsall":
        for hwnd, title, rect, visible in windows_all():
            print(f"{hwnd}\t{rect}\tvisible={visible}\t{title}")
    else:
        raise SystemExit(f"unknown command {command}")


if __name__ == "__main__":
    main(sys.argv[1:])
