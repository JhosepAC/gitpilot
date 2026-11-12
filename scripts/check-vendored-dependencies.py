"""Verify the exact reviewed backports; cargo-audit skips registry advisories for path crates."""

import hashlib
import json
from pathlib import Path
import tomllib


def main():
    if not __debug__:
        raise SystemExit("Backport verification requires Python assertions enabled")
    native = Path(__file__).resolve().parents[1] / "apps/desktop/src-tauri"
    vendor = native / "vendor"
    manifest = json.loads((vendor / "sources.json").read_text(encoding="utf-8"))
    assert manifest["schema"] == 1
    packages = manifest["packages"]
    names = {item["directory"] for item in packages}
    assert {p.name for p in vendor.iterdir() if p.is_dir() and p.name != "patches"} == names
    lock = tomllib.loads((native / "Cargo.lock").read_text(encoding="utf-8"))
    config = tomllib.loads((native / "Cargo.toml").read_text(encoding="utf-8"))
    for item in packages:
        directory = vendor / item["directory"]
        actual = {p.relative_to(directory).as_posix() for p in directory.rglob("*") if p.is_file()}
        assert actual == set(item["files"]), f"Unexpected or missing files: {directory.name}"
        for name, expected in item["files"].items():
            raw = (directory / name).read_bytes().replace(b"\r\n", b"\n")
            assert hashlib.sha256(raw).hexdigest() == expected, f"Unreviewed change: {directory.name}/{name}"
        package = tomllib.loads((directory / "Cargo.toml").read_text(encoding="utf-8"))["package"]
        resolved = [p for p in lock["package"] if p["name"] == package["name"]]
        assert len(resolved) == 1 and resolved[0]["version"] == package["version"]
        assert "source" not in resolved[0], f"Backport not selected: {package['name']}"
        assert config["patch"]["crates-io"][package["name"]]["path"] == f"vendor/{directory.name}"
        print(f"Verified reviewed backport: {directory.name} ({len(actual)} files)")
    sqlite = [p for p in lock["package"] if p["name"] == "libsqlite3-sys"]
    assert len(sqlite) == 1 and sqlite[0]["version"] == "0.39.1"
    assert not any(p["name"] in {"rsa", "proc-macro-error", "proc-macro-error2"} for p in lock["package"])


if __name__ == "__main__":
    main()
