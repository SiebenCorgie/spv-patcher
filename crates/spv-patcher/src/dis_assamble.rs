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

use crate::{patch::Patcher, PatcherError};

///Single loaded module.
///
/// Note that we currently only support one entry-point per module.
pub struct Module {
    //Binary SPIR-V template data.
    module_binary: Vec<u8>,
    spv_mod: rspirv::dr::Module,
}

impl Module {
    pub fn new(spirv_binary: Vec<u8>) -> Result<Self, PatcherError> {
        let spv_mod = rspirv::dr::load_bytes(&spirv_binary)?;

        if spv_mod.entry_points.len() > 1 {
            return Err(PatcherError::MultipleEntryPoints);
        }

        if spv_mod.entry_points.len() == 0 {
            return Err(PatcherError::NoEntryPoint);
        }

        Ok(Module {
            module_binary: spirv_binary,
            spv_mod,
        })
    }

    ///Returns the spirv template used by this module when patching
    pub fn spirv(&self) -> &rspirv::dr::Module {
        &self.spv_mod
    }

    ///Returns the template SPIR-V code. Note that this module might not be a valid SPIR-V module.
    pub fn template_code(&self) -> &[u8] {
        &self.module_binary
    }

    pub fn patch<'a>(&'a self) -> Patcher<'a> {
        Patcher {
            module: self,
            ir_state: crate::patch::IrState::SpirV(self.spv_mod.clone()),
        }
    }
}
