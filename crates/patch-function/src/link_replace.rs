use smallvec::SmallVec;
use spv_patcher::rspirv::dr::Instruction;

///Ways of identifying a function or linkage point.
pub enum FuncIdent {
    ///Either a debug name of a function, or the string literals used by [Linkage](https://registry.khronos.org/SPIR-V/specs/unified1/SPIRV.html#Linkage).
    Name(&'static str),
    ///Signature of a function based on input and output parameters. Note that simple fuctions like `() -> i32` might possibly be occur multiple times in a module.
    /// Be sure to select the right one when patching.
    Signature {
        ///In-Order types of arguments to the function.
        arg_types: SmallVec<[Instruction; 3]>,
        ///The result type of the function
        return_type: Instruction,
    },
}

///Implements [SPIR-V's linking](https://registry.khronos.org/SPIR-V/specs/unified1/SPIRV.html#_linkage) as well as replacing already defined
/// functions.
pub struct LinkReplacePatch {
    ident: FuncIdent,
}
