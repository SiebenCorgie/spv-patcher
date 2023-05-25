use smallvec::SmallVec;
use spv_patcher::{
    rspirv::{dr::Module, spirv::Op},
    spirv_ext::SpirvExt,
};

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
        let mut results = SmallVec::new();
        let mut collection_decl = None;
        for inst in spirv.all_inst_iter() {
            match inst.class.opcode {
                Op::Function => {
                    //read function's result type and start collecting
                    let return_type = inst.result_type.expect("Function had no return type!");

                    let debug_name = spirv.get_name(inst.result_id.unwrap());

                    //For sanity, check that we already exited the collection loop.
                    assert!(
                        collection_decl.is_none(),
                        "Found OpFunction, but already collecting arguments."
                    );
                    collection_decl = Some(FuncDeclaration {
                        debug_name,
                        signature: FuncSignature {
                            return_type,
                            argument_types: SmallVec::new(),
                        },
                    });
                }
                Op::FunctionParameter => {
                    if let Some(collecting) = &mut collection_decl {
                        let param_ty = inst
                            .result_type
                            .expect("Function parameter had no result type");
                        collecting.signature.argument_types.push(param_ty);
                    } else {
                        log::warn!("Found function parameter, but not collecting function OP, is the module valid?");
                    }
                }
                _ => {
                    //Found some other instruction, finish collection.
                    if let Some(coll) = collection_decl.take() {
                        results.push(coll);
                    }
                }
            }
        }
        //Post take as well if the last basic-block was a `import` declared function.
        if let Some(coll) = collection_decl.take() {
            results.push(coll);
        }
        results
    }
}
