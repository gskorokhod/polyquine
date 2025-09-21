use quote::quote;

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/fail/*.rs");
    t.pass("tests/ui/succeed/*.rs");

    use polyquine::Quine;

    mod private {
        use polyquine::Quine;

        #[derive(Quine)]
        #[path_prefix(some_module)]
        pub struct SomeStruct {
            pub value: String,
        }
    }

    pub mod some_module {}
    let mut temp_dir = tempfile::tempdir().unwrap();
    temp_dir.disable_cleanup(true);
    let mod_file = temp_dir.path().join("mod.rs");
    let mod_contents = stringify!(
        mod constructor;
        mod private {
            use polyquine::Quine;

            #[derive(Quine)]
            #[path_prefix(crate::some_module)]
            pub struct SomeStruct {
                pub value: String,
            }
        }
        pub mod some_module {
            pub use crate::private::SomeStruct;
        }
        fn main() {}
    );
    std::fs::write(&mod_file, mod_contents).unwrap();

    let generated_file = temp_dir.path().join("constructor.rs");

    let tokens = private::SomeStruct {
        value: "Hello".to_string(),
    }
    .ctor_tokens();

    let content = quote! {
        use super::*;
        fn nope() {
            let s = #tokens;
            assert_eq!(s.value, "Hello".to_string());
        }
    };

    std::fs::write(&generated_file, content.to_string()).unwrap();

    t.pass(mod_file);
}
