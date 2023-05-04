#let titel = "A Runtime SPIR-V patcher for code specialization of graphics and compute kernel"
#align(center, text(17pt)[
  *#titel*
])
#grid(
  columns: (1fr, 0fr),
  align(center)[
    Tendsin Mende \
    Technische Universität Dresden \
    #link("mailto:tendsin.mende@mailbox.tu-dresden.de")
  ],
)

#let abstract = "
Vulkan, SysCL as well as OpenCL, can be programmed on the GPU using the SPIR-V format. SPIR-V acts as IR between the (high level) programming language (e.g. GLSL, SysCL, OpenCL C / C++) and the graphics driver. The SPIR-V programs must be completely defined before they are passed to the graphics driver. That is, no driver-side linking of program parts can be assumed.

The project tries to extend the concept of specialization constants to specialization code. This allows shader code to be runtime tranformed by user generated content, or procedurally generated content.
"

#align(center)[
  #set par(justify: false)
  *Abstract* \
  #abstract
]



#show: rest => columns(2, rest)



= Motivation


Vulkan as well as OpenCL, the two modern, open graphics and GPGPU APIs, can be programmed on the GPU using the SPIR-V format. SPIR-V acts as IR between the (high level) programming language (e.g. GLSL, SysCL, OpenCL C / C++) and the graphics driver @spir-overview. The SPIR-V programs must be completely defined before they are passed to the graphics driver. That is, no driver-side linking of program parts can be assumed.

SPIR-V's standard includes linking capabilities, but these are not implemented in the high-level graphics frontends (both GLSL and HLSL) @offlinelinking. Furthermore, the planned system could not only link functions, but change whole parts of the program.

For example, in the Godot project it is necessary to redefine code that uses specialization constants to make it DXIL compatible @godotdxil. This cannot be done by linking.

Currently, shader code-specialization (or optimization) is done by compiling every permutation of a shader into a separate file which is loaded at runtime. Source @offlinelinking goes into depth on how those systems work in practice. Apart from their complexity, such systems have the disadvantage, that every possible state must be known at compile time, which is why integrating user-generated content, or procedurally generated content is difficult.

The project tries to extend the concept of specialization constants @sysclspec/@spvspec to _specialization code_. This is to be realised conceptually via a SPIR-V $->$ SPIR-V patch mechanism.


= IR-Analyzis

- SPIR-V is communication format, not necessarly compiler intern (source, this one blog post)
- Compiler side likes LLVM and MLIR
- For some reason drivers and languages (GLSLang, HLSL-frontend to SPIR-V, Rust-GPU don't like)
    - Probably because runtime / distribution
- Runtime / distribution is consideration


== Shader related IRs

- MLIR Dialects (focus on machine learning tho)
- NIR (mesa)
- SPIRT (rustgpu)
- Not sure what glslang and HLSL do internally

== Compiler related IRs

- MLIR has both SPIR-V dialect and generic GPU dialect
- DXIL is basically LLVM + Header

== Decision

$=>$ Operate on SPIR-V directly for most, use SPIR-T to lower to RVSDG for flow-analyzis if needed


= Patches

1. Non-Uniform fix
2. Interface transformation / matching
3. Function Linking / injecting

= Implementation

#lorem(40)
= Testing

#lorem(40)
= Benchmarking

#lorem(40)

test text this is text


This is another text



$ Q = rho A v + C $

#bibliography("works.bib")
