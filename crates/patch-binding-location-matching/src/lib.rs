//! # Binding location matching
//!
//! Custom patching pass that matches the output of one entrypoint, to the input parameters of another entrypoint.
//! Heavily inspired by [spirv-location-injector](https://github.com/expenses/spirv-location-injector), used for testing the patching mechanism and ergonomics.
#![deny(warnings)]

pub struct BindingLocationMatching {}
