//! # Benchmark
//!
//! Runs a series of vulkan compute and graphics workloads and checks their runtime. Possibly power usage and memory-consumption as well (if we can somehow profile that on the gpu).
//!
//! Writes down the result as a json file and possibly compares it to another json-result file for regression.
#![deny(warnings)]
#![allow(unused)]

use bench::Benchmark;
use marpii::context::Ctx;
use marpii_rmg::Rmg;
use reporter::Reporter;

mod bench;
mod bench_task;
mod buffer_to_image;
mod reporter;

pub const RUN_COUNT: usize = 10_000;

fn run_bench(
    benchmark: &mut dyn Benchmark,
    reporter: &mut Reporter,
    rmg: &mut Rmg,
    run_count: usize,
) {
    log::info!("Running benchmark: {}", benchmark.name());
    benchmark.bench_unmodified(rmg, reporter, run_count);
    benchmark.bench_patched_compiled(rmg, reporter, run_count);
    benchmark.bench_patched_runtime(rmg, reporter, run_count);
}

fn main() {
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Warn)
        .init()
        .unwrap();

    //Load Vulkan context for GPGPU workload
    let validation = {
        let is_available = marpii::context::Instance::load()
            .unwrap()
            .is_layer_available("VK_LAYER_KHRONOS_validation");

        if is_available {
            log::info!("Starting Vulkan with validation layers!");
        }
        is_available
    };

    let ctx = Ctx::new_default_headless(validation).unwrap();
    let mut rmg = Rmg::new(ctx).unwrap();

    let mut reporter = Reporter::new();

    //Right now we are always running all benchmarks

    //DynReplace benchmark
    let mut dyn_bench = bench::dyn_replace::DynReplaceBench::load(&mut rmg).unwrap();
    //dyn_bench.safe_last_as_image = true;
    run_bench(&mut dyn_bench, &mut reporter, &mut rmg, RUN_COUNT);

    let mut const_bench = bench::static_replace::StaticReplaceBench::load(&mut rmg).unwrap();
    //const_bench.safe_last_as_image = true;
    run_bench(&mut const_bench, &mut reporter, &mut rmg, RUN_COUNT);

    rmg.wait_for_idle();
    std::thread::sleep(std::time::Duration::from_secs(1));

    //reporter.save();
    reporter.show();
}
