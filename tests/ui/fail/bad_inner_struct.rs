use polyquine::Quine;
#[allow(unused_imports)]
use std::fmt::{Display, Formatter};

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

fn main() {
    let bad: Test<BadInner> = Test { value: BadInner {} };
    bad.ctor_tokens();
}
