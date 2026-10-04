# Ryu attribution and source

The Ryu-derived portions of `src/codegen_float.h` are copyright 2018
Ulf Adams and contributors and distributed under the upstream
Boost Software License 1.0 option. The full license is included both in
`LICENSE-Boost` and in the generated header so emitted C is self-contained.

Upstream: https://github.com/ulfjack/ryu

Pinned commit: `4c0618b0e44f7ef027ebae05d2cc7812048f7c8f`

Upstream files and SHA-256 hashes:

- `ryu/common.h`: `0bbd71d26da6193e678d0776cf418f43f287c73d6fd6725353df0aadf70f2a19`
- `ryu/d2s_intrinsics.h`: `1d05702f2edacce428223d4b43dd3095c1bd1f3ad30128ce1761d84356dddc7d`
- `ryu/d2s_full_table.h`: `2618f6e5fae6c4443899b184efe3d08295dd267dc9f1a994c983c7caca59ebe6`
- `ryu/d2s.c`: `d24323c7eb77d63f1e52c415212b50060b776f0883525d907d04954fcd48cf64`

Modifications retain the shortest binary64-to-decimal converter, its full
power tables, and only the needed common/arithmetic helpers. Multiplication
uses the upstream portable 64-bit C path; symbols are prefixed `cx_ryu_` or
`CX_RYU_`. Debug/size-option branches and unused wrappers are removed.

The upstream final exact-midpoint tie-to-even adjustment is omitted to match
Rust Display's decimal tie rounding away from zero. For example, `2^-25`
prints `0.000000029802322387695313` in Rust, while unmodified Ryu selects a
final `2`. Both decimals round-trip to the same binary64 value.

Cussy's `cx_format_float` expands the resulting mantissa/exponent to fixed decimal,
including signed zero, without libc floating-point formatting or locale
sensitivity. Non-finite values are handled by the caller.

The differential verifier checks all finite binary exponent boundaries,
selected mantissa boundaries, decimal powers and neighboring doubles, and
seeded random bit patterns against the host Rust compiler's `f64` Display.
It also checks exact roundtrip bits and output buffer guards:

```sh
python3 scripts/verify_native_float.py --extra-random 1000000
python3 scripts/verify_native_float.py --ubsan
```
