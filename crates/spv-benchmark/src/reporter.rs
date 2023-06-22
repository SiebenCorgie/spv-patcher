use ahash::AHashMap;

use crate::bench::Benchmark;

pub enum BenchRunType {
    Unmodified,
    PatchedCompiled,
    PatchedRuntime,
    CompileTime,
    PatchTime,
    Other(String),
}

pub struct BenchRun {
    ///Type of the test run
    ty: BenchRunType,
    pipeline_runtime: u64,
}

///Reporter for a single benchmark run
pub struct Reporter {
    benches: AHashMap<String, Vec<BenchRun>>,
}

impl Reporter {
    pub fn new() -> Self {
        Reporter {
            benches: AHashMap::default(),
        }
    }

    pub fn push_run(&mut self, run: BenchRun, name: &str) {
        if let Some(runs) = self.benches.get_mut(name) {
            runs.push(run);
        } else {
            self.benches.insert(name.to_owned(), vec![run]);
        }
    }

    pub fn report_unmodified(&mut self, benchmark: &dyn Benchmark, pipeline_runtime: u64) {
        self.push_run(
            BenchRun {
                ty: BenchRunType::Unmodified,
                pipeline_runtime,
            },
            benchmark.name(),
        );
    }

    pub fn report_patched_compiled(&mut self, benchmark: &dyn Benchmark, pipeline_runtime: u64) {
        self.push_run(
            BenchRun {
                ty: BenchRunType::PatchedCompiled,
                pipeline_runtime,
            },
            benchmark.name(),
        );
    }
    pub fn report_patched_runtime(&mut self, benchmark: &dyn Benchmark, pipeline_runtime: u64) {
        self.push_run(
            BenchRun {
                ty: BenchRunType::PatchedRuntime,
                pipeline_runtime,
            },
            benchmark.name(),
        );
    }
}
