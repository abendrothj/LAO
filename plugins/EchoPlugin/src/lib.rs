use lao_plugin_api::{PluginInput, PluginMetadata, PluginOutput, PluginVTable, PluginVTablePtr};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

unsafe extern "C" fn name() -> *const c_char {
    CString::new("EchoPlugin").unwrap().into_raw()
}

unsafe extern "C" fn run(input: *const PluginInput) -> PluginOutput {
    if input.is_null() {
        println!("[EchoPlugin] Received null input");
        return PluginOutput {
            text: std::ptr::null_mut(),
        };
    }
    let c_str = CStr::from_ptr((*input).text);
    let s = c_str.to_string_lossy();
    println!("[EchoPlugin] Received input: {}", s);

    // Validate input - should be a simple string, not YAML object or empty
    if s.trim().is_empty() || s.contains("not:") || s.contains("{") || s.contains("}") {
        let error_msg = "error: invalid input for Echo plugin";
        let out = CString::new(error_msg).unwrap();
        println!("[EchoPlugin] Returning error: {}", error_msg);
        return PluginOutput {
            text: out.into_raw(),
        };
    }

    let out = CString::new(s.as_ref()).unwrap();
    println!("[EchoPlugin] Returning output: {}", out.to_string_lossy());
    PluginOutput {
        text: out.into_raw(),
    }
}

unsafe extern "C" fn free_output(output: PluginOutput) {
    if !output.text.is_null() {
        let _ = CString::from_raw(output.text);
    }
}

unsafe extern "C" fn run_with_buffer(
    input: *const PluginInput,
    buffer: *mut c_char,
    buffer_len: usize,
) -> usize {
    if input.is_null() || buffer.is_null() || buffer_len == 0 {
        return 0;
    }
    let c_str = std::ffi::CStr::from_ptr((*input).text);
    let bytes = c_str.to_bytes();
    if bytes.is_empty() {
        return 0;
    }
    let max_copy = std::cmp::min(bytes.len(), buffer_len - 1);
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer as *mut u8, max_copy);
    *buffer.add(max_copy) = 0; // null terminator
    max_copy
}

unsafe extern "C" fn get_metadata() -> PluginMetadata {
    // Use simple static strings with proper null termination
    static NAME: &str = "EchoPlugin\0";
    static VERSION: &str = "1.0.0\0";
    static DESCRIPTION: &str = "Simple echo plugin for LAO\0";
    static AUTHOR: &str = "LAO Team\0";
    static TAGS: &str = "[\"echo\", \"test\", \"debug\"]\0";
    static CAPABILITIES: &str = "[{\"name\":\"echo\",\"description\":\"Echo input back as output\",\"input_type\":\"Text\",\"output_type\":\"Text\"}]\0";

    PluginMetadata {
        name: NAME.as_ptr() as *const c_char,
        version: VERSION.as_ptr() as *const c_char,
        description: DESCRIPTION.as_ptr() as *const c_char,
        author: AUTHOR.as_ptr() as *const c_char,
        dependencies: std::ptr::null(),
        tags: TAGS.as_ptr() as *const c_char,
        input_schema: std::ptr::null(),
        output_schema: std::ptr::null(),
        capabilities: CAPABILITIES.as_ptr() as *const c_char,
    }
}

unsafe extern "C" fn validate_input(input: *const PluginInput) -> bool {
    if input.is_null() {
        return false;
    }
    let c_str = CStr::from_ptr((*input).text);
    let text = c_str.to_string_lossy();
    !text.trim().is_empty() && !text.contains("not:") && !text.contains("{") && !text.contains("}")
}

unsafe extern "C" fn get_capabilities() -> *const c_char {
    static CAPABILITIES: &str = "[{\"name\":\"echo\",\"description\":\"Echo input back as output\",\"input_type\":\"Text\",\"output_type\":\"Text\"}]\0";
    CAPABILITIES.as_ptr() as *const c_char
}

#[no_mangle]
pub static PLUGIN_VTABLE: PluginVTable = PluginVTable {
    version: 1,
    name,
    run,
    free_output,
    run_with_buffer,
    get_metadata,
    validate_input,
    get_capabilities,
};

#[no_mangle]
pub extern "C" fn plugin_vtable() -> PluginVTablePtr {
    &PLUGIN_VTABLE
}

#[cfg(test)]
mod tests {
    use super::*;
    use lao_plugin_api::*;
    use std::ffi::CString;

    #[test]
    fn test_plugin_name() {
        unsafe {
            let name_ptr = name();
            let name_cstr = CStr::from_ptr(name_ptr);
            let name_str = name_cstr.to_str().unwrap();
            assert_eq!(name_str, "EchoPlugin");
        }
    }

    #[test]
    fn test_validate_input() {
        unsafe {
            let valid_input = CString::new("Hello World").unwrap();
            let input = PluginInput {
                text: valid_input.into_raw(),
            };
            assert!(validate_input(&input));

            let empty_input = CString::new("").unwrap();
            let input = PluginInput {
                text: empty_input.into_raw(),
            };
            assert!(!validate_input(&input));

            let forbidden_input = CString::new("not: allowed").unwrap();
            let input = PluginInput {
                text: forbidden_input.into_raw(),
            };
            assert!(!validate_input(&input));
        }
    }

    #[test]
    fn test_run_success() {
        unsafe {
            let input_text = CString::new("Echo").unwrap();
            let input = PluginInput {
                text: input_text.into_raw(),
            };

            let output = run(&input);
            let output_cstr = CStr::from_ptr(output.text);
            let output_str = output_cstr.to_str().unwrap();

            assert_eq!(output_str, "Echo");

            free_output(output);
        }
    }

    #[test]
    fn test_run_error() {
        unsafe {
            let input_text = CString::new("{ invalid json }").unwrap();
            let input = PluginInput {
                text: input_text.into_raw(),
            };

            let output = run(&input);
            let output_cstr = CStr::from_ptr(output.text);
            let output_str = output_cstr.to_str().unwrap();

            assert!(output_str.contains("error: invalid input"));

            free_output(output);
        }
    }
}
