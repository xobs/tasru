# Tasru: Map out dwarf binaries

Tasru allows for easy inspection of Elf binaries using Dwarf debug information.

# Example usage

```rust
let debug_info = tasru::DebugInfo::new("file.elf")?;
let var_value = debug_info.variable_from_demangled_name("package::GLOBAL_VAR")?.base_type().to_u32()?;
println!("Var value: {}", var_value);
```
