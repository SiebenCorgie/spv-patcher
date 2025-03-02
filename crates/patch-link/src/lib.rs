//! A [spirv-linker](https://crates.io/crates/spirv-linker) based linker patch.
//!
//! Contains an auxilary patch, that lets you find function based on attributes
//! and make them be _imported_.
//!

pub use patch_function::rspirv::dr::Module;
pub use patch_function::FuncIdent;
use patch_function::{
    rspirv::{
        dr::{Instruction, Operand},
        spirv::{Decoration, LinkageType, Op},
    },
    FunctionFinder,
};
use spv_patcher::{patch::Patch, PatcherError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LinkError {
    #[error(transparent)]
    LinkerLibError(spirv_linker::LinkerError),
    #[error("Found multiple ({0}) candidates to be marked as _import_.")]
    MultipleCandidates(usize),
    #[error("No candidate found to mark as import")]
    NoCandidate,
    #[error("Function {id} was already marked {lty:?}")]
    WasMarked { id: u32, lty: LinkageType },
    #[error("Function candidate did not exist in function-section of the module")]
    CandidateInvalid,
}

#[derive(Clone)]
pub struct Linker {
    //The module that is being linked
    pub linkage_module: Module,
    ///True, if the resulting module should / could be a library.
    pub is_result_library: bool,
}

impl Linker {
    pub fn new_for_bytes(byte: &[u8]) -> Self {
        Linker {
            linkage_module: spv_patcher::rspirv::dr::load_bytes(byte).unwrap(),
            is_result_library: false,
        }
    }
}

impl Patch for Linker {
    fn apply<'a>(
        self,
        patcher: spv_patcher::patch::Patcher<'a>,
    ) -> Result<spv_patcher::patch::Patcher<'a>, spv_patcher::PatcherError> {
        /* TODO: Write our own linker, cause all the other linkers seem
                 to be broken as well :((
        use patch_function::rspirv::binary::Assemble;
        let new_module = {
            let spvmodule = patcher.ir_state.as_spirv();


            let spvmod = spvmodule.assemble();
            let as_byte: Vec<u8> = bytemuck::cast_slice(&spvmod).to_vec();
            std::fs::write("src_module.spv", as_byte);

            let spvmod = self.linkage_module.assemble();
            let as_byte: Vec<u8> = bytemuck::cast_slice(&spvmod).to_vec();
            std::fs::write("link_module.spv", as_byte);

          /*
            //just call the linker, assuming there is something to link
            let new_module = spirv_linker::link(
                &mut [spvmodule, &mut self.linkage_module],
                &spirv_linker::Options {
                    lib: self.is_result_library,
                    partial: true,
                },
            )
            .map_err(|e| PatcherError::Internal(e.into()))?;
            */
            patch_function::rspirv::dr::from_file("out.spv").unwrap();
        };

        //now overwrite
        *patcher.ir_state.as_spirv() = new_module;
        */
        Ok(patcher)
    }
}

///Patch that makes a function be declared as _import_ instead of being defined. Potentually
/// removes the definition of this function.
pub struct MakeFunctionImport {
    ///The name that is used on the linkage attribute
    pub name: String,
    pub identify_by: FuncIdent,
}

impl Patch for MakeFunctionImport {
    fn apply<'a>(
        self,
        mut patcher: spv_patcher::patch::Patcher<'a>,
    ) -> Result<spv_patcher::patch::Patcher<'a>, PatcherError> {
        let spv = patcher.ir_state.as_spirv();
        let mut candidates = FunctionFinder::find(&spv, &self.identify_by);

        let fdef = match candidates.len() {
            0 => return Err(PatcherError::Internal(LinkError::NoCandidate.into())),
            1 => candidates.remove(0),
            x => {
                return Err(PatcherError::Internal(
                    LinkError::MultipleCandidates(x).into(),
                ))
            }
        };

        //find the function in the functions list
        let function_index = {
            let mut found = None;
            for (fidx, func) in spv.functions.iter().enumerate() {
                if func.def.as_ref().unwrap() == &fdef {
                    found = Some(fidx);
                    break;
                }
            }

            if let Some(f) = found {
                f
            } else {
                return Err(PatcherError::Internal(LinkError::CandidateInvalid.into()));
            }
        };

        //now mark that as _import_ and remove its block
        //However, make sure first that it isn't yet declared as import
        for ann in &spv.annotations {
            match (
                ann.class.opcode,
                ann.operands.get(0),
                ann.operands.get(1),
                ann.operands.get(2),
                ann.operands.get(3),
            ) {
                (
                    Op::Decorate,
                    Some(Operand::IdRef(fid)),
                    Some(Operand::Decoration(Decoration::LinkageAttributes)),
                    Some(Operand::LiteralString(_s)),
                    Some(Operand::LinkageType(lty)),
                ) => {
                    if *fid == fdef.result_id.unwrap() {
                        return Err(PatcherError::Internal(
                            LinkError::WasMarked {
                                id: *fid,
                                lty: lty.clone(),
                            }
                            .into(),
                        ));
                    }
                }
                _ => {}
            }
        }

        //Alright, add decoration an remove function's blocks
        spv.annotations.push(Instruction::new(
            Op::Decorate,
            fdef.result_id.clone(),
            None,
            vec![
                Operand::Decoration(Decoration::LinkageAttributes),
                Operand::LiteralString(self.name),
                Operand::LinkageType(LinkageType::Import),
            ],
        ));

        let mut fdef = spv.functions.remove(function_index);
        fdef.blocks.clear();
        //Add to the front _cause the specs wants that_
        spv.functions.insert(0, fdef);
        //spv.functions[function_index].end = None;
        //spv.functions[function_index].parameters.clear();

        //As a touchup, if the module does not contain the link-capability, add it
        let mut is_linkable = false;
        for cap in &spv.capabilities {
            match (cap.class.opcode, cap.operands.get(0)) {
                (
                    Op::Capability,
                    Some(Operand::Capability(spv_patcher::rspirv::spirv::Capability::Linkage)),
                ) => {
                    is_linkable = true;
                }
                _ => {}
            }
        }

        if !is_linkable {
            log::warn!("Source module had no linking capability. Adding it...");
            spv.capabilities.push(Instruction::new(
                Op::Capability,
                None,
                None,
                vec![Operand::Capability(
                    spv_patcher::rspirv::spirv::Capability::Linkage,
                )],
            ));
        }

        Ok(patcher)
    }
}
