#!/usr/bin/env python3
"""Build Cussy's mobile runtime with Rust and the platform SDK (Python 3.9+)."""

import argparse
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent
BUILD = ROOT / "mobile" / "build"
ANDROID_ABIS = {
    "arm64-v8a": "aarch64-linux-android",
    "x86_64": "x86_64-linux-android",
}
NDK_VERSION = "27.2.12479018"


def run(arguments, env=None):
    print("+", " ".join(map(str, arguments)), flush=True)
    subprocess.run(list(map(str, arguments)), cwd=ROOT, env=env, check=True)


def output(arguments):
    return subprocess.check_output(arguments, text=True).strip()


def rust_build(target, args, extra_env):
    run(["rustup", "target", "add", "--toolchain", args.toolchain, target])
    env = dict(os.environ, **extra_env)
    # Keep build artifacts in one predictable directory even if the caller uses
    # CARGO_TARGET_DIR for their desktop build.
    env["CARGO_TARGET_DIR"] = str(BUILD / "target")
    run(["rustup", "run", args.toolchain, "cargo", "build", "--release", "--locked",
         "--manifest-path", ROOT / "mobile/runtime/Cargo.toml", "--target", target], env)
    return BUILD / "target" / target / "release/libcussy_mobile.a"


def android(args):
    sdk = args.sdk or os.environ.get("ANDROID_HOME") or os.environ.get("ANDROID_SDK_ROOT")
    ndk_path = args.ndk or os.environ.get("ANDROID_NDK_HOME")
    if not ndk_path and sdk:
        ndk_path = Path(sdk) / "ndk" / NDK_VERSION
    if not ndk_path:
        raise SystemExit("Set ANDROID_HOME to your Android SDK, or pass --ndk PATH.")
    ndk = Path(ndk_path).expanduser().resolve()
    host_tag = {"Darwin": "darwin-x86_64", "Linux": "linux-x86_64", "Windows": "windows-x86_64"}.get(platform.system())
    if not host_tag:
        raise SystemExit("Android builds require macOS, Linux, or Windows.")
    tools = ndk / "toolchains/llvm/prebuilt" / host_tag / "bin"
    for abi in args.abis:
        target = ANDROID_ABIS[abi]
        linker = tools / f"{target}26-clang{'.cmd' if os.name == 'nt' else ''}"
        if not linker.is_file():
            raise SystemExit(f"Missing NDK compiler: {linker}\nInstall ndk;{NDK_VERSION} with sdkmanager.")
        env = {f"CARGO_TARGET_{target.upper().replace('-', '_')}_LINKER": str(linker)}
        library = rust_build(target, args, env)
        destination = ROOT / "mobile/android/app/src/main/jniLibs" / abi
        destination.mkdir(parents=True, exist_ok=True)
        shutil.copy2(library, destination / library.name)
        print(f"Built {abi}: {destination / library.name}")


def ios(args):
    if platform.system() != "Darwin":
        raise SystemExit("iOS builds require macOS with full Xcode and the iOS SDKs.")
    sdk_device = output(["xcrun", "--sdk", "iphoneos", "--show-sdk-path"])
    sdk_simulator = output(["xcrun", "--sdk", "iphonesimulator", "--show-sdk-path"])
    common = {"IPHONEOS_DEPLOYMENT_TARGET": "16.0"}
    device = rust_build("aarch64-apple-ios", args, dict(common, SDKROOT=sdk_device))
    simulator_libraries = [rust_build(target, args, dict(common, SDKROOT=sdk_simulator))
                           for target in ("aarch64-apple-ios-sim", "x86_64-apple-ios")]
    simulator = BUILD / "ios-simulator/libcussy_mobile.a"
    simulator.parent.mkdir(parents=True, exist_ok=True)
    run(["xcrun", "lipo", "-create", *simulator_libraries, "-output", simulator])
    framework = BUILD / "CussyRuntime.xcframework"
    if framework.exists():
        shutil.rmtree(framework)
    run(["xcodebuild", "-create-xcframework", "-library", device,
         "-headers", ROOT / "mobile/include", "-library", simulator,
         "-headers", ROOT / "mobile/include", "-output", framework])
    shutil.copy2(ROOT / "mobile/assets/LICENSES.txt", framework / "LICENSES.txt")
    print(f"Built {framework}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("platform", choices=("android", "ios"))
    parser.add_argument("--toolchain", default="1.88.0", help="Installed rustup toolchain (default: 1.88.0)")
    parser.add_argument("--sdk", type=Path, help="Android SDK directory")
    parser.add_argument("--ndk", type=Path, help="Android NDK directory")
    parser.add_argument("--abis", nargs="+", choices=ANDROID_ABIS, default=list(ANDROID_ABIS))
    args = parser.parse_args()
    if not shutil.which("rustup"):
        parser.error("Install Rust with rustup first: https://rustup.rs")
    if subprocess.run(["rustup", "run", args.toolchain, "rustc", "--version"],
                      stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode:
        parser.error(f"Install the Rust toolchain first: rustup toolchain install {args.toolchain} --profile minimal")
    BUILD.mkdir(parents=True, exist_ok=True)
    try:
        (android if args.platform == "android" else ios)(args)
    except subprocess.CalledProcessError as error:
        print(f"Mobile build failed (exit {error.returncode}). Check the SDK/toolchain output above.", file=sys.stderr)
        raise SystemExit(error.returncode) from error


if __name__ == "__main__":
    main()
