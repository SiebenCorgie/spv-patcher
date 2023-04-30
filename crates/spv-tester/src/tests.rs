//! Collects all pre defined tests.

use crate::Test;

pub fn parse_test_run(name: &str) -> Option<Test> {
    match name {
        "const_mutate" => {
            Some(Test {
                name: name.to_owned(),
                run: Box::new(|| {
                    log::info!("const mutate run");
                    //setup simple const_mutate patch and run it.

                    let mut test = crate::const_mutate::ConstMutateTest::load()
                        .expect("Failed to load ConstMutatePatch");
                    test.patch_i32(1, 42).unwrap();
                }),
            })
        }
        _ => None,
    }
}
