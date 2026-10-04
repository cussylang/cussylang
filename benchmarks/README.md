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
| Fibonacci(24) | 0.666275s | 0.088038s | 7.57× |
| 100,000 function calls | 0.336262s | 0.093778s | 3.59× |
| 200,000-iteration numeric loop | 0.118544s | 0.109285s | 1.08× |

[Raw timing samples and SHA-256 hashes](results/macos-arm64-0.1.1.json) are included.
The smaller numeric-loop difference is more sensitive to system load. These are
end-to-end CLI timings for these programs on this machine, not universal speedup
guarantees, cross-platform results, or comparisons with C. The language remains
a checked interpreter with the same execution model.

## Comparison with other languages

The [README table](../README.md#performance) compares five languages across six execution modes for
three identical algorithms (Cussy runs both interpreted and natively compiled):

- Recursive Fibonacci(24), returning `46368`.
- 200,000 iterations of `x = (x * 1664525 + 1013904223) % 2147483647`,
  starting at `12345`, returning `588316310`.
- Trial-division prime counting from 2 through 3000, returning `430`.

All [15 source programs](languages) are included. C and Rust use 64-bit integers;
every arithmetic intermediate fits below 2^53, so JavaScript's numbers represent
the integers exactly. The runner calculates expected answers independently using
iterative Fibonacci, affine-transform exponentiation, and a prime sieve. Every
warmup and timed run must return the exact answer with a successful exit status.

Install Clang, Rust, Node.js, and Python 3.9 or newer, then run:

```sh
cargo build --release --locked
python3 scripts/compare_languages.py --json language-results.json
```

Use `python` on Windows if needed. All five language toolchains are required; missing
tools fail explicitly. `--cussy PATH`, `--runs N`, and `--warmup N` allow another
Cussy executable or sample count. The checked-in report uses seven samples after
one warmup for each program. Python is invoked directly without a version-manager
shim. Native programs compile into a temporary directory, which is removed after
measurement; all compilation finishes before warmups and timing begin.

Each sample starts a fresh process. Language order rotates between samples and
workloads. Wall time includes startup, parsing/checking for source interpreters,
execution, output, and shutdown. A separate warmup can warm filesystem and OS
caches but does not preserve a JavaScript JIT between processes. The runner allows
native inlining and constant folding; it adds no artificial `volatile` operations
or `noinline` restrictions. Native build time is excluded.

The [October 3, 2026 report](results/languages-macos-arm64-0.2.0.json) records
Apple M1 Max/macOS 26.6.2 arm64, Cussy 0.2.0 interpreted and native, Apple Clang 17.0.0 `-O3`,
Rust 1.96.1 `-O`, CPython 3.10.14, and Node.js 22.22.0. It includes compiler flags,
tool and program SHA-256 hashes, all samples, medians, minimums/maximums, and run
order. The runner rejects changes to tools or source files during measurement.

The measured Cussy compiler/interpreter source is pinned to
[commit 37f407b](https://github.com/cussylang/cussylang/tree/37f407b6cf997fa64ba682862550342074c17908).
These timings precede a later floating-point `min`/`max` signed-zero portability
fix, which the integer workloads do not exercise. The report retains the original
measured binary hashes; it does not claim timings for the later patched binary.

Native Cussy uses `cussy compile --cc clang` with default `-O3`, checked integer
arithmetic, `-fno-fast-math`, and `-ffp-contract=off`. It receives the exact same
`.cussy` input as the interpreter. All Cussy-native, C, and Rust executables finish
compilation before timing. Each Cussy native executable runs directly without
invoking the Cussy driver. Its command, hash, and seven samples are in the report.

| Workload | Interpreter (ms) | Native Cussy (ms) | Interpreter/native ratio |
|---|---:|---:|---:|
| Fibonacci(24) | 121.165 | 8.772 | 13.81× |
| Integer recurrence, 200k | 88.949 | 5.477 | 16.24× |
| Primes ≤3000 | 24.120 | 5.060 | 4.77× |

Native Cussy was 4.77–16.24× faster than its interpreter here. Its elapsed times
were 1.06–1.25× those of C and 0.76–1.59× those of Rust. These short CLI
workloads include substantial startup cost, so they do not isolate execution
throughput or establish general equivalence with C. They do not assess memory
usage, long-running JIT code, I/O-heavy applications, or other machines.

The earlier [v0.1.1 interpreter-only report](results/languages-macos-arm64-0.1.1.json)
is preserved as a historical measurement; its samples were collected separately.
