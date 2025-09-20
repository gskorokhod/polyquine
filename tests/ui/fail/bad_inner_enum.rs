use polyquine::Quine;
#[allow(unused_imports)]
use std::fmt::{Display, Formatter};

struct BadInner {}

#[derive(Quine)]
enum TestEnum<T: Quine> {
    A(T),
    B,
}

fn main() {
    let bad = TestEnum::A(BadInner {});
    // bad.ctor_tokens();
}
