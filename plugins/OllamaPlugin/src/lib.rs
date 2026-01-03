use anyhow::Result;
use lao_plugin_api::{PluginInput, PluginOutput, PluginVTablePtr};
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub tags: Vec<String>,
    pub capabilities: Vec<lao_plugin_api::PluginCapability>,
    pub dependencies: Vec<lao_plugin_api::PluginDependency>,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            name: "OllamaPlugin".to_string(),
            version: "0.1.0".to_string(),
            description: "AI model integration using Ollama".to_string(),
            author: "LAO Team".to_string(),
            tags: vec!["ai".to_string(), "ollama".to_string(), "llm".to_string()],
            capabilities: vec![lao_plugin_api::PluginCapability {
                name: "generate".to_string(),
                description: "Generate text using Ollama models".to_string(),
                input_type: lao_plugin_api::PluginInputType::Text,
                output_type: lao_plugin_api::PluginOutputType::Text,
            }],
            dependencies: vec![],
        }
    }
}

// Plugin configuration - use const instead of static mut
fn get_plugin_config() -> PluginConfig {
    PluginConfig::default()
}

// Plugin name function
unsafe extern "C" fn name() -> *const c_char {
    let config = get_plugin_config();
    let name_cstring = CString::new(config.name.as_str()).unwrap();
    name_cstring.into_raw()
}

// Plugin run function
unsafe extern "C" fn run(input: *const PluginInput) -> PluginOutput {
    if input.is_null() {
        error!("Received null input");
        let error_msg = CString::new("error: null input").unwrap();
        return PluginOutput {
            text: error_msg.into_raw(),
        };
    }

    let c_str = CStr::from_ptr((*input).text);
    let input_text = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => {
            error!("Invalid UTF-8 in input");
            let error_msg = CString::new("error: invalid UTF-8 input").unwrap();
            return PluginOutput {
                text: error_msg.into_raw(),
            };
        }
    };

    info!("Processing input: {}", input_text);

    // Validate input
    if !validate_input_internal(input_text) {
        let error_msg = CString::new("error: invalid input format").unwrap();
        return PluginOutput {
            text: error_msg.into_raw(),
        };
    }

    // Process input
    let result = match process_input(input_text) {
        Ok(output) => output,
        Err(e) => {
            error!("Processing error: {}", e);
            format!("error: {}", e)
        }
    };

    info!("Returning output: {}", result);
    let output_cstring = CString::new(result).unwrap();
    PluginOutput {
        text: output_cstring.into_raw(),
    }
}

// Free output function
unsafe extern "C" fn free_output(output: PluginOutput) {
    if !output.text.is_null() {
        let _ = CString::from_raw(output.text);
    }
}

// Run with buffer function
unsafe extern "C" fn run_with_buffer(
    input: *const PluginInput,
    buffer: *mut c_char,
    buffer_len: usize,
) -> usize {
    if input.is_null() || buffer.is_null() {
        return 0;
    }

    let c_str = CStr::from_ptr((*input).text);
    let input_text = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let result = match process_input(input_text) {
        Ok(output) => output,
        Err(_) => "error: processing failed".to_string(),
    };

    let result_bytes = result.as_bytes();
    let copy_len = std::cmp::min(result_bytes.len(), buffer_len - 1);

    std::ptr::copy_nonoverlapping(result_bytes.as_ptr(), buffer as *mut u8, copy_len);

    // Null terminate
    *buffer.add(copy_len) = 0;

    copy_len
}

// Get metadata function
unsafe extern "C" fn get_metadata() -> lao_plugin_api::PluginMetadata {
    // Use static byte arrays to ensure proper memory management
    static NAME: &[u8] = b"OllamaPlugin\0";
    static VERSION: &[u8] = b"1.0.0\0";
    static DESCRIPTION: &[u8] = b"Ollama integration plugin for LAO\0";
    static AUTHOR: &[u8] = b"LAO Team\0";
    static TAGS: &[u8] = b"[\"llm\", \"ollama\", \"text-generation\"]\0";
    static CAPABILITIES: &[u8] = b"[{\"name\":\"text-generation\",\"description\":\"Generate text using Ollama\",\"input_type\":\"Text\",\"output_type\":\"Text\"}]\0";

    lao_plugin_api::PluginMetadata {
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

// Validate input function
unsafe extern "C" fn validate_input(input: *const PluginInput) -> bool {
    if input.is_null() {
        return false;
    }

    let c_str = CStr::from_ptr((*input).text);
    let input_text = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };

    validate_input_internal(input_text)
}

// Get capabilities function
unsafe extern "C" fn get_capabilities() -> *const c_char {
    static CAPABILITIES: &[u8] = b"[{\"name\":\"text-generation\",\"description\":\"Generate text using Ollama\",\"input_type\":\"Text\",\"output_type\":\"Text\"}]\0";
    CAPABILITIES.as_ptr() as *const c_char
}

// Internal validation function
fn validate_input_internal(input: &str) -> bool {
    !input.trim().is_empty()
}

// Internal processing function
fn process_input(input: &str) -> Result<String> {
    // Call Ollama API
    let client = reqwest::blocking::Client::new();
    let response = client
        .post("http://localhost:11434/api/generate")
        .json(&serde_json::json!({
            "model": "llama2",
            "prompt": input,
            "stream": false
        }))
        .send()?;

    let result: serde_json::Value = response.json()?;
    Ok(result["response"].as_str().unwrap_or("").to_string())
}

// Plugin vtable
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
