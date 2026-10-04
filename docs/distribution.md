# Install and distribute Cussy

Download prebuilt packages from
[GitHub Releases](https://github.com/cussylang/cussylang/releases/latest).
The executable includes the standard library and does not require Rust or Python.
Running interpreted programs needs no additional compiler. `cussy compile` requires
Clang/GCC and a platform C development toolchain; see [native compilation](native.md).

| Platform | Archive suffix |
| --- | --- |
| Linux x64 (GNU libc; built on Ubuntu 22.04) | `linux-x64.tar.gz` |
| macOS Apple Silicon | `macos-arm64.tar.gz` |
| macOS Intel | `macos-x64.tar.gz` |
| Windows x64 | `windows-x64.zip` |
| Android ARM64 / x86_64 development app | `android.apk` |
| iPhone/iPad app, requires your signing | `ios-unsigned.ipa` |
| iOS device + simulator embedding runtime | `ios-runtime.zip` |
| Rust source, tests, examples, and documentation | `source.tar.gz` |
| VS Code syntax extension starter | `vscode-starter.zip` |

Every name begins with `cussy-<version>-`. `SHA256SUMS` contains the SHA-256 hashes
of all release downloads. See the [mobile guide](mobile.md) for phone installation
and the required iOS signing step. On Linux, verify a downloaded file with `sha256sum`;
on macOS, use `shasum -a 256`; on Windows PowerShell, use
`Get-FileHash -Algorithm SHA256`. Compare its hash with the matching line in
`SHA256SUMS`.

## Prebuilt executable

Extract the archive for your operating system and CPU. For example, on Apple
Silicon macOS:

```sh
tar -xzf cussy-0.3.0-macos-arm64.tar.gz
cd cussy-0.3.0
./bin/cussy --version
./bin/cussy run examples/hello.cussy
./bin/cussy run examples/graphdash/main.cussy
./bin/cussy repl
```

On Windows, extract the ZIP using Explorer or PowerShell:

```powershell
Expand-Archive .\cussy-0.3.0-windows-x64.zip -DestinationPath .
Set-Location .\cussy-0.3.0
.\bin\cussy.exe run examples\hello.cussy
.\bin\cussy.exe repl
```

Add the extracted `bin` directory to PATH for a `cussy` command available
elsewhere. You may also copy the single executable to a user-owned directory
already on PATH. Keep the extracted examples together when running them: file
imports are resolved beside the importing program, while file I/O and graph
outputs use the current working directory.

The macOS and Windows binaries are not commercially code-signed or notarized.
Their operating systems may show an unrecognized-publisher warning. Building
from the tagged source is another installation option.

## Build from source

Install Rust and Cargo at the version required by `Cargo.toml` or newer. Install
the tagged release directly from GitHub:

```sh
cargo install --git https://github.com/cussylang/cussylang --tag v0.3.0 --locked
cussy --version
```

Or clone and build it yourself:

```sh
git clone --branch v0.3.0 https://github.com/cussylang/cussylang.git
cd cussylang
cargo build --release --locked
cargo install --path . --locked
```

Cargo places the executable in `target/release/cussy` (`cussy.exe` on Windows).
The standard library is embedded during compilation. See
[contributing](../CONTRIBUTING.md) for tests and development setup.

`.csyb` files are checked interpreter artifacts, not standalone executables.
Use the same Cussy version that created an artifact, or rebuild it from `.cussy`
source after upgrading.

## Local packaging

The packaging helper requires Python 3.9+ and Cargo; `--smoke-test` also requires
Clang or a compiler selected through `CUSSY_CC`. It reads the version from
`Cargo.toml`, packages an already-built release executable, and writes archives
and `SHA256SUMS` to `dist/`:

```sh
cargo build --release --locked
python3 scripts/package.py --smoke-test
```

Use `python` on Windows if that is your Python command. A target build must use
the same target when packaging:

```sh
cargo build --release --locked --target aarch64-apple-darwin
python3 scripts/package.py --target aarch64-apple-darwin --kind binary --smoke-test
```

`--smoke-test` runs the packaged executable and therefore needs a compatible
host. `--kind source` creates the source and VS Code archives without requiring
a binary. `--binary <path>` and `--output-dir <path>` support other build/output
locations. Each invocation writes checksums for the archives it creates; the
release workflow combines all platforms into one final `SHA256SUMS`.

The VS Code ZIP contains a syntax extension folder, not a Marketplace release or
VSIX. Follow [the editor instructions](../editors/vscode/README.md) to install it.

## Maintainer release process

1. Update `Cargo.toml`, `Cargo.lock`, the VS Code extension version, and the
   changelog. Refresh versioned installation examples.
2. Run the contributor checks and `python3 scripts/package.py --smoke-test`.
3. Push the reviewed commit to `main`, confirm CI passes, then create and push a
   matching tag, for example `git tag v0.3.0` and `git push origin v0.3.0`.
4. The release workflow builds and tests Linux x64, macOS arm64, macOS x64, and
   Windows x64 on matching hosts. Each job tests its extracted package. The mobile
   workflow builds Android and iOS apps and runs tests in both platform simulators.
5. Once every platform succeeds, a separate job creates the source/editor
   archives, checks the complete asset set, computes checksums, and publishes
   the GitHub release with generated notes.

A failed platform blocks publication. Fix and rerun the failed workflow before
claiming the release is available. GitHub Actions are pinned to reviewed commit
SHAs, and Dependabot proposes dependency updates monthly.
