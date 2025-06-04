use polyquine::ToTokens;
use quote::{ToTokens, quote};
use std::collections::VecDeque;

#[derive(ToTokens)]
enum TestEnum {
    Unit,
    Basic(i32, String),
    IntVec(Vec<i32>),
    IntVec2D(Vec<Vec<i32>>),
    Recursive(#[polyquine(recursive)] Box<TestEnum>),
    Option(Option<i32>),
    RecursiveVecOption(#[polyquine(recursive)] Vec<Option<TestEnum>>),
    BasicNamed {
        id: i32,
        name: String,
    },
    RecursiveNamed {
        name: Vec<String>,
        #[polyquine(recursive)]
        inner: Box<TestEnum>,
    },
    IntVecDeque(VecDeque<i32>),
    IntSet(std::collections::HashSet<i32>),
}

#[test]
fn test_enum_unit() {
    let test_enum = TestEnum::Unit;
    let tokens = test_enum.to_token_stream();
    assert_eq!(
        tokens.to_string(),
        quote! {
            TestEnum::Unit
        }
        .to_string()
    );
}

#[test]
fn test_enum_basic() {
    let test_enum = TestEnum::Basic(42, "Hello".to_string());
    let tokens = test_enum.to_token_stream();
    assert_eq!(
        tokens.to_string(),
        quote! {
            TestEnum::Basic(42i32.into(), "Hello".into())
        }
        .to_string()
    );
}

#[test]
fn test_enum_int_vec() {
    let test_enum = TestEnum::IntVec(vec![1, 2, 3]);
    let tokens = test_enum.to_token_stream();
    assert_eq!(
        tokens.to_string(),
        quote! {
            TestEnum::IntVec(vec![1i32.into(), 2i32.into(), 3i32.into()])
        }
        .to_string()
    );
}

#[test]
fn test_enum_int_vec_2d() {
    let test_enum = TestEnum::IntVec2D(vec![vec![1, 2], vec![3, 4]]);
    let tokens = test_enum.to_token_stream();
    assert_eq!(
        tokens.to_string(),
        quote! {
            TestEnum::IntVec2D(vec![vec![1i32.into(), 2i32.into()], vec![3i32.into(), 4i32.into()]])
        }
        .to_string()
    );
}

#[test]
fn test_enum_recursive() {
    let inner_enum = TestEnum::Basic(100, "Nested".to_string());
    let test_enum = TestEnum::Recursive(Box::new(inner_enum));
    let tokens = test_enum.to_token_stream();
    assert_eq!(
        tokens.to_string(),
        quote! {
            TestEnum::Recursive(Box::new(TestEnum::Basic(100i32.into(), "Nested".into()).into()))
        }
        .to_string()
    );
}

#[test]
fn test_enum_option_some() {
    let test_enum = TestEnum::Option(Some(42));
    let tokens = test_enum.to_token_stream();
    assert_eq!(
        tokens.to_string(),
        quote! {
            TestEnum::Option(Some(42i32.into()))
        }
        .to_string()
    );
}

#[test]
fn test_enum_option_none() {
    let test_enum = TestEnum::Option(None);
    let tokens = test_enum.to_token_stream();
    assert_eq!(
        tokens.to_string(),
        quote! {
            TestEnum::Option(None)
        }
        .to_string()
    );
}

#[test]
fn test_enum_recursive_vec_option() {
    let inner_enum = TestEnum::Basic(200, "Inner".to_string());
    let test_enum = TestEnum::RecursiveVecOption(vec![Some(inner_enum), None]);
    let tokens = test_enum.to_token_stream();
    assert_eq!(tokens.to_string(), quote! {
        TestEnum::RecursiveVecOption(vec![Some(TestEnum::Basic(200i32.into(), "Inner".into()).into()), None])
    }.to_string());
}

#[test]
fn test_enum_basic_named() {
    let test_enum = TestEnum::BasicNamed {
        id: 1,
        name: "Test".to_string(),
    };
    let tokens = test_enum.to_token_stream();
    assert_eq!(
        tokens.to_string(),
        quote! {
            TestEnum::BasicNamed { id: 1i32.into(), name: "Test".into() }
        }
        .to_string()
    );
}

#[test]
fn test_enum_recursive_named() {
    let inner_enum = TestEnum::Basic(300, "Inner Named".to_string());
    let test_enum = TestEnum::RecursiveNamed {
        name: vec!["Outer".to_string(), "Named".to_string()],
        inner: Box::new(inner_enum),
    };
    let tokens = test_enum.to_token_stream();
    assert_eq!(
        tokens.to_string(),
        quote! {
            TestEnum::RecursiveNamed {
                name: vec!["Outer".into(), "Named".into()],
                inner: Box::new(TestEnum::Basic(300i32.into(), "Inner Named".into()).into())
            }
        }
        .to_string()
    );
}

#[test]
fn test_enum_int_vec_deque() {
    let test_enum = TestEnum::IntVecDeque(VecDeque::<i32>::from([1, 2, 3]));
    let tokens = test_enum.to_token_stream();
    assert_eq!(
        tokens.to_string(),
        quote! {
            TestEnum::IntVecDeque(VecDeque::<i32>::from([1i32.into(), 2i32.into(), 3i32.into()]))
        }
        .to_string()
    );
}

#[test]
fn test_enum_int_set() {
    let set = std::collections::HashSet::<i32>::from([1]);
    let test_enum = TestEnum::IntSet(set);
    let tokens = test_enum.to_token_stream();
    assert_eq!(
        tokens.to_string(),
        quote! {
            TestEnum::IntSet(std::collections::HashSet::<i32>::from([1i32.into()]))
        }
        .to_string()
    );
}