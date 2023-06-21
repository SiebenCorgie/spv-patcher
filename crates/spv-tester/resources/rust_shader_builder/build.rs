use std::{
    fs::create_dir_all,
    path::{Path, PathBuf},
};

use spirv_builder::{
    Capability, MetadataPrintout, ModuleResult, SpirvBuilder, SpirvBuilderError, SpirvMetadata,
};

///Builds the shader crate and moves all files to a location that can be found by the renderer's loader.
pub fn compile_rust_shader(
    output_name: &str,
    shader_crate: &str,
    destination_folder: &str,
) -> Result<(), SpirvBuilderError> {
    let shader_crate_location = Path::new(shader_crate).canonicalize().unwrap();
    if !shader_crate_location.exists() {
        println!("cargo:warning=no crate at: {shader_crate_location:?}");
        return Err(SpirvBuilderError::CratePathDoesntExist(
            shader_crate_location,
        ));
    }

    println!("cargo:warning=Building shader {shader_crate_location:?}");

    let spirv_target_location = Path::new(destination_folder).canonicalize().unwrap();

    if !spirv_target_location.exists() {
        create_dir_all(&spirv_target_location).expect("Could not create spirv directory!");
    }

    let compiler_result = SpirvBuilder::new(&shader_crate_location, "spirv-unknown-vulkan1.2")
        .spirv_metadata(SpirvMetadata::Full)
        .print_metadata(MetadataPrintout::None)
        .capability(Capability::Int8)
        .capability(Capability::Int16)
        .capability(Capability::ImageQuery)
        .capability(Capability::RuntimeDescriptorArray)
        //.capability(Capability::GroupNonUniform)
        //.capability(Capability::InputAttachment)
        //.capability(Capability::InputAttachmentArrayDynamicIndexing)
        //.capability(Capability::InputAttachmentArrayNonUniformIndexing)
        .capability(Capability::RuntimeDescriptorArray)
        .capability(Capability::SampledImageArrayDynamicIndexing)
        .capability(Capability::SampledImageArrayNonUniformIndexing)
        .capability(Capability::ShaderNonUniform)
        .capability(Capability::StorageBufferArrayDynamicIndexing)
        .capability(Capability::StorageBufferArrayNonUniformIndexing)
        .capability(Capability::StorageImageArrayDynamicIndexing)
        .capability(Capability::StorageImageArrayNonUniformIndexing)
        .capability(Capability::StorageImageReadWithoutFormat)
        .capability(Capability::StorageImageWriteWithoutFormat)
        //.capability(Capability::UniformBufferArrayDynamicIndexing)
        //.capability(Capability::UniformBufferArrayNonUniformIndexing)
        .capability(Capability::VulkanMemoryModel)
        .build()?;

    println!("cargo:warning=Generated following Spirv entrypoints:");
    for e in &compiler_result.entry_points {
        println!("cargo:warning=    {e}");
    }
    let move_spirv_file = |spv_location: &Path, entry: Option<String>| {
        let mut target = spirv_target_location.clone();
        if let Some(e) = entry {
            target = target.join(format!("{output_name}_{e}.spv"));
        } else {
            target = target.join(format!("{output_name}.spv"));
        }

        println!("cargo:warning=Copying {spv_location:?} to {target:?}");
        std::fs::copy(spv_location, &target).expect("Failed to copy spirv file!");
    };

    match compiler_result.module {
        ModuleResult::MultiModule(modules) => {
            //Note currently ignoring entry name since all of them should be "main", just copying the
            //shader files. Later might use a more sophisticated approach.
            for (entry, src_file) in modules {
                move_spirv_file(&src_file, Some(entry));
            }
        }
        ModuleResult::SingleModule(path) => {
            move_spirv_file(&path, None);
        }
    };
    Ok(())
}

fn glslang_exists() -> bool {
    match std::process::Command::new("glslangValidator").spawn() {
        Ok(_) => true,
        Err(e) => {
            if let std::io::ErrorKind::NotFound = e.kind() {
                false
            } else {
                true
            }
        }
    }
}

fn build_glsl(path: &str, target: &str) {
    //TODO: build all files that do not end with ".glsl". and copy to
    // RESDIR as well.

    if PathBuf::from(target).exists() {
        std::fs::remove_file(target).unwrap();
    }

    let command = std::process::Command::new("glslangValidator")
        .arg("-g")
        .arg("-V")
        .arg(path)
        .arg("-o")
        .arg(target)
        .output()
        .unwrap();

    if !command.status.success() {
        println!(
            "cargo:warning=Out: {:?}",
            std::str::from_utf8(&command.stdout).unwrap()
        );
        println!(
            "cargo:warning=Err: {}",
            std::str::from_utf8(&command.stderr).unwrap()
        );
    }
}

const RESDIR: &str = "..";

// Builds rust shader crate and all glsl shaders.
fn main() {
    println!("cargo:rerun-if-changed=../compute_add.comp");
    println!("cargo:rerun-if-changed=../forward_declare.comp");
    println!("cargo:rerun-if-changed=../nonuniform_patch.comp");
    println!("cargo:rerun-if-changed=../no_inline_function");

    assert!(glslang_exists(), "glslangValidator does not exist. Consider installing it locally in order to be able to compile the GLSL template Shader.");
    //build shader crate. generates a module per entry point
    compile_rust_shader("no_inline_function", "../no_inline_function", RESDIR).unwrap();

    let glsl_files = ["../compute_add.comp", "../nonuniform_patch.comp"];

    for file in &glsl_files {
        let dir = Path::new(file);
        let name = dir
            .file_name()
            .as_ref()
            .unwrap()
            .to_string_lossy()
            .to_string();

        if name.ends_with(".comp") | name.ends_with(".vert") | name.ends_with(".frag") {
            let target = format!("{RESDIR}/{name}.spv");

            println!("cargo:warning=Compiling {:?} to {:?}", dir, target);
            let src_path = dir.canonicalize().unwrap();
            let target_path = PathBuf::from(target);

            build_glsl(src_path.to_str().unwrap(), target_path.to_str().unwrap());
        }
    }
}
