Polyquine: Teach Rust types to codegen their own constructor!

crates.io: [polyquine](https://crates.io/crates/polyquine)

```
cargo add polyquine
```

# Purpose

This crate contains:
- A `Quine` trait. Types that implement `Quine` have a method `ctor_tokens(&self) -> TokenStream`;
  The tokens returned by it are valid Rust expression that, when evaluated, produces the original value.
- Implementations for:
  - All primitives (`i32`, `bool`, etc.)
  - `String`
  - Fixed-size arrays `[T; N]`
  - Some `std::collections` types (`Vec`, `HashMap`, `HashSet`, etc.)
  - Tuples of up to 12 elements
  - `Box<T>`, `Option<T>`
- Declarative macros to implement `Quine` for:
  - Iterables (`derive_iterable`).
    The given iterable must:
    - Have an `.iter()` method that returns an iterator over its elements.
    - Implement `From<[T; N]>`
- A `#[derive(Quine)]` macro to derive the trait for enums and structs.
  All fields thereof must implement `Quine`.

For example:

```rust
#[derive(Quine)]
struct Node {
    value: i32,
    next: Option<Box<Node>>,
}

let node = Node {
    value: 1i32,
    next: Some(Box::new(Node {
        value: 2i32,
        next: None,
    })),
};
assert_ts_eq(
    &node.ctor_tokens(),
    // Evaluate this and you get the value of `node` back!
    &quote! {Node { value: 1i32, next: Some(Box::new(Node { value: 2i32, next: None })) }},
);
```

# Contributing

Contributions are always welcome!

- If you need `Quine` to work with a specific type from `std` or a third-party crate, 
  or have any other feature suggestions, open an issue tagged "feature request".
- If you encounter any bugs / unexpected behaviour, open an issue.
  Please include the error message and relevant code snippets.
- Pull requests are very much appreciated! I will do my best to review them ASAP.
  When submitting a patch, please:
  - Make sure that the linter is happy and all tests pass:
    ```
    cargo fmt
    cargo clippy --workspace --fix
    cargo test --workspace
    ```
  - Include a short message explaining the changes and reasoning behind them

See the issues for planned feature tracking and known bugs
  

# Notes

- Version `0.0.2` is a complete rewrite.
  We now use our own trait instead of deriving `quote::ToTokens`.
  All field attributes from version `0.0.1` are no longer supported.
- We ignore references, i.e. the implementation for `&T` is the same as for `T`.
  This may cause issues - please open an issue if you encounter any.
- Please open an issue if any `std` type is not supported and you need it.
- `Rc`, `Arc` and friends are unlikely to ever be supported.
  This is due to their shared ownership semantics:
  We can have multiple `Rc`'s pointing to the same memory.
  When implementing `Quine`, the best we can do is re-construct their values separately.
  When evaluated, we will now have multiple independent copies of the original value - not a single value behind a shared reference.

# Acknowledgements

Partially inspired by the [parsel](https://github.com/H2CO3/parsel/blob/master/parsel_derive/src/to_tokens.rs) crate.

# Why?

This is intended for cases where you need to construct a value and do some non-trivial logic on it at compile time (probably in a procedural macro), then take the result out of the macro as valid Rust code.

For example, when you are parsing a DSL at compile time and outputting the constructed (and possibly simplified / transformed) AST as the result of your macro call.

See [this PR](https://github.com/conjure-cp/conjure-oxide/pull/710) as proof that this use case is not completely made up.

# Production readiness

still no.

# Why the name?

A quine is a program that outputs its own source code.
This crate teaches an arbitrary enum / struct to generate a Rust program that constructs it, thereby making it a quine.
So, polyquine! This is only slightly pretentious.

---



also, trans rights! 🏳️‍⚧️
