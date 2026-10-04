#!/usr/bin/env python3
"""Package an already-built Cussy binary and/or source with Python 3.9+ and Cargo."""

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parent.parent
TARGETS = {
    "x86_64-unknown-linux-gnu": ("linux-x64", "cussy"),
    "aarch64-apple-darwin": ("macos-arm64", "cussy"),
    "x86_64-apple-darwin": ("macos-x64", "cussy"),
    "x86_64-pc-windows-msvc": ("windows-x64", "cussy.exe"),
}
COMMON_FILES = [
    "README.md", "LICENSE", "CHANGELOG.md", "docs", "examples", "stdlib",
    "research", "benchmarks", "editors", "CONTRIBUTING.md", "SECURITY.md",
    "CODE_OF_CONDUCT.md",
    "assets/brand",
]
SOURCE_FILES = COMMON_FILES + [
    "Cargo.toml", "Cargo.lock", ".gitignore", ".github", "cussy", "src", "tests",
    "scripts", "mobile",
]
EXCLUDED_PARTS = {".git", "target", "dist", "build", ".gradle", ".cxx", ".externalNativeBuild", "jniLibs", "DerivedData", "xcuserdata", "__pycache__", "node_modules", ".venv", ".DS_Store"}


def package_version():
    metadata = json.loads(subprocess.check_output([
        "cargo", "metadata", "--manifest-path", str(ROOT / "Cargo.toml"),
        "--format-version", "1", "--no-deps", "--offline", "--locked",
    ], text=True, cwd=ROOT))
    return next(
        package["version"] for package in metadata["packages"]
        if Path(package["manifest_path"]).resolve() == ROOT / "Cargo.toml"
    )


def files(names):
    for name in names:
        path = ROOT / name
        if not path.exists():
            raise SystemExit(f"Missing package input: {path}")
        candidates = sorted(path.rglob("*")) if path.is_dir() else [path]
        for source in candidates:
            relative = source.relative_to(ROOT)
            if (not source.is_file() or source.is_symlink()
                    or EXCLUDED_PARTS.intersection(relative.parts)
                    or source.suffix in {".pyc", ".pyo", ".csyb"}
                    or source.name == "local.properties"
                    or source.name.endswith(("~", ".swp"))):
                continue
            yield source


def write_archive(destination, entries):
    if destination.suffix == ".zip":
        with zipfile.ZipFile(destination, "w", zipfile.ZIP_DEFLATED) as archive:
            for source, name in entries:
                archive.write(source, name)
    else:
        with tarfile.open(destination, "w:gz") as archive:
            for source, name in entries:
                archive.add(source, arcname=name, recursive=False)


def smoke_test(archive_path, prefix, executable, version):
    """Exercise the extracted binary, imports, graphing, and standalone artifacts."""
    with tempfile.TemporaryDirectory(prefix="cussy-package-") as directory:
        if archive_path.suffix == ".zip":
            with zipfile.ZipFile(archive_path) as archive:
                archive.extractall(directory)
        else:
            with tarfile.open(archive_path) as archive:
                if hasattr(tarfile, "data_filter"):
                    archive.extractall(directory, filter="data")
                else:
                    archive.extractall(directory)
        root = Path(directory) / prefix
        binary = root / "bin" / executable

        def run(*arguments):
            return subprocess.check_output(
                [str(binary), *arguments], cwd=root, text=True, timeout=60,
            )

        if run("--version").strip() != f"cussy {version}":
            raise SystemExit("Binary version differs from Cargo.toml; rebuild first")
        if "Hello, Cussy!" not in run("run", "examples/hello.cussy"):
            raise SystemExit("Packaged hello example produced unexpected output")
        run("run", "examples/graphdash/main.cussy")
        if not (root / "graphdash.svg").is_file():
            raise SystemExit("Packaged graphdash example did not create its graph")
        run("build", "examples/hello.cussy", "-o", "hello.csyb")
        native = root / ("hello-native.exe" if executable.endswith(".exe") else "hello-native")
        run("compile", "examples/hello.cussy", "-o", str(native))
        shutil.rmtree(root / "examples")
        shutil.rmtree(root / "stdlib")
        if "Hello, Cussy!" not in run("run", "hello.csyb"):
            raise SystemExit("Packaged artifact did not run independently")
        native_output = subprocess.check_output([str(native)], cwd=root, text=True, timeout=60)
        if "Hello, Cussy!" not in native_output:
            raise SystemExit("Native executable did not run independently")
    print(f"Smoke tests passed: {archive_path.name}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", choices=TARGETS, help="Rust target used by cargo build --target")
    parser.add_argument("--binary", type=Path, help="Override the built executable path")
    parser.add_argument("--output-dir", type=Path, default=ROOT / "dist")
    parser.add_argument("--kind", choices=("all", "binary", "source"), default="all")
    parser.add_argument("--verify-tag", help="Require v<Cargo.toml version>")
    parser.add_argument("--smoke-test", action="store_true", help="Run the extracted binary on a matching host")
    args = parser.parse_args()
    version = package_version()
    if args.verify_tag and args.verify_tag != f"v{version}":
        parser.error(f"tag {args.verify_tag!r} does not match Cargo.toml version v{version}")
    if args.smoke_test and args.kind == "source":
        parser.error("--smoke-test requires a binary package")
    output = args.output_dir.resolve()
    output.mkdir(parents=True, exist_ok=True)
    prefix = f"cussy-{version}"
    archives = []

    if args.kind in {"all", "binary"}:
        target = args.target
        if target is None:
            rustc_info = subprocess.check_output(["rustc", "-vV"], text=True)
            target = next(
                line.removeprefix("host: ") for line in rustc_info.splitlines()
                if line.startswith("host: ")
            )
        if target not in TARGETS:
            parser.error(f"unsupported package target: {target}")
        label, executable = TARGETS[target]
        build_dir = ROOT / "target" / args.target if args.target else ROOT / "target"
        binary = (args.binary or build_dir / "release" / executable).resolve()
        if not binary.is_file():
            command = "cargo build --release --locked"
            if args.target:
                command += f" --target {target}"
            parser.error(f"missing executable {binary}; run {command}")
        extension = ".zip" if executable.endswith(".exe") else ".tar.gz"
        binary_archive = output / f"{prefix}-{label}{extension}"
        entries = [(binary, f"{prefix}/bin/{executable}")]
        entries.extend(
            (source, f"{prefix}/{source.relative_to(ROOT).as_posix()}")
            for source in files(COMMON_FILES)
        )
        write_archive(binary_archive, entries)
        if args.smoke_test:
            smoke_test(binary_archive, prefix, executable, version)
        archives.append(binary_archive)

    if args.kind in {"all", "source"}:
        source_archive = output / f"{prefix}-source.tar.gz"
        write_archive(source_archive, (
            (source, f"{prefix}/{source.relative_to(ROOT).as_posix()}")
            for source in files(SOURCE_FILES)
        ))
        archives.append(source_archive)
        editor_archive = output / f"{prefix}-vscode-starter.zip"
        write_archive(editor_archive, (
            (source, f"cussy-language/{source.relative_to(ROOT / 'editors/vscode').as_posix()}")
            for source in files(["editors/vscode"])
        ))
        archives.append(editor_archive)

    checksums = []
    for archive in archives:
        checksums.append(f"{hashlib.sha256(archive.read_bytes()).hexdigest()}  {archive.name}")
        print(f"{archive} ({archive.stat().st_size:,} bytes)")
    (output / "SHA256SUMS").write_text("\n".join(checksums) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
