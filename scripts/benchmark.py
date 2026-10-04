#!/usr/bin/env python3
"""Measure complete CLI runs with fixed inputs and verified output (Python 3)."""

import argparse
import hashlib
import json
import platform
import statistics
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CASES = {
    "fibonacci": "46368\n",
    "function_calls": "5000050000\n",
    "numeric_loop": "59999900000\n",
}


def positive(value):
    number = int(value)
    if number < 1:
        raise argparse.ArgumentTypeError("must be at least 1")
    return number


def run(binary, name):
    start = time.perf_counter()
    result = subprocess.run(
        [str(binary), "run", str(ROOT / "benchmarks" / (name + ".cussy")),
         "--fuel", "50000000"],
        cwd=ROOT,
        text=True,
        capture_output=True,
        timeout=120,
        check=True,
    )
    elapsed = time.perf_counter() - start
    if result.stdout != CASES[name]:
        raise RuntimeError(f"{name}: unexpected output {result.stdout!r}")
    return elapsed


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/cussy")
    parser.add_argument("--compare", type=Path, help="baseline executable to compare")
    parser.add_argument("--runs", type=positive, default=7)
    parser.add_argument("--warmup", type=positive, default=1)
    parser.add_argument("--json", type=Path, help="save machine-readable measurements")
    args = parser.parse_args()

    binaries = {"current": args.binary.resolve()}
    if args.compare:
        binaries["baseline"] = args.compare.resolve()
    report = {
        "platform": platform.platform(),
        "measurement": "CLI wall time, including startup, parse, check, and execution",
        "runs": args.runs,
        "warmup": args.warmup,
        "binaries": {
            label: {"path": str(path), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
            for label, path in binaries.items()
        },
        "cases": {},
    }
    for name in CASES:
        samples = {label: [] for label in binaries}
        for _ in range(args.warmup):
            for binary in binaries.values():
                run(binary, name)
        for iteration in range(args.runs):
            # Alternate order so one binary is not always measured first.
            labels = list(binaries)
            if iteration % 2:
                labels.reverse()
            for label in labels:
                samples[label].append(run(binaries[label], name))
        result = {
            label: {"median_seconds": statistics.median(times), "samples_seconds": times}
            for label, times in samples.items()
        }
        result["source_sha256"] = hashlib.sha256(
            (ROOT / "benchmarks" / (name + ".cussy")).read_bytes()
        ).hexdigest()
        current = result["current"]["median_seconds"]
        line = f"{name:18} {current:.6f} s"
        if "baseline" in result:
            baseline = result["baseline"]["median_seconds"]
            speedup = baseline / current
            result["speedup"] = speedup
            line += f"  baseline {baseline:.6f} s  {speedup:.2f}x"
        print(line, flush=True)
        report["cases"][name] = result
    for label, binary in binaries.items():
        if hashlib.sha256(binary.read_bytes()).hexdigest() != report["binaries"][label]["sha256"]:
            raise RuntimeError(f"{binary} changed during measurement; discard timings and rerun")
    if args.json:
        args.json.write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
