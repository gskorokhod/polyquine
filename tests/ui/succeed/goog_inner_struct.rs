use polyquine::Quine;
use std::fmt::{Display, Formatter};

#[derive(Quine)]
struct Test<T: Display + Quine> {
    value: T,
}

fn main() {
    let good: Test<String> = Test {
        value: String::from("Hello World"),
    };
}
