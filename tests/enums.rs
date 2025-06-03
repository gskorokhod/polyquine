use polyquine::ToTokens;
use quote::ToTokens;

#[derive(ToTokens)]
enum TestEnum {
    Unit,
    Tuple(i32, String),
}

#[test]
fn test_enum_to_tokens() {
    let test_enum = TestEnum::Tuple(42, "Hello".to_string());
    let tokens = test_enum.to_token_stream();
    println!("{}", tokens);
}