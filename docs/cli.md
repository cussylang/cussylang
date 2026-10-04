# Command line and REPL

```text
cussy run source.cussy [--fuel N] [--allow-ffi] [-- args...]
cussy run image.csyb [same options]
cussy check source.cussy
cussy build source.cussy [-o output.csyb]
cussy compile source.cussy [-o program] [--emit exe|asm|obj|c] [-O0|-O1|-O2|-O3|-Os]
cussy compile source.cussy [--cc PATH] [--cpu native]
cussy fmt source.cussy [--stdout | --check]
cussy repl
cussy --version
cussy --help
```

Source files use `.cussy`; legacy `.csy` files remain supported. Build images
continue to use `.csyb`.

Options follow the command. Diagnostic personality is `--normal` (default),
`--brainrot`, or `--jole`. Normal errors are concise; both meme modes still include
file, location, and technical explanation. CLI errors go to stderr. Program output
goes to stdout. `check` prints VERIFIED, `build` prints BUILD VERIFIED; meme build
modes add `whitecaplol is our papa`.

- `run`: load, check, execute; exit code is the program int result's low eight bits.
- `check`: resolve imports and check all functions without running user code.
  It cannot predict runtime I/O failure, bounds errors, or dynamic pointer lifetime.
- `build`: write a JSON AST image containing modules and diagnostic source text.
  Default output replaces the source suffix with `.csyb`. It never overwrites a
  loaded source path. No standalone native executable or Rust compiler is needed
  to run a built image; the Cussy runtime is required.
- `compile`: generate a native executable (default), assembly (`--emit asm`), an
  object (`--emit obj`), or GNU C11 source (`--emit c`). Compilation defaults to
  `-O3`; `--cpu native` tunes for the build machine. `--cc` or `CUSSY_CC` selects
  a Clang/GCC executable. Native executables run without Cussy. Unsupported
  operations fail explicitly. See the [native guide](native.md).
- `fmt`: writes token-preserving formatting in place. `--stdout` previews it;
  `--check` leaves the file alone and exits 1 if formatting would change it.
  Strings and comment content remain comments; the formatter does not type-check.
- `--fuel`: sets AST execution steps, default 5,000,000, a work budget rather than
  milliseconds. It includes graph evaluation. It does not interrupt blocking input,
  filesystem calls, sleeps, or native functions. Every recursive call is also subject
  to a depth limit of 128.
- `--allow-ffi`: enables the explicitly unsafe dynamic-library boundary on Unix.
- `--`: for `run`/`repl`, remaining tokens become the program's `arg()` values.
  For `compile`, it ends option parsing; compiled programs receive their own
  command-line arguments when launched.

Driver/type/runtime errors exit 1; usage errors exit 2. Invalid or incompatible
build images are rejected. Artifacts require the exact Cussy version that built
them. Treat sources and artifacts as executable programs, not a sandbox format.

## REPL

```text
$ cussy repl
CUSSY REPL v0.2.0
C + Desmos + Geometry Dash + irreversible jole exposure
whitecaplol is our papa
whitecaplol hop on stream
>>> int aura = 100;
>>> aura += 50;
>>> aura
150
>>> graph math;
>>> sqrt(81)
9
>>> :quit
escaped successfully
```

Declarations need normal semicolons. An expression with no trailing semicolon is
printed using `jole`. With a semicolon it executes silently (apart from its own
output). Braced functions/blocks can span lines. Imports and definitions persist;
initializers of already loaded declarations are not replayed. Ordinary statements
run in a temporary void function and its locals do not become global variables.
Declarations entered directly at the prompt become persistent globals.

`:help` explains commands, `:reset` clears state, `:quit`/`:q` exits. EOF also exits.
No line-editing/history dependency is included; use the terminal's normal input.
Duplicate declarations are errors; reset to redefine a function. Execution errors
can leave prior side effects, including global mutation or file writes; inputs are
not transactions. Prompts are shown only for interactive terminal input. An unfinished block at EOF
returns an error instead of silently discarding the input.

## Harmless commands

`cussy jole`, `cussy stream`, `cussy papa`, `cussy aura`, `cussy cooked`, `cussy lore`.
They print local text and exit. They never contact YouTube, start broadcasts, post
messages, or mutate settings. `lore` prints the complete invented vocabulary.
