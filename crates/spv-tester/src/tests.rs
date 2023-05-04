//! Collects all pre defined tests.

use crate::{print::DisassamblerPrinter, Test};

pub trait TestRun {
    ///Human readable name of the test. Also used to find blessed result.
    fn name() -> &'static str;
}

pub fn parse_test_run(name: &str) -> Option<Test> {
    match name {
        "const_mutate" => {
            Some(Test {
                name: name.to_owned(),
                run: Box::new(|blessed| {
                    log::info!("const mutate run");
                    //setup simple const_mutate patch and run it.

                    let mut test = crate::const_mutate::ConstMutateTest::load()
                        .expect("Failed to load ConstMutatePatch");
                    let res = test.patch_i32(1, 42).unwrap();

                    let dis = DisassamblerPrinter::from_bytecode(&res);
                    log::trace!("Post mutate: {}", dis);

                    //check if we have a blessed result, otherwise error out.
                    if blessed.bless {
                        if blessed
                            .blessed_results
                            .insert("const_mutate".to_owned(), res)
                            .is_some()
                        {
                            log::info!("Overwrote old blessed result for const_mutate");
                        }
                    } else {
                        if let Some(blessed_res) = blessed.blessed_results.get("const_mutate") {
                            if &res == blessed_res {
                                log::info!("PASS: const_mutate");
                            } else {
                                log::error!("ERROR: const_mutate");
                            }
                        } else {
                            log::error!("Could not find blessed result for const_mutate");
                        }
                    }
                }),
            })
        }
        _ => None,
    }
}
