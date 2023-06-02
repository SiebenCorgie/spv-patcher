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
                //Used to collect parameter types after an OpFunction
                let mut collecting_parameters_for = None;
                let mut parameters = SmallVec::<[u32; 3]>::new();
                for inst in spirv.all_inst_iter() {
                    match inst.class.opcode {
                        Op::Function => {
                            assert!(
                                collecting_parameters_for.is_none(),
                                "OpFunction, while already collecting a function's parameters"
                            );
                            parameters.clear();
                            collecting_parameters_for = Some(inst.clone());
                            log::info!("Found function with result type {:?}", inst.result_type);
                            parameters.push(inst.result_type.unwrap());
                            //push result type as first parameter
                        }
                        Op::FunctionParameter => {
                            assert!(collecting_parameters_for.is_some());
                            if let Some(ty_id) = inst.result_type {
                                parameters.push(ty_id);
                            } else {
                                panic!("FunctionParameter had no type id!");
                            }
                        }
                        _ => {
                            //if we where collecting up to this point, this means the function header is finished and we can check the
                            // type matching.
                            if let Some(src_function) = collecting_parameters_for.take() {
                                //NOTE: for sanity, this *should* be OpLine or OpLabel. However, Labels can be deleted, and sometimes OpLines
                                // are emitted wrong. Which is a error-on paper, but sometimes happens.
                                assert!(
                                    inst.class.opcode == Op::Label || inst.class.opcode == Op::Line,
                                    "Post Function header should be OpLine or OpLabel, was {:?}",
                                    inst.class.opcode
                                );

                                //first, match the ops result type, which should be the first collected parameter.
                                let mut param_iter = parameters.iter();
                                let resty = param_iter.next().unwrap();
                                if resty != return_type {
                                    log::info!(
                                        "Testing return_type_equality: Result type missmatch: {} != {}",
                                        resty,
                                        return_type
                                    );
                                    continue;
                                }

                                //now parallel iter all function args and the expected args
                                let args_match = param_iter.zip(argument_types.iter()).fold(true, |is_matching, (actual, expected)|{
                                    if is_matching{
                                        if actual != expected{
                                            log::info!("Argument type missmatch, expected: {}, found: {}", expected, actual);
                                        }
                                        actual == expected
                                    }else{
                                        //Already unmatched
                                        false
                                    }
                                });

                                //args match as well, therefore we can push the initial OpFunction
                                if args_match {
                                    results.push(src_function);
                                }
                            }
                        }
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
