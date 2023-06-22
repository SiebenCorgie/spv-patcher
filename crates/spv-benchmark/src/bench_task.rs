use marpii::{ash::vk, resources::PushConstant};
use marpii_rmg::{BufferHandle, Rmg, Task};
use vulkan_patchable_pipeline::PatchablePipeline;

///Simple compute task that can use a set of read and write buffers of type `T`
/// and a push-constant of type `P`.
///
/// Use [on_record] to setup push-constant content before draw.
///
/// ///Similar to the Test's compute task implementation. But this one safes how long the
/// GPU took.

pub struct ComputeTask<P: 'static, T: 'static> {
    pub pipeline: PatchablePipeline,
    pub push: PushConstant<P>,
    pub read_buffers: Vec<BufferHandle<T>>,
    pub write_buffers: Vec<BufferHandle<T>>,
    pub resolution: [u32; 2],
    pub on_record:
        Box<dyn Fn(&mut PushConstant<P>, &mut marpii_rmg::Resources, &marpii_rmg::CtxRmg)>,
}

impl<P: 'static, T: 'static> Task for ComputeTask<P, T> {
    fn name(&self) -> &'static str {
        "BenchComputeTask"
    }
    fn queue_flags(&self) -> vk::QueueFlags {
        vk::QueueFlags::COMPUTE
    }
    fn register(&self, registry: &mut marpii_rmg::ResourceRegistry) {
        for rb in &self.read_buffers {
            registry
                .request_buffer(
                    rb,
                    vk::PipelineStageFlags2::COMPUTE_SHADER,
                    vk::AccessFlags2::SHADER_STORAGE_READ,
                )
                .unwrap();
        }

        for wb in &self.write_buffers {
            registry
                .request_buffer(
                    wb,
                    vk::PipelineStageFlags2::COMPUTE_SHADER,
                    vk::AccessFlags2::SHADER_STORAGE_WRITE,
                )
                .unwrap();
        }

        registry.register_asset(self.pipeline.get_pipeline().clone());
    }
    fn pre_record(
        &mut self,
        resources: &mut marpii_rmg::Resources,
        ctx: &marpii_rmg::CtxRmg,
    ) -> Result<(), marpii_rmg::RecordError> {
        (self.on_record)(&mut self.push, resources, ctx);
        /*
                self.push.get_content_mut().src =
                    resources.resource_handle_or_bind(&self.src_buffer).unwrap();

                self.push.get_content_mut().dst =
                    resources.resource_handle_or_bind(&self.dst_buffer).unwrap();
        */
        Ok(())
    }

    fn record(
        &mut self,
        device: &std::sync::Arc<marpii::context::Device>,
        command_buffer: &vk::CommandBuffer,
        _resources: &marpii_rmg::Resources,
    ) {
        //bind commandbuffer, setup push constant and execute

        //NOTE we always start workgroups of size 8x8, therefore scale dispatch by 64
        let dispatch_x = (self.resolution[0] as f32 / 8.0).ceil() as u32;
        let dispatch_y = (self.resolution[1] as f32 / 8.0).ceil() as u32;
        unsafe {
            device.inner.cmd_bind_pipeline(
                *command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                self.pipeline.get_pipeline().pipeline,
            );
            device.inner.cmd_push_constants(
                *command_buffer,
                self.pipeline.get_pipeline().layout.layout,
                vk::ShaderStageFlags::ALL,
                0,
                self.push.content_as_bytes(),
            );

            device
                .inner
                .cmd_dispatch(*command_buffer, dispatch_x, dispatch_y, 1);
        }
    }
}

impl<P: Default + 'static, T: 'static> ComputeTask<P, T> {
    pub fn new(
        rmg: &mut Rmg,
        spirv_bytes: Vec<u8>,
        read_buffers: Vec<BufferHandle<T>>,
        write_buffers: Vec<BufferHandle<T>>,
        resolution: [u32; 2],
        on_record: impl Fn(&mut PushConstant<P>, &mut marpii_rmg::Resources, &marpii_rmg::CtxRmg)
            + 'static,
    ) -> Self {
        let pipeline = PatchablePipeline::from_spirv(spirv_bytes, rmg).unwrap();
        let push = PushConstant::new(P::default(), vk::ShaderStageFlags::COMPUTE);

        let on_record = Box::new(on_record);

        ComputeTask {
            pipeline,
            push,
            read_buffers,
            write_buffers,
            resolution,
            on_record,
        }
    }
}
