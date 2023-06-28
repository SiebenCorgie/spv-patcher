use std::{fs::OpenOptions, path::Path};

use ahash::AHashMap;
use plotters::{
    data::fitting_range,
    prelude::{
        Boxplot, CandleStick, ChartBuilder, IntoDrawingArea, IntoSegmentedCoord, Quartiles,
        SegmentValue, RED,
    },
    series::Histogram,
    style::{Color, IntoFont, WHITE},
};
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

    pub fn save(&self) {
        let s = serde_json::to_string(self).unwrap();
        if Path::new("spv-benchmark-run.json").exists() {
            std::fs::remove_file("spv-benchmark-run.json").unwrap();
        }

        std::fs::write("spv-benchmark-run.json", s);
    }

    ///Renders the currently known benchmark results.
    pub fn show(&self) {
        for (bench_name, results) in &self.benches {
            let name = format!("{bench_name}.svg");
            let root = plotters::backend::SVGBackend::new(&name, (1024, 1024)).into_drawing_area();
            root.fill(&WHITE).unwrap();

            ///Currently rendering one graph per benchmark
            let mut data_map: AHashMap<String, _> = AHashMap::default();

            let mut bins: AHashMap<BenchRunType, Vec<f64>> = AHashMap::with_capacity(3);
            for run in results {
                if let Some(bin) = bins.get_mut(&run.ty) {
                    bin.push(run.pipeline_runtime);
                } else {
                    bins.insert(run.ty.clone(), vec![run.pipeline_runtime]);
                }
            }

            for (bin_ty, mut results) in bins {
                for res in &mut results {
                    *res = *res / 1_000_000.0;
                }
                /*
                                let (min, max) = results
                                    .iter()
                                    .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), v| {
                                        (min.min(*v), max.max(*v))
                                    });
                */
                data_map.insert(format!("{:?}", bin_ty), results);
            }

            let (data_min, data_max) = data_map.iter().fold(
                (f32::INFINITY, f32::NEG_INFINITY),
                |(min, max), (_name, results)| {
                    let (lmin, lmax) = results
                        .iter()
                        .fold((f32::INFINITY, f32::NEG_INFINITY), |(lmin, lmax), v| {
                            (lmin.min(*v as f32), lmax.max(*v as f32))
                        });
                    (min.min(lmin), max.max(lmax))
                },
            );

            let chart_label = data_map
                .iter()
                .map(|(name, _)| name.clone())
                .collect::<Vec<_>>();

            let value_range = fitting_range(data_map.iter().map(|(_name, dta)| dta).flatten());

            let mut chart = ChartBuilder::on(&root)
                .x_label_area_size(40)
                .y_label_area_size(40)
                .caption(&name, ("sans-serif", 20))
                .build_cartesian_2d(
                    chart_label.into_segmented(),
                    value_range.start as f32..value_range.end as f32,
                )
                .unwrap();

            chart
                .configure_mesh()
                .disable_x_mesh()
                .bold_line_style(&WHITE.mix(0.3))
                .y_desc("Time in ms")
                .x_desc("Patch type")
                .axis_desc_style(("sans-serif", 15))
                .draw()
                .unwrap();

            let plots: Vec<Boxplot<_, _>> = data_map
                .iter()
                .enumerate()
                .map(|(idx, (name, d))| {
                    Boxplot::new_vertical(SegmentValue::CenterOf(name), &Quartiles::new(d))
                })
                .collect::<Vec<_>>();

            chart.draw_series(plots).unwrap();

            // To avoid the IO failure being ignored silently, we manually call the present function
            root.present().expect("Unable to write result to file, please make sure 'plotters-doc-data' dir exists under current dir");
        }
    }
}
