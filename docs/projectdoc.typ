#import "notes.typ"


#set page(
    paper: "a4",
    footer: notes.display(), // Footnotes
    footer-descent: 0pt
)

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



/* #show: rest => columns(2, rest) */



= Motivation


Vulkan as well as OpenCL, the two modern, open graphics and GPGPU APIs, can be programmed on the GPU using the SPIR-V format. SPIR-V acts as IR between the (high level) programming language (e.g. GLSL, SysCL, OpenCL C / C++) and the graphics driver @spir-overview. The SPIR-V programs must be completely defined before they are passed to the graphics driver. That is, no driver-side linking of program parts can be assumed.

SPIR-V's standard includes linking capabilities, but these are not implemented in the high-level graphics frontends (both GLSL and HLSL) @offlinelinking. Furthermore, the planned system could not only link functions, but change whole parts of the program.

For example, in the Godot project it is necessary to redefine code that uses specialisation constants to make it DXIL compatible @godotdxil. This cannot be done by linking.

Currently, shader code-specialisation (or optimisation) is done by compiling every permutation of a shader into a separate file which is loaded at runtime. Source @offlinelinking goes into depth on how those systems work in practice. Apart from their complexity, such systems have the disadvantage, that every possible state must be known at compile time, which is why integrating user-generated content, or procedurally generated content is difficult.

The project tries to extend the concept of specialisation constants @sysclspec/@spvspec to _specialisation code_. This is to be realised conceptually via a SPIR-V $->$ SPIR-V patch mechanism.


= IR-Analysis

A seen by Khronos documentation, SPIR-V is intended as a _communication format_ between compiler infrastructure (at compile time) and driver infrastructure at runtime.

#figure(
  image("2020-spir-landing-page-01_2.jpg", width: 90%),
  caption: [
      SPIR-V Language Ecosystem \ https://www.khronos.org/spir/
  ],
) <sprifig>


None of its stated goals (as seen in section _1.1 Goals_ of the specification @spvspec) contain strictly compiler related transformation goals. Instead it focuses on stability, easy parseability and easy translation from and into other IR formats.

As a result most compilers and drivers use another internal IR to do either compilation to SPIR-V, or from SPIR-V to GPU specific code.


As @sprifig shows, multiple languages as well as compiler infrastructures like LLVM and MLIR have the capability to compile to SPIR-V.
On the other site compute and graphics APIs like Vulkan or OpenCL consume SPIR-V directly, or translate it into other intermediate formats like DXIL before supplying it to the API.
Internally at least Linux's MESA driver uses another custom IR, called NIR @nir, to translate SPIR-V to the actual GPU code.

Another interesting opensource shader-compiler is the _AMD compiler_ (ACO) within mesa as well @aco. It is a backend to the former mentioned NIR specifically for AMD-Hardware.


Conceputally we can split shader related IRs based on their position relation to SPIR-V. On one hand we have compilation focused IRs like LLVM, MLIR or, the more shader oriented IRs like SPIR-T. On the other hand we have runtime GPU-Code generation focused IRs like NIR.

/*
- SPIR-V is communication format, not necessarly compiler intern (source, this one blog post)
- Compiler side likes LLVM and MLIR
- For some reason drivers and languages (GLSLang, HLSL-frontend to SPIR-V, Rust-GPU don't like)
    - Probably because runtime / distribution
- Runtime / distribution is consideration
*/


== Compiler related IRs

On the compiler site we have roughly two aproaches to translating a highlevel language to SPIR-V.
First we have common LLVM based compiler stacks like SYSCL's. Secondly we have more monolitic aproaches like GLSL's and HLSL's stack. An observation is, that GPGPU related languages seem to favor the LLVM (or MLIR) stacks, while graphics related languages favor a custom monolitic stack.

While I couldn't make out a single common reason for this, two main factors play a roll. The first one being controll over the compiler stack, including (simple) distribution and design decissions (See #notes.note[_In defense of NIR_][https://www.gfxstrand.net/faith/blog/2022/01/in-defense-of-nir/] for better explaination). The second being simplicity. Graphics shader are often focused on a certain kind of work (like fragement shading, vertex tranformation etc.). Therfore, more informed tranformations can be implemented directly, compared to general-purpose GPU programs.

/*
- IREE: ML related IR _above_ SPIR-V
- MLIR has both SPIR-V dialect and generic GPU dialect
- DXIL is basically LLVM + Header
- NV-PTX(?)
*/
== Shader related IRs

- MLIR Dialects (focus on machine learning tho)
- NIR (mesa)
- SPIRT (rustgpu)
- Not sure what glslang and HLSL do internally


== Decision

$=>$ Operate on SPIR-V directly for most, use SPIR-T to lower to RVSDG for flow-analyzis if needed


= Patches

== NonUniform decoration
=== Problem description
Currently parts of the program are analysed by the driver (see ACO description) for diverging execution. Others have to be explicitly tagged by the
programmer. Mostly when descriptors are indexed non-uniformly.
In GLSL this is done via `nonuniformEXT(int i)`. For instance like this:


```C
layout(location = 0) flat in int i;
layout(set = 0, binding = 0) uniform sampler2D tex[2];
/*void main(){...*/
vec4 color = texture(tex[nonuniformEXT(i)], ...);
/*...}*/
```

This effectively marks the index `i` as _possibly different per invocation group_. However in practice this has several problems:
+ When this is needed is not always easy to see
+ When forgotten, bugs are subtil
+ Some drivers seem to handle it well if forgotten.

=== Intuitive solution
The first observation is, that only descriptor indexing related instructions need to be marked `NonUniform`. Therefore, the pass does not have to explore all indexing, but just the ones indexing into descriptors bindings.

A second observation is, that _per-invocation non-uniform indexing_ has a finite count of sources. One is non-uniform control flow, the other is non-uniform input variables. The latter is found by tracing the index calculation for known non-uniform input variables like `invocation-index` or `vertex-index` etc.

//TODO: checkout if SPIR-T can help here.

Finding non-uniform control-flow is not as easy though. The ACO compiler actually does most of its work in that are. Therefore, we reverse the problem and decorate _every_ descriptor_indexing as _NonUniform_ by default, and just remove the decoration, if we are absolutely sure that it isn't needed.

TODO: Benchmark the result for performance regression.

=== Implementation

The implementation has four stages:

1. find and cache variables with an potentially non-uniform value (`seed_variables`)
2. find all `OpAccessChain` OPs that access a RuntimeArray, and trace the indices. (`trace_indices`)
3. decorate `OpAccessChain` on RuntimeArrays with non-uniform index (`decorate`)
4. fix module extensions and capabilities by adding `SPV_EXT_descriptor_indexing` and all non-uniform capabilities (`fix_capabilities_and_extensions`)

In practice some of the added capabilities might not be needed. However, they do not influence the resulting ISA-code, since capabilities only signal the possibility of non-uniform access.
The pass might decorate _too many_ access as `NonUniform`. The performance implication must be tested in the benchmark.

=== Performance comparison

Todo:
- Compare pass-decorated vs hand-decorated for runtime performance
- Check time needed for the patch


== Shader interface transformation
=== Input / Output matching
==== Problem description
==== Reference and implementation
=== Binding specification
==== Problem description
==== Binding description Vulkan
==== Binding description OpenCL
==== Implementation

== Function injection
=== Handling functions in shader code

Shaders/Kernels are often small, GPU specialised programs. A property of this GPU-Specialisation is that the programmer, and compiler try to eliminate invocation-group wide controll-flow divergence. This combination often results in highly inline programs with few function calls.

SPIR-V contains a `DontInline` hint, but this cannot be used in all front ends. Namely GLSLang (the GLSL-compiler) which is often used for Shader programming does not contain a way to annotate functions in any way.

We therefore have a problem when it comes to identifying a certain callsite in SPIR-V modules. Because those might already be inlined.

The solution is to come up with two versions of the function-patching mechanism.

1. Linking-like replacement of function
2. Injecting custom function call as variable assignment

=== Linking or replacing

This patch is the simplest methode. We enumerate all found functions in a module and let the _patcher_ decide which function's body is replaced.
The patch only needs to match the function's signature and return type.

==== Implementation

- known function enumeration
- argument id matching to new code

=== Injection

When injecting we insert the custom function into the module. The _patcher_ can then select a variable (or multiple) in the _main controll flow_ with a type that matches the function's return type. The correct variable might be identifyable by debug information like a variable name string.
The patch then injects custom code that executes the inserted function and writes the result to the selected variable.

==== Implementation

- variable enumeration based on return type
- DCE pass to remove unneeded code for mutated variable

= Testing

#lorem(40)
= Benchmarking

= Conclusion

#bibliography("works.bib")
