# Cussy

**C, but somebody opened Desmos.**

[![CI](https://github.com/cussylang/cussylang/actions/workflows/ci.yml/badge.svg)](https://github.com/cussylang/cussylang/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/cussylang/cussylang)](https://github.com/cussylang/cussylang/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Cussy is a statically checked, C-inspired language with a native compiler and
an interpreter, written in Rust.
Curly braces, semicolons, functions, arrays, structures, checked pointers—and
actual graph rendering. Its own lexer, parser, type checker, runtime, module
loader, standard library, REPL, formatter, and deliberately excessive jole lore.

Source files use **`.cussy`**. Legacy `.csy` files remain compatible.

**whitecaplol hop on stream.**

**whitecaplol is our papa.**

## Install

**Desktop, no Rust needed:** download the archive for your operating system from
[GitHub Releases](https://github.com/cussylang/cussylang/releases/latest), extract it,
and run `bin/cussy` (`bin/cussy.exe` on Windows). See the
[installation and checksum instructions](docs/distribution.md).

**Build from source:** install [Rust](https://www.rust-lang.org/tools/install)
1.88 or newer, then:

```sh
cargo install --git https://github.com/cussylang/cussylang --tag v0.3.0 --locked
cussy --version
```

Or build a local checkout:

```sh
git clone https://github.com/cussylang/cussylang.git
cd cussylang
cargo build --release --locked
cargo run --release -- run examples/hello.cussy
```

`target/release/cussy` is the standalone executable (`cussy.exe` on Windows).
The standard library is embedded: there are no runtime packages to install.
Unix users can also run `./cussy`, the local incremental-build launcher.

## iOS and Android

The mobile apps edit, open, save, and run `.cussy` programs locally, with output
and diagnostics in the app. Android 8.0+ supports ARM64 devices and x86_64
emulators; iPhone and iPad require iOS/iPadOS 16+.

- **Android:** install the development APK from
  [Releases](https://github.com/cussylang/cussylang/releases/latest).
- **iOS/iPadOS:** build the included Xcode project with your Apple signing team,
  or sign the unsigned IPA yourself. The IPA is not directly installable without
  signing. A device/simulator XCFramework is also available for embedding.

Mobile execution uses the interpreter, including arrays, records, and checked
pointers. It has a run budget and supports embedded standard imports; host file
I/O, plot export, native FFI, and multi-file imports are unavailable in the apps.
These are development distributions, not App Store or Play Store listings.
See [mobile installation, builds, and runtime API](docs/mobile.md).

## Compile to machine code

With Clang installed, compile a `.cussy` program to an optimized executable:

```sh
cussy compile examples/fibonacci.cussy -o fibonacci
./fibonacci
cussy compile examples/fibonacci.cussy --emit asm -o fibonacci.s
```

Use `-o fibonacci.exe` on Windows. Native compilation defaults to `-O3`; optional
`--cpu native` tunes for the build machine. Executables run without Cussy installed.
`--emit obj` produces an object file, and `--emit c` exposes the generated C.
The pipeline lowers checked Cussy to typed C and uses Clang/GCC to optimize and
generate machine code. It contains no interpreter fallback.

The native backend supports the scalar core: numbers, booleans, characters,
immutable strings, functions, variables, loops, conditionals, math, and basic I/O.
Graphs, arrays, structures, pointers, dynamic strings, and some library operations
still use `cussy run`; unsupported native constructs produce a clear compile error.
See the [native compilation guide](docs/native.md) for toolchains, supported
operations, checks, and optimization flags.

## Start joling

Save this as `hello.cussy`:

```cussy
graph stream;
graph papa;

int whitecap() {
    jole("Cussy booting...");
    stream {
        attempt (int i = 0; i < 3; i++) {
            yap("Attempt %d | stream %b\n", i, streamstatus());
        }
    }
    papa();
    verify 0;
}
```

```sh
cussy run hello.cussy
cussy check hello.cussy
cussy build hello.cussy -o hello.csyb
cussy run hello.csyb
cussy fmt hello.cussy --stdout
cussy repl
```

`build` creates a **portable checked AST image**, including imported code and
source locations. It runs with the matching Cussy version without the original
source files. Execution uses the same interpreter as `run`.

## Compile, graph, jole

```cussy
graph desmos;
graph f(x) = x^2 + 3x - 4;
domain f [-10, 10];
range f [-15, 50];

int whitecap() {
    list nums = [1, 2, 3, 4, 5];
    jole(nums * 2);
    plot f to "graph.svg";
    verify 0;
}
```

Open the resulting SVG in a browser. More programs are in the
[examples index](examples/README.md), including sorting, pointers, file I/O,
parametric circles, regression, and a multi-file graph/game simulation.

## What is here

- C-shaped functions, recursion, lexical scopes, integers, floats, Unicode strings
  and chars, booleans, arrays, structures, enum constants, and checked pointers.
- `addaterm` aliases for types, constants and ordinary functions; `check`, `ticker`,
  `attempt`, `verify`, and other familiar control flow with Cussy names.
- Points, vectors, numeric lists, graph expressions, domains/ranges, polar and
  parametric curves, piecewise expressions, sliders, linear regression, SVG plots.
- Optional GD simulation helpers: attempts, progress, practice checkpoints,
  collisions, noclip state, frame timing, and level objects.
- Fourteen embedded `.cussy` modules, file/environment I/O, seeded random numbers,
  and explicitly enabled Unix C FFI for `double function(double)` symbols.
- Normal, brainrot and jole diagnostics. Every personality keeps the technical
  explanation. Serious programs do not need the brainrot library.
- A persistent REPL, comment-preserving formatter, and VS Code highlighting starter.
- Native iOS/iPadOS and Android editors with an embedded Cussy runtime.

## Performance

Fresh-process CLI timings on an **Apple M1 Max, macOS 26.6.2 arm64**, measured
October 3, 2026. Values are median **milliseconds** from seven runs after one
warmup per program; **lower is faster**. Every run produced the same verified
answer for its workload.

| Language / implementation | Fibonacci(24) | Integer recurrence, 200k iterations | Count primes ≤3000 |
|---|---:|---:|---:|
| **Cussy 0.2.0 native**, Clang `-O3` | 8.77 | 5.48 | 5.06 |
| Cussy 0.2.0 interpreter, release build | 121.16 | 88.95 | 24.12 |
| C, Apple Clang 17.0.0 `-O3` | 7.02 | 5.17 | 4.16 |
| Rust 1.96.1 `-O` | 5.52 | 7.18 | 4.75 |
| Python, CPython 3.10.14 | 50.50 | 66.35 | 37.99 |
| JavaScript, Node.js 22.22.0 | 51.83 | 58.93 | 56.80 |

These measurements include process startup, source parsing where applicable,
execution, and shutdown. Native compilation happens before timing, with compiler
optimizations enabled. Native Cussy completed these commands **4.8–16.2× faster than its interpreter**.
Native Cussy took 1.06–1.25× C's elapsed time in these runs.
Short commands are sensitive to process startup costs; these are not isolated execution-throughput
measurements or long-running JIT benchmarks.

[Equivalent source programs](benchmarks/languages),
[raw samples and environment metadata](benchmarks/results/languages-macos-arm64-0.2.0.json),
and [methodology and reproduction commands](benchmarks/README.md#comparison-with-other-languages)
are included. These results describe these programs and this machine, not a
general ranking of languages.

The interpreter keeps its existing checked semantics. Function bodies are shared and call
frames reuse global storage, reducing call overhead. The
[benchmark suite](benchmarks/README.md) contains reproducible workloads and measured
results; those are workload-specific improvements, not a claim of C execution speed.

## Documentation

- [Language guide](docs/language.md) and [keyword table](docs/keywords.md)
- [Standard library](docs/stdlib.md), [CLI and REPL](docs/cli.md)
- [Architecture and limits](docs/architecture.md)
- [Installation and distributions](docs/distribution.md)
- [Native compilation and assembly](docs/native.md)
- [Channel research and attribution](docs/research.md)
- [VS Code extension starter](editors/vscode/README.md)
- [Full lore inventory](docs/lore.txt)

Cussy 0.2 includes native compilation for its scalar core and an interpreter for
the full language, including checked pointers and local SVG plots. It has no raw
pointer arithmetic, GUI graph controls, LSP, or live Geometry Dash integration.
The explicit FFI boundary can call unsafe native
code. See the architecture guide for precise semantics and resource limits.

## Contribute

[Open an issue](https://github.com/cussylang/cussylang/issues/new/choose) or read
[CONTRIBUTING.md](CONTRIBUTING.md). For a local development check:

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

CI runs on Linux, macOS and Windows, with a separate minimum-Rust-version check.
Tagged releases test and package platform binaries, source and editor archives,
and checksums. See [CHANGELOG.md](CHANGELOG.md), [SECURITY.md](SECURITY.md), and the
[code of conduct](CODE_OF_CONDUCT.md).

Installing Cussy may cause spontaneous graphing, excessive jole usage, and
unexplained urges to tell whitecaplol to hop on stream.

**jole means jole. Not joke.** Unofficial fan project; no creator or platform
endorsement is implied. Original code is [MIT licensed](LICENSE); the native
floating-point printer includes [Ryu under the Boost license](src/vendor/ryu/NOTICE.md).
