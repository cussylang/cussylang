# Examples

Run from the repository root using `./cussy run examples/NAME.cussy`, or install
Cussy and use `cussy run`. Graph files are written into the process working directory.

| Example | Demonstrates |
|---|---|
| hello | Prelude/stdio and entry point |
| fibonacci | Recursive and forward-call-ready functions |
| fizzbuzz | Branches, modulo, loops |
| primes | Boolean functions and early verification |
| bubble_sort | Mutable array pointer and sorting |
| pointers | Local/heap checked pointers and free |
| structs | Objects, point fields, vector parameters |
| file_reader | UTF-8 file I/O, optional path after `--` |
| desmos_graph | `3x`, powers, lists, regression, SVG |
| parametric_circle | Point-valued graph and TAU domain |
| polar_piecewise | Polar helper and lazy conditional graph |
| gd_attempts | Attempts and player progress |
| percentage | Practice checkpoints and frame time |
| heliopolis | 100-day-themed simulation and graph |
| addaterm | Type/value/function aliases, including jole type |
| jole | JOLE, rejole, unjole, megajole, jolepoint family |
| stream | Scoped and persistent stream state |
| papa | Papa lore and constants |
| aura_loss | Optional brainrot state and recovery |
| full_brainrot | The deliberately excessive 1001-jole loop |
| switch | Lore enum and trigger branches |
| slider | Programmatic slider and ticker time |
| graphdash/main | Multi-file graph/game simulation project |
| native_cos | Actual unary C ABI call, opt-in |

`file_reader` defaults to `examples/data.txt`; supply a different path after `--`
when running elsewhere. Native FFI requires a platform library:

```sh
cussy run examples/native_cos.cussy --allow-ffi -- /usr/lib/libSystem.B.dylib # macOS
cussy run examples/native_cos.cussy --allow-ffi -- libm.so.6                # Linux
```

`cooked/type_error.cussy`, `cooked/dangling.cussy`, and `cooked/double_free.cussy`
are intentionally invalid. Their expected outcomes are AURA-12, COOKED, and COOKED,
respectively; they are successful demonstrations when the runtime refuses them.

```sh
cussy run examples/cooked/dangling.cussy --brainrot
```

`graphdash/level.cussy` is a library module; execute `graphdash/main.cussy`, not that
file alone. Output graphs demonstrate the renderer; they are original small curves,
not recreations or exports of the creators' actual Desmos projects.
