use spv_patcher::rspirv::dr::Module;

use crate::FuncIdent;

pub struct ConstantReplace {
    ///Module from which the replacement code is taken
    pub replacement_module: Module,
    ///Identification of the function that is being replaced while patching.
    pub ident: FuncIdent,
}
