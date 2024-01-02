use rspirv::{
    dr::{Instruction, Operand},
    spirv::Op,
};

use thiserror::Error;

use crate::PatcherError;

use super::{Patch, Patcher};

#[derive(Error, Debug)]
pub enum MemoryModelError {
    #[error("Expected source addressing model: {expected:?}, but was {was:?}")]
    NotExpectedSourceAM {
        expected: rspirv::spirv::AddressingModel,
        was: rspirv::spirv::AddressingModel,
    },
    #[error("Expected source memory model: {expected:?}, but was {was:?}")]
    NotExpectedSourceMM {
        expected: rspirv::spirv::MemoryModel,
        was: rspirv::spirv::MemoryModel,
    },
}

///A patch that lets you change the addressing model & memory model of a module to another one.
pub struct MemoryModel {
    pub from: (rspirv::spirv::AddressingModel, rspirv::spirv::MemoryModel),
    pub to: (rspirv::spirv::AddressingModel, rspirv::spirv::MemoryModel),
}

impl Patch for MemoryModel {
    fn apply<'a>(self, mut patcher: Patcher<'a>) -> Result<Patcher<'a>, PatcherError> {
        //Find the memory model instruction in the SPIR-V module
        let spv = patcher.ir_state.as_spirv();
        if let Some(memmodel) = &mut spv.memory_model {
            //Check addressing model
            if let Some(model) = memmodel.operands.get_mut(0) {
                let current_am = model.unwrap_addressing_model();
                if current_am == self.from.0 {
                    *model = Operand::AddressingModel(self.to.0);
                } else {
                    return Err(PatcherError::Internal(Box::new(
                        MemoryModelError::NotExpectedSourceAM {
                            expected: self.from.0,
                            was: current_am,
                        },
                    )));
                }
            }

            //Check memory model
            if let Some(model) = memmodel.operands.get_mut(1) {
                let current_mm = model.unwrap_memory_model();
                if current_mm == self.from.1 {
                    *model = Operand::MemoryModel(self.to.1);
                } else {
                    return Err(PatcherError::Internal(Box::new(
                        MemoryModelError::NotExpectedSourceMM {
                            expected: self.from.1,
                            was: current_mm,
                        },
                    )));
                }
            }
        } else {
            log::warn!(
                "No memory model was set in module, overwriting to {:?}",
                self.to
            );
            spv.memory_model = Some(Instruction::new(
                Op::MemoryModel,
                None,
                None,
                vec![
                    Operand::AddressingModel(self.to.0),
                    Operand::MemoryModel(self.to.1),
                ],
            ))
        }

        Ok(patcher)
    }
}
