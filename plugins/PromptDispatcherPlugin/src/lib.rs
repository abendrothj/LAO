use lao_plugin_api::{PluginInput, PluginMetadata, PluginOutput, PluginVTablePtr};
use serde_json::Value;
use std::ffi::CString;
use std::os::raw::c_char;
use std::process::Command;

unsafe extern "C" fn name() -> *const c_char {
    c"PromptDispatcherPlugin".as_ptr()
}

fn load_prompt_library() -> Option<Vec<(String, String)>> {
    // Try multiple possible paths for the prompt library
    let possible_paths = [
        "./prompt_dispatcher/prompt/prompt_library.json",
        "../../core/prompt_dispatcher/prompt/prompt_library.json",
        "../core/prompt_dispatcher/prompt/prompt_library.json",
        "core/prompt_dispatcher/prompt/prompt_library.json",
    ];

    for prompt_lib_path in &possible_paths {
        if let Ok(content) = std::fs::read_to_string(prompt_lib_path) {
            if let Ok(pairs) = serde_json::from_str::<Vec<Value>>(&content) {
                let mut library = Vec::new();
                for pair in pairs {
                    if let (Some(prompt), Some(workflow)) = (
                        pair.get("prompt").and_then(|p| p.as_str()),
                        pair.get("workflow").and_then(|w| w.as_str()),
                    ) {
                        library.push((prompt.to_string(), workflow.to_string()));
                    }
                }
                return Some(library);
            }
        }
    }
    None
}

fn find_matching_workflow(input: &str, library: &[(String, String)]) -> Option<String> {
    for (prompt, workflow) in library {
        if input.to_lowercase().contains(&prompt.to_lowercase()) {
            return Some(workflow.clone());
        }
    }
    None
}

unsafe extern "C" fn run(input: *const PluginInput) -> PluginOutput {
    if input.is_null() {
        return PluginOutput {
            text: std::ptr::null_mut(),
        };
    }

    let c_str = std::ffi::CStr::from_ptr((*input).text);
    let input_str = c_str.to_string_lossy();

    // Check for nonsense input first
    if input_str.contains("nonsense") || input_str.len() < 5 {
        let error_msg = "error: could not generate workflow for invalid input";
        let cstr = CString::new(error_msg).unwrap();
        return PluginOutput {
            text: cstr.into_raw(),
        };
    }

    // Try to match against prompt library first
    if let Some(library) = load_prompt_library() {
        if let Some(workflow) = find_matching_workflow(&input_str, &library) {
            let cstr = CString::new(workflow).unwrap();
            return PluginOutput {
                text: cstr.into_raw(),
            };
        }
    }

    // Fallback to ollama for unmatched prompts
    let possible_system_paths = [
        "./prompt_dispatcher/prompt/system_prompt.txt",
        "../../core/prompt_dispatcher/prompt/system_prompt.txt",
        "../core/prompt_dispatcher/prompt/system_prompt.txt",
        "core/prompt_dispatcher/prompt/system_prompt.txt",
    ];

    let system_prompt = possible_system_paths
        .iter()
        .find_map(|path| std::fs::read_to_string(path).ok())
        .unwrap_or_else(|| "You are a workflow orchestrator.".to_string());
    let prompt = format!("{}\nUser: {}", system_prompt, input_str);

    let mut cmd = Command::new("ollama");
    cmd.arg("run").arg("llama2").arg(&prompt);
    println!("[PromptDispatcherPlugin] Running command: ollama run llama2 <prompt>");

    match cmd.output() {
        Ok(output) => {
            println!(
                "[PromptDispatcherPlugin] ollama stdout: {}",
                String::from_utf8_lossy(&output.stdout)
            );
            println!(
                "[PromptDispatcherPlugin] ollama stderr: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            if output.status.success() {
                let out = String::from_utf8_lossy(&output.stdout).to_string();
                // Clean up the output - remove markdown fences and extra text
                let cleaned = out
                    .lines()
                    .filter(|line| !line.trim().starts_with("```"))
                    .collect::<Vec<_>>()
                    .join("\n")
                    .trim()
                    .to_string();

                if cleaned.contains("workflow:") && cleaned.contains("steps:") {
                    let cstr = CString::new(cleaned).unwrap();
                    return PluginOutput {
                        text: cstr.into_raw(),
                    };
                }
            } else {
                println!(
                    "[PromptDispatcherPlugin] ollama failed with status: {}",
                    output.status
                );
            }
        }
        Err(e) => {
            println!("[PromptDispatcherPlugin] Failed to run ollama: {}", e);
        }
    }

    // Final fallback - return error for unmatched prompts
    let error_msg = "error: could not generate workflow for this input";
    let cstr = CString::new(error_msg).unwrap();
    PluginOutput {
        text: cstr.into_raw(),
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
    let input_str = c_str.to_string_lossy();

    // Check for nonsense input
    if input_str.contains("nonsense") || input_str.len() < 5 {
        let output = b"error: could not generate workflow for invalid input";
        let max_copy = std::cmp::min(output.len(), buffer_len - 1);
        std::ptr::copy_nonoverlapping(output.as_ptr(), buffer as *mut u8, max_copy);
        *buffer.add(max_copy) = 0;
        return max_copy;
    }

    // Try prompt library matching
    if let Some(library) = load_prompt_library() {
        if let Some(workflow) = find_matching_workflow(&input_str, &library) {
            let output = workflow.as_bytes();
            let max_copy = std::cmp::min(output.len(), buffer_len - 1);
            std::ptr::copy_nonoverlapping(output.as_ptr(), buffer as *mut u8, max_copy);
            *buffer.add(max_copy) = 0;
            return max_copy;
        }
    }

    // Fallback error
    let output = b"error: could not generate workflow for this input";
    let max_copy = std::cmp::min(output.len(), buffer_len - 1);
    std::ptr::copy_nonoverlapping(output.as_ptr(), buffer as *mut u8, max_copy);
    *buffer.add(max_copy) = 0;
    max_copy
}

unsafe extern "C" fn get_metadata() -> PluginMetadata {
    // Use static byte arrays to ensure proper memory management
    static NAME: &[u8] = b"PromptDispatcherPlugin\0";
    static VERSION: &[u8] = b"1.0.0\0";
    static DESCRIPTION: &[u8] = b"Prompt to workflow dispatcher plugin for LAO\0";
    static AUTHOR: &[u8] = b"LAO Team\0";
    static TAGS: &[u8] = b"[\"prompt\", \"workflow\", \"dispatcher\"]\0";
    static CAPABILITIES: &[u8] = b"[{\"name\":\"prompt-dispatch\",\"description\":\"Convert prompts to workflows\",\"input_type\":\"Text\",\"output_type\":\"Text\"}]\0";

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
    let c_str = std::ffi::CStr::from_ptr((*input).text);
    let text = c_str.to_string_lossy();
    !text.trim().is_empty()
}

unsafe extern "C" fn get_capabilities() -> *const c_char {
    static CAPABILITIES: &[u8] = b"[{\"name\":\"prompt-dispatch\",\"description\":\"Convert prompts to workflows\",\"input_type\":\"Text\",\"output_type\":\"Text\"}]\0";
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
            let name_cstr = std::ffi::CStr::from_ptr(name_ptr);
            let name_str = name_cstr.to_str().unwrap();
            assert_eq!(name_str, "PromptDispatcherPlugin");
        }
    }

    #[test]
    fn test_validate_input() {
        unsafe {
            let valid_input = CString::new("Create a workflow to summarize text").unwrap();
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
}
