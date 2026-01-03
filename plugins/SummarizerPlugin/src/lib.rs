use lao_plugin_api::{PluginInput, PluginMetadata, PluginOutput, PluginVTablePtr};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

unsafe extern "C" fn name() -> *const c_char {
    c"SummarizerPlugin".as_ptr()
}

unsafe extern "C" fn run(input: *const PluginInput) -> PluginOutput {
    if input.is_null() {
        return PluginOutput {
            text: std::ptr::null_mut(),
        };
    }
    let c_str = CStr::from_ptr((*input).text);
    let text = c_str.to_string_lossy();
    let client = reqwest::blocking::Client::new();
    let res = client
        .post("http://localhost:11434/api/generate")
        .json(&serde_json::json!({
            "model": "mistral",
            "prompt": format!("Summarize this:\n\n{}", text),
            "stream": false
        }))
        .send();
    let summary = match res {
        Ok(resp) => {
            let json: serde_json::Value = resp.json().unwrap_or_default();
            json["response"].as_str().unwrap_or("").to_string()
        }
        Err(e) => format!("Summarizer error: {}", e),
    };
    let out = CString::new(summary).unwrap().into_raw();
    PluginOutput { text: out }
}

unsafe extern "C" fn free_output(output: PluginOutput) {
    if !output.text.is_null() {
        let _ = CString::from_raw(output.text);
    }
}

unsafe extern "C" fn run_with_buffer(
    _input: *const lao_plugin_api::PluginInput,
    _buffer: *mut std::os::raw::c_char,
    _buffer_len: usize,
) -> usize {
    0 // Not implemented for SummarizerPlugin
}

unsafe extern "C" fn get_metadata() -> PluginMetadata {
    // Use static byte arrays to ensure proper memory management
    static NAME: &[u8] = b"SummarizerPlugin\0";
    static VERSION: &[u8] = b"1.0.0\0";
    static DESCRIPTION: &[u8] = b"Text summarization plugin for LAO\0";
    static AUTHOR: &[u8] = b"LAO Team\0";
    static TAGS: &[u8] = b"[\"summarization\", \"text\", \"ai\"]\0";
    static CAPABILITIES: &[u8] = b"[{\"name\":\"summarize\",\"description\":\"Summarize text using AI models\",\"input_type\":\"Text\",\"output_type\":\"Text\"}]\0";

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
    !text.trim().is_empty()
}

unsafe extern "C" fn get_capabilities() -> *const c_char {
    static CAPABILITIES: &[u8] = b"[{\"name\":\"summarize\",\"description\":\"Summarize text using AI models\",\"input_type\":\"Text\",\"output_type\":\"Text\"}]\0";
    CAPABILITIES.as_ptr() as *const c_char
}

#[no_mangle]
pub static PLUGIN_VTABLE: lao_plugin_api::PluginVTable = lao_plugin_api::PluginVTable {
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
            assert_eq!(name_str, "SummarizerPlugin");
        }
    }

    #[test]
    fn test_validate_input() {
        unsafe {
            let valid_input = CString::new("Some text to summarize").unwrap();
            let input = PluginInput {
                text: valid_input.into_raw(),
            };
            assert!(validate_input(&input));

            let empty_input = CString::new("   ").unwrap();
            let input = PluginInput {
                text: empty_input.into_raw(),
            };
            assert!(!validate_input(&input));
        }
    }

    // Note: We skip testing `run` here because it makes a real HTTP request to localhost:11434.
    // In a full CI environment, we'd mock the HTTP client or have a mock server.
    // For now, testing validation and metadata is sufficient for basic sanity.
}
