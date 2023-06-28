use std::path::PathBuf;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

use patch_function::DynamicReplace;

use crate::mandelbrot_append::const_mandelbrot_patch;

mod mandelbrot_append;

fn dynamic_patch_benchmark(c: &mut Criterion) {
    const SRC_CODE: &[u8] = include_bytes!("../resources/bench_const_replace.spv");
    let module = spv_patcher::Module::new(SRC_CODE.to_vec()).unwrap();

    //Schedule patch
    c.bench_with_input(
        BenchmarkId::new("MandelbrotPatchDynamic", "dynamic replace and assemble"),
        &module,
        |b, module| {
            b.iter(|| {
                module
                    .patch()
                    .patch(mandelbrot_append::dyn_mandelbrot_patch())
                    .unwrap()
                    .assemble()
            })
        },
    );

    c.bench_with_input(
        BenchmarkId::new("MandelbrotPatchDynamic", "dynamic replace"),
        &module,
        |b, module| {
            b.iter(|| {
                module
                    .patch()
                    .patch(mandelbrot_append::dyn_mandelbrot_patch())
                    .unwrap()
            })
        },
    );
}

fn constant_patch_benchmark(c: &mut Criterion) {
    const SRC_CODE: &[u8] = include_bytes!("../resources/bench_const_replace.spv");
    let module = spv_patcher::Module::new(SRC_CODE.to_vec()).unwrap();

    //Schedule patch
    c.bench_with_input(
        BenchmarkId::new(
            "MandelbrotPatchConstant",
            "const replace using spirv-link and assemble",
        ),
        &module,
        |b, module| {
            b.iter(|| {
                module
                    .patch()
                    .patch(const_mandelbrot_patch())
                    .unwrap()
                    .assemble()
            })
        },
    );

    c.bench_with_input(
        BenchmarkId::new("MandelbrotPatchConstant", "const replace using spirv-link"),
        &module,
        |b, module| b.iter(|| module.patch().patch(const_mandelbrot_patch()).unwrap()),
    );
}

fn compile_glsl(c: &mut Criterion) {
    let path = "resources/mandelbrot.comp";
    let target = "resources/mandelbrot_bench_glsl.comp.spv";

    if PathBuf::from(target).exists() {
        std::fs::remove_file(target).unwrap();
    }

    c.bench_function("Compile GLSL", |b| {
        b.iter(|| {
            let command = std::process::Command::new("glslangValidator")
                .arg("-g")
                .arg("-V")
                .arg(path)
                .arg("-o")
                .arg(target)
                .output()
                .unwrap();

            if !command.status.success() {
                println!("Out: {:?}", std::str::from_utf8(&command.stdout).unwrap());
                println!("Err: {}", std::str::from_utf8(&command.stderr).unwrap());
            }
            assert!(command.status.success())
        })
    });
}

criterion_group!(
    benches,
    dynamic_patch_benchmark,
    constant_patch_benchmark,
    compile_glsl
);
criterion_main!(benches);
