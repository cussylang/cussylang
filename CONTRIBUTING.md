# Contributing to Cussy

Issues and pull requests are welcome at
[cussylang/cussylang](https://github.com/cussylang/cussylang). Start with a small,
reproducible example for bugs or a concrete use case for language changes.

## Local setup

Install Rust and Cargo at the minimum version listed in `Cargo.toml` or newer,
then clone the repository:

```sh
git clone https://github.com/cussylang/cussylang.git
cd cussylang
cargo build --locked
cargo run -- run examples/hello.cussy
```

Use `.cussy` for new source files. Legacy `.csy` imports remain supported.
Read [the architecture](docs/architecture.md) and [language guide](docs/language.md)
before changing syntax, type checking, or runtime behavior.

## Before opening a pull request

Keep changes focused. Include a regression test for a behavior fix, and update
examples or documentation when users will see a change. Preserve useful
diagnostics and checked-pointer behavior. Performance changes should include a
reproducible comparison and must preserve language behavior.

Run these commands from the repository root on any supported platform:

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked --release
```

On macOS/Linux, `./scripts/verify.sh` runs the same checks. To verify a release
archive as well, run `python3 scripts/package.py --smoke-test` with Python 3.9 or
newer. On Windows, use `python` instead of `python3` if needed. The package smoke
test extracts into a temporary directory and exercises the executable, imports,
SVG output, and an artifact without its original source files.

CI runs checks on Linux, macOS, and Windows, plus a separate minimum-Rust check.
A passing local test does not prove behavior on a platform you have not tested;
describe the environment used in your pull request.

By contributing, you agree that your contributions are licensed under the
project's [MIT license](LICENSE). Follow the [code of conduct](CODE_OF_CONDUCT.md).
For vulnerabilities, use the [security reporting process](SECURITY.md).

## Releases

Maintainers follow [the distribution guide](docs/distribution.md#maintainer-release-process).
The `v<version>` tag must match `Cargo.toml`. The release workflow tests each
platform and publishes only after all four binary packages succeed.
