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

use spirt::Context;

use crate::PatcherError;
///Loaded, to be patched module.
pub struct Module<'a> {
    pub(crate) ctx: &'a Rc<Context>,
    pub(crate) module: spirt::Module,
}

impl<'a> Module<'a> {
    pub fn load(ctx: &'a Rc<Context>, binary: Vec<u8>) -> Result<Module<'a>, PatcherError> {
        let module = spirt::Module::lower_from_spv_bytes(ctx.clone(), binary)?;

        Ok(Module { ctx, module })
    }

    //TODO functions
    // - instruction to line -> Converts instruction to possible OpLine region, which in turn is turned, if found to a string and possibly
    //   line and / or column.
    //
    // - iter basic blocks -> adaptor that iterates all known blocks
    // - iter entry points -> adaptor that iterates all entry blocks
    // - iter inputs -> iterates all input variables (everything tagged with StorageClass::Input)
    // - iter outputs -> iterates all output variables (everything tagged with StorageClass::Output)
    // - iter bindings -> Iterates all general bindings. Includes not just input / output, but descriptor bindings as well.
    //
}
