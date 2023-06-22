use crate::reporter::Reporter;

///Dynamic replacement benchmark
pub mod dyn_replace;

///Benchmark interface. A benchmark is defined as:
///
/// - A *unmodified* pass: benchmarking the unpatched code
/// - A *patched-compiled* patched pass: benchmarking the compiled-patched pass (via glslang or rustc)
/// - A *patched-runtime*: benchmarking patched code via the spv-patch tool.
pub trait Benchmark {
    fn name(&self) -> &str;
    fn bench_unmodified(&mut self, rmg: &mut marpii_rmg::Rmg, reporter: &mut Reporter, runs: usize);
    fn bench_patched_compiled(
        &mut self,
        rmg: &mut marpii_rmg::Rmg,
        reporter: &mut Reporter,
        runs: usize,
    );
    fn bench_patched_runtime(
        &mut self,
        rmg: &mut marpii_rmg::Rmg,
        reporter: &mut Reporter,
        runs: usize,
    );
}
