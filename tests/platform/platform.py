import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import smoke_lib as lib


def log_written() -> bool:
    exe = lib.stage(lib.default_config())
    lib.stop_apps()
    mark = lib.mark_log()
    lib.start(exe, "--tray")
    lib.wait_pids()
    ok = lib.wait_log("app created, tray=true", mark, 10)
    lib.stop_apps()
    return lib.report("log file written under LOCALAPPDATA", ok and lib.LOG.exists(), str(lib.LOG))


def single_instance() -> bool:
    exe = lib.stage(lib.default_config())
    lib.stop_apps()
    lib.start(exe, "--tray")
    if not lib.wait_pids():
        return lib.report("single instance", False, "first instance did not start")
    lib.start(exe, "--tray")
    time.sleep(2.5)
    running = len(lib.pids())
    lib.stop_apps()
    return lib.report("single instance", running == 1, f"processes={running}")


def main(argv: list[str]) -> int:
    cases = {"log": log_written, "instance": single_instance}
    if not argv:
        print(" | ".join(cases))
        return lib.finish([log_written(), single_instance()])
    return lib.finish([cases[argv[0]]()])


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
