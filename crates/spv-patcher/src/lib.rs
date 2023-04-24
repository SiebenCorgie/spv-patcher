//! # SpirV-Patcher.
//!
//! Main library crate providing the semantic analysis of a SpirV module and patching capabilities.
//!
//! It is structured in (currently) three main passes.
//!
//! - Disassamble / Assamble: Takes care of loading / dissasabling a SpirV module.
//! - Patch: defines the `Patch` API, as well as implemented patching passes.
//! - Verify: Provides custom verification methods, as well as using `spirv-val` to verify a SpirV module.
//!

pub mod dis_assamble;
pub mod patch;
pub mod verify;
