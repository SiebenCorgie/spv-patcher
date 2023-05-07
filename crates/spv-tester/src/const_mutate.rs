use marpii::context::Ctx;
use marpii_rmg::Rmg;
use spv_patcher::{
    patch::MutateConstant, rspirv::spirv::ExecutionModel, spirv_ext::SpirvExt, Module, PatcherError,
};

use crate::{compute_task::SimpleComputeTask, tests::TestRun};

const CONST_MUTATE_SPV: &'static [u8] = include_bytes!("../resources/compute_add.spv");

pub struct ConstMutateTest {
    //Represents our GPU site test task
    test_task: SimpleComputeTask<u32>,
}

impl ConstMutateTest {
    const BUFSIZE: usize = 64;
    //loads the shader
    pub fn load(rmg: &mut marpii_rmg::Rmg) -> Result<Self, PatcherError> {
        let initial_data = &[0u32; Self::BUFSIZE];
        let test_task =
            SimpleComputeTask::new(rmg, CONST_MUTATE_SPV.to_vec(), initial_data).unwrap();

        Ok(ConstMutateTest { test_task })
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
                .get_pipeline()
                .get_module()
                .spirv()
                .get_execution_model()
                == ExecutionModel::GLCompute
        );

        self.test_task.get_pipeline().patch_pipeline(rmg, |patch| {
            patch
                //.print()
                .patch(MutateConstant::Integer { from, to })
        })
    }

    ///Patches
    pub fn patch_f32(
        &mut self,
        rmg: &mut marpii_rmg::Rmg,
        from: f32,
        to: f32,
    ) -> Result<(), PatcherError> {
        self.test_task.get_pipeline().patch_pipeline(rmg, |patch| {
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
    ) -> Result<(), crate::tests::TestError> {
        //load our pipeline, run it, then patch it and run it again.

        {
            let run_one = rmg
                .record()
                .add_meta_task(&mut self.test_task)
                .unwrap()
                .execute()
                .unwrap();

            let mut target_buffer = [0u32; Self::BUFSIZE];
            let res = self.test_task.get_data(rmg, &mut target_buffer);
            if let Ok(size) = res {
                if size != Self::BUFSIZE {
                    log::error!("Download size did not match: {} != {}", size, Self::BUFSIZE);
                }

                for i in 0..size {
                    assert!(
                        target_buffer[i] == 33,
                        "Should be {}, was {}",
                        34,
                        target_buffer[i]
                    );
                }
            } else {
                log::error!("Failed to download buffer!");
            }
        }

        //patch pipeline
        self.patch_i32(rmg, 33, 42);

        {
            let run_one = rmg
                .record()
                .add_meta_task(&mut self.test_task)
                .unwrap()
                .execute()
                .unwrap();

            let mut target_buffer = [0u32; Self::BUFSIZE];
            let res = self.test_task.get_data(rmg, &mut target_buffer);
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
            } else {
                log::error!("Failed to download buffer!");
            }
        }

        Ok(())
    }
}
