//! Test patch that mutates a given constant.

use rspirv::{dr::Operand, spirv::Op};

use crate::{spirv_ext::SpirvExt, PatcherError};

use super::Patch;

pub enum MutateConstant {
    Integer { from: u32, to: u32 },
    Float { from: f32, to: f32 },
}

impl Patch for MutateConstant {
    fn apply<'a>(
        self,
        mut patcher: super::Patcher<'a>,
    ) -> Result<super::Patcher<'a>, PatcherError> {
        let spv_mod = patcher.ir_state.as_spirv();

        if spv_mod.has_extension("GL_EXT_nonuniform_qualifier") {
            log::info!("Has extension!");
        }

        for c in spv_mod.types_global_values.iter_mut() {
            if c.class.opcode == Op::Constant {
                match (&mut c.operands[0], &self) {
                    (Operand::LiteralBit32(lit), MutateConstant::Integer { from, to }) => {
                        if *lit == *from {
                            log::trace!("Mutatet Constant, found right const {} -> {}", from, to);
                            *lit = *to;
                        }
                    }
                    (Operand::LiteralBit32(lit), MutateConstant::Float { from, to }) => {
                        let litf32 = f32::from_bits(*lit);
                        if litf32 == *from {
                            log::trace!("Mutatet Constant, found right const {} -> {}", from, to);
                            *lit = to.to_bits();
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(patcher)
    }
}
