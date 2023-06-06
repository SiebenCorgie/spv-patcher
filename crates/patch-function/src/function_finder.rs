use smallvec::SmallVec;
use spv_patcher::{
    rspirv::{
        dr::{Function, Instruction, Module},
        spirv::Op,
    },
    spirv_ext::SpirvExt,
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FuncSignature {
    pub return_type: u32,
    pub argument_types: SmallVec<[u32; 3]>,
}

impl FuncSignature {
    pub fn signature_matches(&self, module_function: &Function) -> bool {
        let rt = module_function.def.as_ref().unwrap().result_type.unwrap();
        if rt != self.return_type {
            log::info!(
                "Testing return_type_equality: Result type missmatch: {} != {}",
                rt,
                self.return_type
            );
            return false;
        }

        if module_function.parameters.len() != self.argument_types.len() {
            log::info!(
                "Argument count missmatch: {} != {}",
                module_function.parameters.len(),
                self.argument_types.len()
            );
            return false;
        }

        module_function
            .parameters
            .iter()
            .zip(self.argument_types.iter())
            .fold(true, |matches, (mfp, sp)| {
                if matches {
                    if mfp.result_type.as_ref().unwrap() == sp {
                        log::info!(
                            "ArgTypeMatch {} == {}!",
                            mfp.result_type.as_ref().unwrap(),
                            sp
                        );
                        true
                    } else {
                        log::info!(
                            "Argument type missmatch, expected: {}, found: {}",
                            mfp.result_type.as_ref().unwrap(),
                            sp
                        );
                        false
                    }
                } else {
                    false
                }
            })
    }
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
            FuncIdent::Signature(sig) => {
                //Iterate all functions, collect parameter type-ids and push it
                // into our collection
                for func in spirv.functions.iter() {
                    if sig.signature_matches(func) {
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
