use spv_patcher::{patch::Patch, rspirv::dr::Instruction};

use crate::{function_finder::FuncIdent, FunctionFinder};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ReplaceError {
    #[error("The signatures of the searched-for function, and the one that will take its place do not match.")]
    SignatureMissmatch,
}

///Implements [SPIR-V's linking](https://registry.khronos.org/SPIR-V/specs/unified1/SPIRV.html#_linkage) as well as replacing already defined
/// functions.
#[allow(dead_code)]
pub struct LinkReplace {
    ident: FuncIdent,
    func: Vec<Instruction>,
}

impl LinkReplace {
    pub fn new(to_replace: FuncIdent, replaced: Vec<Instruction>) -> Result<Self, ReplaceError> {
        //TODO check that replaced and `to_replace` have matching signatures.

        Ok(LinkReplace {
            ident: to_replace,
            func: replaced,
        })
    }
}

impl Patch for LinkReplace {
    fn apply<'a>(
        self,
        mut patcher: spv_patcher::patch::Patcher<'a>,
    ) -> Result<spv_patcher::patch::Patcher<'a>, spv_patcher::PatcherError> {
        let spv_mod = patcher.ir_state.as_spirv();
        let funcs = FunctionFinder::find(&spv_mod, &self.ident);
        if funcs.len() == 0 {
            return Err(spv_patcher::PatcherError::Internal(
                format!(
                    "Found no function to replace with signature {:?}",
                    self.ident
                )
                .into(),
            ));
        }

        if funcs.len() > 1 {
            log::warn!(
                "Found more than one function matching {:?}, using first one",
                &self.ident
            );
        }

        let _function_instruction = &funcs[0];
        //Apply patch to function at instruction

        Ok(patcher)
    }
}
