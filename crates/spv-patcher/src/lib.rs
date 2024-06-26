//! # SpirV-Patcher.
//!
//! Main library crate providing the semantic analysis of a SpirV module and patching capabilities.
//!
//! It is structured in (currently) three main passes.
//!
//! - Disassamble / Assamble: Takes care of loading / disassabling a SpirV module.
//! - Patch: defines the `Patch` API, as well as implemented patching passes.
//! - Verify: Provides custom verification methods, as well as using `spirv-val` to verify a SpirV module.
//!
#![deny(warnings)]

use std::error::Error;

use thiserror::Error;

pub use rspirv;
pub use spirt;

mod dis_assamble;
pub use dis_assamble::Module;
mod print;
pub use print::DisassamblerPrinter;
pub mod patch;
pub mod spirv_ext;
pub mod type_tree;
mod validator;
pub mod verify;
pub use validator::Validator;

#[derive(Error, Debug)]
pub enum PatcherError {
    #[error("Failed to lower spir-v module with: {0}")]
    LowerError(#[from] std::io::Error),
    #[error("Could not parse spirv binary code: {0}")]
    SpirVParseError(#[from] rspirv::binary::ParseState),
    #[error("SPIR-V build error")]
    SpirVBuildError(#[from] rspirv::dr::Error),
    #[error("Could not load SPIR-V binary, multiple entry-points exist")]
    MultipleEntryPoints,
    #[error("Could not load SPIR-V binary, no entry-point exists")]
    NoEntryPoint,
    #[error("Patch internal runtime error: {0}")]
    Internal(Box<dyn Error>),
}
