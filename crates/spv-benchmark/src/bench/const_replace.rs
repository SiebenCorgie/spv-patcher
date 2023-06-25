use marpii::resources::PushConstant;
use marpii_rmg_shared::ResourceHandle;
use marpii_rmg_tasks::DownloadBuffer;
use patch_function::{
    rspirv::{
        dr::Operand,
        spirv::{FunctionControl, LinkageType, LoopControl, StorageClass},
    },
    ConstantReplace,
};
use spv_patcher::{PatcherError, Validator};

use crate::{bench_task::ComputeTask, buffer_to_image::safe_as_image};

use super::Benchmark;

const REPLACE_SPV: &'static [u8] = include_bytes!("../../resources/bench_const_replace.spv");
const PATCHED_COMPILED_SPV: &'static [u8] =
    include_bytes!("../../resources/bench_const_replace_mandelbrot.spv");

#[repr(C, align(16))]
struct ConstReplacePush {
    pub pad0: ResourceHandle,
    pub dst: ResourceHandle,
    pub width: u32,
    pub height: u32,
}

impl Default for ConstReplacePush {
    fn default() -> Self {
        ConstReplacePush {
            pad0: ResourceHandle::INVALID,
            dst: ResourceHandle::INVALID,
            width: ConstReplaceBench::RESOLUTION[0],
            height: ConstReplaceBench::RESOLUTION[1],
        }
    }
}

///Uses the DynReplace patch to modify an existing shader code
/// template.
///
/// Right now we are modifying a simple `-(vec.length())` call to a simple iterative madelbrot set.
///
/// For debugging purpose we can render the final result to an image.
pub struct ConstReplaceBench {
    //src_data: UploadBuffer<f32>,
    dst_data: DownloadBuffer<f32>,
    //Represents our GPU site test task
    bench_task: ComputeTask<ConstReplacePush, f32>,
    pub safe_last_as_image: bool,
    replacement_patch: ConstantReplace,
}

impl ConstReplaceBench {
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
            move |push: &mut PushConstant<ConstReplacePush>, resources, _ctx| {
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
            move |push: &mut PushConstant<ConstReplacePush>, resources, _ctx| {
                //push.get_content_mut().src = resources.resource_handle_or_bind(&src_hdl).unwrap();
                push.get_content_mut().dst = resources.resource_handle_or_bind(&dst_hdl).unwrap();
                push.get_content_mut().width = Self::RESOLUTION[0];
                push.get_content_mut().height = Self::RESOLUTION[1];
            },
        );

        //In this case, pre load / fill the replacement module already
        //Build a module that basically consists of
        // fn f(a: u32, b: u32) -> u32{
        //     let tmp = a * b;
        //     return tmp;
        // }
        let replacement_module = {
            let mut builder = spv_patcher::rspirv::dr::Builder::new();

            builder.capability(patch_function::rspirv::spirv::Capability::Shader);
            builder.capability(patch_function::rspirv::spirv::Capability::VulkanMemoryModel);
            builder.capability(patch_function::rspirv::spirv::Capability::Linkage);
            builder.memory_model(
                patch_function::rspirv::spirv::AddressingModel::Logical,
                patch_function::rspirv::spirv::MemoryModel::Vulkan,
            );
            builder.set_version(1, 5);

            let entry_point_id = builder.id();
            //decorate function as export
            builder.decorate(
                entry_point_id,
                patch_function::rspirv::spirv::Decoration::LinkageAttributes,
                [
                    Operand::LiteralString("replacement_function".to_owned()),
                    LinkageType::Export.into(),
                ],
            );

            //load external instruction set for length call
            let glsl_instructions = builder.ext_inst_import("GLSL.std.450");

            let ty_bool = builder.type_bool();
            let ty_f32 = builder.type_float(32);
            let ty_vec2 = builder.type_vector(ty_f32, 2);
            let ty_pointer_vec2 = builder.type_pointer(None, StorageClass::Function, ty_vec2);
            let ty_u32 = builder.type_int(32, 0);
            let ty_pointer_u32 = builder.type_pointer(None, StorageClass::Function, ty_u32);
            //function f(vec2, vec2) -> f32;
            let function_type = builder.type_function(ty_f32, [ty_vec2, ty_vec2]);
            let _function_id = builder.begin_function(
                ty_f32,
                Some(entry_point_id),
                FunctionControl::empty(),
                function_type,
            );
            //Add parametres
            let arg1 = builder.function_parameter(ty_vec2).unwrap();
            let arg2 = builder.function_parameter(ty_vec2).unwrap();

            //Now begin function's main block
            builder.begin_block(None).unwrap();

            //Setup our two variables, n and z.
            let id_n = builder.variable(ty_pointer_u32, None, StorageClass::Function, None);
            let id_z = builder.variable(ty_pointer_vec2, None, StorageClass::Function, None);

            let const_u32_1 = builder.constant_bit32(ty_u32, 1);
            let const_u32_1024 = builder.constant_bit32(ty_u32, 1024);
            let const_f32_zero = builder.constant_bit32(ty_f32, 0.0f32.to_bits());
            let const_f32_two = builder.constant_bit32(ty_f32, 2.0f32.to_bits());
            let const_f32_1k = builder.constant_bit32(ty_f32, 1000.0f32.to_bits());
            let const_f32_025 = builder.constant_bit32(ty_f32, 0.25f32.to_bits());
            let const_f32_015 = builder.constant_bit32(ty_f32, 0.15f32.to_bits());
            let const_initial_n = builder.constant_bit32(ty_u32, 0);
            let const_initial_z =
                builder.constant_composite(ty_vec2, [const_f32_zero, const_f32_zero]);

            builder.store(id_n, const_initial_n, None, []).unwrap();
            builder.store(id_z, const_initial_z, None, []).unwrap();

            //now setup our loop header
            //for that, build our test criteria by loading n and checking for < 1024, and
            // loading z' length and test testing for < 1000.0

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

            //LOOP CONDITION
            builder.begin_block(Some(loop_condition)).unwrap();
            let n_val = builder.load(ty_u32, None, id_n, None, []).unwrap();
            let z_val = builder.load(ty_vec2, None, id_z, None, []).unwrap();
            let z_length = builder
                .ext_inst(ty_f32, None, glsl_instructions, 66, [Operand::IdRef(z_val)])
                .unwrap();
            let length_bit = builder
                .f_unord_less_than(ty_bool, None, z_length, const_f32_1k)
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
            let tmp_new_y = builder.f_mul(ty_f32, None, tmp_x, tmp_y).unwrap();
            let new_y = builder
                .f_mul(ty_f32, None, const_f32_two, tmp_new_y)
                .unwrap();
            let new_tmp_z = builder
                .composite_construct(ty_vec2, None, [new_x, new_y])
                .unwrap();
            //now add p to the new z value
            let new_z = builder.f_add(ty_vec2, None, new_tmp_z, arg2).unwrap();
            //store back to z's pointer
            builder.store(id_z, new_z, None, []).unwrap();

            let current_n_val = builder.load(ty_u32, None, id_n, None, []).unwrap();
            let new_n_val = builder
                .i_add(ty_u32, None, current_n_val, const_u32_1)
                .unwrap();
            builder.store(id_n, new_n_val, None, []).unwrap();
            //Now branch into the continue block
            builder.branch(loop_continue).unwrap();

            //LOOP CONTINUE
            builder.begin_block(Some(loop_continue)).unwrap();
            builder.branch(loop_header).unwrap(); // go directly back to header

            //POST LOOP
            builder.begin_block(Some(post_loop)).unwrap();

            //Post loop, load z and assemble final result
            let cz = builder.load(ty_vec2, None, id_z, None, []).unwrap();
            // Extended instruction ID
            // cos == 14
            // log2 == 30
            let z_dot_z = builder.dot(ty_f32, None, cz, cz).unwrap();
            let log2_1 = builder
                .ext_inst(
                    ty_f32,
                    None,
                    glsl_instructions,
                    30,
                    [Operand::IdRef(z_dot_z)],
                )
                .unwrap();
            let log2_2 = builder
                .ext_inst(
                    ty_f32,
                    None,
                    glsl_instructions,
                    30,
                    [Operand::IdRef(log2_1)],
                )
                .unwrap();
            let cn = builder.load(ty_u32, None, id_n, None, []).unwrap();
            let n_f32 = builder.convert_s_to_f(ty_f32, None, cn).unwrap();
            let subbed = builder.f_sub(ty_f32, None, n_f32, log2_2).unwrap();
            let in_muled = builder.f_mul(ty_f32, None, const_f32_015, subbed).unwrap();
            let cosed = builder
                .ext_inst(
                    ty_f32,
                    None,
                    glsl_instructions,
                    14,
                    [Operand::IdRef(in_muled)],
                )
                .unwrap();

            let final_mulled = builder.f_mul(ty_f32, None, const_f32_025, cosed).unwrap();

            //now assign result to return instruction
            let _ = builder.ret_value(final_mulled).unwrap();

            builder.end_function().unwrap();

            builder.module()
        };

        let replacement_patch = ConstantReplace::new(replacement_module, 0)
            .map_err(|e| PatcherError::Internal(e.into()))?;

        Ok(ConstReplaceBench {
            bench_task,
            //src_data: src,
            dst_data: dst,
            safe_last_as_image: false,
            replacement_patch,
        })
    }

    pub fn patch(&mut self, rmg: &mut marpii_rmg::Rmg) -> Result<(), PatcherError> {
        self.bench_task.pipeline.patch_pipeline(rmg, |patch| {
            patch
                //.print()
                .patch(self.replacement_patch.clone())
        })
    }
}

impl Benchmark for ConstReplaceBench {
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

                let (min, max) = target_buffer
                    .iter()
                    .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), v| {
                        (min.min(*v), max.max(*v))
                    });
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
        self.load_shader(rmg, PATCHED_COMPILED_SPV);
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

                let (min, max) = target_buffer
                    .iter()
                    .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), v| {
                        (min.min(*v), max.max(*v))
                    });

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
        "ConstReplaceMandelbrot"
    }
}
