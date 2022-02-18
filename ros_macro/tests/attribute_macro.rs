use ros_macro::*;

// macro converts struct S to struct H
#[ros_struct]
struct World {
    hello: String,
}

#[test]
fn test_macro() {
    // due to macro we have struct H in scope
    let demo = World { hello: "".into() };
}
