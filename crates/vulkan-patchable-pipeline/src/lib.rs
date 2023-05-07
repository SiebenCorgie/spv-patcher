//! Simple utility crate we use in the tester and benchmarking application to load a base
//! (Compute) shader module that can be patched at runtime.

use marpii::{
    ash::vk,
    resources::{ComputePipeline, ShaderModule, ShaderStage},
    MarpiiError,
};
use marpii_rmg::Rmg;
use spv_patcher::{
    patch::Patcher, rspirv::spirv::ExecutionModel, spirv_ext::SpirvExt, Module, PatcherError,
};
pub struct PatchablePipeline {
    module: Module,
    pipeline: Arc<ComputePipeline>,
}
use std::sync::Arc;

impl PatchablePipeline {
    pub fn from_spirv(spirv_binary: Vec<u8>, rmg: &mut Rmg) -> Result<Self, PatcherError> {
        let module = Module::new(spirv_binary)?;

        assert!(
            module.spirv().get_execution_model() == ExecutionModel::GLCompute,
            "Test Pipeline was not GLCompute ExecutionModel, was {:?}",
            module.spirv().get_execution_model()
        );

        //Load the pipeline assuming its using the bindless layout of RMG.
        // NOTE: Later on this would actually be patchabel with the interface matching pass.
        //No additional descriptors for us
        let layout = rmg.resources.bindless_layout();
        let shader_module =
            ShaderModule::new_from_bytes(&rmg.ctx.device, module.template_code()).unwrap();
        let shader_stage = ShaderStage::from_module(
            shader_module.into(),
            vk::ShaderStageFlags::COMPUTE,
            "main".to_owned(),
        );

        let pipeline = Arc::new(
            ComputePipeline::new(&rmg.ctx.device, &shader_stage, None, layout)
                .map_err(|e| MarpiiError::from(e))
                .unwrap(),
        );

        Ok(PatchablePipeline { module, pipeline })
    }

    pub fn get_pipeline(&self) -> Arc<ComputePipeline> {
        self.pipeline.clone()
    }

    ///Applies `patching` to the template code and builds a new pipeline based on that. Panics if the resulting code is invalid.
    pub fn patch_pipeline(
        &mut self,
        rmg: &mut Rmg,
        patching: impl FnOnce(Patcher) -> Result<Patcher, PatcherError>,
    ) -> Result<(), PatcherError> {
        let patched = patching(self.module.patch())?.assemble();
        let patched_module =
            ShaderModule::new_from_bytes(&rmg.ctx.device, bytemuck::cast_slice(&patched)).unwrap();

        let layout = rmg.resources.bindless_layout();
        let shader_stage = ShaderStage::from_module(
            patched_module.into(),
            vk::ShaderStageFlags::COMPUTE,
            "main".to_owned(),
        );

        let pipeline = Arc::new(
            ComputePipeline::new(&rmg.ctx.device, &shader_stage, None, layout)
                .map_err(|e| MarpiiError::from(e))
                .unwrap(),
        );

        self.pipeline = pipeline;

        Ok(())
    }

    pub fn get_module(&self) -> &Module {
        &self.module
    }
}
