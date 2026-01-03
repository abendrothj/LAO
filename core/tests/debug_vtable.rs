use lao_plugin_api::*;
use std::mem;

#[test]
fn debug_vtable_memory() {
    // Try to load the echo plugin and examine its vtable memory
    let dll_path = std::path::Path::new("../plugins/echo_plugin.dll");
    if !dll_path.exists() {
        println!("DLL not found at: {}", dll_path.display());
        return;
    }

    unsafe {
        let library = libloading::Library::new(dll_path).expect("Failed to load library");
        let plugin_vtable: libloading::Symbol<PluginVTablePtr> = library
            .get(b"plugin_vtable")
            .expect("Failed to get plugin_vtable symbol");

        let vtable = *plugin_vtable;
        println!("VTable pointer: {:?}", vtable);

        // Read the first 64 bytes of memory at the vtable pointer
        let vtable_bytes = std::slice::from_raw_parts(vtable as *const u8, 64);

        println!("First 64 bytes of vtable:");
        for (i, &byte) in vtable_bytes.iter().enumerate() {
            if i % 8 == 0 {
                println!();
            }
            print!("{:02x} ", byte);
        }
        println!();

        // Try to interpret the first 8 bytes as different types
        let first_8_bytes = &vtable_bytes[0..8];
        let as_u32_le = u32::from_le_bytes([
            first_8_bytes[0],
            first_8_bytes[1],
            first_8_bytes[2],
            first_8_bytes[3],
        ]);
        let as_u32_be = u32::from_be_bytes([
            first_8_bytes[0],
            first_8_bytes[1],
            first_8_bytes[2],
            first_8_bytes[3],
        ]);
        let as_u64_le = u64::from_le_bytes([
            first_8_bytes[0],
            first_8_bytes[1],
            first_8_bytes[2],
            first_8_bytes[3],
            first_8_bytes[4],
            first_8_bytes[5],
            first_8_bytes[6],
            first_8_bytes[7],
        ]);

        println!("First 8 bytes as u32 (little endian): {}", as_u32_le);
        println!("First 8 bytes as u32 (big endian): {}", as_u32_be);
        println!("First 8 bytes as u64 (little endian): {}", as_u64_le);
        println!(
            "First 8 bytes as u64 (little endian) hex: 0x{:x}",
            as_u64_le
        );

        // Check if this looks like a function pointer
        if as_u64_le > 0x1000 && as_u64_le < 0x7fffffffffff {
            println!("This looks like a valid function pointer");
        } else {
            println!("This doesn't look like a valid function pointer");
        }
    }
}
