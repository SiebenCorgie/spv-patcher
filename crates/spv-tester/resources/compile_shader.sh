#/bin/sh

glslangValidator -g -V compute_add.comp -o compute_add.spv
glslangValidator -g -V nonuniform_patch.comp -o nonuniform_patch.spv
