# yini

A tiny, opinionated text data format and parser (think simple INI with a few
extras). It keeps punctuation to a minimum while still supporting nested
structs, arrays, tuples, and enum-like variants.

## Quick Start

Add the crate to your `Cargo.toml`, then feed the parser an input `&str`:

```rust
use yini::Parser;

let mut parser = Parser::new(r#"
name "Alice"
age 30
"#);
let root = parser.parse();
assert_eq!(root.get("age").and_then(|v| v.as_int()), Some(30));
```

The parser reports non-fatal issues through `parser.errors()`, allowing you to
inspect partially parsed documents.

## Format Basics

- Every line defines a `key value` pair—whether at the top level or inside a
  struct. Keys are identifiers or quoted strings.

- Values can be: strings (quoted), integers, floats, booleans (`true`/`false`),
  structs `{ ... }`, arrays `[ ... ]`, or tuples `( ... )`.

- Structs use braces and contain their own `key value` lines; these nested
  entries follow the exact same rules as top-level pairs.

- Arrays use separate elements. Elements may be single values, structs, or
  tuples.

- Tuples are groups of two-or-more values written in parentheses , e.g.
  `(a b c)`.

- Variants start with `:` (e.g. `:state`) and may carry an immediate payload in
  parentheses, braces, or brackets—`:` must touch the payload (`:state(1 2)` or
  `:state{key value}`).

- Comments start with `#` and run to end-of-line.

Examples:

```text
key 42
name "Alice"
coords [1, 2, 3]
pairs [ (k1 "v1") ("k2", "v2") ]   # array of 2-tuples
triple (a, b, c)                     # 3-tuple as a value for `triple`
screen :fullscreen( 1024 768 )
mode :windowed # without payload

person {
    name "Alice"
    age 30
    tags [developer, rust]
}
```

## Benchmark

The repository contains a small micro-benchmark in `examples/benchmark.rs`. Run
it in release mode for meaningful numbers:

```bash
cargo run --release --example benchmark
```

The benchmark parses a synthetic data set repeatedly to give an aggregate
throughput figure.

Rationale:

- The format aims for readability and minimal punctuation.

- keys must start on a newline, to be easy to read.
