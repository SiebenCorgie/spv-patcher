//! # Tester
//!
//! Runs a series of compute and graphics tests, comparing the results to the "blessed" expected values.
//! Those tests are mainly *integration tests.* The patcher's unit tests reside in its local
//! `spv-patcher/tests` directory.
#![deny(warnings)]

use marpii::context::Ctx;
use marpii_rmg::Rmg;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::{BufReader, BufWriter},
    path::Path,
};

mod compute_task;
///Constant mutation test. Takes a compute shader and mutates a constant that is added to a buffer.
mod const_mutate;
mod non_uniform;
mod print;
mod test_runs;
mod validator;

const BLESSED_FILE: &'static str = "BlessedTests.json";

#[derive(Serialize, Deserialize)]
pub struct BlessedDB {
    //true if this run blesses the BlessedDB.
    bless: bool,
    blessed_results: HashMap<String, Vec<u8>>,
}

fn main() {
    //Setup logger at info level.
    // NOTE: If we set it to trace we'd get all the RMG debug info as well.
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .init()
        .unwrap();

    //try to load blessed results
    let mut blessed_db = if Path::new(BLESSED_FILE).exists() {
        if let Ok(f) = std::fs::OpenOptions::new().read(true).open(BLESSED_FILE) {
            let reader = BufReader::new(f);
            let mut blessed_db: BlessedDB =
                serde_json::from_reader(reader).expect("Could not parse blessed file!");
            blessed_db.bless = false;
            blessed_db
        } else {
            log::error!("Failed to open blessed file");
            BlessedDB {
                bless: false,
                blessed_results: HashMap::default(),
            }
        }
    } else {
        log::error!("Could not find blessed file, running with bless-flag");
        BlessedDB {
            bless: true,
            blessed_results: HashMap::default(),
        }
    };

    //Load Vulkan context for GPGPU workload
    //TODO read ENV-Variable or something for existence, and check if we are in a Debug build instead.
    let validation_layer = true;
    let ctx = Ctx::new_default_headless(validation_layer).unwrap();
    let mut rmg = Rmg::new(ctx).unwrap();

    let mut args = std::env::args();
    let _prog = args.next();
    //Read args and parse to test runs
    let mut runs = Vec::new();
    for arg in args {
        if &arg == "bless" {
            log::info!("Blessing db in this run");
            blessed_db.bless = true;
            continue;
        }

        if let Some(testrun) = test_runs::parse_test_run(&arg, &mut rmg) {
            log::trace!("Found: {}", arg);
            runs.push(testrun);
        }
    }

    for run in &mut runs {
        log::info!("Running: {}", run.name());
        if let Err(e) = run.run(&mut blessed_db, &mut rmg) {
            log::error!("Failed to run test {}: {}", run.name(), e);
        }
    }

    if blessed_db.bless {
        if Path::new(BLESSED_FILE).exists() {
            let _ = std::fs::remove_file(BLESSED_FILE);
        }
        //before writing, reset the bless flag, otherwise we'd load it again.
        blessed_db.bless = false;
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(BLESSED_FILE)
            .unwrap();
        serde_json::to_writer(BufWriter::new(file), &blessed_db).unwrap();
    }
}
