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

mod mutate_constant;
pub use mutate_constant::MutateConstant;
use spirt::{transform::Transformer, visit::Visitor};

///A patch is just defined as something that is both, a visitor and a tranformer.
///
/// The patcher guarantees that a module is visited before transformed. This makes it possible to first collect information
/// before actually patching.
pub trait Patch: Transformer {}
