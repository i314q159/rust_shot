import ctypes
import ctypes.wintypes as wintypes
import os
import shutil
import subprocess
import time
from pathlib import Path

from PIL import ImageGrab

import probe

user32 = ctypes.windll.user32

ROOT = Path(os.environ["TEMP"]) / "rustshot-verify"
APP_DIR = ROOT / "app"
SHOTS = ROOT / "shots"
SAVED = SHOTS / "saved"
LOG = Path(os.environ["LOCALAPPDATA"]) / "RustShot" / "rustshot.log"
REPO = Path(__file__).resolve().parents[1]
BUILT = REPO / "target" / "release" / "rust_shot.exe"
INSTALLED = Path(r"C:\Program Files\RustShot\rust_shot.exe")


def default_config(save_dir: Path | None = None, **overrides) -> str:
    values = {
        "save_dir": f"'{save_dir or SAVED}'",
        "auto_save": "true",
        "theme": "'light'",
        "lang": "'zh'",
        "auto_update": "'notice'",
        "proxy": "'system'",
    }
    values.update(overrides)
    return "".join(f"{key} = {value}\n" for key, value in values.items())


def stage(config: str | None = None) -> Path:
    if not BUILT.exists():
        raise SystemExit(f"missing {BUILT}; run cargo build --release first")
    stop_apps()
    APP_DIR.mkdir(parents=True, exist_ok=True)
    SHOTS.mkdir(parents=True, exist_ok=True)
    exe = APP_DIR / "rust_shot.exe"
    for attempt in range(6):
        try:
            shutil.copy2(BUILT, exe)
            break
        except PermissionError:
            if attempt == 5:
                raise
            time.sleep(0.4)
    for stale in (APP_DIR / "config.toml", APP_DIR / "config.txt", APP_DIR / "unins000.exe"):
        stale.unlink(missing_ok=True)
    if config is not None:
        (APP_DIR / "config.toml").write_text(config, encoding="utf-8")
    return exe


def clean_saved() -> None:
    shutil.rmtree(SAVED, ignore_errors=True)
    SAVED.mkdir(parents=True, exist_ok=True)


def isolated_env() -> dict[str, str]:
    local = ROOT / "localappdata"
    local.mkdir(parents=True, exist_ok=True)
    env = os.environ.copy()
    env["LOCALAPPDATA"] = str(local)
    return env


def start(exe: Path, *flags: str, env: dict[str, str] | None = None) -> subprocess.Popen:
    return subprocess.Popen([str(exe), *flags], cwd=str(exe.parent), env=env)


def stop_apps() -> None:
    subprocess.run(
        ["taskkill", "/F", "/IM", "rust_shot.exe"],
        capture_output=True,
        check=False,
    )
    time.sleep(0.6)


def pids(name: str = "rust_shot.exe") -> list[int]:
    out = subprocess.run(
        ["tasklist", "/FI", f"IMAGENAME eq {name}", "/FO", "CSV", "/NH"],
        capture_output=True,
        text=True,
    ).stdout
    found = []
    for line in out.splitlines():
        parts = [part.strip('"') for part in line.split(",")]
        if len(parts) > 1 and parts[0].lower() == name.lower():
            found.append(int(parts[1]))
    return found


def wait_pids(name: str = "rust_shot.exe", timeout: float = 8.0) -> bool:
    deadline = time.time() + timeout
    while time.time() < deadline:
        if pids(name):
            return True
        time.sleep(0.2)
    return False


def wait_window(title_part: str, timeout: float = 8.0) -> tuple[int, str, tuple[int, int, int, int]] | None:
    deadline = time.time() + timeout
    while time.time() < deadline:
        for hwnd, title, rect in probe.windows():
            if title_part in title and rect[0] > -10000:
                return hwnd, title, rect
        time.sleep(0.15)
    return None


def window_rect(hwnd: int) -> tuple[int, int, int, int]:
    rect = wintypes.RECT()
    user32.GetWindowRect(hwnd, ctypes.byref(rect))
    return rect.left, rect.top, rect.right, rect.bottom


def onscreen(hwnd: int) -> bool:
    rect = window_rect(hwnd)
    return user32.IsWindowVisible(hwnd) and rect[0] > -10000 and rect[2] > rect[0]


def close_window(hwnd: int) -> None:
    user32.PostMessageW(hwnd, 0x0010, 0, 0)


def log_tail(count: int = 60) -> list[str]:
    if not LOG.exists():
        return []
    return LOG.read_text(encoding="utf-8", errors="replace").splitlines()[-count:]


def mark_log() -> int:
    return len(LOG.read_text(encoding="utf-8", errors="replace").splitlines()) if LOG.exists() else 0


def log_since(mark: int) -> list[str]:
    if not LOG.exists():
        return []
    lines = LOG.read_text(encoding="utf-8", errors="replace").splitlines()
    return lines[mark:]


def wait_log(text: str, mark: int = 0, timeout: float = 15.0) -> bool:
    deadline = time.time() + timeout
    while time.time() < deadline:
        if any(text in line for line in log_since(mark)):
            return True
        time.sleep(0.2)
    return False


def clipboard_image():
    try:
        return ImageGrab.grabclipboard()
    except Exception:
        return None


def wait_file(folder: Path, pattern: str = "*.png", timeout: float = 10.0) -> Path | None:
    deadline = time.time() + timeout
    while time.time() < deadline:
        files = sorted(folder.glob(pattern))
        if files:
            return files[-1]
        time.sleep(0.2)
    return None


def report(name: str, ok: bool, detail: str = "") -> bool:
    suffix = f" ({detail})" if detail else ""
    print(f"{'PASS' if ok else 'FAIL'} {name}{suffix}")
    return ok


def finish(results: list[bool]) -> int:
    ok = all(results)
    print("PASS" if ok else "FAIL", f"{sum(results)}/{len(results)}")
    return 0 if ok else 1
