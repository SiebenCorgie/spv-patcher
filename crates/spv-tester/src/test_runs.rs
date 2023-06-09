//! Collects all pre defined tests.

use std::fmt::{Debug, Display};

use crate::{
    const_mutate::ConstMutateTest, dynamic_replace::DynReplaceTest,
    function_finder::FuncFinderTest, non_uniform::NonUniformTest, BlessedDB,
};
use marpii_rmg::{Rmg, RmgError};
use marpii_rmg_tasks::{NoTaskError, TaskError};
use spv_patcher::PatcherError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TestError {
    RmgError(#[from] RmgError),
    PatchError(#[from] PatcherError),
    TaskError(#[from] TaskError<NoTaskError>),
}

impl Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestError::RmgError(e) => write!(f, "{}", e),
            TestError::PatchError(e) => write!(f, "{}", e),
            TestError::TaskError(e) => write!(f, "{}", e),
        }
    }
}

pub trait TestRun {
    ///Human readable name of the test. Also used to find blessed result.
    fn name(&self) -> &'static str;
    fn run(&mut self, blessing: &mut BlessedDB, rmg: &mut Rmg) -> Result<(), TestError>;
}

///Tries to parse a `name` to a test run.
pub fn parse_test_run(name: &str, rmg: &mut Rmg) -> Option<Box<dyn TestRun>> {
    match name {
        "const_mutate" => Some(Box::new(ConstMutateTest::load(rmg).unwrap())),
        "non_uniform_patch" => Some(Box::new(NonUniformTest::load(rmg).unwrap())),
        "function_finder" => Some(Box::new(FuncFinderTest::new())),
        "dyn_replace" => Some(Box::new(DynReplaceTest::load(rmg).unwrap())),
        "const_replace" => Some(Box::new(ConstReplaceTest::load(rmg).unwrap())),
        _ => {
            log::error!("No test named \"{}\" found!", name);
            None
        }
    }
}
