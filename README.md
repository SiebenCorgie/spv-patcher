<div align="center">

# SpirV Patcher

SpirV patching utility operating on the IR level to patch new code, or specialise existing code.

</div>



All in one SpirV analyser, patcher and verifyer. Focuses on speed to make partial runtime SpirV patching possible.

Compared to simple linking, this patcher allows not just linking in new functions, but also patching existing code / basic-blocks. This enables for instance

- rewrite resource bindings
- patch code to make it compatible. For instance making [SpirV DXIL compatible]()
- injecting runtime linked code 
- make pipeline [stages compatible](https://github.com/expenses/spirv-location-injector)
- make non-uniform buffer access valid
