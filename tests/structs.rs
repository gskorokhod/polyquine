use polyquine::ToTokens;
use quote::{ToTokens, quote};

#[derive(ToTokens)]
struct Basic {
    id: i32,
    name: String,
}

#[test]
fn test_struct_basic() {
    let basic = Basic {
        id: 42,
        name: "Hello".to_string(),
    };
    let tokens = basic.to_token_stream();
    assert_eq!(
        tokens.to_string(),
        quote! {
            Basic { id: 42i32.into(), name: "Hello".into() }
        }
        .to_string()
    );
}

#[derive(ToTokens)]
struct BasicUnnamed(i32, String);

#[test]
fn test_struct_basic_unnamed() {
    let basic_unnamed = BasicUnnamed(100, "Unnamed".to_string());
    let tokens = basic_unnamed.to_token_stream();
    assert_eq!(
        tokens.to_string(),
        quote! {
            BasicUnnamed(100i32.into(), "Unnamed".into())
        }
        .to_string()
    );
}

#[derive(ToTokens)]
struct Unit;

#[test]
fn test_struct_unit() {
    let unit = Unit;
    let tokens = unit.to_token_stream();
    assert_eq!(
        tokens.to_string(),
        quote! {
            Unit {}
        }
        .to_string()
    );
}

#[derive(ToTokens)]
struct Recursive {
    leaf: i32,
    #[polyquine(recursive)]
    inner: Option<Box<Recursive>>,
}

#[test]
fn test_struct_recursive() {
    let rec = Recursive {
        leaf: 1,
        inner: Some(Box::new(Recursive {
            leaf: 2,
            inner: Some(Box::new(Recursive {
                leaf: 3,
                inner: None,
            })),
        })),
    };
    let tokens = rec.to_token_stream();
    assert_eq!(
        tokens.to_string(),
        quote! {
            Recursive { leaf: 1i32.into(), inner: Some(Box::new(Recursive { leaf: 2i32.into(), inner: Some(Box::new(Recursive { leaf: 3i32.into(), inner: None }.into())) }.into())) }
        }
        .to_string()
    );
}

#[derive(ToTokens)]
struct BasicVec {
    items: Vec<Basic>,
}

#[test]
fn test_struct_basic_vec() {
    let vec_struct = BasicVec {
        items: vec![
            Basic {
                id: 1,
                name: "Item1".to_string(),
            },
            Basic {
                id: 2,
                name: "Item2".to_string(),
            },
        ],
    };
    let tokens = vec_struct.to_token_stream();
    assert_eq!(
        tokens.to_string(),
        quote! {
            BasicVec { items: vec![Basic { id: 1i32.into(), name: "Item1".into() }.into(), Basic { id: 2i32.into(), name: "Item2".into() }.into()] }
        }
        .to_string()
    );
}

#[derive(ToTokens)]
struct TreeNode {
    value: i32,
    #[polyquine(recursive)]
    children: Vec<TreeNode>,
}

#[test]
fn test_struct_tree_node() {
    let tree = TreeNode {
        value: 1,
        children: vec![
            TreeNode {
                value: 2,
                children: vec![],
            },
            TreeNode {
                value: 3,
                children: vec![TreeNode {
                    value: 4,
                    children: vec![],
                }],
            },
        ],
    };
    let tokens = tree.to_token_stream();
    assert_eq!(
        tokens.to_string(),
        quote! {
            TreeNode { value: 1i32.into(), children: vec![TreeNode { value: 2i32.into(), children: vec![] }.into(), TreeNode { value: 3i32.into(), children: vec![TreeNode { value: 4i32.into(), children: vec![] }.into()] }.into()] }
        }
        .to_string()
    );
}
