use marpii::{ash::vk, resources::PushConstant};
use marpii_rmg::{recorder::task::MetaTask, BufferHandle, Rmg, Task};
use marpii_rmg_shared::ResourceHandle;
use marpii_rmg_tasks::{DownloadBuffer, DownloadError, DynamicBuffer, NoTaskError, TaskError};

use bytemuck::Pod;
use spv_patcher::spirt::Module;
use vulkan_patchable_pipeline::PatchablePipeline;

use crate::tests::TestError;

#[repr(C, align(16))]
struct ComputePush {
    src: ResourceHandle,
    dst: ResourceHandle,
    buffer_size: u32,
    //Padding for full 16byte just to be sure
    pad0: u32,
}

///Inner compute task of `SimpleComputeTask`, uses the current pipeline to schedule a single run.
struct ComputeTask<T: Pod> {
    pipeline: PatchablePipeline,
    push: PushConstant<ComputePush>,
    src_buffer: BufferHandle<T>,
    dst_buffer: BufferHandle<T>,
}

impl<T: Pod> Task for ComputeTask<T> {
    fn name(&self) -> &'static str {
        "SimpleComputeTask"
    }
    fn queue_flags(&self) -> vk::QueueFlags {
        vk::QueueFlags::COMPUTE
    }
    fn register(&self, registry: &mut marpii_rmg::ResourceRegistry) {
        registry
            .request_buffer(
                &self.src_buffer,
                vk::PipelineStageFlags2::COMPUTE_SHADER,
                vk::AccessFlags2::SHADER_STORAGE_READ,
            )
            .unwrap();
        registry
            .request_buffer(
                &self.dst_buffer,
                vk::PipelineStageFlags2::COMPUTE_SHADER,
                vk::AccessFlags2::SHADER_STORAGE_WRITE,
            )
            .unwrap();
        registry.register_asset(self.pipeline.get_pipeline().clone());
    }
    fn pre_record(
        &mut self,
        resources: &mut marpii_rmg::Resources,
        ctx: &marpii_rmg::CtxRmg,
    ) -> Result<(), marpii_rmg::RecordError> {
        self.push.get_content_mut().src =
            resources.resource_handle_or_bind(&self.src_buffer).unwrap();

        self.push.get_content_mut().dst =
            resources.resource_handle_or_bind(&self.dst_buffer).unwrap();

        Ok(())
    }

    fn record(
        &mut self,
        device: &std::sync::Arc<marpii::context::Device>,
        command_buffer: &vk::CommandBuffer,
        resources: &marpii_rmg::Resources,
    ) {
        //bind commandbuffer, setup push constant and execute

        //NOTE we always start workgroups of size 64, therefore scale dispatch by 64
        let dispatch_size = (self.src_buffer.count() as f32 / 64.0).ceil() as u32;
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
                .cmd_dispatch(*command_buffer, dispatch_size, 1, 1);
        }
    }
}

/// A simple compute task that pushes a given buffer to the gpu,
/// executes the set pipeline, and reads back the buffer.
pub struct SimpleComputeTask<T: Pod> {
    //The upload direction
    upload_buffer: DynamicBuffer<T>,
    //Download direction
    download_buffer: DownloadBuffer<T>,
    //Actual compute pass
    pass: ComputeTask<T>,
}

impl<T: Pod> SimpleComputeTask<T> {
    pub fn new(rmg: &mut Rmg, spirv_bytes: Vec<u8>, initial_data: &[T]) -> Result<Self, TestError> {
        let upload_buffer =
            DynamicBuffer::new(rmg, initial_data).map_err(|e| TestError::TaskError(e))?;
        let download_buffer_hdl = {
            let mut desc = upload_buffer.buffer_handle().buf_desc().clone();
            desc = desc.add_usage(vk::BufferUsageFlags::TRANSFER_SRC);
            rmg.new_buffer_uninitialized(desc, Some("DownloadBuffer"))
                .map_err(|e| TestError::RmgError(e))?
        };
        let download_buffer = DownloadBuffer::new(rmg, download_buffer_hdl.clone())
            .map_err(|e| TestError::TaskError(TaskError::Task(NoTaskError)))?;

        let pipeline = PatchablePipeline::from_spirv(spirv_bytes, rmg)
            .map_err(|e| TestError::PatchError(e))?;
        let push = PushConstant::new(
            ComputePush {
                src: ResourceHandle::INVALID,
                dst: ResourceHandle::INVALID,
                buffer_size: initial_data.len() as u32,
                pad0: 0,
            },
            vk::ShaderStageFlags::COMPUTE,
        );
        let pass = ComputeTask {
            pipeline,
            push,
            src_buffer: upload_buffer.buffer_handle().clone(),
            dst_buffer: download_buffer_hdl.clone(),
        };

        Ok(SimpleComputeTask {
            upload_buffer,
            download_buffer,
            pass,
        })
    }

    ///Writes the data to GPU
    pub fn set_data(&mut self, data: &[T]) {
        self.upload_buffer.write(data, 0);
    }

    ///Reads the (possibly) processed data from the GPU.
    pub fn get_data(
        &self,
        rmg: &mut Rmg,
        read_to: &mut [T],
    ) -> Result<usize, TaskError<DownloadError>> {
        self.download_buffer.download(rmg, read_to)
    }

    pub fn get_pipeline(&mut self) -> &mut PatchablePipeline {
        &mut self.pass.pipeline
    }
}

impl<T: Pod> MetaTask for SimpleComputeTask<T> {
    fn record<'a>(
        &'a mut self,
        recorder: marpii_rmg::Recorder<'a>,
    ) -> Result<marpii_rmg::Recorder<'a>, marpii_rmg::RecordError> {
        recorder
            .add_task(&mut self.upload_buffer)?
            .add_task(&mut self.pass)?
            .add_task(&mut self.download_buffer)
    }
}
