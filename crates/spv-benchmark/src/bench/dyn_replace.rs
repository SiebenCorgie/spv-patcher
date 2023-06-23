use marpii::resources::PushConstant;
use marpii_rmg_shared::ResourceHandle;
use marpii_rmg_tasks::{DownloadBuffer, UploadBuffer};
use spv_patcher::PatcherError;

use crate::{bench_task::ComputeTask, buffer_to_image::safe_as_image};

use super::Benchmark;

const REPLACE_SPV: &'static [u8] = include_bytes!("../../resources/bench_const_replace.spv");

#[repr(C, align(16))]
struct DynReplacePush {
    pub pad0: ResourceHandle,
    pub dst: ResourceHandle,
    pub width: u32,
    pub height: u32,
}

impl Default for DynReplacePush {
    fn default() -> Self {
        DynReplacePush {
            pad0: ResourceHandle::INVALID,
            dst: ResourceHandle::INVALID,
            width: DynReplaceBench::RESOLUTION[0],
            height: DynReplaceBench::RESOLUTION[1],
        }
    }
}

///Uses the DynReplace patch to modify an existing shader code
/// template.
///
/// Right now we are modifying a simple `-(vec.length())` call to a simple iterative madelbrot set.
///
/// For debugging purpose we can render the final result to an image.
pub struct DynReplaceBench {
    //src_data: UploadBuffer<f32>,
    dst_data: DownloadBuffer<f32>,
    //Represents our GPU site test task
    bench_task: ComputeTask<DynReplacePush, f32>,
    pub safe_last_as_image: bool,
}

impl DynReplaceBench {
    const RESOLUTION: [u32; 2] = [2048, 2048];

    //loads the shader
    pub fn load(rmg: &mut marpii_rmg::Rmg) -> Result<Self, PatcherError> {
        //let initial_data = vec![0.0f32; (Self::RESOLUTION[0] * Self::RESOLUTION[1]) as usize];

        //let src = UploadBuffer::new(rmg, &initial_data).unwrap();
        //let src_hdl = src.buffer.clone();
        let dst: DownloadBuffer<f32> =
            DownloadBuffer::new_for(rmg, (Self::RESOLUTION[0] * Self::RESOLUTION[1]) as usize)
                .unwrap();
        let dst_hdl = dst.gpu_handle();
        let bench_task = ComputeTask::new(
            rmg,
            REPLACE_SPV.to_vec(),
            vec![],
            vec![dst_hdl.clone()],
            Self::RESOLUTION,
            move |push: &mut PushConstant<DynReplacePush>, resources, _ctx| {
                //push.get_content_mut().src = resources.resource_handle_or_bind(&src_hdl).unwrap();
                push.get_content_mut().dst = resources.resource_handle_or_bind(&dst_hdl).unwrap();
                push.get_content_mut().width = Self::RESOLUTION[0];
                push.get_content_mut().height = Self::RESOLUTION[1];
            },
        );

        Ok(DynReplaceBench {
            bench_task,
            //src_data: src,
            dst_data: dst,
            safe_last_as_image: false,
        })
    }

    pub fn patch(&mut self, rmg: &mut marpii_rmg::Rmg) -> Result<(), PatcherError> {
        self.bench_task.pipeline.patch_pipeline(rmg, |patch| {
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

impl Benchmark for DynReplaceBench {
    fn bench_patched_compiled(
        &mut self,
        rmg: &mut marpii_rmg::Rmg,
        reporter: &mut crate::reporter::Reporter,
        runs: usize,
    ) {
    }
    fn bench_patched_runtime(
        &mut self,
        rmg: &mut marpii_rmg::Rmg,
        reporter: &mut crate::reporter::Reporter,
        runs: usize,
    ) {
    }
    fn bench_unmodified(
        &mut self,
        rmg: &mut marpii_rmg::Rmg,
        reporter: &mut crate::reporter::Reporter,
        runs: usize,
    ) {
        for idx in 0..runs {
            //Run bench
            rmg.record()
                //.add_task(&mut self.src_data)
                //.unwrap()
                .add_task(&mut self.bench_task)
                .unwrap()
                .execute()
                .unwrap();

            //Wait for the timings and report
            let timing_ns = self.bench_task.get_last_timing();
            reporter.report_unmodified(self, timing_ns);

            //If we are the last run, an the flag is set, write back as image.
            if idx == (runs - 1) && self.safe_last_as_image {
                //download last buffer
                rmg.record()
                    .add_task(&mut self.dst_data)
                    .unwrap()
                    .execute()
                    .unwrap();

                let mut target_buffer =
                    vec![0.0f32; (Self::RESOLUTION[0] * Self::RESOLUTION[1]) as usize];
                let res = self.dst_data.download(rmg, &mut target_buffer).unwrap();

                //std::thread::sleep(std::time::Duration::from_secs(2));

                /*
                for i in 0..Self::RESOLUTION[1] {
                    for x in 0..Self::RESOLUTION[0] {
                        print!(
                            " {:.1} ",
                            target_buffer[(i * Self::RESOLUTION[0] + x) as usize]
                        );
                    }
                    println!();
                }
                */

                safe_as_image(
                    Self::RESOLUTION[0],
                    Self::RESOLUTION[1],
                    &target_buffer,
                    self.name(),
                );
            }
        }

        //In this case, just load the pipeline with the unmodified code and run it
        {}
    }
    fn name(&self) -> &str {
        "DynReplaceMandelbrot"
    }
}
