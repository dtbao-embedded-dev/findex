#!/usr/bin/env python3
"""
Build the `findex-tag` helper for a release bundle.

Usage:
    python docs/scripts/build-helper.py

Tauri bundles a helper binary through `bundle.externalBin`
(`src-tauri/tauri.release.conf.json`), which must exist as
`src-tauri/binaries/findex-tag-<target triple>.exe` before the app is built. This
builds the helper in release mode and copies it there. `beforeBuildCommand` runs
it, so `pnpm tauri build` never needs it run by hand.

Standard library only.
"""

import pathlib
import shutil
import subprocess
import sys

TAURI = pathlib.Path(__file__).resolve().parents[2] / "src-tauri"


def host_triple():
    """The triple rustc builds for, as Tauri names sidecars after it."""
    info = subprocess.run(
        ["rustc", "-vV"], cwd=TAURI, capture_output=True, text=True, check=True
    ).stdout
    for line in info.splitlines():
        if line.startswith("host: "):
            return line.removeprefix("host: ").strip()
    raise SystemExit("rustc -vV did not name a host triple")


def main():
    subprocess.run(
        ["cargo", "build", "--release", "-p", "findex-tag"], cwd=TAURI, check=True
    )
    built = TAURI / "target" / "release" / "findex-tag.exe"
    dest = TAURI / "binaries" / f"findex-tag-{host_triple()}.exe"
    dest.parent.mkdir(exist_ok=True)
    shutil.copy2(built, dest)
    print(f"helper: {dest.relative_to(TAURI.parent)}")


if __name__ == "__main__":
    sys.exit(main())
