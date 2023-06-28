#/bin/sh

# Build rust shaders
cargo build
# then try to run GLSL->SPIR-V compiler
glslangValidator -g -V compute_add.comp -o compute_add.spv
glslangValidator -g -V nonuniform_patch.comp -o nonuniform_patch.spv
glslangValidator -g -V forward_declare.comp -o forward_declare.spv
