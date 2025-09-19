use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use ustr::Ustr;

pub trait Quine {
    fn ctor_tokens(&self) -> TokenStream;
}

impl Quine for Ustr {
    fn ctor_tokens(&self) -> TokenStream {
        let s = self.as_str();
        quote! {Ustr::from(#s)}
    }
}

impl<T: Quine> Quine for &T {
    fn ctor_tokens(&self) -> TokenStream {
        (**self).ctor_tokens()
    }
}

impl<T: Quine> Quine for Box<T> {
    fn ctor_tokens(&self) -> TokenStream {
        let inner = self.as_ref().ctor_tokens();
        quote! {Box::new(#inner)}
    }
}

impl<T: Quine> Quine for Option<T> {
    fn ctor_tokens(&self) -> TokenStream {
        match self {
            Some(value) => {
                let inner = value.ctor_tokens();
                quote! {Some(#inner)}
            }
            None => quote! {None},
        }
    }
}

impl<T: Quine, const N: usize> Quine for [T; N] {
    fn ctor_tokens(&self) -> TokenStream {
        let elements = self.iter().map(|elem| elem.ctor_tokens());
        quote! {[#(#elements),*]}
    }
}

derive_primitive!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64, bool, char, &str
);

derive_trivial!(String);

derive_tuple_all!(A B C D E F G H I J K);

derive_iterable!(Vec<T>);
derive_iterable!(VecDeque<T>);
derive_iterable!(HashSet<T>);
derive_iterable!(HashMap<K, V>);
derive_iterable!(BTreeMap<K, V>);

mod test {
    use super::*;
    #[allow(unused)]
    use crate::Quine;
    #[allow(unused)]
    use std::fmt::Display;
    #[allow(unused)]
    use std::fmt::Formatter;

    #[allow(unused)]
    fn assert_ts_eq(left: &TokenStream, right: &TokenStream) {
        assert_eq!(left.to_string(), right.to_string());
    }

    #[test]
    fn test_vec() {
        let vec = Vec::from([1i32, 2i32, 3i32]);
        assert_ts_eq(&vec.ctor_tokens(), &quote! {Vec::from([1i32, 2i32, 3i32])});
    }

    #[test]
    fn test_tuple() {
        let tuple = (1i32, true);
        assert_ts_eq(&tuple.ctor_tokens(), &quote! {(1i32, true)});
    }

    #[test]
    fn test_hashmap() {
        let map = HashMap::from([(1i32, "one")]);
        assert_ts_eq(&map.ctor_tokens(), &quote! {HashMap::from([(1i32, "one")])});
    }

    #[test]
    fn test_box() {
        let boxed = Box::new(String::from("hello, world!"));
        assert_ts_eq(
            &boxed.ctor_tokens(),
            &quote! {Box::new(String::from("hello, world!"))},
        );
    }

    #[test]
    fn test_slice() {
        let slice = [1i32, 2i32, 3i32];
        assert_ts_eq(&slice.ctor_tokens(), &quote! {[1i32, 2i32, 3i32]});
    }

    #[test]
    fn test_struct() {
        #[derive(Quine)]
        struct TestStruct {
            a: i32,
            b: bool,
        }

        let test_struct = TestStruct { a: 1i32, b: true };
        assert_ts_eq(
            &test_struct.ctor_tokens(),
            &quote! {TestStruct { a: 1i32, b: true }},
        );
    }

    #[test]
    fn test_tuple_struct() {
        #[derive(Quine)]
        struct TestTupleStruct(i32, bool);

        let test_tuple_struct = TestTupleStruct(1i32, true);
        assert_ts_eq(
            &test_tuple_struct.ctor_tokens(),
            &quote! {TestTupleStruct(1i32, true)},
        );
    }

    #[test]
    fn test_unit_struct() {
        #[derive(Quine)]
        struct TestUnitStruct;

        let test_unit_struct = TestUnitStruct {};
        assert_ts_eq(&test_unit_struct.ctor_tokens(), &quote! {TestUnitStruct {}});
    }

    #[test]
    fn test_nested_struct() {
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
            &quote! {Node { value: 1i32, next: Some(Box::new(Node { value: 2i32, next: None })) }},
        );
    }

    #[test]
    fn test_enum() {
        #[derive(Quine)]
        enum TestEnum {
            A,
            B(i32),
            C { name: String },
        }

        let a = TestEnum::A;
        let b = TestEnum::B(1i32);
        let c = TestEnum::C {
            name: String::from("John"),
        };

        assert_ts_eq(&a.ctor_tokens(), &quote! {TestEnum::A});
        assert_ts_eq(&b.ctor_tokens(), &quote! {TestEnum::B(1i32)});
        assert_ts_eq(
            &c.ctor_tokens(),
            &quote! {TestEnum::C { name: String::from("John") }},
        );
    }

    #[test]
    fn test_ast() {
        #[derive(Quine)]
        struct Metadata {
            src: String,
        }

        #[derive(Quine)]
        enum Ast {
            Num(Box<Metadata>, isize),
            Mul(Box<Metadata>, Box<Ast>, Box<Ast>),
            Sum(Box<Metadata>, Vec<Ast>),
        }

        let ast = Ast::Sum(
            Box::new(Metadata {
                src: String::from("1 + (2 * 3)"),
            }),
            vec![
                Ast::Num(
                    Box::new(Metadata {
                        src: String::from("1"),
                    }),
                    1,
                ),
                Ast::Mul(
                    Box::new(Metadata {
                        src: String::from("2 * 3"),
                    }),
                    Box::new(Ast::Num(
                        Box::new(Metadata {
                            src: String::from("2"),
                        }),
                        2,
                    )),
                    Box::new(Ast::Num(
                        Box::new(Metadata {
                            src: String::from("3"),
                        }),
                        3,
                    )),
                ),
            ],
        );

        assert_ts_eq(
            &ast.ctor_tokens(),
            &quote! {
                Ast::Sum(
                    Box::new(Metadata {
                        src: String::from("1 + (2 * 3)")
                    }),
                    Vec::from([
                        Ast::Num(
                            Box::new(Metadata {
                                src: String::from("1")
                            }),
                            1isize
                        ),
                        Ast::Mul(
                            Box::new(Metadata {
                                src: String::from("2 * 3")
                            }),
                            Box::new(Ast::Num(
                                Box::new(Metadata {
                                    src: String::from("2")
                                }),
                                2isize
                            )),
                            Box::new(Ast::Num(
                                Box::new(Metadata {
                                    src: String::from("3")
                                }),
                                3isize
                            ))
                        )
                    ])
                )
            },
        );
    }

    #[test]
    fn test_ustr() {
        let u1 = Ustr::from("the quick brown fox");
        assert_ts_eq(
            &u1.ctor_tokens(),
            &quote! {Ustr::from("the quick brown fox")},
        );
    }

    #[test]
    fn test_struct_with_generic() {
        struct BadInner {}

        impl Display for BadInner {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "BadInner")
            }
        }

        #[derive(Quine)]
        struct Test<T: Display> {
            value: T,
        }

        let bad: Test<BadInner> = Test { value: BadInner {} };
        //bad.ctor_tokens(); // Should not compile

        let good: Test<String> = Test {
            value: String::from("Hello World"),
        };
        assert_ts_eq(
            &good.ctor_tokens(),
            &quote! {
                Test {
                    value: String::from("Hello World")
                }
            },
        );
    }

    #[test]
    fn test_enum_with_generic() {
        struct BadInner {}

        #[derive(Quine)]
        enum TestEnum<T> {
            A(T),
            B,
        }

        let bad = TestEnum::A(BadInner {});
        // bad.ctor_tokens();

        let good = TestEnum::A(String::from("Hello World"));
        assert_ts_eq(
            &good.ctor_tokens(),
            &quote! {
                TestEnum::A(String::from("Hello World"))
            },
        )
    }
}
