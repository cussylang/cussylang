# Cussy language guide — 0.2.0

This document describes implemented behavior. Cussy is C-shaped and statically
checked before execution, with an interpreter that checks runtime values, bounds,
and storage lifetime. The default entry point is `whitecap`.

## 1. Installation

Install Rust 1.88+ with Cargo, then run `cargo install --path . --locked` in the
project. Ensure Cargo's binary directory is on PATH. No C compiler, Python, Node,
Desmos account, or Geometry Dash installation is required. The `./cussy` launcher
is available on Unix; `cargo run --release -- ...` works on all Rust platforms.
First build downloads the locked serde dependencies. Subsequent builds can work
offline after those dependencies are cached.

## 2. Hello World

```cussy
int whitecap() {
    jole("hello, jole");
    verify 0;
}
```

Save as `hello.cussy`, run `cussy run hello.cussy`. `int main()` is also accepted;
`whitecap` wins if both exist. The entry point takes no parameters and returns
`int` or `void`. Integer return values become the process's low eight exit-code
bits. Zero means success. `verify verified;` in an int function therefore exits 1.
Program arguments are available through `graph system;` and `argc()/arg(i)`.

## 3. Variables

```cussy
int attempts = 0;
float speed = 1.5;
locked int limit = 100;
```

Declarations require explicit types. Blocks create lexical scopes. Duplicate
names in one scope are errors; inner blocks can shadow outer variables. Functions
see their own locals and globals, not their caller's locals. Non-locked variables
without initializers get a type-specific zero/empty/null value. Locked variables
require an initializer. Global initializers run in dependency/declaration order;
functions may be declared later, but a global they read must already exist.

## 4. Primitive types

| Type | Representation / rule |
|---|---|
| `int`, `long` | Signed 64-bit integer |
| `unsigned long`, `unsigned int`, `unsigned` | Unsigned 64-bit integer |
| `float`, `double` | Finite IEEE-754 binary64 |
| `bool` | `verified` / `unverified` |
| `char` | One Unicode scalar, e.g. `'λ'` |
| `string` | UTF-8 string, e.g. `"jole"` |
| `void` | No returned value |

Numeric literals support decimal digits, underscores, decimal points, exponents,
and a `u` suffix for full-width unsigned values: `18446744073709551615u`.
Float promotion is implicit; narrowing to integer uses `to_int`. Bool can initialize
or return int as 0/1. A signed value assigned to unsigned is checked for negativity.
Mixed signed/unsigned arithmetic produces unsigned and rejects negative operands.
Integer arithmetic never silently wraps. Integer division truncates toward zero.
Converting large integers to float can lose precision. Hex literals are not part
of v0.1. The minimum int is expressible as `-9223372036854775807 - 1`.

## 5. Operators and comments

From tightest to loosest: calls/index/fields/postfix updates; pointer and logical
prefix operators; power `^` (right associative); numeric unary signs; `* / %`;
`+ -`; `< > <= >=`; `== !=`; `&&`; `||`; `?:`; assignments. Unary numeric signs bind
less tightly than power, so `-2^2` is -4. `^` is exponentiation, never XOR.

`= += -= *= /= %=`, prefix/postfix `++ --`, `&`, `*`, `.`, and `->` are supported.
Logical operators short-circuit. Conditional expressions evaluate only the chosen
branch. Conditions accept booleans or numbers. Strings concatenate with `+`.
Equality compares aggregate values; pointer equality compares storage identity.
Argument and operand evaluation is left to right. There are no bitwise operators.

Use `// line comments` or nested `/* block /* comments */ work */`.
String/char escapes: `\n`, `\r`, `\t`, `\0`, `\\`, `\"`, `\'`.

## 6. Functions

```cussy
int fib(int n) {
    check (n < 2) { verify n; }
    verify fib(n - 1) + fib(n - 2);
}
void greet(string name) { jole("hello", name); }
```

Signatures are collected before bodies, enabling forward calls and recursion.
Arguments, arrays, and objects are passed by value. Pointers preserve aliasing.
Every path through a non-void ordinary function must return a compatible value.
The analysis is conservative: even an apparently infinite loop or exhaustive
trigger needs a final `verify` in a non-void function. No user function overloads,
closures, variadic declarations, or function-pointer type syntax in v0.1.

## 7. Control flow

```cussy
attempt (int i = 0; i < 10; i++) {
    check (i == 2) { noclip; }
    check (i == 8) { crash; }
    otherwise { jole(i); }
}
ticker (attempts < 3) { attempts++; }
trigger (attempts) {
    checkpoint 0: jole("fresh attempt"); crash;
    checkpoint 3: jole("third attempt"); crash;
    practice: jole("keep joling");
}
```

`noclip` runs the for-loop step before checking the next condition. `crash` exits
the closest loop or trigger. Trigger labels are scalar literals; one `practice`
branch is allowed. Arms never fall through. Variables in each arm or loop body
are scoped separately. C spellings work as listed in the keyword table.

## 8. addaterm

```cussy
addaterm attempts unsigned long;
addaterm aura int;
addaterm jole string;
addaterm megajole jole;
addaterm PAPA "whitecaplol";
addaterm say = jole;
```

Type aliases are transparent and may chain to an already declared/imported type.
The last two forms create locked value aliases. Initializers evaluate once at
module initialization; expression aliases are values, not re-evaluated macros.
Use `=` to disambiguate a value/function whose name is also a type alias.
`addaterm say = jole;` aliases the function even when `jole` names the string type.
Ordinary function aliases retain their checked signature. Polymorphic prelude
operations such as `alloc` should be called directly so their result type can be
inferred. There are no namespace aliases, operator aliases, or textual macros.

## 9. jole

`jole()` prints `jole`. `jole(a, b)` prints values separated by a space and appends
a newline unless the rendered text already ends in one. It is always available;
serious programs do not need the brainrot module. `yap` provides explicit newline
and format control. Import `graph jole;` for `rejole`, `unjole`, `megajole`,
`joling`, `jolemaxxing`, `jolepoint`, and `JOLE`.

**jole means jole. Not joke.**

## 10. stream

```cussy
graph stream;
void demo() {
    stream { assert(onstream()); jole(STREAM); }
    hop_on_stream();
}
```

A `stream` block temporarily makes `streamstatus()` true, including in called
functions. Exiting the block restores its depth on return, break, or an error.
`hop_on_stream()` prints `whitecaplol hop on stream` and sets the runtime's
persistent hopped flag. `streamcheck()` prints the request when offstream and
returns the current status. These are local simulation states; they do not inspect
or start YouTube broadcasts, threads, network streams, or background tasks.

## 11. papa

`graph papa;` provides `PAPA = "whitecaplol"`, `papa()`, `ourpapa()`, and
`blessedbypapa()`. `papa()` prints **whitecaplol is our papa**. It is a fan-project
jole, not a statement of affiliation. Compiler success only adds papa lore in the
optional brainrot/jole modes. Normal mode stays compact.

## 12. Arrays

```cussy
int a[3] = [3, 1, 2];
int[] b = a;
b[0] = 9;
int* first = &a[0];
```

Arrays are homogeneous, bounds-checked values. `int a[3];` zero-initializes three
elements. `int a[] = [...]` and `int[] a = [...]` infer initializer length.
The `[3]` is an allocation/initialization length check, not part of the type;
a later whole-array assignment can replace its length. `len(a)` returns length.
Arrays copy on assignment and function passing. Use `int[]*` to modify a caller's
array. Replacing an array with a different length invalidates pointers to its old
elements. Same-length assignments preserve element storage. There is no implicit
array-to-pointer decay and no pointer arithmetic.

## 13. Strings

Strings use double quotes; chars use single quotes. Indexes and `len` count Unicode
scalars, not UTF-8 bytes or grapheme clusters. String elements are immutable;
replace the whole string instead. `graph string;` adds slicing, substring search,
replacement, and Unicode upper/lowercase. `to_string(value)`, `to_int(value)`, and
`to_float(value)` are prelude conversions; invalid conversions produce diagnostics.
`listen()` reads one line from stdin and removes the line ending.

## 14. Structures

```cussy
object Cube { string name; point position; int aura; };
Cube c = Cube { name: "graphdemon", position: (1, 2), aura: 100 };
```

Fields have declared types. Omitted fields receive defaults; unknown/duplicate
fields are errors. Access with `c.aura` or `ptr->aura`. Structures copy by value.
Use pointers for recursive links: `object Node { int value; Node* next; };`.
A structure containing itself directly is rejected. `lore Difficulty { EASY,
DEMON = 10 };` creates an int type alias and locked integer constants.

## 15. Memory

```cussy
int x = 100;
int* p = &x;
*p += 10;
int* heap = alloc(x);
free(heap);
heap = cooked;
```

Pointers reference typed interpreter storage, never raw addresses. Dereferencing
null, a freed allocation, or storage from an expired scope raises `COOKED`.
`free(cooked)` does nothing; double free and freeing stack storage are errors.
Free does not silently overwrite other pointer variables with null. Aggregate
field/element references are also invalidated when their owning storage expires.
`locked` prevents mutation of owned fields; a locked pointer binding can still
mutate its pointee. Raw casts and arbitrary address arithmetic are unavailable.

`hitbox(value)` reports logical payload size: int/float/pointer 8, bool 1, char 4,
point/vector 16, strings in UTF-8 bytes, aggregates as the sum of fields. This is
not a native `sizeof`, allocator accounting, alignment, or a C ABI guarantee.

## 16. Modules

`graph math;` loads an embedded standard library. `graph "level.cussy";` resolves
relative to the importing file. Imports without an extension prefer `.cussy`,
falling back to legacy `.csy` only when the `.cussy` path does not exist.
A module executes once per program. Diamond imports deduplicate and cycles are
rejected. Imported declarations occupy one shared global namespace, with duplicate
names rejected. Import types before using them. Libraries need no `whitecap`;
the final executable program does. `.cussy` is the primary extension for programs
and libraries; explicitly named `.csy` files remain compatible.

Built artifacts contain module ASTs and source text, so original files and a
separate installed stdlib folder are unnecessary. `graph` statements do not fetch
remote packages; there is no package registry or network import mechanism.

The math module's `min` and `max` define signed-zero ties consistently on every
platform: `min(-0.0, 0.0)` is `-0`, and `max(-0.0, 0.0)` is `0`, in either
argument order. Equal-sign zero inputs retain their sign.

## 17. Desmos types

`point p = (5, 10);` creates a point. `vector v = (3, 4);` accepts the same literal
with vector semantics. `.x` and `.y` are writable floats. Equal point/vector types
support addition/subtraction and right-hand scalar multiplication/division.
`graph graph;` supplies `dot`, `magnitude`, `polar`, and `plot_points`.

`list nums = [1, 2, 3];` creates a numeric list. `nums * 2`, `2 * nums`, or two
equal-length lists use element-wise arithmetic. Unequal lengths are errors.
Lists normalize numeric elements to float. For strings or records use typed arrays.

## 18. Graph expressions, sliders, regression

```cussy
graph desmos;
Slider gain = slider(1, 0, 4, 0.25);
graph f(x) = gain.value * (x^2 + 3x - 4);
graph circle(t) = (cos(t), sin(t));
graph rose(t) = polar(2 * cos(5 * t), t);
graph piecewise(x) = x < 0 ? -x : x^2;
domain circle [0, TAU];
range f [-20, 20];
```

A graph is a named one-parameter function returning a number or point. Numeric
literal/identifier adjacency, such as `3x`, is multiplication inside graph
expressions only. Use explicit `*` elsewhere, including `2 * (x + 1)`.
Graph return inference proceeds in declaration order; declare graph dependencies
before graph functions that use them. Numeric ordinary functions remain callable.

`domain f [lo, hi];` chooses the sampling interval (default -10 to 10). `range`
sets the vertical plot window. Both require finite increasing endpoints. They
control plotting, not the validity of direct `f(x)` calls. Domain errors while
sampling create gaps; ordinary program errors stop rendering. `plot f;` saves
`f.svg` in the working directory; `plot f to "path.svg";` selects a path.

The SVG renderer samples 801 positions, draws axes/grid, clips to the plot window,
and breaks large jumps. Point/parametric graphs keep equal unit scales on both
axes, expanding the viewport beyond requested range bounds if necessary. It is a sampled visualization, not a symbolic graph engine
or adaptive proof of continuity. Point graphs and polar helpers render through
the same pipeline. Sliders are mutable values changed by `slide(&gain, ticks)`;
they are not GUI widgets. Re-render to see a changed slider. `ticker_time()` from
`time` provides elapsed monotonic seconds; ticker loops are synchronous.

`Regression fit = regression([1, 2, 3], [3, 5, 7]);` computes ordinary least-squares
slope, intercept, and R². Lists need equal lengths, at least two observations, and
nonzero x variance. Constant y reports R² = 1 by this library's convention.
`predict(fit, x)` evaluates the line. This version has linear regression only.

## 19. Geometry Dash utilities

`graph gd;` defines Player, Hitbox, and LevelObject records. Use `start_attempt`,
`advance`, `set_checkpoint`, `set_practice`, `set_noclip`, `hits`, `collision`,
`percentage`, `frame_time`, and `level_verified`. Touching AABB edges do not count
as collisions. Progress clamps to 0..100, and practice restarts from the saved
percentage. `frames` counts calls to `advance`; it is not an actual game clock.

These are deterministic simulation helpers. They do not connect to the game,
read its memory, import level files, change game physics, or verify a real level.
`examples/graphdash/main.cussy` combines the helpers, local modules, and graphing.

## 20. Brainrot library

`graph brainrot;` imports jole, stream, papa, and math and adds a mutable `aura`
(initially 100), aura gain/loss/check, locked-in/washed state, glaze/unglaze,
megaglaze, chat, unc, peak, and jolemode. `AURA_MAX` is a lore constant, not an
automatic clamp; checked integer arithmetic still applies. Most meme combinations
are documented lore rather than reserved words. See the API and `cussy lore`.

## 21. Standard library

The prelude is small: output/input, assertions, lengths, logical sizes, conversions,
and checked allocation. Fourteen optional source modules are embedded in the
binary and written in Cussy. Their host primitives are prefixed `__`; prefer the
public library names. [The API reference](stdlib.md) lists every module and function.

## 22. Error handling

Parse/type errors prevent execution. Runtime errors terminate the current run
with file, line, column, an underline where source is available, and a technical
explanation. Select `--normal`, `--brainrot`, or `--jole`; none hides the technical
error. `assert(condition, optionalMessage)` raises `ASSERT` on failure.

Codes include JOLE42 (syntax), D404 (lookup/import), AURA-12 (types), LOCKED,
WASHED (missing return), COOKED (pointer), DOMAIN, AURA_OVERFLOW, IO, FORMAT, FFI,
PAPA01 (missing entry), and LIMIT. Programs have no exception/catch syntax in v0.1;
validate inputs using conditionals and return explicit status values.

## 23. FFI

```cussy
graph system;
float answer = native1("/usr/lib/libSystem.B.dylib", "cos", 0.0);
```

Run with `--allow-ffi`. Unix only: `dlopen` resolves a named C-ABI
`double function(double)` and calls it. Linux's example uses `libm.so.6`.
A wrong symbol ABI or unsafe library can crash or corrupt the process; symbol
names do not carry type metadata. An empty library string searches the current
process. Library load/unload hooks may execute. This capability is explicit and
only for trusted libraries with known signatures. Strings, structs, callbacks,
variadic functions, other signatures, and Windows dynamic FFI are not supported.
Rust consumers can embed the public Loader → Checker → Runtime API directly.

## 24. Compiler architecture

UTF-8 source → lexer tokens with spans → Pratt/recursive-descent parser → AST →
module expansion → symbol/type/return analysis → checked tree interpreter.
`build` serializes the AST and sources as versioned JSON `.csyb`; `run` rechecks
it before execution. The image is portable between hosts running this exact
Cussy version, subject to any platform-specific I/O or FFI inside the program.
It is not machine code or bytecode. The formatter operates on tokens and retains
comments/literals. The REPL preserves declarations and runtime state between
inputs. The Rust project exports a library as well as the `cussy` binary.

See [architecture and limits](architecture.md) for module boundaries, budgets,
platform coverage, and extension points. Cussy — the graph is verified; chat is cooked.
