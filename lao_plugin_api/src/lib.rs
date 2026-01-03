use std::ffi::{c_char, CStr};

#[repr(C)]
pub struct PluginInput {
    pub text: *mut c_char,
}

#[repr(C)]
pub struct MultiModalInput {
    pub input_type: u32, // Maps to PluginInputType as discriminant
    pub text_data: *mut c_char,
    pub file_path: *mut c_char,
    pub binary_data: *mut u8,
    pub binary_size: usize,
    pub metadata: *mut c_char, // JSON metadata for additional context
}

#[repr(C)]
pub struct PluginOutput {
    pub text: *mut c_char,
}

#[repr(C)]
pub struct PluginMetadata {
    pub name: *const c_char,
    pub version: *const c_char,
    pub description: *const c_char,
    pub author: *const c_char,
    pub dependencies: *const c_char, // JSON array of dependency strings
    pub tags: *const c_char,         // JSON array of tag strings
    pub input_schema: *const c_char, // JSON schema for input validation
    pub output_schema: *const c_char, // JSON schema for output validation
    pub capabilities: *const c_char, // JSON array of capabilities
}

#[repr(C)]
pub struct PluginVTable {
    pub version: u32,
    pub name: unsafe extern "C" fn() -> *const c_char,
    pub run: unsafe extern "C" fn(*const PluginInput) -> PluginOutput,
    pub free_output: unsafe extern "C" fn(PluginOutput),
    pub run_with_buffer: unsafe extern "C" fn(*const PluginInput, *mut c_char, usize) -> usize,
    pub get_metadata: unsafe extern "C" fn() -> PluginMetadata,
    pub validate_input: unsafe extern "C" fn(*const PluginInput) -> bool,
    pub get_capabilities: unsafe extern "C" fn() -> *const c_char, // JSON array of capabilities
}

pub type PluginVTablePtr = *const PluginVTable;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PluginInputType {
    Text,
    Json,
    Binary,
    File,
    Audio,
    Image,
    Video,
    Any,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PluginOutputType {
    Text,
    Json,
    Binary,
    File,
    Audio,
    Image,
    Video,
    Any,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginCapability {
    pub name: String,
    pub description: String,
    pub input_type: PluginInputType,
    pub output_type: PluginOutputType,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginDependency {
    pub name: String,
    pub version: String,
    pub optional: bool,
}

#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub dependencies: Vec<PluginDependency>,
    pub tags: Vec<String>,
    pub capabilities: Vec<PluginCapability>,
    pub input_schema: Option<String>,
    pub output_schema: Option<String>,
}

impl PluginInfo {
    pub fn from_metadata(metadata: &PluginMetadata) -> Self {
        // Helper function to safely convert C string pointer to String
        fn safe_cstr_to_string(ptr: *const c_char) -> String {
            if ptr.is_null() {
                return String::new();
            }
            // Try to create CStr, if it fails, return empty string
            unsafe {
                match CStr::from_ptr(ptr).to_str() {
                    Ok(s) => s.to_string(),
                    Err(_) => String::new(),
                }
            }
        }

        let name = safe_cstr_to_string(metadata.name);
        let version = safe_cstr_to_string(metadata.version);
        let description = safe_cstr_to_string(metadata.description);
        let author = safe_cstr_to_string(metadata.author);

        let dependencies = if !metadata.dependencies.is_null() {
            let deps_str = safe_cstr_to_string(metadata.dependencies);
            serde_json::from_str::<Vec<PluginDependency>>(&deps_str).unwrap_or_default()
        } else {
            Vec::new()
        };

        let tags = if !metadata.tags.is_null() {
            let tags_str = safe_cstr_to_string(metadata.tags);
            serde_json::from_str::<Vec<String>>(&tags_str).unwrap_or_default()
        } else {
            Vec::new()
        };

        let capabilities = if !metadata.capabilities.is_null() {
            let caps_str = safe_cstr_to_string(metadata.capabilities);
            serde_json::from_str::<Vec<PluginCapability>>(&caps_str).unwrap_or_default()
        } else {
            Vec::new()
        };

        let input_schema = if !metadata.input_schema.is_null() {
            let schema_str = safe_cstr_to_string(metadata.input_schema);
            if schema_str.is_empty() {
                None
            } else {
                Some(schema_str)
            }
        } else {
            None
        };

        let output_schema = if !metadata.output_schema.is_null() {
            let schema_str = safe_cstr_to_string(metadata.output_schema);
            if schema_str.is_empty() {
                None
            } else {
                Some(schema_str)
            }
        } else {
            None
        };

        PluginInfo {
            name,
            version,
            description,
            author,
            dependencies,
            tags,
            capabilities,
            input_schema,
            output_schema,
        }
    }
}
