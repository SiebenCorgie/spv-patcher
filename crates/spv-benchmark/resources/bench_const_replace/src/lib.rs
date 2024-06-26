#![cfg_attr(target_arch = "spirv", no_std)]
#![feature(asm_experimental_arch)]

use marpii_rmg_task_shared::glam::{UVec3, Vec2, Vec3Swizzles};
use marpii_rmg_task_shared::spirv_std::{spirv, RuntimeArray, TypedBuffer};
use marpii_rmg_task_shared::ResourceHandle;

#[repr(C, align(16))]
pub struct PushLayout {
    pub pad0: ResourceHandle,
    pub dst: ResourceHandle,
    pub width: u32,
    pub height: u32,
}

#[inline(never)]
pub fn calculation(coord: Vec2, p: Vec2) -> f32 {
    (coord - p).length()
}

#[spirv(compute(threads(8, 8, 1)))]
pub fn main(
    #[spirv(push_constant)] push: &PushLayout,
    #[spirv(global_invocation_id)] id: UVec3,
    //#[spirv(descriptor_set = 0, binding = 0, storage_buffer)] src_buffer: &RuntimeArray<
    //    TypedBuffer<[f32; 1024]>,
    //>,
    #[spirv(descriptor_set = 0, binding = 0, storage_buffer)] dst_buffer: &mut RuntimeArray<
        TypedBuffer<RuntimeArray<f32>>,
    >,
) {
    let coord = id.xy();
    if coord.x >= push.width || coord.y >= push.height {
        return;
    }

    let p = Vec2::new(push.width as f32 / 2.0, push.height as f32 / 2.0);
    let c = calculation(coord.as_vec2(), p);

    let safe_at = (coord.y * push.width) + coord.x;
    //write if possible
    if !push.dst.is_invalid() {
        unsafe {
            *dst_buffer
                .index_mut(push.dst.index() as usize)
                .index_mut(safe_at as usize) = c;
        };
    }
}
