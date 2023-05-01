//TODO Patch ideas:
//     make-var-const: Rewrite variable to constant. Possibly fold afterwards?
//     link kernel-in-kernel: takes a kernel, and tries to embedded another kernel into it by linking the host's data to the former input
//                            parameters of the to-be-embedded kernel.
//     max-iteration-count: augments a selected loop with a counter that makes it quit after a constant amount of loops (good
//                          for debugging gpu hangs I'd say)
//     replace-function: takes a function by-id and replaces its content based on the supplied spirv. Verifies that
//                       the function's interfaces match. Basically linking, but for already defined functions.
//     link: Simple link pass, basically what `spv-link` does. Searches for imports and replaces them with export-marked
//           code of a given module. Ref: https://github.com/KhronosGroup/SPIRV-Tools/blob/main/source/link/linker.cpp
//     inject-function: replaces a variable assignment with a given function that returns the assign varible's type.
//                      Can use context information to wire function's arguments.

//! Patch utilities and pre-implemented patches.

//Test patch
// TODO: Remove in favor of *correct* patches
mod mutate_constant;

use crate::PatcherError;
pub use mutate_constant::MutateConstant;
use rspirv::binary::Assemble;
use std::rc::Rc;

pub trait Patch {
    fn apply<'a>(self, patcher: Patcher<'a>) -> Result<Patcher<'a>, PatcherError>;
}

///Represents current internal IR state.
pub enum IrState {
    SpirV(rspirv::dr::Module),
    SpirT {
        ctx: Rc<spirt::Context>,
        module: spirt::Module,
    },
}

impl IrState {
    pub fn is_spirv(&self) -> bool {
        if let IrState::SpirV(_) = self {
            true
        } else {
            false
        }
    }

    ///Returns current state as SpirV-IR. Might translate if needed.
    pub fn as_spirv(&mut self) -> &mut rspirv::dr::Module {
        if let IrState::SpirT { module, .. } = self {
            let emitter = module
                .lift_to_spv_module_emitter()
                .expect("Could not lift SPIR-T to SPIR-V");
            let spv = rspirv::dr::load_words(&emitter.words).unwrap();

            *self = IrState::SpirV(spv);
        };

        //Return module
        if let IrState::SpirV(s) = self {
            s
        } else {
            panic!("Failed to convert to spirv")
        }
    }

    pub fn as_spirt(&mut self) -> &mut spirt::Module {
        if let IrState::SpirV(spv) = self {
            let ctx = Rc::new(spirt::Context::new());
            let spv_bytes: Vec<u8> = bytemuck::allocation::cast_vec(spv.assemble());
            let module = spirt::Module::lower_from_spv_bytes(ctx.clone(), spv_bytes).unwrap();
            *self = IrState::SpirT { ctx, module }
        }

        if let IrState::SpirT { ctx: _, module } = self {
            module
        } else {
            panic!("Failed to lower to spir-t")
        }
    }
}

pub struct Patcher<'module> {
    pub module: &'module crate::Module,
    pub ir_state: IrState,
}

impl<'module> Patcher<'module> {
    pub fn print(self) -> Self {
        match &self.ir_state {
            IrState::SpirT { ctx: _, module } => {
                print!(
                    "SPIR-T:\n{}",
                    spirt::print::Plan::for_module(module).pretty_print()
                );
            }
            IrState::SpirV(spv) => {
                println!("SPIR-V\n{:#?}", spv);
            }
        }
        self
    }

    pub fn patch(self, to_apply: impl crate::patch::Patch) -> Result<Self, PatcherError> {
        to_apply.apply(self)
    }

    ///Assembles the patched spriv code. Can be directly loaded into a OpenCL or OpenGL pipeline.
    pub fn assemble(self) -> Vec<u32> {
        match self.ir_state {
            IrState::SpirT { ctx: _, module } => module.lift_to_spv_module_emitter().unwrap().words,
            IrState::SpirV(spv) => spv.assemble(),
        }
    }
}
