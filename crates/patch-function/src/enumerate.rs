use smallvec::SmallVec;
use spv_patcher::rspirv::dr::Module;

use crate::function_finder::FuncSignature;

pub struct FuncDeclaration {
    pub debug_name: Option<String>,
    pub signature: FuncSignature,
}

///Reusable function enumerator. Searches for all available functions, and enumerates their
/// [FuncIdent](crate::FuncIdent) and possibly their name.
pub struct FuncEnumerator;
impl FuncEnumerator {
    pub fn enumerate(_spirv: &Module) -> SmallVec<[FuncDeclaration; 3]> {
        //Walk the module, for each OpFunction, collect the return type-id,
        // as well ass all argument type-ids.
        // Then check if we have a debug name.
        let results = SmallVec::new();

        results
    }
}
