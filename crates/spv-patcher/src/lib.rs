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

use std::rc::Rc;

use patch::Patch;
use thiserror::Error;

pub use spirt;

pub mod dis_assamble;
pub mod patch;
pub mod verify;

#[derive(Error, Debug)]
pub enum PatcherError {
    #[error("Failed to lower spir-v module with: {0}")]
    LowerError(#[from] std::io::Error),
}

pub struct Patcher {
    //SPIR-T context
    ctx: Rc<spirt::Context>,
}

impl Patcher {
    pub fn new() -> Self {
        Patcher {
            ctx: Rc::new(spirt::Context::new()),
        }
    }

    ///loads SPIR-V module into this patcher's context.
    pub fn load_module(&self, spirv_bytes: Vec<u8>) -> Result<dis_assamble::Module, PatcherError> {
        dis_assamble::Module::load(&self.ctx, spirv_bytes)
    }
}
