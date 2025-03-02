#[test]
fn strip_opline() {
    let module =
        spv_patcher::Module::new(include_bytes!("big_debug_shader.spirv").to_vec()).unwrap();

    let before_count = module.spirv().all_inst_iter().count();
    assert_eq!(
        before_count, 499,
        "Initial instruction count does not match"
    );
    let new = module
        .patch()
        .patch(patch_strip_debug::StripDebug {
            strip_op_line: true,
            strip_op_source: false,
        })
        .unwrap()
        .unwrap_module();

    let after_count = new.all_inst_iter().count();

    assert_eq!(
        before_count - 93,
        after_count,
        "removed instruction count does not match"
    );
}

#[test]
fn strip_opsource() {
    let module =
        spv_patcher::Module::new(include_bytes!("big_debug_shader.spirv").to_vec()).unwrap();

    let before_count = module.spirv().all_inst_iter().count();
    assert_eq!(
        before_count, 499,
        "Initial instruction count does not match"
    );
    let new = module
        .patch()
        .patch(patch_strip_debug::StripDebug {
            strip_op_line: false,
            strip_op_source: true,
        })
        .unwrap()
        .unwrap_module();

    let after_count = new.all_inst_iter().count();
    //found this by counting allright :D
    let expected_opline_count = 36;
    assert_eq!(
        before_count - expected_opline_count,
        after_count,
        "removed instruction count does not match"
    );
}

#[test]
fn strip_all() {
    let module =
        spv_patcher::Module::new(include_bytes!("big_debug_shader.spirv").to_vec()).unwrap();

    let before_count = module.spirv().all_inst_iter().count();
    assert_eq!(
        before_count, 499,
        "Initial instruction count does not match"
    );
    let new = module
        .patch()
        .patch(patch_strip_debug::StripDebug {
            strip_op_line: true,
            strip_op_source: true,
        })
        .unwrap()
        .unwrap_module();

    let after_count = new.all_inst_iter().count();
    //found this by counting allright :D
    let expected_opline_count = 36 + 93;
    assert_eq!(
        before_count - expected_opline_count,
        after_count,
        "removed instruction count does not match"
    );
}
