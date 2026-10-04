#!/usr/bin/env python3
"""Compare verified, fresh-process CLI workloads; requires Python 3.9+ and all tools."""

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
from zoneinfo import ZoneInfo, ZoneInfoNotFoundError

ROOT = Path(__file__).resolve().parent.parent
SOURCE_DIR = ROOT / "benchmarks" / "languages"
LANGUAGES = ("cussy", "c", "rust", "python", "javascript")
EXTENSIONS = dict(zip(LANGUAGES, ("cussy", "c", "rs", "py", "js")))
C_FLAGS = ["-O3", "-std=c11", "-Wall", "-Wextra", "-pedantic"]
RUST_FLAGS = ["-O", "--edition=2024"]


def positive(value):
    value = int(value)
    if value < 1:
        raise argparse.ArgumentTypeError("must be at least 1")
    return value


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def capture(command):
    return subprocess.run(command, check=True, capture_output=True, text=True).stdout.strip()


def require(name):
    found = shutil.which(name)
    if not found:
        raise RuntimeError(f"required tool '{name}' is missing; no languages will be skipped")
    # Preserve the invoked name: rustup/other toolchain proxies use argv[0].
    return Path(found).absolute()


def expected_outputs():
    # Independent reference algorithms: iterative Fibonacci, affine-transform
    # exponentiation, and a sieve (rather than recursion or trial division).
    previous, current = 0, 1
    for _ in range(24):
        previous, current = current, previous + current
    modulus = 2147483647
    multiplier, increment, exponent = 1664525, 1013904223, 200000
    result_multiplier, result_increment = 1, 0
    while exponent:
        if exponent & 1:
            result_multiplier = result_multiplier * multiplier % modulus
            result_increment = (result_increment * multiplier + increment) % modulus
        increment = increment * (multiplier + 1) % modulus
        multiplier = multiplier * multiplier % modulus
        exponent >>= 1
    recurrence = (result_multiplier * 12345 + result_increment) % modulus
    sieve = [True] * 3001
    sieve[0] = sieve[1] = False
    for candidate in range(2, 55):
        if sieve[candidate]:
            for multiple in range(candidate * candidate, 3001, candidate):
                sieve[multiple] = False
    return {
        "fibonacci": f"{previous}\n",
        "integer_recurrence": f"{recurrence}\n",
        "primes": f"{sum(sieve)}\n",
    }


def measure(command, expected):
    start = time.perf_counter()
    result = subprocess.run(
        command, cwd=ROOT, text=True, capture_output=True, timeout=120, check=True,
    )
    elapsed = time.perf_counter() - start
    if result.stdout != expected:
        raise RuntimeError(f"expected output {expected!r}; got {result.stdout!r}")
    return elapsed


def summarize(samples):
    return {
        "median_seconds": statistics.median(samples),
        "minimum_seconds": min(samples),
        "maximum_seconds": max(samples),
        "samples_seconds": samples,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    executable_name = "cussy.exe" if os.name == "nt" else "cussy"
    parser.add_argument("--cussy", type=Path, default=ROOT / "target/release" / executable_name)
    parser.add_argument("--runs", type=positive, default=7)
    parser.add_argument("--warmup", type=positive, default=1)
    parser.add_argument("--json", type=Path, help="save full metadata and raw measurements")
    args = parser.parse_args()
    cussy = args.cussy.resolve()
    if not cussy.is_file():
        parser.error("Cussy executable is missing; first run cargo build --release")
    # Run the current interpreter directly, avoiding pyenv/asdf shim overhead.
    tools = {"cussy": cussy, "python": Path(sys.executable).resolve()}
    tools.update({name: require(name) for name in ("clang", "rustc", "node")})
    tool_hashes = {name: digest(path) for name, path in tools.items()}
    now = datetime.now(timezone.utc)
    local_timezone = "America/Chicago"
    try:
        local_time = now.astimezone(ZoneInfo(local_timezone))
    except ZoneInfoNotFoundError:
        # Some standard Windows Python installations have no IANA timezone DB.
        local_time, local_timezone = now, "UTC"
    clang_version = capture([str(tools["clang"]), "--version"])
    clang_version = "\n".join(
        line for line in clang_version.splitlines() if not line.startswith("InstalledDir:")
    )
    cpu = platform.processor() or "unknown"
    if platform.system() == "Darwin":
        cpu = capture(["sysctl", "-n", "machdep.cpu.brand_string"])
    report = {
        "measured_at_utc": now.isoformat(),
        "local_date": local_time.date().isoformat(),
        "local_timezone": local_timezone,
        "environment": {
            "os": platform.system(), "os_version": platform.release(),
            "platform": platform.platform(), "architecture": platform.machine(),
            "cpu": cpu, "logical_cpus": os.cpu_count(),
        },
        "versions": {
            "cussy": capture([str(cussy), "--version"]),
            "clang": clang_version,
            "rustc": capture([str(tools["rustc"]), "--version", "--verbose"]),
            "python": platform.python_implementation() + " " + sys.version.splitlines()[0],
            "node": capture([str(tools["node"]), "--version"]),
            "v8": capture([str(tools["node"]), "-p", "process.versions.v8"]),
        },
        "tool_executable_sha256": tool_hashes,
        "methodology": {
            "measurement": "fresh-process CLI wall time, including startup and shutdown",
            "interpreted_inputs": "source parsing/checking and execution included",
            "native_compilation": "completed before all warmups and timed runs; excluded",
            "cussy_build": "cargo build --release (thin LTO, stripped)",
            "cussy_fuel": 50000000,
            "c_flags": C_FLAGS,
            "rust_flags": RUST_FLAGS,
            "runs_per_case_language": args.runs,
            "warmup_processes_per_case_language": args.warmup,
            "warmup_scope": "fresh processes: warm filesystem/OS caches, no persistent JIT state",
            "order": "rotate language order by case index plus sample index",
            "inputs": "same fixed constants and algorithms in all five languages",
            "arithmetic": "integer intermediates below 2^53; exact in JavaScript and i64",
            "validation": "exit status and exact stdout checked on every process",
            "reference_algorithms": "iterative Fibonacci, affine exponentiation, prime sieve",
            "optimization": "native compilers may inline, fold constants, and optimize freely",
            "limitations": "short workloads include substantial startup overhead; not steady-state throughput or a general language ranking",
        },
        "cases": {},
    }
    expected = expected_outputs()
    source_hashes = {}
    with tempfile.TemporaryDirectory(prefix="cussy-language-benchmarks-") as build:
        build = Path(build)
        commands = {}
        for case in expected:
            sources = {language: SOURCE_DIR / f"{case}.{extension}"
                       for language, extension in EXTENSIONS.items()}
            source_hashes.update({path: digest(path) for path in sources.values()})
            native_binaries = {}
            compile_commands = {}
            for language, compiler, flags in (("c", "clang", C_FLAGS), ("rust", "rustc", RUST_FLAGS)):
                suffix = ".exe" if os.name == "nt" else ""
                binary = build / f"{case}-{language}{suffix}"
                source = sources[language]
                subprocess.run(
                    [str(tools[compiler]), *flags, str(source), "-o", str(binary)],
                    check=True, capture_output=True, text=True,
                )
                native_binaries[language] = binary
                compile_commands[language] = [
                    compiler, *flags, str(source.relative_to(ROOT)), "-o", "<build>/" + binary.name,
                ]
            commands[case] = {
                "cussy": [str(cussy), "run", str(sources["cussy"]), "--fuel", "50000000"],
                "c": [str(native_binaries["c"])],
                "rust": [str(native_binaries["rust"])],
                "python": [str(tools["python"]), str(sources["python"])],
                "javascript": [str(tools["node"]), str(sources["javascript"])],
            }
            report["cases"][case] = {
                "expected_stdout": expected[case],
                "sources": {
                    language: {"path": str(path.relative_to(ROOT)), "sha256": source_hashes[path]}
                    for language, path in sources.items()
                },
                "native_compile_commands": compile_commands,
                "native_executable_sha256": {language: digest(path) for language, path in native_binaries.items()},
                "measurements": {},
                "timed_orders": [],
            }
        # All compiler processes have exited before the first warmup or sample.
        print("Median fresh-process CLI time (milliseconds):", flush=True)
        print(f"{'workload':20}" + "".join(f"{language:>14}" for language in LANGUAGES), flush=True)
        for case_index, case in enumerate(expected):
            for _ in range(args.warmup):
                for language in LANGUAGES:
                    measure(commands[case][language], expected[case])
            samples = {language: [] for language in LANGUAGES}
            for sample in range(args.runs):
                offset = (case_index + sample) % len(LANGUAGES)
                order = LANGUAGES[offset:] + LANGUAGES[:offset]
                report["cases"][case]["timed_orders"].append(list(order))
                for language in order:
                    samples[language].append(measure(commands[case][language], expected[case]))
            results = {language: summarize(times) for language, times in samples.items()}
            report["cases"][case]["measurements"] = results
            print(f"{case:20}" + "".join(
                f"{results[language]['median_seconds'] * 1000:14.3f}" for language in LANGUAGES
            ), flush=True)
    # Refuse to save misleading provenance if anything was rebuilt during timing.
    for name, path in tools.items():
        if digest(path) != tool_hashes[name]:
            raise RuntimeError(f"{name} executable changed during measurement; rerun")
    for path, original in source_hashes.items():
        if digest(path) != original:
            raise RuntimeError(f"{path.name} source changed during measurement; rerun")
    report["finished_at_utc"] = datetime.now(timezone.utc).isoformat()
    if args.json:
        serialized = json.dumps(report, indent=2) + "\n"
        if str(Path.home()) in serialized or str(ROOT) in serialized:
            raise RuntimeError("report unexpectedly contains a private absolute path")
        args.json.parent.mkdir(parents=True, exist_ok=True)
        args.json.write_text(serialized, encoding="utf-8")


if __name__ == "__main__":
    try:
        main()
    except (OSError, RuntimeError, subprocess.SubprocessError) as error:
        print(f"benchmark failed: {error}", file=sys.stderr)
        if isinstance(error, subprocess.CalledProcessError) and error.stderr:
            print(error.stderr, file=sys.stderr)
        sys.exit(1)
