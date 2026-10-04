# Native compilation

`cussy compile` turns the supported Cussy core into a native executable, assembly,
or an object file. Generated executables run without Cussy, Rust, or a C compiler
installed. They use the operating system's usual C runtime libraries.

```sh
cussy compile examples/fibonacci.cussy -o fibonacci
./fibonacci

cussy compile examples/fibonacci.cussy --emit asm -o fibonacci.s
cussy compile examples/fibonacci.cussy --emit obj -o fibonacci.o
```

On Windows use `-o fibonacci.exe` and run `.\fibonacci.exe`. Without `-o`, the
output uses the source name with its extension replaced by `.exe` on Windows,
no extension on Unix, `.s` for assembly, or `.o`/`.obj` for objects.

## Toolchain

Install Clang and a working platform C development toolchain. On macOS, Apple's
Command Line Tools (`xcode-select --install`) provide these. On Ubuntu/Debian,
install `clang` and `build-essential`. On Windows, use LLVM/Clang with the Visual
Studio C++ Build Tools and Windows SDK, and run from a developer command prompt.

Clang is the default. GCC is also supported through `--cc gcc`. `--cc PATH`
selects a compiler executable, and `CUSSY_CC` sets the default. These accept an
executable name or path, not a shell command containing extra flags.

```sh
cussy compile program.cussy --cc gcc -o program
cussy compile program.cussy --emit c -o program.c
```

`--emit c` needs no external compiler. It exposes the generated source for
inspection or use with a compatible GNU C11 compiler. The pipeline is:

```text
.cussy → lexer/parser → type checker → typed C → Clang/GCC → assembly/object/executable
```

For a manual Unix build, use
`clang -std=gnu11 -O3 -fno-fast-math -ffp-contract=off program.c -lm -o program`.
On Windows, link `-lshell32` instead of `-lm` for Unicode command-line arguments.
The driver selects these platform libraries automatically. The
[Clang command reference](https://clang.llvm.org/docs/CommandGuide/clang.html)
describes its assembly (`-S`), object (`-c`), and optimization options.

Programs use native scalar values, native control flow, and native function
calls. There is no embedded AST interpreter or automatic interpreter fallback
inside the resulting executable.

## Optimization

`-O3` is the default. `-O0`, `-O1`, `-O2`, and `-Os` select other compiler
optimization levels. Functions and runtime helpers are available to the optimizer
in one translation unit, allowing inlining, constant folding, and dead-code
elimination. The C compiler handles instruction selection and register allocation.

```sh
cussy compile program.cussy -O3 --cpu native -o program
```

`--cpu native` permits instructions for the build machine's CPU. Use the default
when distributing a binary to other machines. This is host compilation; assembly
and executables target the compiler's platform and architecture. Executables and
objects are OS formats such as Mach-O, ELF, or PE/COFF, not freestanding firmware
images.

The driver disables fast-math and floating-point contraction. Native and
interpreted `min`/`max` use the same defined signed-zero tie rules. Checked integer
operations still report overflow and division by zero; optimization does not
permit C's undefined signed-overflow behavior. Native execution has no interpreter
fuel counter. Infinite loops can therefore run until interrupted.

## Supported core and boundaries

The first native backend targets scalar programs: signed/unsigned 64-bit integers,
double-precision floats, booleans, Unicode characters, immutable strings, typed
functions and recursion, variables, constants/aliases, arithmetic, comparisons,
conditionals, and loop/switch control flow. Supported math and basic I/O operations
compile with the program. Arguments and expressions retain Cussy's evaluation
order.

Native builtins include `jole`, `yap`, `assert`, `len` for strings, scalar `hitbox`,
`to_int`, `to_float`, `to_string` for an existing string, the math module's scalar
functions, and the system module's `argc`/`arg`. String equality and Unicode
indexing work; literal-only concatenation is folded at compile time. Ternary
branches must have the same scalar type. The 128-call recursion guard remains.

Arrays, records, pointers, numeric lists, points/vectors, and plot statements are
not yet supported by native compilation. Dynamic string construction and some
host-library operations also require the interpreter. Unsupported constructs
produce `NATIVE_UNSUPPORTED` with a source location; they are never silently
replaced with interpretation. See the diagnostic for the specific operation.
All declarations and function bodies are checked for native support, including
unused functions and object declarations. This can reject an imported module
whose unsupported function is never called. Unused aliases to host builtins are
allowed, but calling an unsupported builtin is rejected.

Use `cussy run program.cussy` for the full existing language. `cussy build` still
creates a versioned `.csyb` interpreter artifact; it is separate from native
compilation. Native and interpreted performance results are labeled separately in
the [benchmarks](../benchmarks/README.md).

The native driver checks source/output paths and builds in a private temporary
directory beside the destination. It replaces the output only after successful
generation and compilation, preserving an existing executable when a build fails.

The native floating-point printer includes an adapted
[Ryu implementation and its Boost license](../src/vendor/ryu/NOTICE.md).
