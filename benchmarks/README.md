# Interpreter benchmarks

Build a release executable, then run the fixed workloads:

```sh
cargo build --release
python3 scripts/benchmark.py
```

Every timed run checks the exit status and exact output. The runner records seven
samples after one warmup and reports the median. Timings cover the entire CLI
process, including startup, parsing, type checking, and execution. They depend on
the machine and current system load; they are not comparisons with native C.

For a before/after comparison, preserve a release executable before editing:

```sh
cp target/release/cussy /tmp/cussy-before
# Make changes, then rebuild with the same Rust version and release profile.
cargo build --release
python3 scripts/benchmark.py --compare /tmp/cussy-before --json benchmark-results.json
```

The runner alternates binary order between samples, uses identical source files
and fuel limits for both binaries, and stores executable/source SHA-256 hashes in
the optional JSON report. The workloads cover recursive calls, repeated small
calls, and arithmetic in a numeric loop. Publish benchmark claims only with the actual environment and measurements.


## Measured v0.1.1 improvement

Measured October 3, 2026 on macOS arm64 with Rust 1.96.1. Both executables used
the same release profile (thin LTO, stripped); the baseline is the previous
interpreter before sharing function ASTs and keeping global storage across calls.
Seven alternating samples follow one warmup per executable/workload.

| Workload | Before (median) | After (median) | Ratio |
|---|---:|---:|---:|
| Fibonacci(24) | 0.497680s | 0.075169s | 6.62× |
| 100,000 function calls | 0.260133s | 0.072714s | 3.58× |
| 200,000-iteration numeric loop | 0.087512s | 0.070743s | 1.24× |

[Raw timing samples and SHA-256 hashes](results/macos-arm64-0.1.1.json) are included.
The smaller numeric-loop difference is more sensitive to system load. These are
end-to-end CLI timings for these programs on this machine, not universal speedup
guarantees, cross-platform results, or comparisons with C. The language remains
a checked interpreter with the same execution model.
