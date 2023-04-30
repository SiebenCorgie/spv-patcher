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

use std::rc::Rc;

use rspirv::binary::Assemble;

use crate::{patch::Patcher, PatcherError};

pub struct Module {
    //Binary SPIR-V template data.
    module_binary: Vec<u8>,
}

impl Module {
    pub fn new(spirv_binary: Vec<u8>) -> Result<Self, PatcherError> {
        Ok(Module {
            module_binary: spirv_binary,
        })
    }

    pub fn patch<'a>(&'a self) -> Result<Patcher<'a>, PatcherError> {
        //try to load into rspirv. This will throw an error if it's malformed.
        let spirv = rspirv::dr::load_bytes(&self.module_binary)?;

        Ok(Patcher {
            module: self,
            ir_state: crate::patch::IrState::SpirV(spirv),
        })
    }
}
