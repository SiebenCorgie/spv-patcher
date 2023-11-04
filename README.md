<div align="center">

# SPIR-V Patcher

SPIR-V patching utility operating on the IR level to patch new code, or specialise existing code.

[![dependency status](https://deps.rs/repo/gitlab/tendsinmende/spv-patcher/status.svg)](https://deps.rs/repo/gitlab/tendsinmende/spv-patcher)

</div>



All in one SPIR-V analyser, patcher and verifyer. Focuses on speed to make partial runtime SPIR-V patching possible.

Compared to simple linking, this patcher allows not just linking in new functions, but also patching existing code / basic-blocks. This enables for instance

- rewrite resource bindings
- patch code to make it compatible. For instance making [SPIR-V DXIL compatible](https://godotengine.org/article/d3d12-adventures-in-shaderland/)
- injecting runtime linked code 
- make pipeline [stages compatible](https://github.com/expenses/spirv-location-injector)
- make non-uniform buffer access valid

## Dependencies

Generally all build dependencies get resolved by `cargo`. For runtime we depend on two tools / SDKs:
1. Vulkan SDK (specifically the LunarG validation layer)
2. [SPIR-V tools](https://github.com/KhronosGroup/SPIRV-Tools)

The tools `spirv-val` and `spirv-dis` must be accessible in `$PATH` for all tests to succeed. Otherwise the SPIR-V-Validator will be skipped when testing.

## Building

1. Download & Install Rust, usually via [Rustup](https://rustup.rs/).
2. Build test shader (if you intend to use testing) by changing to `crates/spv-tester/resources` and executing `compile_shader.sh` in that directory.
3. Compile everything via `cargo build --release`, or run a test with `cargo run --bin spv-tester -- const_mutate`


## Testing

To run all tests execute

``` shell
cargo run --bin spv-tester static_mutate non_uniform_patch static_replace dyn_replace function_finder
```

You might want to bless your results if you are sure they are correct by appending `bless` to the sequence.


## Benchmarking

There are two benchmarks. One is the GPU-Performance benchmark. Use 

``` shell
cargo run --bin spv-benchmark --release
```

to run GPU the benchmarks.

For the CPU site we use [Criterion](https://github.com/bheisler/criterion.rs). This is one of the main Rust benchmarking frameworks. It integrates with our projects. Therefor running 

``` shell
cargo bench
```

is enough. The Results can be found at `target/criterion/report/index.html`
