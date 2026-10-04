#!/usr/bin/env python3
"""Refresh the bundled notices from the locked Rust sources already in Cargo's cache."""
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent.parent
metadata = json.loads(subprocess.check_output(
    ["cargo", "metadata", "--locked", "--offline", "--format-version", "1"], cwd=ROOT))
sections = ["Cussy mobile: license notices\n",
            "Cussy\n" + (ROOT / "LICENSE").read_text(),
            "Ryu-derived formatter\nCopyright 2018 Ulf Adams and contributors.\n"
            "https://github.com/ulfjack/ryu\n"
            "Pinned source: 4c0618b0e44f7ef027ebae05d2cc7812048f7c8f\n"
            "Adaptations are described in src/vendor/ryu/NOTICE.md.\n\n"
            + (ROOT / "src/vendor/ryu/LICENSE-Boost").read_text()]
for package in sorted(metadata["packages"], key=lambda p: p["name"]):
    if not package.get("source"):
        continue
    directory = Path(package["manifest_path"]).parent
    licenses = sorted(directory.glob("LICENSE*"))
    if not licenses:
        raise SystemExit(f"No license text found for {package['name']}")
    # Include all upstream license texts, including supplemental Unicode notices.
    texts = "\n\n".join(f"{path.name}\n{path.read_text()}" for path in licenses if path.is_file())
    sections.append(f"{package['name']} {package['version']}\n{package.get('repository', '')}\n"
                    f"Declared license: {package['license']}\n\n{texts}")
destination = ROOT / "mobile/assets/LICENSES.txt"
destination.parent.mkdir(parents=True, exist_ok=True)
destination.write_text("\n\n".join(sections), encoding="utf-8")
print(destination)
