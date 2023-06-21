use std::io::{BufWriter, Write};

use ahash::AHashMap;
use spv_patcher::{
    patch::Patch,
    rspirv::{
        binary::Assemble,
        dr::{Instruction, Module, Operand},
        spirv::{Capability, Decoration, LinkageType, Op},
    },
    spirv_ext::SpirvExt,
};

use crate::function_finder::FuncSignature;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConstantReplaceError {
    #[error("No function found for index {0} in the replacement module")]
    InvalidReplacementIndex(usize),
    #[error("SPIRV builder error: {0}")]
    BuilderError(#[from] spv_patcher::rspirv::dr::Error),
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
}

///Declares a *whole* spirv module and a function index into the module that will replace a function with the same identification
/// within a module that is being patched.
#[derive(Clone)]
pub struct ConstantReplace {
    ///Module from which the replacement code is taken
    pub replacement_module: Module,
    ///index into the replacement module's function vector which selects the function that is being replaced.
    pub replacement_index: usize,

    ///If true, the temporary, linkable shader files are kept after linking
    pub keep_temporary_files: bool,

    ///function ident of the function that is replaced in the `replacement_module` context.
    ident: FuncSignature,
}

impl ConstantReplace {
    pub fn new(
        replacement_module: Module,
        replacement_index: usize,
    ) -> Result<Self, ConstantReplaceError> {
        let ident = {
            let f = replacement_module.functions.get(replacement_index).ok_or(
                ConstantReplaceError::InvalidReplacementIndex(replacement_index),
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

        Ok(ConstantReplace {
            replacement_index,
            replacement_module,
            keep_temporary_files: false,
            ident,
        })
    }

    //Merge
    pub fn allow_linking(&self, dst: &mut Module) {
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
                log::info!("dec: {:#?}", dec);

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
            return;
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

        'function_def_iter: for f in dst.functions.iter() {
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
                        return;
                    }
                }

                //if no continue was called up to this point, decorate this function with the correct call
                log::info!("Found matching function in dst");

                if !dst.has_extension("Linkage") {
                    //if needed, add linkage attrib
                    dst.extensions.push(Instruction::new(
                        Op::Capability,
                        None,
                        None,
                        vec![Operand::Capability(Capability::Linkage)],
                    ));
                }
                //therefor insert decorations into annotations
                dst.annotations.push(Instruction::new(
                    Op::Decorate,
                    f.def_id(),
                    None,
                    vec![
                        Operand::Decoration(Decoration::LinkageAttributes),
                        Operand::LiteralString(name.clone()),
                        Operand::LinkageType(LinkageType::Import),
                    ],
                ));
            }
        }
    }

    fn call_linker(&self, dst: &mut Module) -> Result<(), ConstantReplaceError> {
        const LINK_DST_FILENAME: &'static str = "dst_linkable.spv";
        const LINK_PATCH_FILENAME: &'static str = "patch_linkable.spv";

        let assembled_dst = dst.assemble();
        let assembled_patch = self.replacement_module.assemble();

        //write both to a file, sadly, the linker can not yet read from stdin.
        let dst_linkable_file = std::fs::OpenOptions::new()
            .create(true)
            .append(false)
            .write(true)
            .open(LINK_DST_FILENAME)?;
        let mut dst_writer = BufWriter::new(dst_linkable_file);
        dst_writer.write(bytemuck::cast_slice(&assembled_dst))?;

        let patch_linkable_file = std::fs::OpenOptions::new()
            .create(true)
            .append(false)
            .write(true)
            .open(LINK_PATCH_FILENAME)?;
        let mut patch_writer = BufWriter::new(patch_linkable_file);
        patch_writer.write(bytemuck::cast_slice(&assembled_patch))?;

        //now try to call the linker

        let out = std::process::Command::new("spirv-link")
            .args([
                "--verify-ids",
                "-o",
                "patched.spv",
                LINK_DST_FILENAME,
                LINK_PATCH_FILENAME,
            ])
            .output();

        match out {
            Ok(r) => log::info!("Successfully linked files: {:?}!", r),
            Err(e) => log::error!("Failed to link files!\n{}", e),
        }

        Ok(())
    }
}

impl Patch for ConstantReplace {
    fn apply<'a>(
        self,
        mut patcher: spv_patcher::patch::Patcher<'a>,
    ) -> Result<spv_patcher::patch::Patcher<'a>, spv_patcher::PatcherError> {
        let spv = patcher.ir_state.as_spirv();

        //Right now we do the following:
        //
        // 1. Rewrite `spv` to allow re-linking the target function.
        // 2. Rewrite the patching function in order to be used by the spirv-linker
        // 3. Execute the spirv-linker binary which should produce the merged/linked binary
        self.allow_linking(spv);

        self.call_linker(spv)
            .map_err(|e| spv_patcher::PatcherError::Internal(e.into()))?;

        Ok(patcher)
    }
}
