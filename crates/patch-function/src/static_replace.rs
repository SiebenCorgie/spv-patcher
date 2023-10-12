use std::rc::Rc;

use ahash::AHashMap;
use spv_patcher::{
    patch::Patch,
    rspirv::{
        binary::Assemble,
        dr::{Instruction, Module, Operand},
        spirv::{Capability, Decoration, LinkageType, Op},
    },
    spirt::{self, Context},
    spirv_ext::SpirvExt,
};

use crate::function_finder::FuncSignature;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum StaticReplaceError {
    #[error("No function found for index {0} in the replacement module")]
    InvalidReplacementIndex(usize),
    #[error("SPIRV builder error: {0}")]
    BuilderError(#[from] spv_patcher::rspirv::dr::Error),
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Could not parse back linked module. This is probably a spirv-link error: {0}")]
    ParseBack(String),
    #[error("Could not merge patch code into SPIR-T module")]
    MergeError(spirt::passes::merge::MergeError),
    #[error("Module already has linking annotation, which is not supported")]
    ExistingLinkingAnnotation,
    #[error("Module has no function that matches the function signature of the replacement_index function.")]
    SignatureMatchError,
}

///Declares a *whole* spirv module and a function index into the module that will replace a function with the same identification
/// within a module that is being patched.
#[derive(Clone)]
pub struct StaticReplace {
    ///Module from which the replacement code is taken
    pub replacement_module: Module,
    ///index into the replacement module's function vector which selects the function that is being replaced.
    pub replacement_index: usize,

    ///function ident of the function that is replaced in the `replacement_module` context.
    ident: FuncSignature,
}

impl StaticReplace {
    pub fn new(
        replacement_module: Module,
        replacement_index: usize,
    ) -> Result<Self, StaticReplaceError> {
        let ident = {
            let f = replacement_module.functions.get(replacement_index).ok_or(
                StaticReplaceError::InvalidReplacementIndex(replacement_index),
            )?;

            let return_type = f.def.as_ref().unwrap().result_type.unwrap();
            let argument_types = f
                .parameters
                .iter()
                .map(|arg| arg.result_type.unwrap())
                .collect();

            FuncSignature {
                return_type,
                argument_types,
            }
        };

        Ok(StaticReplace {
            replacement_index,
            replacement_module,
            ident,
        })
    }

    //Add linking annotation to the dst module. Returns the function-id that
    // was chosen.
    pub fn allow_linking(&self, dst: &mut Module) -> Result<(), StaticReplaceError> {
        //NOTE(siebencorgie): For sanity, verify that we do-not have
        // any linkage annotation at-all in the module.
        //
        // Otherwise the later remove-annotation pass will break that.
        for ann in dst.annotations.iter() {
            if let Operand::Decoration(Decoration::LinkageAttributes) = ann.operands[1] {
                log::error!(
                    "Module already has linkage annotation, before adding it to the module!"
                );
                return Err(StaticReplaceError::ExistingLinkingAnnotation);
            }
        }

        //Pass1: find exported function name
        let name = if let Some(f) = self
            .replacement_module
            .functions
            .get(self.replacement_index)
        {
            assert!(
                f.def.as_ref().unwrap().class.opcode == Op::Function,
                "Def was not a function"
            );

            let mut is_marked_link = false;
            let func_id = f.def.as_ref().unwrap().result_id.unwrap();
            //find decorations for function id, one of which must contain LinkageAttributes

            let mut linkage_name = String::with_capacity(0);
            'linkage_search_loop: for dec in self.replacement_module.annotations.iter() {
                match (dec.operands.get(0), dec.operands.get(1)) {
                    (
                        Some(Operand::IdRef(r)),
                        Some(Operand::Decoration(Decoration::LinkageAttributes)),
                    ) => {
                        if *r == func_id {
                            log::info!("Found linkage attributes for function: {}", func_id);

                            for operand in &dec.operands[2..] {
                                match operand {
                                    Operand::LiteralString(s) => linkage_name = s.clone(),
                                    Operand::LinkageType(LinkageType::Export) => {
                                        is_marked_link = true
                                    }
                                    _ => {}
                                }
                            }
                            break 'linkage_search_loop;
                        }
                    }
                    _ => {}
                }
            }

            if !is_marked_link {
                log::warn!("While we found a linkage function (with name {}), it is not marked *export*. Continuing ...", linkage_name);
            }
            log::info!("Found linkage function: {}", linkage_name);
            linkage_name
        } else {
            log::error!(
                "Could not find {}-th function in replacement module. Linking will fail!",
                self.replacement_index
            );
            return Err(StaticReplaceError::InvalidReplacementIndex(
                self.replacement_index,
            ));
        };

        //Pass2: find a function with the same-ish signature, and add the linkage decoration needed for the linker to
        //       find it.
        //NOTE: We currently just test for a similar-ish signature by building both type trees and then
        //      comparing the parameter types of both.
        //
        //      This is currently not *perfect*. For that we'd merge the patch function into the `dst` module
        //      before linking. That's what the spirv-linker does. However, in our case the target function is not
        //      decorated with linkage attributes which makes that harder.
        //
        //      In the future there might be a SPIR-T based merge pass that would allow that.

        let mut src_arg_ty_table = self
            .ident
            .argument_types
            .iter()
            .map(|i| (*i, None))
            .collect::<AHashMap<u32, Option<u32>>>();

        let mut match_list = Vec::with_capacity(1);

        'function_def_iter: for (fidx, f) in dst.functions.iter().enumerate() {
            if f.parameters.len() == self.ident.argument_types.len() {
                //we do that by pattern matching for now 👀
                // check if type pattern is the same.
                for (dst_f_ty, ident_ty) in
                    f.parameters.iter().zip(self.ident.argument_types.iter())
                {
                    let dst_f_ty = *dst_f_ty.result_type.as_ref().unwrap();
                    if let Some(seen_ty) = src_arg_ty_table.get_mut(ident_ty) {
                        //don't know the mapping atm. push into table
                        if let Some(seen_dst_ty) = seen_ty {
                            if dst_f_ty != *seen_dst_ty {
                                log::error!("We know type={} in the patch module as type={} in the destination module already. However, we tried mapping as type={}.", ident_ty, seen_dst_ty, dst_f_ty);
                                continue 'function_def_iter;
                            }
                        } else {
                            //need to insert seen type
                            *seen_ty = Some(dst_f_ty);
                        }
                    } else {
                        log::error!("src_arg_ty_table was incomplete, Linking will fail!");
                        return Err(StaticReplaceError::InvalidReplacementIndex(
                            self.replacement_index,
                        ));
                    }
                }

                //if no continue was called up to this point, decorate this function with the correct call
                log::info!("Found matching function in dst");

                match_list.push(fidx);
            }
        }

        if match_list.len() == 0 {
            log::error!(
                "Did not find any matching function that could be patched in the source module!"
            );
            return Err(StaticReplaceError::SignatureMatchError);
        }
        if match_list.len() > 1 {
            log::warn!("Found more than one matching function for linking. Using the first one.");
        }

        //Add linkage if needed
        if !dst.has_extension("Linkage") {
            //if needed, add linkage attrib
            dst.extensions.push(Instruction::new(
                Op::Capability,
                None,
                None,
                vec![Operand::Capability(Capability::Linkage)],
            ));
        }

        let def_id = dst.functions[match_list[0]].def_id();
        //Add decoration to function's id
        dst.annotations.push(Instruction::new(
            Op::Decorate,
            def_id,
            None,
            vec![
                Operand::Decoration(Decoration::LinkageAttributes),
                Operand::LiteralString(name.clone()),
                Operand::LinkageType(LinkageType::Import),
            ],
        ));

        //Finally, to keep valid SPIR-V, kill all instructions of function
        // f
        dst.functions[match_list[0]].blocks.clear();
        Ok(())
    }

    fn call_linker(
        &self,
        dst: &mut spirt::Module,
        cx: Rc<Context>,
    ) -> Result<(), StaticReplaceError> {
        //TODO: ATM we lower the patched code each time. If we'd make the
        // code private we could do that in a preprocessing step.
        let spv_bytes = self.replacement_module.assemble();
        let link_code =
            spirt::Module::lower_from_spv_bytes(cx, bytemuck::cast_slice(&spv_bytes).to_vec())?;

        spirt::passes::merge::merge(dst, link_code)
            .map_err(|e| StaticReplaceError::MergeError(e))?;
        spirt::passes::legalize::structurize_func_cfgs(dst);
        spirt::passes::link::resolve_imports(dst);

        Ok(())
    }

    //Removes all added linkage annotation, this includes the
    // Linkage capability as well as the export annotation of the just-linked
    // function.
    //
    // NOTE(siebencorgie): I feel like it should be possible to keep those, but
    // Vulkan (and possibly more) seem to not like that.
    fn remove_linkage_annotation(&self, dst: &mut Module) {
        // TODO(siebencorgie): More robustly remove that annotation (the function id changes
        // in between passes, so caching is no option).
        dst.remove_capability(Capability::Linkage);
        dst.annotations.retain(|ann| {
            if let Operand::Decoration(Decoration::LinkageAttributes) = ann.operands[1] {
                false
            } else {
                true
            }
        })
    }
}

impl Patch for StaticReplace {
    fn apply<'a>(
        self,
        mut patcher: spv_patcher::patch::Patcher<'a>,
    ) -> Result<spv_patcher::patch::Patcher<'a>, spv_patcher::PatcherError> {
        //Right now we do the following:
        //
        // 1. Rewrite `spv` to allow re-linking the target function.
        // 2. Rewrite the patching function in order to be used by the spirv-linker
        // 3. Execute the spirv-linker binary which should produce the merged/linked binary

        //First, add linking annotations to our patcher's module
        {
            let spv = patcher.ir_state.as_spirv();
            self.allow_linking(spv)
                .map_err(|e| spv_patcher::PatcherError::Internal(e.into()))?;
        }

        //Now lower to SPIR-T and call SPIR-T's merge and link pass
        {
            let (spirt, ctx) = patcher.ir_state.as_spirt();
            self.call_linker(spirt, ctx)
                .map_err(|e| spv_patcher::PatcherError::Internal(e.into()))?;
        }

        //finally lift back to spirv and remove linkage annotation
        {
            let spv = patcher.ir_state.as_spirv();
            self.remove_linkage_annotation(spv);
        }
        Ok(patcher)
    }
}
