use ahash::AHashMap;
use graplot::{Bar, BarDesc, BarDescArg};
use serde::{Deserialize, Serialize};

use crate::bench::Benchmark;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Hash)]
pub enum BenchRunType {
    Unmodified,
    PatchedCompiled,
    PatchedRuntime,
    CompileTime,
    PatchTime,
    Other(String),
}

impl BarDescArg for BenchRunType {
    fn as_bar_desc(&self) -> Vec<BarDesc> {
        vec![self.as_single_bar_desc()]
    }
    fn as_single_bar_desc(&self) -> BarDesc {
        let s = if let BenchRunType::Other(s) = self {
            s.clone()
        } else {
            format!("{:?}", self)
        };

        s.as_str().as_single_bar_desc()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BenchRun {
    ///Type of the test run
    ty: BenchRunType,
    pipeline_runtime: f64,
}

///Reporter for a single benchmark run
#[derive(Serialize, Deserialize, Debug)]
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

    pub fn report_unmodified(&mut self, benchmark: &dyn Benchmark, pipeline_runtime: f64) {
        self.push_run(
            BenchRun {
                ty: BenchRunType::Unmodified,
                pipeline_runtime,
            },
            benchmark.name(),
        );
    }

    pub fn report_patched_compiled(&mut self, benchmark: &dyn Benchmark, pipeline_runtime: f64) {
        self.push_run(
            BenchRun {
                ty: BenchRunType::PatchedCompiled,
                pipeline_runtime,
            },
            benchmark.name(),
        );
    }
    pub fn report_patched_runtime(&mut self, benchmark: &dyn Benchmark, pipeline_runtime: f64) {
        self.push_run(
            BenchRun {
                ty: BenchRunType::PatchedRuntime,
                pipeline_runtime,
            },
            benchmark.name(),
        );
    }

    ///Renders the currently known benchmark results.
    pub fn show(&self) {
        ///Currently rendering one graph per benchmark
        for (bench_name, results) in &self.benches {
            let mut bins: AHashMap<BenchRunType, Vec<f64>> = AHashMap::with_capacity(3);
            for run in results {
                if let Some(bin) = bins.get_mut(&run.ty) {
                    bin.push(run.pipeline_runtime);
                } else {
                    bins.insert(run.ty.clone(), vec![run.pipeline_runtime]);
                }
            }

            let names = bins
                .keys()
                .map(|k| k.as_single_bar_desc())
                .collect::<Vec<_>>();
            let results = bins
                .values()
                .map(|bin_res| bin_res.iter().fold(0.0f64, |max, v| max.max(*v)))
                .collect::<Vec<_>>();

            let mut bar = Bar::new(names.as_slice(), &results);
            bar.set_title(&bench_name);
            bar.set_xlabel("patch type");
            bar.show();
        }
    }
}
