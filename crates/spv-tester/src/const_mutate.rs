use std::path::Path;

use spv_patcher::{dis_assamble::Module, patch::MutateConstant, Patcher, PatcherError};

const CONST_MUTATE_SPV: &'static [u8] = include_bytes!("../resources/compute_add.spv");

pub struct ConstMutateTest {
    module: Module,
}

impl ConstMutateTest {
    //loads the shader
    pub fn load() -> Result<Self, PatcherError> {
        let patcher = Patcher::new();
        let module = patcher.load_module(CONST_MUTATE_SPV.to_vec())?;

        Ok(ConstMutateTest { module })
    }

    pub fn patch_i32(&mut self, from: i32, to: i32) -> Result<(), PatcherError> {
        let mut patch_builder = self.module.schedule_patches();

        log::trace!("Before: ");

        let ctx = patch_builder.ctx();
        patch_builder = patch_builder
            .print()
            .patch(
                MutateConstant::mut_i32(ctx.as_ref(), from, to)
                    .expect("Could not build MutateConstantPatch"),
            )
            .unwrap()
            .print();

        Ok(())
    }

    pub fn patch_f32(&mut self, from: f32, to: f32) {}
}
