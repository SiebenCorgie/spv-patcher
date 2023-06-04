use marpii::resources::PushConstant;
use marpii_rmg_shared::ResourceHandle;
use marpii_rmg_tasks::{DownloadBuffer, UploadBuffer};
use spv_patcher::PatcherError;

use crate::{compute_task::ComputeTask, test_runs::TestRun};

#[repr(C, align(16))]
struct DynReplacePush {
    pub src: ResourceHandle,
    pub dst: ResourceHandle,
    pub wave_size: u32,
    pub pad0: u32,
}

impl Default for DynReplacePush {
    fn default() -> Self {
        DynReplacePush {
            src: ResourceHandle::INVALID,
            dst: ResourceHandle::INVALID,
            wave_size: DynReplaceTest::BUFSIZE as u32,
            pad0: 0,
        }
    }
}

const REPLACE_SPV: &'static [u8] = include_bytes!("../resources/no_inline_function.spv");

pub struct DynReplaceTest {
    src_data: UploadBuffer<u32>,
    dst_data: DownloadBuffer<u32>,
    //Represents our GPU site test task
    test_task: ComputeTask<DynReplacePush, u32>,
}

impl DynReplaceTest {
    const BUFSIZE: usize = 1024;
    //loads the shader
    pub fn load(rmg: &mut marpii_rmg::Rmg) -> Result<Self, PatcherError> {
        let initial_data = &[2u32; Self::BUFSIZE];
        let src = UploadBuffer::new(rmg, initial_data).unwrap();
        let src_hdl = src.buffer.clone();
        let dst = DownloadBuffer::new_for(rmg, Self::BUFSIZE).unwrap();
        let dst_hdl = dst.gpu_handle();
        let test_task = ComputeTask::new(
            rmg,
            REPLACE_SPV.to_vec(),
            vec![src_hdl.clone()],
            vec![dst_hdl.clone()],
            Self::BUFSIZE as u32,
            move |push: &mut PushConstant<DynReplacePush>, resources, _ctx| {
                push.get_content_mut().src = resources.resource_handle_or_bind(&src_hdl).unwrap();
                push.get_content_mut().dst = resources.resource_handle_or_bind(&dst_hdl).unwrap();
                push.get_content_mut().wave_size = Self::BUFSIZE as u32;
            },
        )
        .unwrap();

        Ok(DynReplaceTest {
            test_task,
            src_data: src,
            dst_data: dst,
        })
    }

    pub fn patch(&mut self, rmg: &mut marpii_rmg::Rmg) -> Result<(), PatcherError> {
        self.test_task.pipeline.patch_pipeline(rmg, |patch| {
            patch
                //.print()
                .patch(patch_function::DynamicReplace::new(
                    patch_function::FuncIdent::Name("calculation".to_owned()),
                    |builder, sig| {
                        //we use IMUL to multiply the two arguments and store that into a new id that is returned
                        let a = &sig.parameter[0];
                        let b = &sig.parameter[1];
                        assert!(
                            (a.1 == b.1) && (a.1 == sig.return_type),
                            "Types do not match!"
                        );
                        //add imul
                        let res_id = builder.i_mul(sig.return_type, None, a.0, b.0).unwrap();

                        //now assign result to return instruction
                        let _ = builder.ret_value(res_id).unwrap();
                        Ok(())
                    },
                ))
        })
    }
}

impl TestRun for DynReplaceTest {
    fn name(&self) -> &'static str {
        "dyn_replace"
    }
    fn run(
        &mut self,
        blessing: &mut crate::BlessedDB,
        rmg: &mut marpii_rmg::Rmg,
    ) -> Result<(), crate::test_runs::TestError> {
        //load our pipeline, run it, then patch it and run it again.
        {
            rmg.record()
                .add_task(&mut self.src_data)
                .unwrap()
                .add_task(&mut self.test_task)
                .unwrap()
                .add_task(&mut self.dst_data)
                .unwrap()
                .execute()
                .unwrap();

            let mut target_buffer = [0u32; Self::BUFSIZE];
            let _res = self.dst_data.download(rmg, &mut target_buffer);
            for (sidx, s) in target_buffer.iter().enumerate() {
                assert!(*s == (sidx as u32 + 2), "Pre-Patch run failed");
            }
        }

        //patch pipeline
        self.patch(rmg).unwrap();
        /*
                println!(
                    "{}",
                    DisassamblerPrinter::from_bytecode(self.test_task.pipeline.patched_code())
                );
        */

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

                //If we are blessing, write this shader code to the db
                if blessing.bless {
                    log::info!("Blessing result: \n{:?}", target_buffer);
                    let bless_buffer = bytemuck::cast_slice(&target_buffer).to_vec();
                    let _old = blessing
                        .blessed_results
                        .insert(self.name().to_string(), bless_buffer);
                } else {
                    if let Some(blessed_result) = blessing.blessed_results.get(self.name()) {
                        //cast result to u32s for testing
                        let test_results: &[u32] = bytemuck::cast_slice(blessed_result);
                        for (idx, (a, b)) in
                            test_results.iter().zip(target_buffer.iter()).enumerate()
                        {
                            assert!(a == b, "Test failed for {} : {} == {}", idx, a, b);
                        }
                        log::info!("DynamicReplace successful");
                    } else {
                        log::error!("Could not test result, there is no blessed result for it!");
                    }
                }
            } else {
                log::error!("Failed to download buffer!");
            }
        }

        Ok(())
    }
}
