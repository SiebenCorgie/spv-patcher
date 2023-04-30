//! Test patch that mutates a given constant.

use rspirv::{
    dr::{Instruction, Operand},
    spirv::Op,
};
use spirt::{
    print::Print,
    spv::{Imm, Inst},
    transform::Transformer,
    visit::Visitor,
    AttrSet, ConstCtor, ConstDef, Context, TypeCtor, TypeDef,
};

use crate::PatcherError;

use super::Patch;

pub struct MutateConstant {
    from: u32,
    to: u32,
}

impl MutateConstant {
    pub fn new(from: u32, to: u32) -> Self {
        MutateConstant { from, to }
    }
}

impl Patch for MutateConstant {
    fn apply<'a>(
        self,
        mut patcher: super::Patcher<'a>,
    ) -> Result<super::Patcher<'a>, PatcherError> {
        let spv_mod = patcher.ir_state.as_spirv();

        for c in spv_mod.types_global_values.iter_mut() {
            if c.class.opcode == Op::Constant {
                if let Operand::LiteralInt32(lit) = &mut c.operands[0] {
                    if *lit == self.from {
                        log::trace!(
                            "Mutatet Constant, found right const {} -> {}",
                            self.from,
                            self.to
                        );
                        *lit = self.to;
                    }
                }
            }
        }

        Ok(patcher)
    }
}
