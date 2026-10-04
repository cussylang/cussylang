#!/usr/bin/env python3
"""Compare native float formatting with Rust Display, including roundtrip/bounds checks."""

import argparse
import hashlib
import itertools
import math
import os
from pathlib import Path
import random
import shutil
import struct
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
HEADER = ROOT / "src" / "codegen_float.h"
C_PROBE = r'''
#include <stdio.h>
#include <stdlib.h>
#include <inttypes.h>
int main(void) {
    uint64_t bits;
    while (scanf("%" SCNx64, &bits) == 1) {
        double value;
        memcpy(&value, &bits, sizeof(value));
        struct { char text[512]; unsigned char guard[16]; } buffer;
        memset(&buffer, 0xA5, sizeof(buffer));
        size_t length = cx_format_float(value, buffer.text);
        assert(length < sizeof(buffer.text));
        assert(buffer.text[length] == 0);
        for (size_t i=0; i<sizeof(buffer.guard); ++i) assert(buffer.guard[i]==0xA5);
        char* end;
        double roundtrip = strtod(buffer.text, &end);
        assert(*end == 0);
        uint64_t roundtrip_bits;
        memcpy(&roundtrip_bits, &roundtrip, sizeof(roundtrip_bits));
        assert(roundtrip_bits == bits);
        puts(buffer.text);
    }
}
'''
RUST_ORACLE = r'''
use std::io::{self, BufRead, Write};
fn main() {
    let input = io::stdin();
    let output = io::stdout();
    let mut output = io::BufWriter::new(output.lock());
    for line in input.lock().lines() {
        let bits = u64::from_str_radix(&line.unwrap(), 16).unwrap();
        writeln!(output, "{}", f64::from_bits(bits)).unwrap();
    }
}
'''


def nonnegative(value):
    value = int(value)
    if value < 0:
        raise argparse.ArgumentTypeError("must be nonnegative")
    return value


def tool(name):
    found = shutil.which(name)
    if not found:
        raise RuntimeError(f"required compiler {name!r} is unavailable")
    # Keep a rustup proxy named rustc; resolving the symlink would change argv[0].
    return str(Path(found).absolute())


def edge_and_random_bits(samples):
    values = set()
    for exponent in range(2047):
        for mantissa in (0, 1, 2, 3, (1 << 51) - 1, 1 << 51, (1 << 52) - 2, (1 << 52) - 1):
            for sign in (0, 1):
                values.add((sign << 63) | (exponent << 52) | mantissa)
    for exponent in range(-323, 309):
        value = 10.0 ** exponent
        for neighbor in (math.nextafter(value, 0.0), value, math.nextafter(value, math.inf)):
            if math.isfinite(neighbor):
                bits = struct.unpack("=Q", struct.pack("=d", neighbor))[0]
                values.add(bits)
                values.add(bits | (1 << 63))
    rng = random.Random(0xC055F10A7)
    for _ in range(samples):
        bits = rng.getrandbits(64)
        if ((bits >> 52) & 2047) != 2047:
            values.add(bits)
    return sorted(values)


def extra_random_bits(samples):
    rng = random.Random(0xF10A7C055)
    count = 0
    while count < samples:
        bits = rng.getrandbits(64)
        if ((bits >> 52) & 2047) != 2047:
            count += 1
            yield bits


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cc", default="clang")
    parser.add_argument("--rustc", default="rustc")
    parser.add_argument("--random", type=nonnegative, default=200000)
    parser.add_argument("--extra-random", type=nonnegative, default=0)
    parser.add_argument("--ubsan", action="store_true", help="enable Clang undefined-behavior checks")
    args = parser.parse_args()
    cc, rustc = tool(args.cc), tool(args.rustc)
    header_bytes = HEADER.read_bytes()
    header = header_bytes.decode("utf-8")
    source_hash = hashlib.sha256(header_bytes).hexdigest()
    with tempfile.TemporaryDirectory(prefix="cussy-float-check-") as directory:
        directory = Path(directory)
        c_source, rust_source = directory / "probe.c", directory / "oracle.rs"
        c_source.write_text(header + "\n" + C_PROBE, encoding="utf-8")
        rust_source.write_text(RUST_ORACLE, encoding="utf-8")
        suffix = ".exe" if os.name == "nt" else ""
        c_binary, rust_binary = directory / ("probe" + suffix), directory / ("oracle" + suffix)
        flags = ["-std=c11", "-O2", "-Wall", "-Wextra", "-Werror", "-pedantic"]
        if args.ubsan:
            flags += ["-fsanitize=undefined", "-fno-sanitize-recover=all"]
        subprocess.run([cc, *flags, str(c_source), "-o", str(c_binary)], check=True, timeout=120)
        subprocess.run([rustc, "-O", str(rust_source), "-o", str(rust_binary)], check=True, timeout=120)
        input_path = directory / "inputs.txt"
        values = itertools.chain(edge_and_random_bits(args.random), extra_random_bits(args.extra_random))
        with input_path.open("w", encoding="ascii") as output:
            for bits in values:
                output.write(f"{bits:016x}\n")
        for binary, output_name in ((c_binary, "native.txt"), (rust_binary, "rust.txt")):
            with input_path.open("rb") as source, (directory / output_name).open("wb") as output:
                subprocess.run([str(binary)], stdin=source, stdout=output, check=True, timeout=120)
        count = 0
        with input_path.open() as inputs, (directory / "native.txt").open() as native, (directory / "rust.txt").open() as rust:
            for bits, actual, expected in itertools.zip_longest(inputs, native, rust):
                count += 1
                if bits is None or actual is None or expected is None or actual != expected:
                    raise RuntimeError(f"mismatch for bits {bits!r}: native={actual!r}, Rust={expected!r}")
    if hashlib.sha256(HEADER.read_bytes()).hexdigest() != source_hash:
        raise RuntimeError("float header changed during verification; rerun")
    print(f"Passed {count:,} finite-f64 comparisons: Rust Display, roundtrip bits, and output-buffer guards.")
    print("codegen_float.h SHA-256:", source_hash)
    print(subprocess.check_output([rustc, "--version"], text=True).strip())
    print("UBSan:", "enabled" if args.ubsan else "disabled")


if __name__ == "__main__":
    main()
