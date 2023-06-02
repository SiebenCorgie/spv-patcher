use smallvec::SmallVec;
use spv_patcher::{
    patch::Patch,
    rspirv::{
        dr::{Builder, Instruction, Module},
        spirv::Op,
    },
    PatcherError,
};

use crate::{
    function_finder::{FuncIdent, FuncSignature},
    FunctionFinder,
};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ReplaceError {
    #[error("The signatures of the searched-for function, and the one that will take its place do not match.")]
    SignatureMissmatch,
    #[error("Replace function must start with OpFunction and end with OpFunctionEnd")]
    StartEndError,
    #[error("Replace function must contain valid (basic-like)Block. This means it hast to declare OpLine after its arguments, and OpReturnValue before OpFunctionEnd")]
    BasicBlockError,
    #[error("Only one OpFunction (followed by parameters) should be supplied!")]
    MultipleFunctions,
    #[error("Could not find an OpFunction in the supplied replacment code")]
    NoFunctionFound,
    #[error("ConstReplace is unimplemented")]
    Unimplemented,
}

///Distintion between always replacing with the same code, or __patch_time__ calculated code.
enum ReplaceStrategy {
    ///Replace with a constant, spirv module.
    Const(Module),
    Runtime(Box<dyn RuntimeReplace>),
}

///Implements [SPIR-V's linking](https://registry.khronos.org/SPIR-V/specs/unified1/SPIRV.html#_linkage) as well as replacing already defined
/// functions.
#[allow(dead_code)]
pub struct LinkReplace {
    ident: FuncIdent,
    replace_strategy: ReplaceStrategy,
    func: Vec<Instruction>,
    //index into module.functions which is going to be replaced.
    replace_index: Option<usize>,
}

impl LinkReplace {
    ///Replaces `to_replace` with a consant SPIR-V Module. Note that the module can potentially contain multiple functions. At replace time a matching function
    /// in the template, and the supplied `replace` function is found based on `to_replace`.
    ///
    /// Possibly new type-definitions, capabilities etc. are carried over (TODO: Actually implement that).
    ///
    /// If you are unsure, consider using `new_dyn` and the [SpirvBuilder](rspirv::dr::Builder)
    pub fn new_const(to_replace: FuncIdent, replaced: Module) -> Result<Self, ReplaceError> {
        //NOTE: We can't check type information here, since we don't know the spirv module at this point.
        Ok(LinkReplace {
            ident: to_replace,
            replace_strategy: ReplaceStrategy::Const(replaced),
            func: Vec::with_capacity(0),
            replace_index: None,
        })
    }

    ///Creates a LinkReplace pass where `replace` (the function that will replace the function at `to_replace`) can be calculated at patch-time.
    /// `replace` gets the current SPIR-V module supplied, as well as the `OpFunction` that is being replaced in that module.
    ///
    /// This let's you generate the replacement code based on the current module and the function, which in turn makes type matching easier.
    pub fn new_dyn<F>(
        to_replace: FuncIdent,
        replace: impl RuntimeReplace,
    ) -> Result<Self, ReplaceError> {
        Ok(LinkReplace {
            ident: to_replace,
            replace_strategy: ReplaceStrategy::Runtime(Box::new(replace)),
            func: Vec::with_capacity(0),
            replace_index: None,
        })
    }

    //Tags the function's region internally.
    fn tag_region(&mut self, module: &Module, function: &Instruction) -> Result<(), ReplaceError> {
        //Give the dynamic replace a chance to calculate the replace code
        let to_replace = match &self.replace_strategy {
            ReplaceStrategy::Const(_const_fn) => {
                log::error!("Const replace not implemented!");
                //TODO implement merging _const_replace into the module. For that we have to
                // merge all type definitions, capabilities etc.
                //
                // Check if that would be faster on a SPIR-T module, or if rspirv has a helper for that.
                // Otherwise write a *generic* merge pass that does the following.
                //
                // 1. Analyse common and missing types, rewrite common-type-ids, add missing.
                // 2. Merge header (capabilites, extensions etc.)
                // 3. Then add code to the *end* (strict higher idx compared to original module)
                return Err(ReplaceError::Unimplemented);
                //const_fn.clone()
            }
            ReplaceStrategy::Runtime(exec) => exec(module, function)?,
        };

        //Find the module_internal index for `function`. The `FunctionFinder` earlier
        // makes sure that the arguments already match
        for (fidx, f) in module.functions.iter().enumerate() {
            if f.def.as_ref() == Some(function) {
                self.replace_index = Some(fidx);
                break;
            }
        }

        if self.replace_index.is_none() {
            log::error!("Could not find replace_function_index. This is an error, since the relpace function must have been valid in the FunctionFinder before.");
            return Err(ReplaceError::StartEndError);
        }

        //Validate that the replace code has the correct arguments
        let (replace_return, replace_args) = {
            //iter replace until we find the first OpFunction

            let mut return_type = None; //track the first return type, and *if we already found a function*.
            let mut parameters = SmallVec::<[u32; 3]>::new();
            for inst in &to_replace {
                match inst.class.opcode {
                    Op::Function => {
                        if return_type.is_some() {
                            return Err(ReplaceError::MultipleFunctions);
                        }
                        return_type = Some(inst.result_type.unwrap());
                    }
                    Op::FunctionParameter => {
                        parameters.push(inst.result_type.unwrap());
                    }
                    _ => {}
                }
            }

            if let Some(retty) = return_type {
                (retty, parameters)
            } else {
                return Err(ReplaceError::NoFunctionFound);
            }
        };

        //test replace args for matching parameters
        let sig = FuncSignature {
            return_type: replace_return,
            argument_types: replace_args,
        };
        if !sig.signature_matches(&module.functions[*self.replace_index.as_ref().unwrap()]) {
            return Err(ReplaceError::SignatureMissmatch);
        }

        //At this point we know that the target function exists in `module`, and that `to_replace` contains a valid AND
        // matching signature. Therefore we can safely replace one with the other.

        self.func = to_replace;
        Ok(())
    }

    //finally replaces the function's code
    fn rewrite_function(&self, _module: &mut Module) -> Result<(), ReplaceError> {
        //replacing is relatively straight forward. We just overwrite the old function (and its blocks) with the new one.
        // However, there is one cavheat:
        //
        // The assigned result_ids must be unique. Since the old function can be somewhere *within* the old
        // module, we need to somehow make the result_ids of the replacement functions unique. We do that by
        // just finding the module wide highest ID, and start counting from there.

        //TODO

        Ok(())
    }
}

impl Patch for LinkReplace {
    fn apply<'a>(
        mut self,
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

        let function_instruction = &funcs[0];
        //Apply patch to function at instruction

        self.tag_region(&spv_mod, &function_instruction)
            .map_err(|e| PatcherError::Internal(e.into()))?;

        self.rewrite_function(spv_mod)
            .map_err(|e| PatcherError::Internal(e.into()))?;

        //If everything went well, return
        Ok(patcher)
    }
}
