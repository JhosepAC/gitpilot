"""Audit registry versions behind local backports as well as normal Cargo dependencies."""

import json
from pathlib import Path
import subprocess
import tempfile

from importlib.util import module_from_spec, spec_from_file_location


def main():
    root = Path(__file__).resolve().parents[1]
    spec = spec_from_file_location("verify_vendor", root / "scripts/check-vendored-dependencies.py")
    verifier = module_from_spec(spec)
    spec.loader.exec_module(verifier)
    verifier.main()
    native = root / "apps/desktop/src-tauri"
    sources = json.loads((native / "vendor/sources.json").read_text(encoding="utf-8"))
    lock = (native / "Cargo.lock").read_text(encoding="utf-8")
    for item in sources["packages"]:
        name, version = item["directory"].rsplit("-", 1)
        entry = f'[[package]]\nname = "{name}"\nversion = "{version}"\n'
        if lock.count(entry) != 1:
            raise SystemExit(f"Missing or ambiguous backport: {name}")
        lock = lock.replace(entry, entry + 'source = "registry+https://github.com/rust-lang/crates.io-index"\n'
                            + f'checksum = "{item["archiveSha256"]}"\n')
    # Audit a temporary shadow lockfile: never modify the application's resolved lockfile.
    with tempfile.TemporaryDirectory(prefix="gitpilot-audit-") as directory:
        shadow = Path(directory) / "Cargo.lock"
        shadow.write_text(lock, encoding="utf-8")
        result = subprocess.run(["cargo", "audit", "--json", "--file", str(shadow)],
                                capture_output=True, text=True, check=False)
    try:
        report = json.loads(result.stdout)
    except json.JSONDecodeError:
        raise SystemExit(f"cargo audit did not return a report (exit {result.returncode})")
    if result.returncode not in (0, 1) or "vulnerabilities" not in report:
        raise SystemExit("cargo audit failed before completing the audit")
    findings = list(report["vulnerabilities"]["list"])
    for warnings in report.get("warnings", {}).values():
        findings.extend(warnings)
    unresolved = []
    for finding in findings:
        package = finding["package"]
        advisory = finding["advisory"]
        if (package["name"], package["version"], advisory["id"]) == ("glib", "0.18.5", "RUSTSEC-2024-0429"):
            print("RUSTSEC-2024-0429: upstream version flagged; exact reviewed GLib backport verified")
        else:
            unresolved.append(finding)
    print("RustSec database:", json.dumps(report["database"], sort_keys=True))
    if unresolved:
        print(json.dumps(unresolved, indent=2))
        raise SystemExit("Unresolved dependency advisories")
    print("No unresolved advisories, including the upstream versions of local backports")


if __name__ == "__main__":
    main()
