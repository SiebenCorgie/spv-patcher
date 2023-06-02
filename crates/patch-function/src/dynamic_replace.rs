use smallvec::SmallVec;
use spv_patcher::{
    patch::Patch,
    rspirv::{
        dr::{Builder, Instruction, Module},
        spirv::{FunctionControl, Op},
    },
};
use thiserror::Error;

use crate::{FuncIdent, FunctionFinder};

#[derive(Error, Debug)]
pub enum DynamicReplaceError {
    #[error("Could not find function-index for given function instruction")]
    NoFunctionIndex,
    #[error("SPIRV builder error: {0}")]
    BuilderError(#[from] spv_patcher::rspirv::dr::Error),
    #[error("Last instruction added to the builder must be OpReturn or OpReturnValue, depending on the supplied signature return-type")]
    InvalidLastInstruction,
}

#[derive(Debug, Clone)]
pub struct RuntimeFunctionSignature {
    ///TypeID of the return value. Use a value of this type for you final
    /// `OpReturnValue %x` where `%x` of type `return_type`.
    pub return_type: u32,
    ///Collection of all parameters in `(value_id, type_id)` form.
    ///
    /// Therefore if given parameters `[(v1,t1), (v2,t2)]` and you do `OpLoad %v3, %v1`, then `%v3` will be of type `%t1`.
    pub parameter: SmallVec<[(u32, u32); 3]>,
    function_type: u32,
}

///Trait alias for the replace function that is executed when using [new_dyn](LinkReplace::dyn_new).
pub trait RuntimeReplace =
    Fn(&mut Builder, RuntimeFunctionSignature) -> Result<(), DynamicReplaceError> + 'static;

pub struct DynamicReplace {
    ///Runtime executed *replace* function. Gets supplied the functions signature.
    /// Note that the builder will have already created the replacement function. Therefore you can
    /// start writing you basic block(s) using the supplied argument's IDs.
    ///
    /// After finishing be sure to write back a correctly typed value based on the return type ID. Otherwise the patch might
    /// fail its post-patching verification.
    pub replace_function: Box<dyn RuntimeReplace>,
    ///Identification of the function that is being overwritten.
    pub ident: FuncIdent,
    replace_index: Option<usize>,
}

impl DynamicReplace {
    pub fn new(to_replace: FuncIdent, replacement: impl RuntimeReplace) -> Self {
        DynamicReplace {
            replace_function: Box::new(replacement),
            ident: to_replace,
            replace_index: None,
        }
    }

    fn find_copy_signature(
        &mut self,
        module: &Module,
        function_instruction: &Instruction,
    ) -> Result<RuntimeFunctionSignature, DynamicReplaceError> {
        //find replace index
        for (fidx, f) in module.functions.iter().enumerate() {
            if f.def.as_ref() == Some(function_instruction) {
                self.replace_index = Some(fidx);
                break;
            }
        }

        let index = if let Some(idx) = &self.replace_index {
            *idx
        } else {
            log::error!("Could not find function's index, this should not happen if the Function finder works correctly.");
            return Err(DynamicReplaceError::NoFunctionIndex);
        };

        let return_type = module.functions[index]
            .def
            .as_ref()
            .unwrap()
            .result_type
            .unwrap();

        let mut parameter = SmallVec::<[(u32, u32); 3]>::new();
        for param in &module.functions[index].parameters {
            parameter.push((param.result_id.unwrap(), param.result_type.unwrap()));
        }

        let function_type = module.functions[index].def.as_ref().unwrap().operands[0]
            .id_ref_any()
            .unwrap();

        Ok(RuntimeFunctionSignature {
            return_type,
            parameter,
            function_type,
        })
    }

    //If successful, returns the newly created function's ID
    fn write_new_function(
        &mut self,
        module: &mut Module,
        mut sig: RuntimeFunctionSignature,
    ) -> Result<u32, DynamicReplaceError> {
        //creates the builder on the module, starts a new function, then executes the clojure on that
        // function.
        // Afterwards we end the function and rewrite all calls to the newly written one.

        let mut tmp_module = Module::new();
        core::mem::swap(&mut tmp_module, module);
        let mut builder = Builder::new_from_module(tmp_module);

        //start a new function based on `sig`
        let function_id = builder.begin_function(
            sig.return_type,
            None,
            FunctionControl::empty(),
            sig.function_type,
        )?;
        //add all parameters
        for p in &mut sig.parameter {
            //add parameter and rewrite id
            let param_id = builder.function_parameter(p.1)?;
            p.0 = param_id;
        }
        //now start basic block
        let _block_id = builder.begin_block(None)?;

        //let the closure take over
        (self.replace_function)(&mut builder, sig)?;

        //Now check if the caller has already terminated the block (via OpReturnValue) if not, throw an error
        //as we can't know what should be returned
        let last_op = builder.pop_instruction()?;
        match last_op.class.opcode {
            Op::Return | Op::ReturnValue => {}
            _ => return Err(DynamicReplaceError::InvalidLastInstruction),
        }

        //push back the instruction
        builder.insert_into_block(spv_patcher::rspirv::dr::InsertPoint::End, last_op)?;

        //Now end function
        builder.end_function()?;

        //swap back modules
        let mut tmp_module = builder.module();
        core::mem::swap(&mut tmp_module, module);

        Ok(function_id)
    }

    fn rewrite_function_ids(
        &self,
        module: &mut Module,
        new_id: u32,
    ) -> Result<(), DynamicReplaceError> {
        //rewrite all function calls to the old function with the new id

        Ok(())
    }

    fn verify_return(&self, _module: &Module) -> Result<(), DynamicReplaceError> {
        Ok(())
    }
}

impl Patch for DynamicReplace {
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
        let sig = self
            .find_copy_signature(&spv_mod, &function_instruction)
            .map_err(|e| spv_patcher::PatcherError::Internal(e.into()))?;

        let new_function_id = self
            .write_new_function(spv_mod, sig)
            .map_err(|e| spv_patcher::PatcherError::Internal(e.into()))?;
        self.rewrite_function_ids(module, new_function_id)
            .map_err(|e| spv_patcher::PatcherError::Internal(e.into()))?;
        self.verify_return(&spv_mod)
            .map_err(|e| spv_patcher::PatcherError::Internal(e.into()))?;

        Ok(patcher)
    }
}
