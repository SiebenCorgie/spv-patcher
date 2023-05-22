//! # Function patching.
//!
//! There are two types of patches here:
//! 1. Linking / Replacing: Either (statically) links a function, or replaces an already known function with new code.
//! 2. Assignment rewrite: Rewrites a known variable to be assigned base on a supplied function. This pass requires the user to
//!    route possibly needed arguments to the function.
#![deny(warnings)]

///Replaces a known function (or import-marked function) with a given function.
mod link_replace;

///Assingment rewrite of some value by function calling. Only the return type has to match the
/// rewritten variables type, but arguments have to be routed.
mod assignment_rewrite;

mod function_finder;

pub use assignment_rewrite::AssignmentRewrite;
pub use function_finder::{FuncIdent, FunctionFinder};
pub use link_replace::LinkReplace;
