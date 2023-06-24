use marpii::resources::PushConstant;
use marpii_rmg_shared::ResourceHandle;
use marpii_rmg_tasks::DownloadBuffer;
use patch_function::rspirv::{
    dr::Operand,
    spirv::{LoopControl, StorageClass},
};
use spv_patcher::{PatcherError, Validator};

use crate::{bench_task::ComputeTask, buffer_to_image::safe_as_image};

use super::Benchmark;

const REPLACE_SPV: &'static [u8] = include_bytes!("../../resources/bench_const_replace.spv");
const PATCHED_COMPILED_SPV: &'static [u8] =
    include_bytes!("../../resources/bench_const_replace_mandelbrot.spv");

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

    ///Loads the shader code as `bench_task`.
    fn load_shader(&mut self, rmg: &mut marpii_rmg::Rmg, shader: &[u8]) {
        let dst_hdl = self.dst_data.gpu_handle().clone();
        self.bench_task = ComputeTask::new(
            rmg,
            shader.to_vec(),
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
    }

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
                        //We try to rebuild the iterative mandelbrot set that the source uses.
                        // For that we parse the inputs (which must be the same and of type Array<f32, 2>)
                        // after that we init n to 0, init z to 0 (which is the same as coord * 0.0 in rust).
                        //
                        // we then init the for loop that iterates either till n=1024 or breaks if z's length
                        // is > 1000.0.
                        //
                        // z iteratively changes via some multiplications.
                        //
                        // Afterwards we use OpPhi to select our value and use the given multiplication chain (+ cos and dot)
                        // to find the actual final parameter.

                        //load external instruction set for length call
                        let glsl_instructions = builder.ext_inst_import("GLSL.std.450");

                        //STEP 1: Verify signature
                        let ty_bool = builder.type_bool();
                        let ty_f32 = builder.type_float(32);
                        let ty_vec2 = builder.type_vector(ty_f32, 2);
                        let ty_pointer_vec2 =
                            builder.type_pointer(None, StorageClass::Function, ty_vec2);
                        let ty_u32 = builder.type_int(32, 0);
                        let ty_pointer_u32 =
                            builder.type_pointer(None, StorageClass::Function, ty_u32);
                        let const_f32_1k = builder.constant_bit32(ty_f32, 1000.0f32.to_bits());
                        let const_u32_1024 = builder.constant_bit32(ty_u32, 1024);

                        assert!(sig.parameter[0].1 == ty_vec2, "Parameter 0 not Vec2<f32>");
                        assert!(sig.parameter[1].1 == ty_vec2, "Parameter 1 not Vec2<f32>");

                        //Setup our two variables, n and z.
                        let id_n =
                            builder.variable(ty_pointer_u32, None, StorageClass::Function, None);
                        let id_z =
                            builder.variable(ty_pointer_vec2, None, StorageClass::Function, None);

                        let const_initial_n = builder.constant_bit32(ty_u32, 0);
                        let const_u32_1 = builder.constant_bit32(ty_u32, 1);
                        let const_f32_zero = builder.constant_bit32(ty_f32, 0.0f32.to_bits());
                        let const_initial_z =
                            builder.constant_composite(ty_vec2, [const_f32_zero, const_f32_zero]);

                        builder.store(id_n, const_initial_n, None, []).unwrap();
                        builder.store(id_z, const_initial_z, None, []).unwrap();

                        //now setup our loop header
                        //for that, build our test criteria by loading n and checking for < 1024, and
                        // loading z' length and test tesitng for < 1000.0

                        //for clarity, allocate all labels needed for our for loop

                        //Header part
                        let loop_header = builder.id();
                        //what happens *in the loop*
                        let loop_body = builder.id();
                        //The block we jump to after handling the loop
                        let post_loop = builder.id();
                        //what happens *post loop*
                        let loop_continue = builder.id();
                        //Block that calculates the loop condition
                        let loop_condition = builder.id();

                        builder.branch(loop_header).unwrap();

                        //LOOP HEADER
                        builder.begin_block(Some(loop_header)).unwrap();
                        builder
                            .loop_merge(post_loop, loop_continue, LoopControl::NONE, [])
                            .unwrap();
                        //now branch into the first conditional body
                        builder.branch(loop_condition).unwrap();
                        //let _ = builder.branch(loop_condition);

                        //LOOP CONDITION
                        builder.begin_block(Some(loop_condition)).unwrap();
                        let n_val = builder.load(ty_u32, None, id_n, None, []).unwrap();
                        let z_val = builder.load(ty_vec2, None, id_z, None, []).unwrap();
                        let z_length = builder
                            .ext_inst(ty_f32, None, glsl_instructions, 66, [Operand::IdRef(z_val)])
                            .unwrap();
                        let length_bit = builder
                            .f_ord_less_than(ty_bool, None, z_length, const_f32_1k)
                            .unwrap();
                        let n_bit = builder
                            .u_less_than(ty_bool, None, n_val, const_u32_1024)
                            .unwrap();
                        //combine both
                        let combined_bit = builder
                            .logical_and(ty_bool, None, length_bit, n_bit)
                            .unwrap();
                        //now end condition block with compare-continue
                        builder
                            .branch_conditional(combined_bit, loop_body, post_loop, [])
                            .unwrap();

                        //LOOP BODY
                        builder.begin_block(Some(loop_body)).unwrap();
                        //build both components of the new vector
                        let current_z_val = builder.load(ty_vec2, None, id_z, None, []).unwrap();
                        let tmp_x = builder
                            .composite_extract(ty_f32, None, current_z_val, [0])
                            .unwrap();
                        let tmp_y = builder
                            .composite_extract(ty_f32, None, current_z_val, [1])
                            .unwrap();

                        let xsqr = builder.f_mul(ty_f32, None, tmp_x, tmp_x).unwrap();
                        let ysqr = builder.f_mul(ty_f32, None, tmp_y, tmp_y).unwrap();
                        let new_x = builder.f_sub(ty_f32, None, xsqr, ysqr).unwrap();
                        let const_two = builder.constant_bit32(ty_f32, 2.0f32.to_bits());
                        let tmp_new_y = builder.f_mul(ty_f32, None, tmp_x, tmp_y).unwrap();
                        let new_y = builder.f_mul(ty_f32, None, const_two, tmp_new_y).unwrap();
                        let new_tmp_z = builder
                            .composite_construct(ty_vec2, None, [new_x, new_y])
                            .unwrap();
                        //now add p to the new z value
                        let new_z = builder
                            .f_mul(ty_vec2, None, new_tmp_z, sig.parameter[1].0)
                            .unwrap();
                        //store back to z's pointer
                        builder.store(id_z, new_z, None, []).unwrap();

                        let current_n_val = builder.load(ty_vec2, None, id_n, None, []).unwrap();
                        let new_n_val = builder
                            .i_add(ty_u32, None, current_n_val, const_u32_1)
                            .unwrap();
                        builder.store(id_n, new_n_val, None, []).unwrap();
                        //Now branch into the contiue block
                        builder.branch(loop_continue).unwrap();

                        //LOOP CONTINUE
                        builder.begin_block(Some(loop_continue)).unwrap();
                        builder.branch(loop_header).unwrap(); // go directly back to header

                        //POST LOOP
                        builder.begin_block(Some(post_loop)).unwrap();
                        //Post loop, load z and assemble final result

                        //Right now just extract x of the first parameter
                        let res_id = builder
                            .composite_extract(ty_f32, None, sig.parameter[0].0, [0])
                            .unwrap();

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
        //Be sure that we use the correct code
        self.load_shader(rmg, PATCHED_COMPILED_SPV);

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
            reporter.report_patched_compiled(self, timing_ns);

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
                let _res = self.dst_data.download(rmg, &mut target_buffer).unwrap();

                safe_as_image(
                    Self::RESOLUTION[0],
                    Self::RESOLUTION[1],
                    &target_buffer,
                    &format!("{}_patched_compiled", self.name()),
                );
            }
        }
    }
    fn bench_patched_runtime(
        &mut self,
        rmg: &mut marpii_rmg::Rmg,
        reporter: &mut crate::reporter::Reporter,
        runs: usize,
    ) {
        //Be sure that we use the correct code
        self.patch(rmg).unwrap();

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
            reporter.report_patched_runtime(self, timing_ns);

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
                let _res = self.dst_data.download(rmg, &mut target_buffer).unwrap();

                safe_as_image(
                    Self::RESOLUTION[0],
                    Self::RESOLUTION[1],
                    &target_buffer,
                    &format!("{}_patched_runtime", self.name()),
                );
            }
        }
    }
    fn bench_unmodified(
        &mut self,
        rmg: &mut marpii_rmg::Rmg,
        reporter: &mut crate::reporter::Reporter,
        runs: usize,
    ) {
        //Be sure that we use the correct code
        self.load_shader(rmg, REPLACE_SPV);
        Validator::validate_code(self.bench_task.pipeline.patched_code())
            .expect("Failed to validate code!");

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
                let _res = self.dst_data.download(rmg, &mut target_buffer).unwrap();

                safe_as_image(
                    Self::RESOLUTION[0],
                    Self::RESOLUTION[1],
                    &target_buffer,
                    self.name(),
                );
            }
        }
    }
    fn name(&self) -> &str {
        "DynReplaceMandelbrot"
    }
}
