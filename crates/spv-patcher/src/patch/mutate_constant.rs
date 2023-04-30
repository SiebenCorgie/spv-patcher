//! Test patch that mutates a given constant.

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
    from: ConstDef,
    to: ConstDef,
    seen: bool,
}

impl MutateConstant {
    pub fn new(from: ConstDef, to: ConstDef) -> Result<Self, PatcherError> {
        assert!(from.ty == to.ty, "constant mutation types need to match");
        Ok(MutateConstant {
            from,
            to,
            seen: false,
        })
    }

    pub fn mut_f32(ctx: &Context, from: f32, to: f32) -> Result<Self, PatcherError> {
        let f32_immediate_kind = &spirt::spv::spec::Spec::get()
            .operand_kinds
            .lookup("f32")
            .unwrap();
        let wk = &spirt::spv::spec::Spec::get().well_known;
        let type_f32 = ctx.intern(TypeDef {
            attrs: AttrSet::default(),
            ctor: TypeCtor::SpvInst(wk.OpTypeFloat.into()),
            ctor_args: [].into_iter().collect(),
        });

        let from_const = ConstDef {
            attrs: AttrSet::default(),
            ty: type_f32,
            ctor: ConstCtor::SpvInst(Inst {
                opcode: wk.OpConstant,
                imms: [Imm::Short(*f32_immediate_kind, from.to_bits())]
                    .into_iter()
                    .collect(),
            }),
            ctor_args: [].into_iter().collect(),
        };

        let to_const = ConstDef {
            attrs: AttrSet::default(),
            ty: type_f32,
            ctor: ConstCtor::SpvInst(Inst {
                opcode: wk.OpConstant,
                imms: [Imm::Short(*f32_immediate_kind, to.to_bits())]
                    .into_iter()
                    .collect(),
            }),
            ctor_args: [].into_iter().collect(),
        };

        Self::new(from_const, to_const)
    }

    pub fn mut_i32(ctx: &Context, from: i32, to: i32) -> Result<Self, PatcherError> {
        /*
        let i32_immediate_kind = &spirt::spv::spec::Spec::get()
            .operand_kinds
            .lookup("OpTypeInt")
            .unwrap();*/
        let wk = &spirt::spv::spec::Spec::get().well_known;
        let type_i32 = ctx.intern(TypeDef {
            attrs: AttrSet::default(),
            ctor: TypeCtor::SpvInst(wk.OpTypeInt.into()),
            ctor_args: [].into_iter().collect(),
        });

        let from_const = ConstDef {
            attrs: AttrSet::default(),
            ty: type_i32,
            ctor: ConstCtor::SpvInst(Inst {
                opcode: wk.OpConstant,
                imms: [Imm::Short(
                    wk.LiteralInteger,
                    u32::from_be_bytes(from.to_be_bytes()),
                )]
                .into_iter()
                .collect(),
            }),
            ctor_args: [].into_iter().collect(),
        };

        let to_const = ConstDef {
            attrs: AttrSet::default(),
            ty: type_i32,
            ctor: ConstCtor::SpvInst(Inst {
                opcode: wk.OpConstant,
                imms: [Imm::Short(
                    wk.LiteralInteger,
                    u32::from_be_bytes(to.to_be_bytes()),
                )]
                .into_iter()
                .collect(),
            }),
            ctor_args: [].into_iter().collect(),
        };

        Self::new(from_const, to_const)
    }
}

impl Transformer for MutateConstant {
    fn in_place_transform_module(&mut self, module: &mut spirt::Module) {
        log::error!("Inplace!");
    }
    fn transform_const_def(
        &mut self,
        ct_def: &spirt::ConstDef,
    ) -> spirt::transform::Transformed<spirt::ConstDef> {
        log::error!("Found const!");
        if ct_def == &self.from {
            spirt::transform::Transformed::Changed(self.to.clone())
        } else {
            spirt::transform::Transformed::Unchanged
        }
    }
}

impl Patch for MutateConstant {}
