use smallvec::SmallVec;
use spv_patcher::{
    rspirv::{
        dr::{Instruction, Module},
        spirv::Op,
    },
    spirv_ext::SpirvExt,
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FuncSignature {
    pub return_type: u32,
    pub argument_types: SmallVec<[u32; 3]>,
}

///Ways of identifying a function or linkage point.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum FuncIdent {
    ///Either a debug name of a function, or the string literals used by [Linkage](https://registry.khronos.org/SPIR-V/specs/unified1/SPIRV.html#Linkage).
    Name(String),
    ///Signature of a function based on input and output parameters. Note that simple functions like `() -> i32` might possibly be occur multiple times in a module.
    /// Be sure to select the right one when patching.
    ///
    /// For instance use the [spv-patcher::spirv_ext::SpirvExt::get_name] function to possibly find the name of a function.
    ///
    /// # Safety
    /// Make sure that the signature uses the same type-ids as the SPIR-V module this patch is applied on.
    ///
    /// For convenience consider using rspirv's [Builder](https://docs.rs/rspirv/latest/rspirv/dr/struct.Builder.html) for instance via [Builder::type_struct](https://docs.rs/rspirv/latest/rspirv/dr/struct.Builder.html#method.type_struct)
    Signature(FuncSignature),
}

impl FuncIdent {
    pub fn is_name_based(&self) -> bool {
        if let FuncIdent::Name(_) = self {
            true
        } else {
            false
        }
    }
}

///Reusable function finder pass. Visits a module and returns all function definitions that match
/// the given [FuncIdent].
pub struct FunctionFinder;

impl FunctionFinder {
    pub fn find(spirv: &Module, ident: &FuncIdent) -> SmallVec<[Instruction; 3]> {
        let mut results = SmallVec::new();
        //Find all OpFunctions, then check if we can match the `ident` against them.
        match ident {
            FuncIdent::Name(name) => {
                //in this case we just search for the debug name.
                for func in spirv.functions.iter() {
                    if let Some(inst) = &func.def {
                        assert!(inst.class.opcode == Op::Function);
                        let named_id = inst.result_id.unwrap();
                        if let Some(dbg_name) = spirv.get_name(named_id) {
                            if &dbg_name == name {
                                results.push(inst.clone());
                            } else {
                                if dbg_name.contains(name) {
                                    log::warn!("Debug name is not exactly the same, but similar, still using it. Expected: {} found {}", name, dbg_name);
                                    results.push(inst.clone());
                                }
                            }
                        }
                    }
                }
            }
            FuncIdent::Signature(FuncSignature {
                return_type,
                argument_types,
            }) => {
                //Iterate all functions, collect parameter type-ids and push it
                // into our collection
                for func in spirv.functions.iter() {
                    if let Some(rt) = func.def.as_ref().map(|d| d.result_type).flatten() {
                        if &rt != return_type {
                            log::info!(
                                "Testing return_type_equality: Result type missmatch: {} != {}",
                                rt,
                                return_type
                            );
                            continue;
                        }
                    } else {
                        log::warn!(
                            "No return type set, this is invalid!, must be at least OpTypeVoid"
                        );
                        continue;
                    }

                    //now parallel iter all function args and the expected args
                    let args_match = {
                        //early out if the count doesn't match, otherwise compare ID's
                        if func.parameters.len() != argument_types.len() {
                            log::info!(
                                "Argument count missmatch: {} != {}",
                                func.parameters.len(),
                                argument_types.len()
                            );
                            false
                        } else {
                            func.parameters.iter().zip(argument_types.iter()).fold(
                                true,
                                |is_matching, (actual, expected)| {
                                    if is_matching {
                                        if actual.result_type.as_ref().unwrap() != expected {
                                            log::info!(
                                                "Argument type missmatch, expected: {}, found: {}",
                                                expected,
                                                actual.result_type.as_ref().unwrap()
                                            );
                                        } else {
                                            log::info!(
                                                "ArgTypeMatch {} == {}!",
                                                expected,
                                                actual.result_type.as_ref().unwrap()
                                            );
                                        }
                                        actual.result_type.as_ref().unwrap() == expected
                                    } else {
                                        //Already unmatched
                                        false
                                    }
                                },
                            )
                        }
                    };

                    //args match as well, therefore we can push the initial OpFunction
                    if args_match {
                        results.push(func.def.clone().unwrap());
                    }
                }
            }
        }

        //FIXME: it might happen that the last basic block is just an import declared function block.
        //       in that case however, we might not execute the *compare* part above.
        //       However, import might also declare OpFunctionEnd, in that case this wouldn't be a problem.

        results
    }
}
