use smallvec::SmallVec;
use spv_patcher::{rspirv::dr::Module, spirv_ext::SpirvExt};

use crate::function_finder::FuncSignature;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FuncDeclaration {
    pub debug_name: Option<String>,
    pub signature: FuncSignature,
}

///Reusable function enumerator. Searches for all available functions, and enumerates their
/// [FuncIdent](crate::FuncIdent) and possibly their name.
//TODO: We could implement that as an actual iterator, but that would borrow the module immutably, which in turn might be not the best
//      idea.
//      The iterator would basically execute one *collection-step* per next() call, yielding until we
//      reach the end of the module.
pub struct FuncEnumerator;
impl FuncEnumerator {
    pub fn enumerate(spirv: &Module) -> SmallVec<[FuncDeclaration; 3]> {
        //Walk the module, for each OpFunction, collect the return type-id,
        // as well as all argument type-ids.
        // Then check if we have a debug name.
        spirv
            .functions
            .iter()
            .map(|f| {
                let debug_name = spirv.get_name(f.def.as_ref().unwrap().result_id.unwrap());
                let signature = FuncSignature {
                    return_type: f.def.as_ref().unwrap().result_type.unwrap(),
                    argument_types: f
                        .parameters
                        .iter()
                        .map(|p| p.result_type.unwrap())
                        .collect(),
                };
                FuncDeclaration {
                    debug_name,
                    signature,
                }
            })
            .collect()
    }
}
