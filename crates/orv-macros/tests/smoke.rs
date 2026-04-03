use orv_macros::orv;

#[test]
fn macro_compiles() {
    let _result = orv! {
        hello world
    };
}
