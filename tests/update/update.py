import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import smoke_lib as lib


def setup_file():
    files = sorted((lib.REPO / "outputs").glob("RustShot-Setup-*.exe"))
    return files[-1] if files else None


def skip_without_install() -> bool:
    setup = setup_file()
    if setup is None:
        return lib.report("apply-update skips non-installed copy", False, "no setup in outputs")
    exe = lib.stage(lib.default_config())
    lib.stop_apps()
    mark = lib.mark_log()
    subprocess.run(
        [str(exe), "--apply-update", "0", str(setup), str(exe)],
        cwd=str(exe.parent),
        timeout=90,
        check=False,
    )
    lines = lib.log_since(mark)
    skipped = any("is not an installed copy, skipping setup" in line for line in lines)
    finished = any("apply update finished ok=false installed=false" in line for line in lines)
    lib.stop_apps()
    return lib.report("apply-update skips non-installed copy", skipped and finished)


def check_online() -> bool:
    exe = lib.stage(lib.default_config(auto_update="'notice'"))
    lib.stop_apps()
    mark = lib.mark_log()
    lib.start(exe, "--check-update")
    ok = lib.wait_log("update check", mark, 40)
    lib.stop_apps()
    return lib.report("update check reaches GitHub", ok)


def main(argv: list[str]) -> int:
    cases = {"skip": skip_without_install, "online": check_online}
    if not argv:
        print(" | ".join(cases))
        return lib.finish([skip_without_install()])
    return lib.finish([cases[argv[0]]()])


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
