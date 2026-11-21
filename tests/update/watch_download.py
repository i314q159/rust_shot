import os
import shutil
import time
from pathlib import Path

SRC = Path(os.environ["TEMP"]) / "RustShot" / "update"
DST = Path(os.environ["TEMP"]) / "rustshot-verify" / "download-snap"

DST.mkdir(parents=True, exist_ok=True)
last = {}
deadline = time.time() + 120
while time.time() < deadline:
    for path in SRC.glob("*"):
        if not path.is_file():
            continue
        try:
            size = path.stat().st_size
        except OSError:
            continue
        if last.get(path.name) == size:
            continue
        last[path.name] = size
        target = DST / f"{path.name}.{size}"
        try:
            shutil.copy2(path, target)
            print(f"{path.name} {size} -> {target.name}", flush=True)
        except OSError:
            pass
    time.sleep(0.08)
