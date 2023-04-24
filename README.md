<div align="center">

# SpirV Patcher

Second iteration of [marp](gitlab.com/tendsinmende/marp). Vulkan wrapper around the [ash](crates.io/crates/ash) crate. Focuses on stable resource creation and usability. Tries to minimized duplication between ash and itself.

[![pipeline status](https://gitlab.com/tendsinmende/marpii/badges/main/pipeline.svg)](https://gitlab.com/tendsinmende/marpii/-/commits/main)
[![dependency status](https://deps.rs/repo/gitlab/tendsinmende/marpii/status.svg)](https://deps.rs/repo/gitlab/tendsinmende/marpii)
[![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/L3L3F09W2)
</div>



All in one SpirV analyser, patcher and verifyer. Focuses on speed to make partial runtime SpirV patching possible.

Compared to simple linking, this patcher allows not just linking in new functions, but also patching existing code / basic-blocks. This enables for instance

- rewrite resource bindings
- patch code to make it compatible. For instance making [SpirV DXIL compatible]()
- injecting runtime linked code 
- make pipeline [stages compatible](https://github.com/expenses/spirv-location-injector)
