import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import probe
import smoke_lib as lib


def tray_start() -> bool:
    exe = lib.stage(lib.default_config())
    lib.stop_apps()
    mark = lib.mark_log()
    lib.start(exe, "--tray")
    started = lib.wait_log("started in tray", mark, 10)
    visible = [w for w in probe.windows() if "RustShot" in w[1] and w[2][0] > -10000]
    lib.stop_apps()
    return lib.report("tray start stays hidden", started and not visible, f"windows={len(visible)}")


def forward_show_panel() -> bool:
    exe = lib.stage(lib.default_config())
    lib.stop_apps()
    lib.start(exe, "--tray")
    if not lib.wait_pids():
        return lib.report("forward --panel", False, "tray instance did not start")
    lib.start(exe, "--panel")
    window = lib.wait_window("RustShot", 10)
    running = len(lib.pids())
    lib.stop_apps()
    ok = window is not None and running == 1
    return lib.report("forward --panel to running instance", ok, f"windows={bool(window)} procs={running}")


def panel_close_hides() -> bool:
    exe = lib.stage(lib.default_config())
    lib.stop_apps()
    mark = lib.mark_log()
    lib.start(exe, "--panel")
    window = lib.wait_window("RustShot")
    if window is None:
        return lib.report("panel close hides to tray", False, "panel did not open")
    lib.close_window(window[0])
    hidden = lib.wait_log("panel close -> hidden to tray", mark, 6)
    time.sleep(1.2)
    gone = not lib.onscreen(window[0])
    lib.stop_apps()
    return lib.report("panel close hides to tray", hidden and gone)


def main(argv: list[str]) -> int:
    cases = {
        "tray": tray_start,
        "forward": forward_show_panel,
        "close": panel_close_hides,
    }
    if not argv:
        print(" | ".join(cases))
        return lib.finish([tray_start(), forward_show_panel(), panel_close_hides()])
    return lib.finish([cases[argv[0]]()])


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
