use marpii::resources::PushConstant;
use marpii_rmg_shared::ResourceHandle;
use marpii_rmg_tasks::{DownloadBuffer, UploadBuffer};
use spv_patcher::{
    patch::MutateConstant, rspirv::spirv::ExecutionModel, spirv_ext::SpirvExt, PatcherError,
};

use crate::{compute_task::ComputeTask, test_runs::TestRun};

const CONST_MUTATE_SPV: &'static [u8] = include_bytes!("../resources/compute_add.spv");

#[repr(C, align(16))]
struct ConstMutPush {
    src: ResourceHandle,
    dst: ResourceHandle,
    buffer_size: u32,
    pad0: u32,
}

impl Default for ConstMutPush {
    fn default() -> Self {
        ConstMutPush {
            src: ResourceHandle::INVALID,
            dst: ResourceHandle::INVALID,
            buffer_size: ConstMutateTest::BUFSIZE as u32,
            pad0: 0xdeadbeef,
        }
    }
}

pub struct ConstMutateTest {
    //Represents our GPU site test task
    test_task: ComputeTask<ConstMutPush, u32>,
    src: UploadBuffer<u32>,
    dst: DownloadBuffer<u32>,
}

impl ConstMutateTest {
    const BUFSIZE: usize = 64;
    //loads the shader
    pub fn load(rmg: &mut marpii_rmg::Rmg) -> Result<Self, PatcherError> {
        let initial_data = &[0u32; Self::BUFSIZE];
        let src = UploadBuffer::new(rmg, initial_data).unwrap();
        let src_hdl = src.buffer.clone();
        let dst = DownloadBuffer::new_for(rmg, Self::BUFSIZE).unwrap();
        let dst_hdl = dst.gpu_handle();
        let test_task = ComputeTask::new(
            rmg,
            CONST_MUTATE_SPV.to_vec(),
            vec![src_hdl.clone()],
            vec![dst_hdl.clone()],
            Self::BUFSIZE as u32,
            move |push: &mut PushConstant<ConstMutPush>, resources, _ctx| {
                println!("Writing src and dst, {:?}, {:?}!", src_hdl, dst_hdl);
                push.get_content_mut().src = resources.resource_handle_or_bind(&src_hdl).unwrap();
                push.get_content_mut().dst = resources.resource_handle_or_bind(&dst_hdl).unwrap();
            },
        )
        .unwrap();

        Ok(ConstMutateTest {
            test_task,
            src,
            dst,
        })
    }

    ///Patches
    pub fn patch_i32(
        &mut self,
        rmg: &mut marpii_rmg::Rmg,
        from: u32,
        to: u32,
    ) -> Result<(), PatcherError> {
        assert!(
            self.test_task
                .pipeline
                .get_module()
                .spirv()
                .get_execution_model()
                == ExecutionModel::GLCompute
        );

        self.test_task.pipeline.patch_pipeline(rmg, |patch| {
            patch
                //.print()
                .patch(MutateConstant::Integer { from, to })
        })
    }

    ///Patches
    #[allow(dead_code)]
    pub fn patch_f32(
        &mut self,
        rmg: &mut marpii_rmg::Rmg,
        from: f32,
        to: f32,
    ) -> Result<(), PatcherError> {
        self.test_task.pipeline.patch_pipeline(rmg, |patch| {
            patch
                //.print()
                .patch(MutateConstant::Float { from, to })
        })
    }
}

impl TestRun for ConstMutateTest {
    fn name(&self) -> &'static str {
        "ConstMutateTest"
    }
    fn run(
        &mut self,
        blessing: &mut crate::BlessedDB,
        rmg: &mut marpii_rmg::Rmg,
    ) -> Result<(), crate::test_runs::TestError> {
        //load our pipeline, run it, then patch it and run it again.

        {
            rmg.record()
                .add_task(&mut self.src)
                .unwrap()
                .add_task(&mut self.test_task)
                .unwrap()
                .add_task(&mut self.dst)
                .unwrap()
                .execute()
                .unwrap();

            std::thread::sleep(std::time::Duration::from_secs(1));
            let mut target_buffer = [0u32; Self::BUFSIZE];
            let res = self.dst.download(rmg, &mut target_buffer);
            if let Ok(size) = res {
                if size != Self::BUFSIZE {
                    log::error!("Download size did not match: {} != {}", size, Self::BUFSIZE);
                }

                println!("{:?}", &target_buffer);

                for i in 0..size {
                    assert!(
                        target_buffer[i] == 33,
                        "Should be {}, was {} @ {}",
                        33,
                        target_buffer[i],
                        i
                    );
                }
            } else {
                log::error!("Failed to download buffer!");
            }
        }

        //patch pipeline
        self.patch_i32(rmg, 33, 42).unwrap();

        //If not currently blessing, assert that the code is the same
        if !blessing.bless {
            if let Some(blessed_code) = blessing.blessed_results.get(self.name()) {
                assert!(
                    blessed_code == self.test_task.pipeline.patched_code(),
                    "Const-Mutate patched code is invalid!"
                );
                log::info!("ConstMutate valid!");
            } else {
                log::info!("Nothing to bless for {}", self.name());
            }
        } else {
            log::info!("Blessing!");
        }

        {
            rmg.record()
                //NOTE: don't need to upload again, instead use the same input
                .add_task(&mut self.test_task)
                .unwrap()
                .add_task(&mut self.dst)
                .unwrap()
                .execute()
                .unwrap();

            let mut target_buffer = [0u32; Self::BUFSIZE];
            let res = self.dst.download(rmg, &mut target_buffer);
            if let Ok(size) = res {
                if size != Self::BUFSIZE {
                    log::error!("Download size did not match: {} != {}", size, Self::BUFSIZE);
                }

                for i in 0..size {
                    assert!(
                        target_buffer[i] == 42,
                        "Should be {}, was {}",
                        42,
                        target_buffer[i]
                    );
                }

                //If we are blessing, write this shader code to the db
                if blessing.bless {
                    let _old = blessing.blessed_results.insert(
                        self.name().to_string(),
                        self.test_task.pipeline.patched_code().to_vec(),
                    );
                }
            } else {
                log::error!("Failed to download buffer!");
            }
        }

        Ok(())
    }
}
