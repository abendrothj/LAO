use lao_plugin_api::*;
use std::mem;

#[test]
fn test_vtable_layout() {
    println!(
        "PluginVTable size: {} bytes",
        mem::size_of::<PluginVTable>()
    );
    println!(
        "PluginVTable alignment: {} bytes",
        mem::align_of::<PluginVTable>()
    );

    println!("\nField offsets:");
    println!(
        "version: offset {}",
        memoffset::offset_of!(PluginVTable, version)
    );
    println!("name: offset {}", memoffset::offset_of!(PluginVTable, name));
    println!("run: offset {}", memoffset::offset_of!(PluginVTable, run));
    println!(
        "free_output: offset {}",
        memoffset::offset_of!(PluginVTable, free_output)
    );
    println!(
        "run_with_buffer: offset {}",
        memoffset::offset_of!(PluginVTable, run_with_buffer)
    );
    println!(
        "get_metadata: offset {}",
        memoffset::offset_of!(PluginVTable, get_metadata)
    );
    println!(
        "validate_input: offset {}",
        memoffset::offset_of!(PluginVTable, validate_input)
    );
    println!(
        "get_capabilities: offset {}",
        memoffset::offset_of!(PluginVTable, get_capabilities)
    );

    // Create a dummy vtable to see what the first field contains
    unsafe extern "C" fn dummy_name() -> *const std::ffi::c_char {
        std::ptr::null()
    }
    unsafe extern "C" fn dummy_run(_: *const PluginInput) -> PluginOutput {
        PluginOutput {
            text: std::ptr::null_mut(),
        }
    }
    unsafe extern "C" fn dummy_free_output(_: PluginOutput) {}
    unsafe extern "C" fn dummy_run_with_buffer(
        _: *const PluginInput,
        _: *mut std::ffi::c_char,
        _: usize,
    ) -> usize {
        0
    }
    unsafe extern "C" fn dummy_get_metadata() -> PluginMetadata {
        PluginMetadata {
            name: std::ptr::null(),
            version: std::ptr::null(),
            description: std::ptr::null(),
            author: std::ptr::null(),
            dependencies: std::ptr::null(),
            tags: std::ptr::null(),
            input_schema: std::ptr::null(),
            output_schema: std::ptr::null(),
            capabilities: std::ptr::null(),
        }
    }
    unsafe extern "C" fn dummy_validate_input(_: *const PluginInput) -> bool {
        true
    }
    unsafe extern "C" fn dummy_get_capabilities() -> *const std::ffi::c_char {
        std::ptr::null()
    }

    let dummy_vtable = PluginVTable {
        version: 1,
        name: dummy_name,
        run: dummy_run,
        free_output: dummy_free_output,
        run_with_buffer: dummy_run_with_buffer,
        get_metadata: dummy_get_metadata,
        validate_input: dummy_validate_input,
        get_capabilities: dummy_get_capabilities,
    };

    println!("\nDummy vtable version: {}", dummy_vtable.version);
    println!(
        "Dummy vtable get_metadata pointer: {:?}",
        dummy_vtable.get_metadata
    );
}
