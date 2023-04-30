//! # Tester
//!
//! Runs a series of compute and graphics tests, comparing the results to the "blessed" expected values.
//! Those tests are mainly *integration tests.* The patcher's unit tests reside in its local
//! `spv-patcher/tests` directory.

///Constant mutation test. Takes a compute shader and mutates a constant that is added to a buffer.
mod const_mutate;

mod tests;

///Collects a set of patches that are applied, writes down debug information and test
/// results
pub struct Test {
    name: String,
    run: Box<dyn FnOnce()>,
}

fn main() {
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Trace)
        .init()
        .unwrap();

    let mut args = std::env::args();
    let _prog = args.next();

    //Read args and parse to test runs
    let mut runs = Vec::new();
    for arg in args {
        if let Some(testrun) = tests::parse_test_run(&arg) {
            log::trace!("Found: {}", arg);
            runs.push(testrun);
        }
    }

    for run in runs {
        log::info!("Running: {}", run.name);
        (run.run)();
    }
}
