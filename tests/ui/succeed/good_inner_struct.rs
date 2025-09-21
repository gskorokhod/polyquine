use polyquine::Quine;
use std::fmt::{Display};

#[derive(Quine)]
struct Test<T: Display> {
    value: T,
}

fn main() {
    let good: Test<String> = Test {
        value: String::from("Hello World"),
    };
    good.ctor_tokens();
}
