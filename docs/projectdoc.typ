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

The project tries to extend the concept of specialization constants to specialization code. This allows shader code to be runtime transformed by user generated content, or procedurally generated content.
"

#align(center)[
  #set par(justify: false)
  *Abstract* \
  #abstract
]

#show link: underline

/* #show: rest => columns(2, rest) */



= Motivation


Vulkan as well as OpenCL, two modern, open graphics and GPGPU APIs, can be programmed on the GPU using the SPIR-V format. SPIR-V acts as IR between the (high level) programming language (e.g. GLSL, SysCL, OpenCL C / C++) and the graphics driver @spir-overview. The SPIR-V programs must be completely defined before they are passed to the graphics driver. That is, no driver-side linking of program parts can be assumed.

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


Conceptually we can split Shader related IRs based on their position in relation to SPIR-V. On one hand we have compilation focused IRs like LLVM, MLIR or, the more shader oriented IRs like SPIR-T. On the other hand we have runtime GPU-Code generation focused IRs like NIR.

/*
- SPIR-V is communication format, not necessarily compiler intern (source, this one blog post)
- Compiler side likes LLVM and MLIR
- For some reason drivers and languages (GLSLang, HLSL-frontend to SPIR-V, Rust-GPU don't like)
    - Probably because runtime / distribution
- Runtime / distribution is consideration
*/


== Compiler related IRs

On the compiler site we have roughly two approaches to translating a highlevel language to SPIR-V.
First we have common LLVM based compiler stacks like SYSCL's. Secondly we have more monolithic approaches like GLSL's and HLSL's stack. An observation is, that GPGPU related languages seem to favour the LLVM (or MLIR) stacks, while graphics related languages favour a custom monolithic stack.

While I couldn't make out a single common reason for this, two main factors play a roll. The first one being controll over the compiler stack, including (simple) distribution and design decisions (See #notes.note[_In defense of NIR_][https://www.gfxstrand.net/faith/blog/2022/01/in-defense-of-nir/] for better explanation). The second being simplicity. Graphics shader are often focused on a certain kind of work (like fragment shading, vertex transformation etc.). Therefore, more informed transformation's can be implemented directly, compared to general-purpose GPU programs.

Another reason for which I couldn't find a citeable source is the history of Shader compilers. Only the latest graphics and GPGPU APIs target some kind of byte-format as input. APIs before that where either semi-non-programmable (DirectX up to version 8, and OpenGL until version 2.0), or took actual programme code as input(DirectX until version 12 which introduces DXIL, OpenGL until version 4.6 which introduces SPIR-V capabilities similar to Vulkan). The compiler would therefore recite within the driver stack. This has two implications.

1. The compiler must be shipped with the driver
2. The compiler must be fast enough to compile the code to executable GPU-Code at runtime.

/*
- IREE: ML related IR _above_ SPIR-V
- MLIR has both SPIR-V dialect and generic GPU dialect
- DXIL is basically LLVM + Header
- NV-PTX(?)
*/
== Shader related IRs


For Shader focused IRs we have specialised IRs for programming languages like MLIR dialects (IREE or the SPIR-V dialect) as well as custom solutions like SPIR-T which is used internally in Rust-Gpu. They focus mostly on specialising code for GPU usage. Interestingly we can see that OpenSource driver stack have their own internal IRs that compile SPIR-V (or some other communication IR format) down to the actual ISA instructions. Two notable toolchains are Mesa's NIR, and a special Shader compiler for AMD's ISA called ACO (#notes.note[_AMD_Compiler_][https://gitlab.freedesktop.org/mesa/mesa/-/blob/main/src/amd/compiler/README.md])


== Decision


I decide to use SPIR-V directly for the most part. If I need to do more complex Shader code transformation's I'll try to use SPIR-T, which is not that different to Mesa's NIR. The lifting and lowering are proven to be fast (I spoke to the main developer _eddyb_).

I decided against lifting to MLIR because the patching mechanism is in its nature a rather technical procedure. I anticipate that I wouldn't gain the right type of flexibility I'd need. This only means the _patching_ part though. MLIR would probably be the right choice if I want to compile some DSL down to SPIR-V, which then gets patched into a template SPIR-V program.


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


Finding non-uniform control-flow is not as easy. The ACO compiler actually does most of its work in that are. Therefore, we reverse the problem and decorate _every_ descriptor_indexing as _NonUniform_ by default, and just remove the decoration, if we are absolutely sure that it isn't needed.


=== Implementation

The implementation has four stages:

1. find and cache variables with an potentially non-uniform value (`seed_variables`)
2. find all `OpAccessChain` OPs that access a RuntimeArray, and trace the indices. (`trace_indices`)
3. decorate `OpAccessChain` on RuntimeArrays with non-uniform index (`decorate`)
4. fix module extensions and capabilities by adding `SPV_EXT_descriptor_indexing` and all non-uniform capabilities (`fix_capabilities_and_extensions`)

In practice some of the added capabilities might not be needed. However, they do not influence the resulting ISA-code, since capabilities only signal the possibility of non-uniform access.
The pass might decorate _too many_ access as `NonUniform`.


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


_The described code can be found in `crates/patch-function/src/dynamic_replace.rs` _

After identifying our _to be replaced_ function, we copy the function's definition and start a new basic-block. At this point we hand over control flow to a user specified function (or _closure_ in Rust terms), that is free to append any custom code. The function is supplied with knowledge about the provided parameters, as well as the expected return type.

In a last pass all call sites of the former function are replaced with a call to the new function. Since all parameters, return IDs etc. do match by definition, this is safe to do.


There are two advantages to the _Constant Replacement_ approach. The first is our runtime knowledge over the whole SPIR-V-Module. We don't need to merge different modules (meaning type-ids, capabilities, extensions etc.), instead we can naturally use and extend the template module.

The second advantage is the in-memory nature of the patch. The template is already loaded and can be mutated largely in-place. In theory this is also possible for the _Constant Replacement_ patch, but for that we'd have to re-implement the `spirv-link` application.

=== Callsite injection

Sadly I didn't have time to implement the call-site injection methode. While the injection itself wouldn't be too difficult, identifying _where_ to place the call-site is more complex. We can't rely on any signature matching, since any assigned variable type can potentially be used multiple times. The only relyable way I found is, by relying on debug information (specifically the `OpLine` instruction of SPIR-V) to identify the correct source-code line, and then searching for the correct assignment operation.


= Testing

== Code-Validation

Before running the generated code, we can validate it against the SPIR-V specification. For that a CLI programme is provided by Khronos called `spirv-val`. It simply takes a shader module on stdin (or as a file) and outputs an error if any invalid code was found. In theory any module passing that check should be valid SPIR-V code and therefore be executable. In practice `spirv-val` does not catch all errors, which is the reason for enabling Vulkan's runtime validation as well.

== Runtime Validation

The second validation happens at runtime. We have two ways of checking the validity of the shader module. One is Vulkan's validation layers. They are mainly used to check that a Vulkan application conforms to the Vulkan specification. However, it also detects runtime invalid shader execution, or malformed code. We check that by turning on the validation layer and routing error output into our test.

Secondly, we check that the test returns the expected output. For that, we download the test run's result and check it against our _blessed_ results. Those _blessed_ results are a list of prior saved results from which we know that they are correct. This lets us ultimately find regression for code that is _valid_ in itself, but produces an incorrect (or unexpected) output.

= Benchmarking

There are two _domains_ we can test. One is the CPU side, where we mostly measure how long any of the patches takes. The other side is GPU-runtime of the resulting code after patching.
The `spv-benchmark` application reflects that. When running the app, only the GPU-side benchmarks are executed. For the CPU-side we use #link("https://github.com/bheisler/criterion.rs")[Criterion.rs]. In can be invoked via `cargo bench`, which will run the benchmark and generate a HTML report at `target/criterion/report/index.html`.

All patching benchmarks take a _simple_ code template that calculates the distance of a pixel to an image's center, and replace that with a simple #link("https://en.wikipedia.org/wiki/Mandelbrot_set")[mandelbrot set] calculation.


== Hardware note

All numbers measured in the following benchmark are taken on a AMD Ryzen 5900X (12 core/24 threads \@ 3.7GHz up to 4.8GHz). The GPU is a AMD 6800XT, the GPU benchmark uses Vulkan and the latest open-source Mesa/RADV driver.

== CPU

=== GLSL Compilation time

For reference to _patching_ we also measure a full compilation of the mandelbrot-set shader via `glslangValidator`. The shader that is compiled can be found in `crates/spv-benchmark/resources/mandelbrot.comp`.



#figure(
  image("glslcompiletime.png", width: 90%),
  caption: [
      GlslangValidator compile time for the Mandelbrot compute shader.
  ],
) <glslct>

The mean compiletime is 64.4ms with a standard deviation of 2ms. Therefore, for the patcher to make sense compile-time wise, we need to be faster than ~64ms. Otherwise a standard recompilation would make more sense than patching.


=== Constant Replace

Using the _Constant Replace_ patch we measure two timinings. One is only the time the patch needs, the other one is _timing including the assembling into the SPIR-V bytecode_. This allows us to distinguish between the time introduced just by patching, and the actual time needed at runtime to get the final shader code out of the system.

#let const_non_assemble = figure(
  image("const_link.png", width: 90%),
  caption: [
      Constant patching without assembling.
  ],
);

#let const_assemble = figure(
  image("const_link_assemble.png", width: 90%),
  caption: [
      Constant patching including assembling into shader bytecode.
  ],
);

#stack(
    grid(columns: 2, const_non_assemble, const_assemble)
)

The measured timings show, that the assembling time does not play a notable role in the overall runtime. However, the mere linking is faster compared to the full recompilation with a mean timing of ~12.8ms.

=== Dynamic Replace

For constant replacement we measure the same times, once _only patching_ and once including the assembly into a usable SPIR-V bytecode module.



#let dyn_non_assemble = figure(
  image("dyn_replace.png", width: 90%),
  caption: [
      Dynamic replacement only
  ],
);

#let dyn_assemble = figure(
  image("dyn_replace_assemble.png", width: 90%),
  caption: [
      Dynamic replacement including assembling into shader bytecode.
  ],
);

#stack(
    grid(columns: 2, dyn_non_assemble, dyn_assemble)
)

The overall timing of the replacement patch is much shorter with a mean timing of ~20μs for a usable SPIR-V bytecode module. The assembly step takes around 4-5μs. The reason for that rather fast assembly time lies in the implementation of the `rspirv` library which keeps the data representation in a easily assembled way in memory. Since the patch is applied directly on that _data representation_, no lifting or lowering pass is needed. The patching comes down to simple memory operations that append the new byte-instructions to the shader module, or mutate existing ones (specifically when rewriting the function call-site).

== GPU

The GPU benchmark compares the timing of both, dynamic and constant replacement of the _simple_ shader to the compiled _mandelbrot_ shader, to the patched _mandebrot_ shader. For compilation, we use Rust-Gpu, since we have to base our shader template on code generated by that compiler. GLSL inlines the mandebrot calculation with no way of preventing it, which makes the resulting template code unsable for our patching methods that currently rely on un-inlined functions.


=== Constant Replace

For constant replacement we have the following runtimes:

#let cr_dedicated = table(
  columns: (auto, auto, auto, auto),
  inset: 7pt,
  align: horizon,
  [Name], [avg], [min], [max],
  [Unmodified], [0.05ms], [0.02ms], [0.07ms],
  [Rust-GPU compiled], [2.31ms], [2.3ms], [2.35ms],
  [Patched Runtime], [1.55ms], [1.50ms], [1.58ms],
);

#let cr_mobile = table(
  columns: (auto, auto, auto),
  inset: 7pt,
  align: horizon,
  [avg], [min], [max],
  [0.35ms], [0.37ms], [0.4ms],
  [11.41ms], [11.41ms], [11.42ms],
  [13.6ms], [13.65ms], [13.7ms],
);

#stack(grid(gutter: 3pt, rows: 2, columns: 2, "Dedicated Graphics", "Integrated Graphics", cr_dedicated, cr_mobile))

As expected, the runtimes are pretty uniform on a per-test basis. The difference in the compiled and patched runtime can be explained with the actual resulting code. Rust-GPU seems to lose some knowledge about possible special instructions available to SPIR-V/GPU architectures. For instance, a call to `OpLength` (which calculate the length of an n-dimensional vector) is broken down into individual square operations and a following square-root of the sums of all components. The handwritten code however retains that knowledge, which in turn, is responsible for considerable shorter runtimes on the dedicated hardware. Interestingly the integrated Ryzen chip's graphics card does not profit from that.

=== Dynamic Replace


#let dy_dedicated = table(
    columns: (auto, auto, auto, auto),
  inset: 7pt,
  align: horizon,
  [Name], [avg], [min], [max],
  [Unmodified], [0.05ms], [0.02ms], [0.07ms],
  [Rust-GPU compiled], [2.0ms], [2.05ms], [2.2ms],
  [Patched Runtime], [1.55ms], [1.50ms], [1.58ms],
)

#let dy_mobile = table(
    columns: (auto, auto, auto),
  inset: 7pt,
  align: horizon,
  [avg], [min], [max],
  [0.35ms], [0.37ms], [0.4ms],
  [11.45ms], [11.45ms], [11.45ms],
  [13.6ms], [13.65ms], [13.7ms],
)


#stack(grid(gutter: 3pt, rows: 2, columns: 2, "Dedicated Graphics", "Integrated Graphics", dy_dedicated, dy_mobile))

For dynamic patching the runtime does not differ that much to constant replacement. This is to be expected, since ideally the code shouldn't differ at all. Combined with the CPU side testing we show, that it is possible to replace performant GPU code in a timely manor via runtime patching. The actual patching in this specific case has a negelectable overhead of 15μm-20μs without any runtime penalty compared to fully compiled code. The latter part however depends on the system that supplies the patched code.

= Conclusion

We showed that it is feasible to replace SPIR-V byte code at runtime before supplying it to the graphics driver API. Appending new code to a known module (called _DynamicReplace_ in this documentation) provides a opportunity for such operations. The reason for that is the rather good abstraction layer that SPIR-V provides in that case. Its not too low (still high-level SSA code), but also low enough that no costly graph operations are necessary to facilitate the replacement.

The linking like replacement (called _ConstantReplace_ in this documentation) is currently not as performant. While reasonable fast, it could probably be implemented faster if kept in memory like the dynamic alternative.
The main obstacle here is, that both modules need to be combined into a single SPIR-V context. This means that we have to analyse common data-types, header compatibility etc. before we can effectively merge both modules. A possible solution would be to lift both modules into a common, more easily mergeable IR. Alternatively one could try to replay the _to be merged_ module's instruction in the context of the template module using the already existing _DynamicReplace_ patch.


Finally we showed that the SPIR-V level IR is also suitable of post-compiler fix-passes. An implemented scenario patches the module to fullfill the `non-uniform` decoration requirement that is often overlooked in practice by programmers. While it can be argued that this is a shortcoming of the source programming language, this kind of patch can be helpful in realworld toolchains.


#bibliography("works.bib")
