#![cfg_attr(target_arch = "spirv", no_std)]
#![feature(asm_experimental_arch)]

use marpii_rmg_task_shared::glam::UVec3;
use marpii_rmg_task_shared::spirv_std::{spirv, RuntimeArray, TypedBuffer};
use marpii_rmg_task_shared::ResourceHandle;

#[repr(C, align(16))]
pub struct PushLayout {
    pub src: ResourceHandle,
    pub dst: ResourceHandle,
    pub wave_size: u32,
    pub pad0: u32,
}

#[inline(never)]
pub fn calculation(a: u32, b: u32) -> u32 {
    a + b
}

#[spirv(compute(threads(64, 1, 1)))]
pub fn main(
    #[spirv(push_constant)] push: &PushLayout,
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(descriptor_set = 0, binding = 0, storage_buffer)] src_buffer: &RuntimeArray<
        TypedBuffer<[u32; 1024]>,
    >,
    #[spirv(descriptor_set = 0, binding = 0, storage_buffer)] dst_buffer: &mut RuntimeArray<
        TypedBuffer<[u32; 1024]>,
    >,
) {
    let coord = id.x;
    if coord >= push.wave_size {
        return;
    }

    //Load if possible, or error color
    let a: u32 = {
        if !push.src.is_invalid() {
            unsafe { src_buffer.index(push.src.index() as usize)[coord as usize] }
        } else {
            0u32
        }
    };

    let b = coord as u32;

    let c = calculation(a, b);

    //write if possible
    if !push.dst.is_invalid() {
        unsafe {
            dst_buffer.index_mut(push.dst.index() as usize)[coord as usize] = c;
        };
    }
}
