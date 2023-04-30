//! Test patch that mutates a given constant.

use rspirv::{dr::Instruction, spirv::Op};
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
    from: i32,
    to: i32,
}

impl MutateConstant {
    pub fn new(from: i32, to: i32) -> Self {
        MutateConstant { from, to }
    }
}

impl Patch for MutateConstant {
    fn apply<'a>(
        self,
        mut patcher: super::Patcher<'a>,
    ) -> Result<super::Patcher<'a>, PatcherError> {
        let spv_mod = patcher.ir_state.as_spirv();

        for c in spv_mod.types_global_values.iter() {
            if c.class.opcode == Op::Constant {
                println!("OPCONST: {:?}", c);
            }
        }

        Ok(patcher)
    }
}
