use spv_patcher::{patch::MutateConstant, Module, PatcherError};

const CONST_MUTATE_SPV: &'static [u8] = include_bytes!("../resources/compute_add.spv");

pub struct ConstMutateTest {
    module: Module,
}

impl ConstMutateTest {
    //loads the shader
    pub fn load() -> Result<Self, PatcherError> {
        let module = Module::new(CONST_MUTATE_SPV.to_vec())?;

        Ok(ConstMutateTest { module })
    }

    pub fn patch_i32(&mut self, from: i32, to: i32) -> Result<(), PatcherError> {
        let mut patch_builder = self.module.patch()?;

        patch_builder = patch_builder
            //.print()
            .patch(MutateConstant::new(1, 42))
            .unwrap()
            //.print()
            ;

        Ok(())
    }

    pub fn patch_f32(&mut self, from: f32, to: f32) {}
}
