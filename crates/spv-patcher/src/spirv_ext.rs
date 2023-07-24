//! Extensions to SPIR-V module. Adds querying capability and analysis.
use rspirv::{
    dr::{Instruction, Operand},
    spirv::{Capability, Decoration, ExecutionModel, Op},
};

use crate::type_tree::TypeTree;

pub trait SpirvExt {
    ///Returns true if the extension is loaded in that module
    fn has_extension(&self, ext: &str) -> bool;
    ///Decorates the given ID.
    fn decorate(&mut self, id: u32, decoration: Decoration);
    fn add_capability(&mut self, capability: Capability);
    fn remove_capability(&mut self, capability: Capability);

    ///Tries to find the assignment instruction for a given name.
    /// Given an `OpName %x name`, tries to return the instruction of `%x = ...`;
    ///
    /// You might use this to identify functions, type or named variables.
    fn get_by_name(&self, name: &str) -> Option<&Instruction>;

    ///Tries to find a debug name (`OpName`) for the given instruction id.
    fn get_name(&self, id: u32) -> Option<String>;

    ///Returns the execution model of this Module. Note that we, per-definition, only have one entry point per [Module](crate::Module).
    fn get_execution_model(&self) -> ExecutionModel;

    fn build_type_tree(&self) -> TypeTree;
}

impl SpirvExt for rspirv::dr::Module {
    fn has_extension(&self, ext: &str) -> bool {
        //NOTE: why are they not in module.extensions?
        for inst in self.global_inst_iter() {
            match inst.class.opcode {
                Op::SourceExtension => {
                    if inst.operands[0] == Operand::LiteralString(ext.to_string()) {
                        return true;
                    }
                }
                Op::Extension => {
                    if inst.operands[0] == Operand::LiteralString(ext.to_string()) {
                        return true;
                    }
                }
                _ => {}
            }
        }

        false
    }

    fn get_execution_model(&self) -> ExecutionModel {
        let exmodel = match self.entry_points[0].operands[0] {
            Operand::ExecutionModel(m) => m,
            _ => panic!("Entry point had no execution mode"),
        };

        exmodel
    }

    fn decorate(&mut self, id: u32, decoration: Decoration) {
        //TODO: Do we want to implement decoration checking here? For instance check that only
        //      BlockDecorated uniforms are decorated non-uniform.

        self.annotations.push(Instruction::new(
            Op::Decorate,
            Some(id),
            None,
            vec![Operand::Decoration(decoration)],
        ))
    }

    fn add_capability(&mut self, capability: Capability) {
        let has_capability = self.capabilities.iter().fold(false, |found, cap| {
            if found {
                true
            } else {
                assert!(cap.class.opcode == Op::Capability);
                //NOTE: must be capability
                cap.operands[0].unwrap_capability() == capability
            }
        });
        if !has_capability {
            self.capabilities.push(Instruction::new(
                Op::Capability,
                None,
                None,
                vec![Operand::Capability(capability)],
            ));
        }
    }

    fn remove_capability(&mut self, capability: Capability) {
        self.capabilities.retain(|inst| {
            if let Operand::Capability(c) = inst.operands[0] {
                if c == capability {
                    false
                } else {
                    true
                }
            } else {
                true
            }
        })
    }

    fn get_by_name(&self, name: &str) -> Option<&Instruction> {
        let mut search_id = None;
        for inst in self.debug_names.iter() {
            if let (Some(id), Operand::LiteralString(dbg)) =
                (inst.operands[0].id_ref_any(), &inst.operands[1])
            {
                if dbg == name {
                    search_id = Some(id);
                    break;
                }
            }
        }

        if let Some(search_id) = search_id {
            for inst in self.all_inst_iter() {
                if inst.result_id == Some(search_id) {
                    return Some(inst);
                }
            }
            None
        } else {
            None
        }
    }

    fn get_name(&self, id: u32) -> Option<String> {
        for inst in self.debug_names.iter() {
            if inst.operands[0].id_ref_any() == Some(id) {
                if let Operand::LiteralString(s) = &inst.operands[1] {
                    return Some(s.clone());
                }
            }
        }
        None
    }

    fn build_type_tree(&self) -> TypeTree {
        TypeTree::from_module(self)
    }
}
