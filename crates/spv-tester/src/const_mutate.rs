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

    ///Patches
    pub fn patch_i32(&mut self, from: u32, to: u32) -> Result<Vec<u32>, PatcherError> {
        let patch_builder = self.module.patch()?;

        let result = patch_builder
            //.print()
            .patch(MutateConstant::new(from, to))
            .unwrap()
            //.print()
            .assemble();

        Ok(result)
    }

    pub fn patch_f32(&mut self, from: f32, to: f32) {}
}
