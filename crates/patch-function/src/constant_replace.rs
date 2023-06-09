use spv_patcher::{
    patch::Patch,
    rspirv::{binary::Assemble, dr::Module},
};

use crate::{function_finder::FuncSignature, FuncIdent};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConstantReplaceError {
    #[error("No function found for index {0} in the replacement module")]
    InvalidReplacementIndex(usize),
    #[error("SPIRV builder error: {0}")]
    BuilderError(#[from] spv_patcher::rspirv::dr::Error),
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
}

///Declares a *whole* spirv module and a function index into the module that will replace a function with the same identification
/// within a module that is being patched.
pub struct ConstantReplace {
    ///Module from which the replacement code is taken
    pub replacement_module: Module,
    ///index into the replacement module's function vector which selects the function that is being replaced.
    pub replacement_index: usize,
    ///function ident of the function that is replaced in the `replacement_module` context.
    #[allow(dead_code)]
    ident: FuncIdent,
}

impl ConstantReplace {
    pub fn new(
        replacement_module: Module,
        replacement_index: usize,
    ) -> Result<Self, ConstantReplaceError> {
        let ident = {
            let f = replacement_module.functions.get(replacement_index).ok_or(
                ConstantReplaceError::InvalidReplacementIndex(replacement_index),
            )?;

            let return_type = f.def.as_ref().unwrap().result_type.unwrap();
            let argument_types = f
                .parameters
                .iter()
                .map(|arg| arg.result_type.unwrap())
                .collect();

            FuncIdent::Signature(FuncSignature {
                return_type,
                argument_types,
            })
        };

        Ok(ConstantReplace {
            replacement_index,
            replacement_module,
            ident,
        })
    }
}

impl Patch for ConstantReplace {
    fn apply<'a>(
        self,
        mut patcher: spv_patcher::patch::Patcher<'a>,
    ) -> Result<spv_patcher::patch::Patcher<'a>, spv_patcher::PatcherError> {
        let (_spirt, spttcx) = patcher.ir_state.as_spirt();

        //For constant replace we need to merge the patcher's *context* and the
        // context of the module we are merging.
        //
        // In SPIRV this would lead to messy id rewriting etc. However, in SPIRT,
        // we can just lower both modules into the same context.
        // Then we copy over one module to the other and rewire the function calls.

        //lower `replacement_module` into spir
        let to_lower_spv = self.replacement_module.assemble();
        let lowered = spv_patcher::spirt::Module::lower_from_spv_bytes(
            spttcx.clone(),
            bytemuck::cast_vec(to_lower_spv),
        )?;

        eprintln!(
            "{}",
            spv_patcher::spirt::print::Plan::for_module(&lowered).pretty_print()
        );

        //TODO find function in `lowered` and write it to
        //     `spirt`, then rewrite function calls in `spirt` to use new
        //     function.

        Ok(patcher)
    }
}
