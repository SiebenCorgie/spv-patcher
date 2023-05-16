<div align="center">

# SPIR-V Patcher

SPIR-V patching utility operating on the IR level to patch new code, or specialise existing code.

</div>



All in one SPIR-V analyser, patcher and verifyer. Focuses on speed to make partial runtime SPIR-V patching possible.

Compared to simple linking, this patcher allows not just linking in new functions, but also patching existing code / basic-blocks. This enables for instance

- rewrite resource bindings
- patch code to make it compatible. For instance making [SPIR-V DXIL compatible]()
- injecting runtime linked code 
- make pipeline [stages compatible](https://github.com/expenses/spirv-location-injector)
- make non-uniform buffer access valid

## Dependencies

Generally all build dependencies get resolved by `cargo`. For runtime we depend on two tools / SDKs:
1. Vulkan SDK (specifically the LunarG validation layer)
2. [SPIR-V tools](https://github.com/KhronosGroup/SPIRV-Tools)

The tools `spirv-val` and `spirv-dis` must be accessible in `$PATH` for all tests to succeed. Otherwise the SPIR-V-Validator will be skipped when testing.

## Building

1. Download & Install Rust and Cargo
2. Set `nightly` as the default toolchain. For instace via rustup:

``` sh
rustup default nightly
```

3. Build test shader (if you intend to use testing) by changing to `crates/spv-tester/resources` and executing `compile_shader.sh` in that directory.

4. Compile everything via `cargo build --release`, or run for instance a test with `cargo run --bin spv-tester -- const_mutate`
