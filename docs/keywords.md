# The keyword budget

Cussy keeps the grammar small enough to learn. Most brainrot is library vocabulary.
Keywords are contextual in the parser; avoid using control/type words as variable
names in the contexts where they introduce statements. Identifiers are ASCII,
case-sensitive, and may include underscores and digits after the first character.
Type and value names are separate, so `addaterm jole string; jole text = "hi";`
can coexist with `jole(text);`.

| C / concept | Cussy syntax | Decision |
|---|---|---|
| `if`, `else` | `check`, `otherwise` | Both C and branded spellings work |
| `while` | `ticker` | C spelling also works |
| `for` | `attempt` | Three-clause loop; C spelling works |
| `break` | `crash` | Leaves nearest loop or trigger |
| `continue` | `noclip` | Continues nearest loop |
| `return` | `verify` | Return statement |
| `struct` | `object` | Named record; C spelling works |
| `typedef` | `addaterm Name Type;` | Name first; transparent alias |
| `const` | `locked` | Immutable binding and owned fields |
| `true`, `false` | `verified`, `unverified` | C-style bool spellings work |
| `main` | `whitecap` | Entry point; `main` fallback works |
| `printf` | `yap(...)` | Prelude function, not a keyword |
| line output | `jole(...)` | Central prelude function |
| line input | `listen()` | Returns a string, not `scanf` |
| `enum` | `lore Name { ... };` | Integer constants; transparent int alias |
| `NULL` | `cooked` | Checked null pointer literal |
| `sizeof` | `hitbox(value)` | Logical payload size, not ABI layout |
| `switch` | `trigger` | No implicit fallthrough |
| `case` | `checkpoint` | Literal label |
| `default` | `practice` | Trigger fallback |
| module import | `graph math;` | Embedded library |
| local import | `graph "helper.cussy";` | Relative to the importing file |
| graph function | `graph f(x) = expression;` | One numeric parameter |
| plot constraints | `domain`, `range` | Sampling interval and vertical view |
| rendering | `plot f to "f.svg";` | `to` and path optional |
| stream context | `stream { ... }` | Scoped runtime stream status |
| primitives | `int`, `long`, `unsigned`, `float`, `double`, `char`, `bool`, `string`, `void` | Actual types |
| graph values | `point`, `vector`, `list` | Actual types |

`fusion`/union, `camping`/static, `tweaking`/volatile and `dash`/goto are **not
implemented keywords**. `addaterm` is not a textual macro preprocessor. `jole`,
`papa`, `aura`, `glaze`, `peak`, `chat`, `unc`, `slider`, `regression`, and
`hop_on_stream` are functions, values, aliases, or ordinary identifiers.

Names such as graphmaxxing, papapilled, heliopilled, demonslop, yapengine and
slopecore live in the documentation and `cussy lore`; they do not occupy the
grammar. `cooked()` cannot be a library call because `cooked` is the null literal.
Use `washed()`, `is_cooked(int*)`, or `p == cooked` as appropriate.
