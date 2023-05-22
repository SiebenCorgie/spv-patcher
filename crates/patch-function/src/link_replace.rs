use spv_patcher::rspirv::dr::Instruction;

use crate::function_finder::FuncIdent;

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
