use std::collections::VecDeque;

use ahash::{AHashMap, AHashSet};
use rspirv::{
    dr::{Instruction, Operand},
    spirv::{Decoration, Op},
};
use smallvec::SmallVec;

#[derive(Hash, PartialEq, Eq, Debug)]
pub enum TTypeOperand {
    LiteralInt32(u32),
    LiteralInt64(u64),
    LiteralString(String),
    IdRef(u32),
}

impl From<Operand> for TTypeOperand {
    fn from(value: Operand) -> Self {
        match value {
            Operand::IdRef(r) => TTypeOperand::IdRef(r),
            Operand::LiteralInt32(i) => TTypeOperand::LiteralInt32(i),
            Operand::LiteralInt64(i) => TTypeOperand::LiteralInt64(i),
            Operand::LiteralString(s) => TTypeOperand::LiteralString(s),
            _ => panic!("Unparseabel operand {}", value),
        }
    }
}

#[derive(Hash, PartialEq, Eq, Debug)]
pub enum TTDecoration {
    Decorate(Decoration),
    MemberDecorate { member: u32, decoration: Decoration },
}

#[derive(Hash, PartialEq, Eq, Debug)]
pub struct TTypeDef {
    op: rspirv::spirv::Op,
    operands: SmallVec<[TTypeOperand; 2]>,
}

///Tree type. A type is considered equivalent, if its construction parameters (immediate operands), as well as
/// all decorations are the same.
#[derive(Hash, PartialEq, Eq, Debug)]
pub struct TType {
    ///The type ID this type is known under, in the module the type tree was created from.
    ///
    /// Must be the same as `self.type_def.result_type.unwrap()`
    pub src_type_id: u32,
    type_def: TTypeDef,
    pub decorations: SmallVec<[TTDecoration; 3]>,
}

///Module-Context independent tree of all registered types and their decoration.
pub struct TypeTree {
    pub known_types: AHashSet<TType>,
}

impl TypeTree {
    ///Parses all types into a type tree
    pub fn from_module(module: &rspirv::dr::Module) -> Self {
        //iterate over all types. Always checking:
        // 1. Are all dependencies already in the tree:
        // 2. what decoration do we have for that type

        //Pre build decoration lookup
        let mut type_decorations = {
            let mut decorations: AHashMap<u32, SmallVec<[TTDecoration; 3]>> = AHashMap::default();

            for inst in &module.annotations {
                match inst.class.opcode {
                    Op::Decorate => {
                        let id = inst.result_id.unwrap();
                        let decoration = if let Operand::Decoration(d) = inst.operands[1] {
                            d
                        } else {
                            panic!("Decoration operand was not a decoration!");
                        };

                        //Insert or add decoration for that type
                        if let Some(ty_decorations) = decorations.get_mut(&id) {
                            ty_decorations.push(TTDecoration::Decorate(decoration))
                        } else {
                            decorations.insert(
                                id,
                                smallvec::smallvec![TTDecoration::Decorate(decoration)],
                            );
                        }
                    }
                    Op::MemberDecorate => {
                        let id = inst.result_id.unwrap();
                        let member = if let Operand::LiteralInt32(i) = inst.operands[1] {
                            i
                        } else {
                            panic!("MemberDecorate operand[1] was not a literal int32");
                        };
                        let decoration = if let Operand::Decoration(d) = inst.operands[2] {
                            d
                        } else {
                            panic!("Decoration operand was not a decoration!");
                        };

                        //Insert or add decoration for that type
                        if let Some(ty_decorations) = decorations.get_mut(&id) {
                            ty_decorations.push(TTDecoration::MemberDecorate { member, decoration })
                        } else {
                            decorations.insert(
                                id,
                                smallvec::smallvec![TTDecoration::MemberDecorate {
                                    member,
                                    decoration
                                }],
                            );
                        }
                    }
                    Op::DecorateId
                    | Op::DecorateString
                    | Op::MemberDecorateString
                    | Op::GroupMemberDecorate
                    | Op::GroupDecorate => {
                        panic!(
                            "{:?} decoration type not supported (yet)!",
                            inst.class.opcode
                        );
                    }
                    _ => {}
                }
            }

            decorations
        };

        //parse tree for all types
        let type_stack = module.types_global_values.iter().filter(|inst| {
            //Small hack. Instead of figuring out all kinds of OpTypeX (there are many),
            // instead filter out all constants
            ![
                Op::Constant,
                Op::ConstantComposite,
                Op::ConstantFalse,
                Op::ConstantNull,
                Op::ConstantPipeStorage,
                Op::ConstantSampler,
                Op::ConstantTrue,
                Op::SpecConstant,
                Op::SpecConstantComposite,
                Op::SpecConstantFalse,
                Op::SpecConstantOp,
                Op::SpecConstantTrue,
            ]
            .contains(&inst.class.opcode)
        });

        //At first, cache type key as well, later drop that for the final tree
        let mut tmp_tree = AHashMap::default();

        //Post stack that allows us to *deffere* a type to a later stage
        let mut type_stack: VecDeque<&Instruction> = type_stack.collect();
        loop {
            let stack_len = type_stack.len();
            if stack_len == 0 {
                break;
            }
            //Swap into tmp for working on the stack
            let mut tmp_stack = VecDeque::new();
            std::mem::swap(&mut type_stack, &mut tmp_stack);

            for ty in tmp_stack {
                for operand in &ty.operands {
                    if let Operand::IdRef(r) = &operand {
                        if !tmp_tree.contains_key(r) {
                            log::info!("Deferring operand with id {:?}", r);
                            type_stack.push_back(ty);
                        }
                    }
                }

                //passed operand check, therefore collect all
                tmp_tree.insert(
                    ty.result_id.as_ref().unwrap(),
                    TType {
                        type_def: TTypeDef {
                            op: ty.class.opcode,
                            operands: ty.operands[1..]
                                .iter()
                                .map(|o| TTypeOperand::from(o.clone()))
                                .collect(),
                        },
                        src_type_id: *ty.result_id.as_ref().unwrap(),
                        decorations: if let Some(dec) =
                            type_decorations.remove(ty.result_id.as_ref().unwrap())
                        {
                            dec.into()
                        } else {
                            SmallVec::new()
                        },
                    },
                );
            }

            if stack_len == type_stack.len() {
                panic!("stack could not be reduced!");
            }
        }

        //now build the actual *TreeSet*
        TypeTree {
            known_types: tmp_tree.into_iter().map(|(_key, value)| value).collect(),
        }
    }
}
