use spv_patcher::patch::Patch;

pub struct StripDebug {
    ///If true, all OpLines will be stripped from the module
    pub strip_op_line: bool,
    ///If true, strips OpSource, and related OpStrings from the module
    pub strip_op_source: bool,
}

impl Patch for StripDebug {
    fn apply<'a>(
        self,
        mut patcher: spv_patcher::patch::Patcher<'a>,
    ) -> Result<spv_patcher::patch::Patcher<'a>, spv_patcher::PatcherError> {
        let spv = patcher.ir_state.as_spirv();

        if self.strip_op_line {
            for function in &mut spv.functions.iter_mut() {
                for block in function.blocks.iter_mut() {
                    //retain only _none-OpLine_
                    block
                        .instructions
                        .retain(|inst| inst.class.opcode != spv_patcher::rspirv::spirv::Op::Line);
                }
            }
        }

        if self.strip_op_source {
            let mut encountered_strings = Vec::new();
            spv.debug_string_source.retain(|inst| {
                if inst.class.opcode == spv_patcher::rspirv::spirv::Op::Source {
                    //if there is a op_string id attached, store that
                    if let Some(spv_patcher::rspirv::dr::Operand::IdRef(id)) = inst.operands.get(2)
                    {
                        encountered_strings.push(*id);
                    }
                    false
                } else {
                    true
                }
            });

            //now strip all _dead_ op-strings
            spv.debug_string_source.retain(|instr| {
                if let Some(result_id) = instr.result_id {
                    !encountered_strings.contains(&result_id)
                } else {
                    true
                }
            });
        }

        Ok(patcher)
    }
}
