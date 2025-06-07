use polyquine::ToTokens;
use quote::{ToTokens, quote};
use std::collections::VecDeque;

// Dummy struct that does not implement ToTokens
struct Untokenizable {
    val: i32,
}

#[derive(ToTokens)]
enum TestEnum {
    // Custom tokeniser for string (appends "Hello, ")
    A(
        i32,
        #[polyquine(custom_with = |s| "Hello, ".to_string() + s)] String,
    ),
    // Custom tokeniser for the struct (which wouldn't support ToTokens otherwise)
    B(
        #[polyquine(custom_with = |v: &Untokenizable| {
        let val = v.val;
        quote! { Untokenizable { val: #val } }
    })]
        Untokenizable,
    ),
}

#[test]
fn test_custom_with_str() {
    let test_enum = TestEnum::A(42, "World".to_string());
    let tokens = test_enum.to_token_stream();
    assert_eq!(
        tokens.to_string(),
        quote! {
            TestEnum::A(42i32.into(), "Hello, World".into())
        }
        .to_string()
    );
}

#[test]
fn test_custom_with_struct() {
    let test_enum = TestEnum::B(Untokenizable { val: 100 });
    let tokens = test_enum.to_token_stream();
    assert_eq!(
        tokens.to_string(),
        quote! {
            TestEnum::B(Untokenizable { val: 100i32 }.into())
        }
        .to_string()
    );
}
