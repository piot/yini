# yini

A tiny, opinionated text data format and parser (think simple INI with a few extras).

## Rules

- Comments start with `#` and run to end-of-line.

- Top-level entries are `key: value` pairs. Keys are identifiers or quoted strings.

- Values can be: strings (quoted), integers, floats, booleans (`true`/`false`),
  objects `{ ... }`, arrays `[ ... ]`, or tuples (parenthesized, comma-separated values).

- Objects use braces and contain their own `key: value` lines.

- Arrays use commas to separate elements. Elements may be single values,
  objects, or tuples.

- Tuples are groups of two-or-more values written in parentheses and separated
  by commas, e.g. `(a, b, c)`. Inside tuples string tokens may be unquoted
  identifiers. Tuples are only recognized inside arrays or immediately
  after a `key:` on the same line.

Examples:

```text
key: 42
name: "Alice"
coords: [1, 2, 3]
pairs: [("k1", "v1"), ("k2", "v2")]   # array of 2-tuples
triple: (a, b, c)                     # 3-tuple as a value for `triple`

person: {
    name: "Alice"
    age: 30
    tags: [developer, rust]
}
```

Rationale:

-- The format aims for readability and minimal punctuation. Commas separate
  array elements; parentheses and commas delimit tuples so identifiers
  inside tuples don't need quotes.

-- Tuples are explicit: they let you express grouped values (like
  key/value pairs or fixed arity records) without extra braces; the
  parentheses make tuple boundaries unambiguous and allow unquoted
  identifiers inside.

- Using newline/colon boundaries for object entries keeps
  key/value parsing simple and predictable.
