# Changelog

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
