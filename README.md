Polyquine: Teach Rust types to codegen their own constructor!

crates.io: [polyquine](https://crates.io/crates/polyquine)

```
cargo add polyquine
```

# Purpose

This crate contains a `#[derive(ToTokens)]` macro.
It derives the `quote::ToTokens` trait such that the tokens for some value are valid Rust code that constructs that value.

```rust
#[derive(ToTokens)]
enum TestEnum {
    Basic(i32, String),
    ...
}

let test_enum = TestEnum::Basic(42, "Hello".to_string());
let tokens = test_enum.to_token_stream();
assert_eq!(
    tokens.to_string(),
    quote! {
        TestEnum::Basic(42i32.into(), "Hello".into())
    }
    .to_string()
);
```

Currently, enums and structs are supported.

It is also possible to bring your own implementation for a specific field:

```rust
#[derive(ToTokens)]
enum TestEnum {
    // Custom tokeniser for string (appends "Hello, ")
    A(
        i32,
        #[polyquine(custom_with = |s| "Hello, ".to_string() + s)] String,
    ),
}

let test_enum = TestEnum::A(42, "World".to_string());
let tokens = test_enum.to_token_stream();
assert_eq!(
    tokens.to_string(),
    quote! {
        TestEnum::A(42i32.into(), "Hello, World".into())
    }
    .to_string()
);
```

# Acknowledgements

Some code was shamelessly stolen from the [parsel](https://github.com/H2CO3/parsel/blob/master/parsel_derive/src/to_tokens.rs) crate, which is cool and you should check it out maybe.

# Why?

This is intended for cases where you need to construct a value and do some non-trivial logic on it at compile time (probably in a procedural macro), then take the result out of the macro as valid Rust code.

For example, when you are parsing a DSL at compile time and outputting the constructed (and possibly simplified / transformed) AST as the result of your macro call.

See [this PR](https://github.com/conjure-cp/conjure-oxide/pull/710) as proof that this use case is not completely made up.

# Features 

- Handles common types like `Vec`, `Box`, `HashMap`, etc, even though they do not implement `ToTokens` in this way themselves
- Handles nested types and recursion
- Can be controlled with additional attributes:
  - `custom_with` - Provide your own ToTokens implementation for a specific field

# TODO

- More testing
- Allow the user to supply their own `ToTokens` implementation for an entire enum variant (not just fields)
- Make fields skippable

# Production readiness

no.

# Why the name?

A quine is a program that outputs its own source code.
This crate teaches an arbitrary enum / struct to generate a Rust program that constructs it, thereby making it a quine.
So, polyquine! This is only slightly pretentious.

---



also, trans rights! 🏳️‍⚧️
