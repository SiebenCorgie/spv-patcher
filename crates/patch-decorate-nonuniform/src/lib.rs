//! # NonUniform Decoration patching
//!
//! ## References
//!
//! For the implementation we have the following references
//!
//! SPIR-V Spec: https://registry.khronos.org/SPIR-V/specs/unified1/SPIRV.html#_uniformity
//! Vulkan EXT: https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VK_EXT_descriptor_indexing.html
//! SPIR-V EXT: https://github.com/KhronosGroup/SPIRV-Registry/blob/main/extensions/EXT/SPV_EXT_descriptor_indexing.asciidoc
//!
//! ## Implementation details
//!
//! - Pass 1:
//!   - Seed all non-uniform variables (varying input variables actually). For simplicity we just tag all input variables
//! - Pass 2:
//!   - Trace seeded variables, any ID that depends on a seeded variable is recursively tagged `possibly-non-uniform`
//!   - Find all AccessChains and depending loads, if also tagged non-uniform, decorate as such.
//!   - Possibly write down accessed chain type to add right capabilities later.
//! - Pass 3:
//!   - If non-uniformity found, and ext not already there, add nonuniform ext to module.
//!   - After analysis, if non-uniformity was found, add Capabilities if not already there.

#![deny(warnings)]

use ahash::AHashMap;
use smallvec::SmallVec;
use spv_patcher::{
    patch::Patch,
    rspirv::{
        dr::{Instruction, Module, Operand},
        spirv::{Capability, Decoration, Op, StorageClass},
    },
    spirv_ext::SpirvExt,
};

#[allow(dead_code)]
struct IndexedType {
    buffer: bool,
    storage_image: bool,
    sampled_image: bool,
    sampler: bool,
}

impl Default for IndexedType {
    fn default() -> Self {
        IndexedType {
            buffer: false,
            storage_image: false,
            sampled_image: false,
            sampler: false,
        }
    }
}

///Analyses control flow and patches `NonUniform` decoration wherever it applies.
///
/// This basically fixes misstakes where `nonuniformEXT(i)` was forgotton when indexing into a descriptor-set-array.
/// Those bugs are hard to find and sometime subtile.
pub struct NonUniformDecorate {
    seeded_variables: AHashMap<u32, Instruction>,
    // Usually this indicates a bindless setup, which in turn needs usually around 5 runtime arrays
    runtime_arrays: SmallVec<[Instruction; 5]>,
    //AccessChain-type instructions we need to decorate.
    to_decorate: SmallVec<[Instruction; 5]>,
    #[allow(dead_code)]
    indexed_type: IndexedType,
}

impl NonUniformDecorate {
    pub fn new() -> Self {
        NonUniformDecorate {
            seeded_variables: AHashMap::default(),
            runtime_arrays: SmallVec::new(),
            to_decorate: SmallVec::new(),
            indexed_type: IndexedType::default(),
        }
    }

    fn seed_variables(&mut self, spirv: &Module) {
        //Seeding pass. This is any input variable.
        for inst in spirv.global_inst_iter() {
            match inst.class.opcode {
                Op::Variable => {
                    //Found a variable, check if it is tagged, and if so, with what.
                    // We are pretty conservative, basically seeding everything where we are not absolutely sure its constant
                    // for the whole invocation.

                    if let Some(operand) = inst.operands.get(0) {
                        match operand {
                            Operand::StorageClass(class) => {
                                match class {
                                    StorageClass::Uniform | StorageClass::PushConstant => {} //those are inputs, but uniform
                                    _ => {
                                        self.seeded_variables
                                            .insert(inst.result_id.unwrap(), inst.clone());
                                        log::info!(
                                            "Adding Variable {} as possibly non-uniform",
                                            inst.result_id.unwrap()
                                        );
                                    }
                                }
                            }
                            _ => log::warn!(
                                "Found non-storage-class operand on OpVariable. Was: {:?}",
                                operand
                            ),
                        }
                    }
                }
                Op::TypeRuntimeArray => {
                    //Found a runtime array definition.
                    // Those are the one our non-uniform decoration might origin
                    // from, collect those as well.
                    log::info!("{} is runtime array", inst.result_id.unwrap());
                    self.runtime_arrays.push(inst.clone());
                }
                _ => {}
            }
        }

        //now forward seed all ids that depend on a already seeded instruction.
        //Do this in a loop until nothing changes, basically fix-point?

        //NOTE thats currently kinda slow since it scales with the
        // trace_queue length (at worst). Consider doing backwards tracing from the
        // AccessChain
        'forward_seeding: loop {
            let mut changed = false;

            for inst in spirv.all_inst_iter() {
                if let Some(resid) = inst.result_id {
                    //check if this instruction's operands are any already seeded variables
                    'uses_search: for op in inst.operands.iter() {
                        if let Some(refs) = op.id_ref_any() {
                            if self.seeded_variables.contains_key(&refs) {
                                log::info!("Forward seeding {}", resid);
                                let was_present =
                                    self.seeded_variables.insert(resid, inst.clone()).is_some();
                                //Only mark if the id wasn't already in there
                                if !was_present {
                                    changed = true;
                                }
                                break 'uses_search;
                            }
                        }
                    }
                } else {
                    //In this case we might have a  OpStore that could change a
                    // assignment. All the others (OpBranch, OpLine etc.) are not interesting afaik.
                    //
                    // TODO: Check that we are actually catching the branches and function calls
                    match inst.class.opcode {
                        Op::Store => {
                            //Check if op_store's src id is in
                            if let (Some(dst_id), Some(src_id)) =
                                (inst.operands[0].id_ref_any(), inst.operands[1].id_ref_any())
                            {
                                if self.seeded_variables.contains_key(&src_id) {
                                    let was_present = self
                                        .seeded_variables
                                        .insert(dst_id, inst.clone())
                                        .is_some();
                                    //Only mark if the id wasn't already in there
                                    if !was_present {
                                        changed = true;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }

            if !changed {
                break 'forward_seeding;
            }
        }
    }

    fn trace_indices(&mut self, spirv: &Module) {
        //Find all AccessChain OPS, then trace the index variable. If it ever crosses an already seeded variable,
        // the whole access must be nonuniform.

        //Tracing works by finding all access-chain ops where the access-ed chain is
        // a runtime-array and the index is already tagged as possibly non-uniform.

        //To find accessed runtime arrays, we use the already seeded OpTypeRuntimeArray instructions
        // and search for all type definitions where the RuntimeArray is used as super type.
        //
        // In Rust-Syntax this means finding every T for a type deceleration RuntimeArray<T>
        //
        // Then we tag every variable with that type. We therfore get any variable of type RuntimeArray<T>

        //Search all TypeDefs
        let mut type_ids = SmallVec::<[u32; 5]>::new();
        let runtime_ids: SmallVec<[u32; 5]> = self
            .runtime_arrays
            .iter()
            .filter_map(|inst| inst.result_id)
            .collect();
        for inst in spirv.types_global_values.iter() {
            //NOTE: Currently just testing each type-def's
            //      operands for our IDs.
            //      We could also match the OpTypeXY's but those can change
            //      in the future.

            if let Some(resid) = inst.result_id {
                for operand in inst.operands.iter() {
                    if let Some(operand_id) = operand.id_ref_any() {
                        if runtime_ids.contains(&operand_id) {
                            log::info!("{} is RuntimeArrayBased!", resid);
                            type_ids.push(resid);
                        }
                    }
                }
            }
        }

        //Now find all variables with one of those types.
        // NOTE: Since those must be bound to a descriptor set (can't construct RuntimeArrays otherwise)
        //       it is enough to search the header
        let mut variable_ids = SmallVec::<[u32; 5]>::new();
        for inst in spirv.global_inst_iter() {
            match inst.class.opcode {
                Op::Variable => {
                    if let Some(type_id) = inst.result_type {
                        if type_ids.contains(&type_id) {
                            log::info!("{} is runtime array variable!", type_id);
                            variable_ids.push(inst.result_id.unwrap());
                        }
                    }
                }
                _ => {}
            }
        }

        //Now we got the IDs of all variables with a RuntimeArray based type. Which means we can do
        // the intersection with non-uniform based
        // indices.

        for inst in spirv.all_inst_iter() {
            match inst.class.opcode {
                //TODO: test for AccessChainBound and PtrAccessChain as well?
                Op::AccessChain => {
                    //NOTE: The specification: https://registry.khronos.org/SPIR-V/specs/unified1/SPIRV.html#OpAccessChain
                    //      tells us that `base` must be the pointer to the source
                    //      composite object. In our case this means this must be the variable to
                    //      our RuntimeArray<T>.
                    if let Some(base_pointer) = inst.operands[0].id_ref_any() {
                        if variable_ids.contains(&base_pointer) {
                            //We are in a RuntimeArray access
                            log::info!("Accessing runtime array!");

                            //Check if any of the following (index) operands is a possibly non-uniform index.
                            for operand_idx in 1..inst.operands.len() {
                                if let Some(operand_id) = inst.operands[operand_idx].id_ref_any() {
                                    if self.seeded_variables.contains_key(&operand_id) {
                                        log::info!(
                                            "{:?} must be decorated because of {:?}",
                                            inst.result_id,
                                            operand_id
                                        );
                                        self.to_decorate.push(inst.clone());
                                    }
                                }
                            }
                        } /*else {
                              log::info!("Access, but no runtime array: {}", base_pointer);
                          }*/
                    } else {
                        log::warn!("No base pointer on access_chain!");
                    }
                }
                _ => {}
            }
        }

        //NOTE: Function boundaries: We currently only tag the *outcome* of function calls as possibly-NonUniform
        //TODO: However, if there is a Array-Access in a non-inlined function we might need to add NonUniform as well.
    }

    //Does the actual decoration of any previously found non-uniform access
    fn decorate(&self, spirv: &mut Module) {
        //For decoration we have three possible ids that need to be decorated.
        // 1. The base pointer in access_chain (always)
        // 2. The nonuniform index used in access chain (always)
        // 3. Any OpLoad where the result id of access_chain is used.
        //    After that any code is invocation-uniform since only the *content* of the id.
        //    Is non-uniform, but access is the same.

        //First do the easy part and decorate the base pointer and the index
        let mut search_load_ids = SmallVec::<[u32; 5]>::new();
        for dec in &self.to_decorate {
            spirv.decorate(dec.result_id.unwrap(), Decoration::NonUniform);
            log::info!("decorate {}", dec.result_id.unwrap());
            search_load_ids.push(dec.result_id.unwrap());
            for operands in dec.operands.iter() {
                if let Some(idref) = operands.id_ref_any() {
                    if self.seeded_variables.contains_key(&idref) {
                        log::info!("decorate {}", idref);
                        spirv.decorate(idref, Decoration::NonUniform);
                    }
                }
            }
        }

        let mut post_decorate = SmallVec::<[u32; 5]>::new();
        for inst in spirv.all_inst_iter() {
            match inst.class.opcode {
                Op::Load => {
                    if search_load_ids.contains(&inst.operands[0].unwrap_id_ref()) {
                        post_decorate.push(inst.result_id.unwrap());
                    }
                }
                Op::Store => {
                    if search_load_ids.contains(&inst.operands[0].unwrap_id_ref()) {
                        post_decorate.push(inst.operands[1].unwrap_id_ref());
                    }
                }
                //NOTE: I think we don't need to mark any else since
                //      all *users* must load the pointer anyways. After
                //      that point the *loaded* id is uniform again.
                _ => {}
            }
        }

        for post in post_decorate {
            log::info!("Post decorate: {}", post);
            spirv.decorate(post, Decoration::NonUniform)
        }
    }

    ///Fixes all capabilities and extensions needed, if there was non-uniform access added.
    fn fix_capabilities_and_extensions(&self, spirv: &mut Module) {
        if self.to_decorate.is_empty() {
            return;
        }

        //Add extension if not present
        if !spirv.has_extension("SPV_EXT_descriptor_indexing") {
            spirv.extensions.push(Instruction::new(
                Op::Extension,
                None,
                None,
                vec![Operand::LiteralString(String::from(
                    "SPV_EXT_descriptor_indexing",
                ))],
            ))
        }

        //For now, push all NonUniform capabilities. However, later we might want to track the non-uniformly accessed
        // types instead
        spirv.add_capability(Capability::ShaderNonUniform);
        spirv.add_capability(Capability::RuntimeDescriptorArray);
        //spirv.add_capability(Capability::InputAttachmentArrayDynamicIndexing);
        //spirv.add_capability(Capability::UniformTexelBufferArrayDynamicIndexing);
        //spirv.add_capability(Capability::StorageTexelBufferArrayDynamicIndexing);
        //spirv.add_capability(Capability::UniformBufferArrayNonUniformIndexing);
        spirv.add_capability(Capability::SampledImageArrayNonUniformIndexing);
        spirv.add_capability(Capability::StorageBufferArrayNonUniformIndexing);
        spirv.add_capability(Capability::StorageImageArrayNonUniformIndexing);
        //spirv.add_capability(Capability::InputAttachmentArrayNonUniformIndexing);
        //spirv.add_capability(Capability::UniformTexelBufferArrayNonUniformIndexing);
        //spirv.add_capability(Capability::StorageTexelBufferArrayNonUniformIndexing);
    }
}

impl Patch for NonUniformDecorate {
    fn apply<'a>(
        mut self,
        mut patcher: spv_patcher::patch::Patcher<'a>,
    ) -> Result<spv_patcher::patch::Patcher<'a>, spv_patcher::PatcherError> {
        let spirv = patcher.ir_state.as_spirv();

        //Pass 1: Seeding non-uniform ids
        self.seed_variables(spirv);
        //Pass 2:
        self.trace_indices(spirv);
        self.decorate(spirv);
        //Pass 3: Fixing up capabilities and extensions
        self.fix_capabilities_and_extensions(spirv);

        //Should be finished at that point.
        Ok(patcher)
    }
}
