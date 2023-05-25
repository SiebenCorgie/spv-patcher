use patch_function::FunctionFinder;

use crate::test_runs::TestRun;

const FUNC_SPV: &'static [u8] = include_bytes!("../resources/nonuniform_patch.spv");

pub struct FuncFinderTest {
    module: spv_patcher::Module,
}

impl FuncFinderTest {
    pub fn new() -> Self {
        FuncFinderTest {
            module: spv_patcher::Module::new(FUNC_SPV.to_vec()).unwrap(),
        }
    }
}

impl TestRun for FuncFinderTest {
    fn name(&self) -> &'static str {
        "FunctionFinderTest"
    }
    fn run(
        &mut self,
        _blessing: &mut crate::BlessedDB,
        _rmg: &mut marpii_rmg::Rmg,
    ) -> Result<(), crate::test_runs::TestError> {
        let spv = self.module.spirv();
        let found = patch_function::FuncEnumerator::enumerate(spv);
        log::info!("Found: {:#?}", found);

        //Now try try to find those function based on the same parameters
        for f in found {
            if let Some(name) = &f.debug_name {
                let ret =
                    FunctionFinder::find(&spv, &patch_function::FuncIdent::Name(name.to_string()));
                if ret.len() > 0 {
                    log::info!("Found by name {}", name);
                } else {
                    log::error!("Could not find function {} by name", name);
                }
            }
            let ret = FunctionFinder::find(
                &spv,
                &patch_function::FuncIdent::Signature(f.signature.clone()),
            );

            if ret.len() < 1 {
                log::error!("Could not find function based on signature!")
            } else {
                log::info!("Found {} function based on signature!", ret.len());
            }
        }

        Ok(())
    }
}
