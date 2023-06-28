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

#show link: underline

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


== Function injection
=== Handling functions in shader code

Shaders/Kernels are often small, GPU specialised programs. A property of this GPU-Specialisation is that the programmer, and compiler try to eliminate invocation-group wide controll-flow divergence. This combination often results in highly inline programs with few function calls.

SPIR-V contains a `DontInline` hint, but this cannot be used in all front ends. Namely GLSLang (the GLSL-compiler) which is often used for Shader programming does not contain a way to annotate functions in any way.

We therefore have a problem when it comes to identifying a certain callsite in SPIR-V modules. Because those might already be inlined.

The solution is to come up with two versions of the function-patching mechanism.

1. Linking-like replacement of non-inlined functions
2. Injecting custom function call as variable assignment

=== Linking and replacing

Both following passes rely on a common function-identification. We allow two types. Identification by type-signature, and identification by debug-name-string. The latter is similar to the way a linker might use a identification string for dynamic linking of two functions. The functionality can be found in `crates/patch-function/src/function_finder.rs`

For injection we have to distinguish two versions:

1. Merging known code into an known template. This is similar to standart linking. We'll call this _Constant Replacement_
2. Loading a template, and, (possibly based on its content) injecting new code. We'll call this _Dynamic Replacement_

==== Constant Replacement

_The described code can be found in `crates/patch-function/src/constant_replace.rs` _

Since constant replacement is similar to linking, we use the `spirv-link` program provided by Khronos for the most part. Since our patcher can not rely on the correct linking annotations in the template code, as well as the _to be injected_ code, we start by modifying both with the correct linking annotations.

Afterwards we assemble both into byte-code and let the linker work. The resulting module is read back into the patcher.


Right now this relies on three files being written (both input files and the output file), which leads to subpar performance compared to the in-memory patches.

==== Dynamic Replacement

After identifying our _to be replaced_ function, we copy the function's definition and start a new basic-block. At this point we hand over control flow to a user specified function (or _closure_ in Rust terms), that is free to append any custom code. The function is supplied with knowledge about the provided parameters, as well as the expected return type.

In a last pass all call sites of the former function are replaced with a call to the new function. Since all parameters, return IDs etc. do match by definition, this is safe to do.


There are two advantages to the _Constant Replacement_ approach. The first is our runtime knowledge over the whole SPIR-V-Module. We don't need to merge different modules (meaning type-ids, capabilities, extensions etc.), instead we can naturally use and extend the template module.

The second advantage is the in-memory nature of the patch. The template is already loaded and can be mutated largely in-place. In theory this is also possible for the _Constant Replacement_ patch, but for that we'd have to re-implement the `spirv-link` application.

=== Callsite injection

Sadly I didn't have time to implement the call-site injection methode. While the injection itself wouldn't be too difficult, identifying _where_ to place the call-site is more complex. We can't rely on any signature matching, since any assigned variable type can potentially be used multiple times. The only relyable way I found is, by relying on debug information (specifically the `OpLine` instruction of SPIR-V) to identify the correct source-code line, and then searching for the correct assignment operation.


= Testing

#lorem(40)
= Benchmarking

Therea are two _domains_ we can test. One is the CPU side, where we mostly measure how long any of the patches takes. The other side is GPU-runtime of the resulting code after patching.
The `spv-benchmark` application reflects that. When running the app, only the GPU-side benchmarks are executed. For the CPU-side we use #link("https://github.com/bheisler/criterion.rs")[Criterion.rs]. In can be invoked via `cargo bench`, which will run the benchmark and generate a HTML report at `target/criterion/report/index.html`.

All patching benchmarks take a _simple_ code template that calculates distance of a pixel to an images center, and replace that with a simple #link("https://en.wikipedia.org/wiki/Mandelbrot_set")[mandelbrot set] calculation.


== CPU

=== Constant Replace



=== Dynamic Replace

== GPU

=== Constant Replace

=== Dynamic Replace

= Conclusion

#bibliography("works.bib")
