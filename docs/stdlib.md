# Standard library

Import a module with `graph name;`. Public wrappers live in `stdlib/*.cussy` and
are embedded into the binary at build time. Imports can be combined and deduplicate.
The `__` names are implementation primitives, not the recommended public API.

## Prelude — no import

| Function | Result and behavior |
|---|---|
| `jole(values...)` | void; space-separated output + newline; zero arguments prints `jole` |
| `yap(string format, values...)` | void; checked formatting with explicit newlines |
| `listen()` | string; one stdin line without its line ending |
| `assert(bool test, message?)` | void; false raises ASSERT |
| `len(string/array/list)` | int; Unicode scalar count or element count |
| `hitbox(value)` | int; logical payload bytes, not native sizeof |
| `to_string(value)` | string; Cussy display representation |
| `to_int(value)` | int; checked conversion; float truncates toward zero |
| `to_float(number/string)` | float; checked finite conversion |
| `alloc(value)` | pointer to an independent typed copy |
| `free(pointer)` | void; null is harmless, expired/non-heap storage is an error |

`yap` supports `%d` int, `%u` unsigned, `%f` numeric, `%s` string, `%c` char,
`%b` bool, `%v` any value, and `%%` literal percent. Width right-aligns with spaces;
`%8d` and `%.2f` work. Float precision defaults to 6. Width is limited to 1000 and
precision to 15. There is no C varargs memory access, `%n`, or zero-fill flag.
Mismatched placeholder counts and types produce FORMAT diagnostics at runtime.

## stdio

`print(string) -> void`, `println(string) -> void`. Both delegate to the prelude.
The module is optional even for Hello World.

## math

`PI`, `TAU`, `E`: locked floats.

`sin`, `cos`, `tan`, `sqrt`, `abs`, `floor`, `ceil`, `round`, `exp`, `log` each take
one numeric argument and return float. `log` is natural logarithm; angles are
radians. `pow(a,b)`, `min(a,b)`, `max(a,b)`, `atan2(y,x)` return float.

`clamp(value, low, high)` validates low ≤ high. `lerp(a,b,t)` interpolates without
clamping t. All results must be finite. Division by zero and invalid square roots
are errors, not NaN values silently spreading through a program.

## graph

Imports math. `polar(radius, angle) -> point`, `dot(vector, vector) -> float`,
`magnitude(vector) -> float`, `plot_points(point[], string path) -> void`.
Point arrays are drawn as an ordered polyline. The `graph`, `domain`, `range`, and
`plot` constructs are language syntax and do not themselves require this module.

## desmos

Imports math and graph.

- `Slider { float value; float low; float high; float step; }`
- `slider(value, low, high, step) -> Slider`: low < high; positive step; clamps value.
- `slide(Slider*, float ticks) -> void`: advances by ticks × step and clamps.
- `Regression { float slope; float intercept; float r2; }`
- `regression(list xs, list ys) -> Regression`: linear least squares with intercept.
- `predict(Regression, float x) -> float`
- `sum(list) -> float`, `mean(list) -> float` (mean rejects empty lists).
- `linspace(float low, float high, int count) -> list`: inclusive endpoints;
  2..100000 values; descending endpoints are permitted.

Regression rejects unequal lengths, fewer than two observations, and constant x.
Constant y uses R² = 1. This is an explicit convention, not an inferential claim.
Sliders and ticker loops are programmatic; they do not open an interactive Desmos UI.

## gd

Imports math. Data structures:

```cussy
object Player {
    int attempts; float percent; bool practice_mode; bool noclip_mode;
    int frames; float checkpoint_percent;
};
object Hitbox { float x; float y; float width; float height; };
object LevelObject { int id; point position; Hitbox bounds; bool solid; };
```

| Function | Behavior |
|---|---|
| `player_new() -> Player` | All fields zero/false |
| `start_attempt(Player*)` | Increments attempts; resets frames and progress, or restores practice checkpoint |
| `advance(Player*, float amount)` | Clamps progress to 0..100; increments frames |
| `set_checkpoint(Player*)` | Saves current progress |
| `set_practice(Player*, bool)` | Toggles practice state |
| `set_noclip(Player*, bool)` | Toggles collision bypass state |
| `collision(Hitbox, Hitbox) -> bool` | AABB overlap; negative sizes rejected; touching edges do not overlap |
| `hits(Player, Hitbox, Hitbox) -> bool` | collision unless noclip enabled |
| `frame_time(float fps) -> float` | 1/fps; positive fps required |
| `percentage(float completed, float total) -> float` | Clamped percentage; total > 0 |
| `level_verified(Player) -> bool` | Progress ≥ 100 |

Language `trigger/checkpoint/practice` dispatches program events. These are local
simulation helpers, not integration with a running game or its level format.

## memory

Documents the allocation prelude and adds `is_cooked(int*) -> bool` for null checks.
Use `p == cooked` for other pointer types. An expired non-null pointer still compares
non-null; lifetime is checked when read, written, or freed. Pointers cannot expose
raw addresses, reinterpret data, or address arbitrary bytes.

## string

`slice(string, int start, int count) -> string` uses Unicode scalar indexes.
`contains(string, string) -> bool`, `replace(string, string from, string to) -> string`,
`uppercase(string) -> string`, `lowercase(string) -> string`. All return independent
values. Slice checks its complete interval. Case conversion can change length.

## time

`ticker_time() -> float`: monotonic seconds since Runtime creation.
`sleep(float seconds) -> void`: synchronous; accepts 0..60 per call.
There is no asynchronous scheduler or background ticker.

## random

Imports math. `seed(int) -> void`, `random() -> float` in [0,1),
`random_int(int low, int high) -> int` in [low,high). The implementation is a seeded
xorshift64 generator, default seed `0xC0551E42`; seed zero normalizes to one.
This is reproducible simulation randomness, not cryptographic randomness.

## system

`read_file(string) -> string`: UTF-8; errors on failed reads or files over 8 MiB.
`write_file(string path, string content) -> void`: creates/replaces a file.
`file_exists(string) -> bool`: whether path is a regular file.
`env(string) -> string`: missing/non-Unicode variable produces empty string.
`argc() -> int`, `arg(int) -> string`: arguments after CLI `--`; indexes are checked.
`native1(string library, string symbol, float argument) -> float`: Unix
`double(double)` C ABI, only with `--allow-ffi`. Read the FFI section first.
Ordinary file/environment access is enabled without the FFI flag.

## jole

`JOLE` is the locked string `"jole"`.
`rejole(string)` prints twice; `unjole(string) -> string` adds an `unjoled:` prefix;
`megajole(string)` prints a MEGAJOLE banner; `joling()` announces the transformation;
`jolemaxxing(int count)` repeats `jole`, rejecting negative count;
`jolepoint(float x,float y) -> point` creates a point. All other listed functions
return void. Every one is ordinary Cussy source.

## stream

`STREAM = "whitecaplol hop on stream"`.
`streamstatus()/onstream()/offstream() -> bool` query scoped depth and hopped state.
`hop_on_stream()/hoponstream()` print the phrase and set hopped state true.
`streamcheck() -> bool` prints when currently offstream.
`streammaxxing()` hops on; `streamcore()` announces that chat is streamified.
This module does not query any creator's actual live status.

## papa

`PAPA = "whitecaplol"`. `papa()` and `ourpapa()` print `whitecaplol is our papa`.
`blessedbypapa()` prints `PAPA APPROVED`. All are void functions.

## brainrot

Imports jole, stream, papa, math. `aura` starts at 100 and is mutable.
`AURA_MAX = 1000000` is a lore constant; no automatic cap is applied.
`COOKED/WASHED = unverified`; `PEAK/LOCKED_IN = verified`.

`aura_gain(int)`/`aura_loss(int)` reject negative amounts, update aura, and return
its new int value. `aura_check() -> int`, `locked_in() -> bool` (aura ≥ 100),
`washed() -> bool` (aura < 0). `glaze(string)`, `unglaze(string)`, `hardglaze(string)`,
`megaglaze(string)`, `chat(string)`, `unc()`, `peak()`, `jolemode()` print lore.
All serious modules work without importing brainrot. Bro may opt out of glazing.
