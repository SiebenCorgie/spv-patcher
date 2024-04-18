//! A [spirv-linker](https://crates.io/crates/spirv-linker) based linker patch.
//!
//! Contains an auxilary patch, that lets you find function based on attributes
//! and make them be _imported_.
//!

pub use patch_function::rspirv::dr::Module;
use spv_patcher::{patch::Patch, PatcherError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LinkError {
    #[error(transparent)]
    LinkerLibError(spirv_linker::LinkerError),
}

#[derive(Clone)]
pub struct Linker {
    //The module that is being linked
    pub linkage_module: Module,
    ///True, if the resulting module should / could be a library.
    pub is_result_library: bool,
}

impl Patch for Linker {
    fn apply<'a>(
        mut self,
        mut patcher: spv_patcher::patch::Patcher<'a>,
    ) -> Result<spv_patcher::patch::Patcher<'a>, spv_patcher::PatcherError> {
        let new_module = {
            let spvmodule = patcher.ir_state.as_spirv();
            //just call the linker, assuming there is something to link
            let new_module = spirv_linker::link(
                &mut [spvmodule, &mut self.linkage_module],
                &spirv_linker::Options {
                    lib: self.is_result_library,
                    partial: true,
                },
            )
            .map_err(|e| PatcherError::Internal(e.into()))?;
            new_module
        };

        //now overwrite
        *patcher.ir_state.as_spirv() = new_module;

        Ok(patcher)
    }
}
