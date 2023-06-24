use marpii::resources::PushConstant;
use marpii_rmg_shared::ResourceHandle;
use marpii_rmg_tasks::{DownloadBuffer, UploadBuffer};
use spv_patcher::{PatcherError, Validator};

use crate::{compute_task::ComputeTask, test_runs::TestRun};

#[repr(C, align(16))]
struct NonUniformPush {
    pub src1: ResourceHandle,
    pub src2: ResourceHandle,
    pub dst: ResourceHandle,
    pub buffer_size: u32,
}

impl Default for NonUniformPush {
    fn default() -> Self {
        NonUniformPush {
            src1: ResourceHandle::INVALID,
            src2: ResourceHandle::INVALID,
            dst: ResourceHandle::INVALID,
            buffer_size: NonUniformTest::BUFSIZE as u32,
        }
    }
}

const NONUNIFORM_SPV: &'static [u8] = include_bytes!("../resources/nonuniform_patch.spv");

pub struct NonUniformTest {
    src_data1: UploadBuffer<u32>,
    src_data2: UploadBuffer<u32>,
    dst_data: DownloadBuffer<u32>,
    //Represents our GPU site test task
    test_task: ComputeTask<NonUniformPush, u32>,
}

impl NonUniformTest {
    const BUFSIZE: usize = 64;
    //loads the shader
    pub fn load(rmg: &mut marpii_rmg::Rmg) -> Result<Self, PatcherError> {
        let initial_data1 = &[0u32; Self::BUFSIZE];
        let src1 = UploadBuffer::new(rmg, initial_data1).unwrap();
        let src1_hdl = src1.buffer.clone();
        let initial_data2 = &[1u32; Self::BUFSIZE];
        let src2 = UploadBuffer::new(rmg, initial_data2).unwrap();
        let src2_hdl = src2.buffer.clone();
        let dst = DownloadBuffer::new_for(rmg, Self::BUFSIZE).unwrap();
        let dst_hdl = dst.gpu_handle();
        let test_task = ComputeTask::new(
            rmg,
            NONUNIFORM_SPV.to_vec(),
            vec![src1_hdl.clone(), src2_hdl.clone()],
            vec![dst_hdl.clone()],
            Self::BUFSIZE as u32,
            move |push: &mut PushConstant<NonUniformPush>, resources, _ctx| {
                push.get_content_mut().src1 = resources.resource_handle_or_bind(&src1_hdl).unwrap();
                push.get_content_mut().src2 = resources.resource_handle_or_bind(&src2_hdl).unwrap();
                push.get_content_mut().dst = resources.resource_handle_or_bind(&dst_hdl).unwrap();
            },
        )
        .unwrap();

        Ok(NonUniformTest {
            test_task,
            src_data1: src1,
            src_data2: src2,
            dst_data: dst,
        })
    }

    pub fn patch(&mut self, rmg: &mut marpii_rmg::Rmg) -> Result<(), PatcherError> {
        self.test_task.pipeline.patch_pipeline(rmg, |patch| {
            patch
                //.print()
                .patch(patch_decorate_nonuniform::NonUniformDecorate::new())
        })
    }
}

impl TestRun for NonUniformTest {
    fn name(&self) -> &'static str {
        "non_uniform_patch"
    }
    fn run(
        &mut self,
        blessing: &mut crate::BlessedDB,
        rmg: &mut marpii_rmg::Rmg,
    ) -> Result<(), crate::test_runs::TestError> {
        //load our pipeline, run it, then patch it and run it again.
        {
            rmg.record()
                .add_task(&mut self.src_data1)
                .unwrap()
                .add_task(&mut self.src_data2)
                .unwrap()
                .add_task(&mut self.test_task)
                .unwrap()
                .add_task(&mut self.dst_data)
                .unwrap()
                .execute()
                .unwrap();

            let mut target_buffer = [0u32; Self::BUFSIZE];
            let res = self.dst_data.download(rmg, &mut target_buffer);
            if let Ok(size) = res {
                if size != Self::BUFSIZE {
                    log::error!("Download size did not match: {} != {}", size, Self::BUFSIZE);
                }

                //In the first run there shouldn't be a uniform result.
                // Check if we have alternating pattern
                let mut is_alternating = true;
                for i in 0..size {
                    if target_buffer[i] != ((i as u32) % 2) {
                        is_alternating = false;
                    }
                }
                if is_alternating {
                    log::error!("Was alternating pattern, but shouldn't be without the patch!");
                } else {
                    log::info!("Unpatched non_uniform wasn't alternating as expected");
                }
            } else {
                log::error!("Failed to download buffer!");
            }
        }

        //patch pipeline
        self.patch(rmg).unwrap();

        /*
                println!(
                    "{}",
                    DisassamblerPrinter::from_bytecode(self.test_task.get_pipeline().patched_code())
                );
        */
        //If not currently blessing, assert that the code is the same
        if !blessing.bless {
            if let Some(blessed_code) = blessing.blessed_results.get(self.name()) {
                assert!(
                    blessed_code == self.test_task.pipeline.patched_code(),
                    "NonUniform patched code is invalid!"
                );
            } else {
                log::info!("Nothing to bless for {}", self.name());
            }
        } else {
            Validator::validate_code(self.test_task.pipeline.patched_code()).unwrap();
            log::info!("Blessing NonUniform!");
        }

        {
            rmg.record()
                //NOTE: Not uploading new data, as we want to use exactly the same input
                .add_task(&mut self.test_task)
                .unwrap()
                .add_task(&mut self.dst_data)
                .unwrap()
                .execute()
                .unwrap();

            let mut target_buffer = [0u32; Self::BUFSIZE];
            let res = self.dst_data.download(rmg, &mut target_buffer);
            if let Ok(size) = res {
                if size != Self::BUFSIZE {
                    log::error!("Download size did not match: {} != {}", size, Self::BUFSIZE);
                }

                let mut is_alternating = true;
                for i in 0..size {
                    if target_buffer[i] != ((i as u32) % 2) {
                        is_alternating = false;
                    }
                }
                if !is_alternating {
                    log::error!(
                        "Expected alternating pattern, but wasn't strictly alternating: {:?}",
                        target_buffer
                    );
                } else {
                    log::info!("Found strictly alternating pattern as expected.");
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
