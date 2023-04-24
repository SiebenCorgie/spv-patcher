//! # Loading SpirV modules
//!
//! Right now we use [RspirV](https://github.com/gfx-rs/rspirv) to load a SpirV module. However, this is hidden for now, since we might
//! change to [SpirT](https://github.com/EmbarkStudios/spirt) in the future for internal representation. The format is actually *made* for this
//! kind of work, instead of just being an IR-Format meant to communicate in byte-format.
//!
//! # Internal representation.
//!
//! This module also defines the internal /*semantic*/ representation of SpirV modules.
//!
//! For now this representation contains
//! - entry points
//! - resource bindings
//! - functions
//! - function bodies
//! - control-flow-graph
//! - basic-blocks
//!
//! This list is non exhaustive and might change / be extended.
//!
//! Each of those points can be patched individually. Additionally patches can use a combination of those. For instance, you might want to patch the module with a constant (basically specialization constans).
//! This touches not just BBs (basic blocks), but possibly CFG (control flow) or resource binding as well.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ModuleError {
    #[error("Failed to parse binary: {0}")]
    ParseError(#[from] rspirv::binary::ParseState),
    #[error("Failed to build structured representation: {0:?}")]
    LiftError(rspirv::lift::ConversionError),
}

impl From<rspirv::lift::ConversionError> for ModuleError {
    fn from(value: rspirv::lift::ConversionError) -> Self {
        ModuleError::LiftError(value)
    }
}

///Loaded, to be patched module.
pub struct Module {
    pub dr: rspirv::dr::Module,
    pub sr: rspirv::sr::module::Module,
}

impl Module {
    pub fn load(binary: &[u8]) -> Result<Module, ModuleError> {
        let mut loader = rspirv::dr::Loader::new();
        {
            let parser = rspirv::binary::Parser::new(binary, &mut loader);
            parser.parse()?;
        }

        let dr = loader.module();
        let sr = rspirv::lift::LiftContext::convert(&dr)?;
        Ok(Module { dr, sr })
    }
}
