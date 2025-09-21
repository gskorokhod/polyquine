use polyquine::Quine;
#[allow(unused_imports)]

struct BadInner {}

#[derive(Quine)]
enum TestEnum<T> {
    A(T),
    B,
}

fn main() {
    let good = TestEnum::A(String::from("Hello World"));
    good.ctor_tokens();

    let bad = TestEnum::A(BadInner {});
    bad.ctor_tokens();
}
