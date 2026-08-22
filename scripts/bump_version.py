#!/usr/bin/env python3
"""Choose the next SemVer and write it to the app version files."""

from __future__ import annotations

import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def existing_versions() -> set[str]:
    out = subprocess.check_output(["git", "tag", "-l", "v*"], text=True)
    versions = set()
    for raw in out.splitlines():
        tag = raw.strip()
        if tag.startswith("v"):
            versions.add(tag[1:])
    return versions


def bump_patch(version: str) -> str:
    major, minor, patch = (int(part) for part in version.split("."))
    return f"{major}.{minor}.{patch + 1}"


def next_version(current: str, taken: set[str]) -> str:
    version = current
    while version in taken:
        version = bump_patch(version)
    return version


def write_json(path: Path, version: str) -> None:
    data = json.loads(path.read_text())
    data["version"] = version
    path.write_text(json.dumps(data, indent=2) + "\n")


def write_cargo_toml(path: Path, version: str) -> None:
    text, count = re.subn(r'(?m)^version = "[^"]+"', f'version = "{version}"', path.read_text(), count=1)
    if count != 1:
        raise SystemExit(f"Could not update version in {path}")
    path.write_text(text)


def write_cargo_lock(path: Path, version: str) -> None:
    text, count = re.subn(
        r'(name = "learndsa"\n)version = "[^"]+"',
        rf'\1version = "{version}"',
        path.read_text(),
        count=1,
    )
    if count != 1:
        raise SystemExit(f"Could not update learndsa version in {path}")
    path.write_text(text)


def main() -> None:
    tauri = ROOT / "src-tauri" / "tauri.conf.json"
    current = json.loads(tauri.read_text())["version"]
    version = next_version(current, existing_versions())
    write_json(tauri, version)
    write_json(ROOT / "package.json", version)
    write_cargo_toml(ROOT / "src-tauri" / "Cargo.toml", version)
    write_cargo_lock(ROOT / "src-tauri" / "Cargo.lock", version)
    print(version)


if __name__ == "__main__":
    main()
