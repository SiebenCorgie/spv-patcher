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

use crate::{patch::Patch, PatcherError};
///Loaded, to be patched module.
#[derive(Clone)]
pub struct Module {
    pub(crate) ctx: Rc<Context>,
    pub(crate) module: spirt::Module,
}

impl Module {
    pub fn load(ctx: &Rc<Context>, binary: Vec<u8>) -> Result<Module, PatcherError> {
        let module = spirt::Module::lower_from_spv_bytes(ctx.clone(), binary)?;

        Ok(Module {
            ctx: ctx.clone(),
            module,
        })
    }

    ///Schedules a patch run.
    pub fn schedule_patches(&mut self) -> PatchBuilder {
        PatchBuilder {
            module: self.clone(),
        }
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

pub struct PatchBuilder {
    module: Module,
}

impl PatchBuilder {
    ///Returns the SPIRT context for this patch-builder.
    pub fn ctx(&self) -> Rc<Context> {
        self.module.ctx.clone()
    }
    pub fn patch(mut self, mut patch: impl Patch) -> Result<Self, PatcherError> {
        {
            //patch.visit_module(&self.module.module);
            patch.in_place_transform_module(&mut self.module.module);
        }
        //call path visitor, then transformer.
        Ok(self)
    }

    //Prints current state
    pub fn print(self) -> Self {
        log::trace!("Printing ...");
        let plan = spirt::print::Plan::for_module(&self.module.module);
        log::trace!("{}", plan.pretty_print());
        self
    }

    ///Finishes patching and returns the SpirV module
    pub fn finish(self) -> Vec<u32> {
        let emitter = self.module.module.lift_to_spv_module_emitter().unwrap();
        emitter.words
    }
}
