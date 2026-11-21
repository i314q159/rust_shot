import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import smoke_lib as lib


def switch(pref: str, expect: str) -> bool:
    exe = lib.stage(lib.default_config(theme=f"'{pref}'"))
    lib.stop_apps()
    mark = lib.mark_log()
    lib.start(exe, "--panel")
    ok = lib.wait_log(expect, mark, 10)
    lib.stop_apps()
    return lib.report(f"theme {pref}", ok, expect)


def main(argv: list[str]) -> int:
    if not argv:
        print("dark | light | system")
        return lib.finish(
            [
                switch("dark", "theme -> dark"),
                switch("light", "theme -> light"),
                switch("system", "(system)"),
            ]
        )
    mapping = {"dark": "theme -> dark", "light": "theme -> light", "system": "(system)"}
    return lib.finish([switch(argv[0], mapping[argv[0]])])


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
