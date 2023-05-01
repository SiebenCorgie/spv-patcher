//! Extensions to SPIR-V module. Adds querying capability and analysis.

use rspirv::{
    dr::{Instruction, Operand},
    spirv::{Op, Word},
};
pub struct InstructionTree {}

pub trait SpirvExt {
    ///Returns true if the extension is loaded in that module
    fn has_extension(&self, ext: &str) -> bool;

    ///Tries to find the assignment instruction for a given name.
    /// Given an `OpName %x name`, tries to return the instruction of `%x = ...`;
    ///
    /// You might use this to identify functions, type or named variables.
    fn get_by_name(&self, name: &str) -> Option<&Instruction>;

    fn get_participating_trees(&self, id: Word) -> Option<InstructionTree>;
}

impl SpirvExt for rspirv::dr::Module {
    fn has_extension(&self, ext: &str) -> bool {
        //NOTE: why are they not in module.extensions?
        for inst in self.global_inst_iter() {
            if inst.class.opcode == Op::SourceExtension {
                if inst.operands[0] == Operand::LiteralString(ext.to_string()) {
                    return true;
                }
            }
        }

        false
    }

    fn get_by_name(&self, name: &str) -> Option<&Instruction> {
        None
    }

    fn get_participating_trees(&self, id: Word) -> Option<InstructionTree> {
        None
    }
}