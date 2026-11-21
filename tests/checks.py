import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

from PIL import Image, ImageGrab

import probe

ROOT = Path(os.environ["TEMP"]) / "rustshot-verify"
APP_DIR = ROOT / "app"
SHOTS = ROOT / "shots"
SAVED = SHOTS / "saved"
LOG = Path(os.environ["LOCALAPPDATA"]) / "RustShot" / "rustshot.log"
REPO = Path(__file__).resolve().parents[1]
INSTALLED = Path(r"C:\Program Files\RustShot\rust_shot.exe")

CONFIG = (
    f"save_dir = '{SAVED}'\n"
    "auto_save = true\n"
    "theme = 'light'\n"
    "lang = 'zh'\n"
    "auto_update = 'notice'\n"
    "proxy = '127.0.0.1:12334'\n"
)


def stage() -> Path:
    APP_DIR.mkdir(parents=True, exist_ok=True)
    shutil.copy2(REPO / "target" / "release" / "rust_shot.exe", APP_DIR / "rust_shot.exe")
    (APP_DIR / "config.toml").write_text(CONFIG, encoding="utf-8")
    (APP_DIR / "config.txt").unlink(missing_ok=True)
    shutil.rmtree(SAVED, ignore_errors=True)
    SAVED.mkdir(parents=True, exist_ok=True)
    return APP_DIR / "rust_shot.exe"


def stop_apps() -> None:
    subprocess.run(
        ["taskkill", "/F", "/IM", "rust_shot.exe"],
        capture_output=True,
        check=False,
    )
    time.sleep(0.6)


def overlay_window(timeout: float = 6.0) -> tuple[int, tuple[int, int, int, int]] | None:
    deadline = time.time() + timeout
    while time.time() < deadline:
        for hwnd, title, rect in probe.windows():
            if "RustShot Overlay" in title:
                return hwnd, rect
        time.sleep(0.15)
    return None


def start_overlay(exe: Path) -> tuple[int, tuple[int, int, int, int]]:
    subprocess.Popen([str(exe), "--capture"], cwd=str(exe.parent))
    window = overlay_window()
    if window is None:
        raise SystemExit("overlay did not appear")
    time.sleep(0.5)
    return window


def log_lines(count: int = 20) -> list[str]:
    if not LOG.exists():
        return []
    return LOG.read_text(encoding="utf-8", errors="replace").splitlines()[-count:]


def enter_copy() -> int:
    exe = stage()
    stop_apps()
    window = start_overlay(exe)
    print(f"overlay {window[1]}")
    probe.drag(300, 200, 800, 600)
    time.sleep(0.3)
    probe.key("ENTER")
    deadline = time.time() + 8
    png = None
    while time.time() < deadline:
        files = sorted(SAVED.glob("*.png"))
        if files:
            png = files[-1]
            break
        time.sleep(0.2)
    clip = ImageGrab.grabclipboard()
    shot_path = SHOTS / "check-enter-copy.png"
    probe.shot(str(shot_path))
    stop_apps()
    if png is None:
        print("FAIL enter-copy: no png written")
        return 1
    size = Image.open(png).size
    print(f"saved {png.name} {size[0]}x{size[1]}")
    print(f"clipboard {'image ' + str(clip.size) if clip else 'empty'}")
    ok = size == (500, 400) and clip is not None
    print("PASS enter-copy" if ok else "FAIL enter-copy")
    return 0 if ok else 1


def handle_drag() -> int:
    exe = stage()
    stop_apps()
    start_overlay(exe)
    probe.drag(300, 200, 800, 600)
    time.sleep(0.3)
    probe.drag(800, 600, 860, 650)
    time.sleep(0.5)
    full = SHOTS / "check-handle.png"
    probe.shot(str(full))
    label = SHOTS / "check-handle-label.png"
    Image.open(full).crop((255, 170, 460, 210)).resize((820, 160)).save(label)
    probe.key("ESC")
    time.sleep(0.5)
    stop_apps()
    print(f"label crop -> {label}")
    return 0


def uac_cancel(setup: Path) -> int:
    stop_apps()
    helper = [str(INSTALLED), "--apply-update", "0", str(setup), str(INSTALLED)]
    subprocess.Popen(helper, cwd=str(INSTALLED.parent))
    deadline = time.time() + 20
    dialog = None
    while time.time() < deadline:
        for hwnd, title, rect in probe.windows():
            lowered = title.lower()
            if "用户帐户控制" in title or "user account control" in lowered:
                dialog = (hwnd, title, rect)
                break
        if dialog:
            break
        time.sleep(0.3)
    probe.shot(str(SHOTS / "check-uac.png"))
    if dialog is None:
        print("no UAC dialog found")
        return 1
    print(f"uac dialog {dialog[0]} {dialog[2]} {dialog[1]}")
    return 0


def uac_verify() -> int:
    time.sleep(1.0)
    lines = log_lines(12)
    for line in lines:
        print(line)
    declined = any("UAC declined" in line for line in lines)
    failed = any("apply update finished ok=false installed=true" in line for line in lines)
    version = subprocess.run(
        [
            "powershell",
            "-NoProfile",
            "-Command",
            "(Get-Item 'C:\\Program Files\\RustShot\\rust_shot.exe').VersionInfo.FileVersion",
        ],
        capture_output=True,
        text=True,
    ).stdout.strip()
    print(f"installed version {version}")
    ok = declined and failed
    print("PASS uac-cancel" if ok else "FAIL uac-cancel")
    return 0 if ok else 1


def tray_menu(x: int, y: int) -> int:
    probe.click(x, y, right=True)
    time.sleep(0.6)
    probe.shot(str(SHOTS / "check-tray-menu.png"))
    return 0


def tooltip_text() -> str:
    for _hwnd, title, _rect in probe.windows():
        if "[tooltips_class32]" in title:
            return title.split("\t")[0]
    return ""


def tray_find(y: int = 1050, first: int = 1180, last: int = 1900, step: int = 8) -> int:
    for x in range(first, last, step):
        probe.move(x, y)
        time.sleep(0.28)
        text = tooltip_text()
        if "RustShot" in text or "截图" in text:
            print(f"tray icon at {x},{y} tooltip={text}")
            return 0
    print("tray icon not found")
    return 1


def tray_pick(x: int, y: int) -> int:
    probe.click(x, y)
    time.sleep(1.4)
    probe.shot(str(SHOTS / "check-tray-update.png"))
    print("clicked", x, y)
    return 0


def main(argv: list[str]) -> int:
    SHOTS.mkdir(parents=True, exist_ok=True)
    if not argv:
        print("enter-copy | handle-drag | uac-cancel <setup.exe> | uac-verify | tray-find | tray-menu x y | tray-pick x y")
        return 2
    command = argv[0]
    if command == "enter-copy":
        return enter_copy()
    if command == "handle-drag":
        return handle_drag()
    if command == "uac-cancel":
        return uac_cancel(Path(argv[1]))
    if command == "uac-verify":
        return uac_verify()
    if command == "tray-find":
        return tray_find()
    if command == "tray-menu":
        return tray_menu(int(argv[1]), int(argv[2]))
    if command == "tray-pick":
        return tray_pick(int(argv[1]), int(argv[2]))
    raise SystemExit(f"unknown command {command}")


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
