# Changelog

## 0.2.0 — 2026-10-03

- Add `cussy compile` for standalone executables, assembly, object files, and
  inspectable C. The scalar backend uses typed native operations and Clang/GCC
  `-O3` optimization, with optional CPU tuning and no interpreter fallback.
- Preserve evaluation order, checked integer arithmetic, source diagnostics,
  Unicode string output, call-depth checks, and global initialization rules.
- Add an adapted, Boost-licensed Ryu formatter for native floating-point output,
  differential validation, and compiler regression tests against the interpreter.
- Reject unsupported aggregate, pointer, graph, dynamic-string, and host operations
  explicitly; the full interpreter remains available through `cussy run`.
- Publish native and interpreter benchmark results separately, and require native
  compilation and standalone execution in platform CI and package smoke tests.
- Stage compiler output so generation/toolchain errors preserve existing files.
- Make floating-point `min`/`max` signed-zero ties deterministic across native
  compilers, Rust versions, and platforms.

## 0.1.1 — 2026-10-03

- Share function bodies and retain global storage across calls to reduce
  interpreter call overhead while preserving scopes and checked-pointer lifetimes.
  Local macOS arm64 measurements showed 7.57× faster Fibonacci(24), 3.59× faster
  100,000 function calls, and 1.08× faster numeric loops. These are workload-specific
  results; see [the benchmark method and raw samples](benchmarks/README.md).
- Fix checking of shadowed `alloc` calls, diagnostics for missing `Regression`
  members, duplicate switch tags, and enum values at the maximum integer ID.
- Bound file reads consistently with runtime limits.
- Validate CLI flags and report incomplete piped REPL input at end of file.
- Give CLI execution a consistent stack budget, fixing recursive-program crashes
  on Windows, and check this through a CLI regression test.
- Add reproducible comparisons with C, Rust, Python, and JavaScript, including
  exact-output verification and complete timing samples.
- Prepare the public `cussylang/cussylang` repository and contributor documentation.
- Add Linux, macOS, and Windows checks and a release workflow for Linux x64,
  macOS Apple Silicon/Intel, and Windows x64.
- Derive package versions from Cargo metadata, use Windows ZIP archives with the
  `.exe` executable, and include source, examples, documentation, and checksums.
- Verify extracted binary distributions, graphing imports, and portable build
  artifacts before releasing.

Cussy remains a statically checked interpreter. `.cussy` is the primary source
extension, `.csy` remains compatible, and `.csyb` is its checked AST artifact.

## 0.1.0

- Initial interpreter, checker, standard library, CLI, REPL, formatter, graphing,
  checked pointers, GD utilities, examples, and VS Code syntax starter.
