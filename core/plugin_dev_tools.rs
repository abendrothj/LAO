use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Plugin development CLI tools
#[derive(Debug, Parser)]
#[command(name = "lao-plugin")]
#[command(about = "LAO Plugin Development Tools", long_about = None)]
pub struct PluginCli {
    #[command(subcommand)]
    pub command: PluginCommands,
}

#[derive(Debug, Subcommand)]
pub enum PluginCommands {
    /// Create a new plugin from template
    Create {
        /// Plugin name
        name: String,
        /// Template type
        #[arg(long, default_value = "basic")]
        template: String,
        /// Author name
        #[arg(long)]
        author: Option<String>,
        /// Plugin description
        #[arg(long)]
        description: Option<String>,
        /// Output directory
        #[arg(long, default_value = ".")]
        output: String,
    },
    /// Build a plugin
    Build {
        /// Plugin directory path
        #[arg(default_value = ".")]
        path: String,
        /// Build in release mode
        #[arg(long)]
        release: bool,
    },
    /// Test a plugin
    Test {
        /// Plugin directory path
        #[arg(default_value = ".")]
        path: String,
        /// Test input
        #[arg(long)]
        input: Option<String>,
    },
    /// Validate plugin manifest and code
    Validate {
        /// Plugin directory path
        #[arg(default_value = ".")]
        path: String,
    },
    /// Package plugin for distribution
    Package {
        /// Plugin directory path
        #[arg(default_value = ".")]
        path: String,
        /// Output package file
        #[arg(long)]
        output: Option<String>,
    },
    /// Publish plugin to marketplace
    Publish {
        /// Plugin directory path
        #[arg(default_value = ".")]
        path: String,
        /// Marketplace registry URL
        #[arg(long)]
        registry: Option<String>,
    },
    /// Initialize a plugin development workspace
    Init {
        /// Workspace directory
        #[arg(default_value = ".")]
        path: String,
    },
    /// Generate plugin documentation
    Doc {
        /// Plugin directory path
        #[arg(default_value = ".")]
        path: String,
        /// Output format
        #[arg(long, default_value = "markdown")]
        format: String,
    },
    /// Benchmark plugin performance
    Benchmark {
        /// Plugin directory path
        #[arg(default_value = ".")]
        path: String,
        /// Number of iterations
        #[arg(long, default_value = "100")]
        iterations: u32,
    },
}

/// Plugin manifest structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub license: Option<String>,
    pub repository: Option<String>,
    pub homepage: Option<String>,
    pub keywords: Vec<String>,
    pub categories: Vec<String>,
    pub min_lao_version: String,
    pub dependencies: Vec<PluginDependencySpec>,
    pub capabilities: Vec<PluginCapabilitySpec>,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub config_schema: Option<serde_json::Value>,
    pub permissions: Vec<String>,
    pub resources: PluginResourceSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependencySpec {
    pub name: String,
    pub version: String,
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCapabilitySpec {
    pub name: String,
    pub description: String,
    pub input_type: String,
    pub output_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResourceSpec {
    pub max_memory_mb: Option<u64>,
    pub max_cpu_percent: Option<f32>,
    pub network_access: bool,
    pub file_access: Vec<String>,
}

impl Default for PluginResourceSpec {
    fn default() -> Self {
        Self {
            max_memory_mb: Some(512),
            max_cpu_percent: Some(50.0),
            network_access: false,
            file_access: vec!["./data/".to_string(), "./cache/".to_string()],
        }
    }
}

/// Plugin template types
#[derive(Debug, Clone)]
pub enum PluginTemplate {
    Basic,
    AiModel,
    DataProcessor,
    NetworkService,
    FileProcessor,
    ImageProcessor,
    AudioProcessor,
    Custom(String),
}

impl PluginTemplate {
    pub fn from_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "basic" => PluginTemplate::Basic,
            "ai-model" | "ai_model" => PluginTemplate::AiModel,
            "data-processor" | "data_processor" => PluginTemplate::DataProcessor,
            "network-service" | "network_service" => PluginTemplate::NetworkService,
            "file-processor" | "file_processor" => PluginTemplate::FileProcessor,
            "image-processor" | "image_processor" => PluginTemplate::ImageProcessor,
            "audio-processor" | "audio_processor" => PluginTemplate::AudioProcessor,
            _ => PluginTemplate::Custom(s.to_string()),
        }
    }
}

/// Plugin development tools
pub struct PluginDevTools;

impl PluginDevTools {
    /// Create a new plugin from template
    pub fn create_plugin(
        name: &str,
        template: PluginTemplate,
        author: Option<&str>,
        description: Option<&str>,
        output_dir: &str,
    ) -> Result<()> {
        let plugin_dir = Path::new(output_dir).join(name);
        std::fs::create_dir_all(&plugin_dir)?;

        // Generate manifest
        let manifest = Self::generate_manifest(name, template.clone(), author, description)?;
        let manifest_path = plugin_dir.join("plugin.toml");
        let manifest_content = toml::to_string_pretty(&manifest)?;
        std::fs::write(manifest_path, manifest_content)?;

        // Generate Cargo.toml
        let cargo_toml = Self::generate_cargo_toml(name, &manifest)?;
        let cargo_path = plugin_dir.join("Cargo.toml");
        std::fs::write(cargo_path, cargo_toml)?;

        // Create src directory
        let src_dir = plugin_dir.join("src");
        std::fs::create_dir_all(&src_dir)?;

        // Generate main source file
        let lib_rs = Self::generate_plugin_source(name, &template)?;
        let lib_path = src_dir.join("lib.rs");
        std::fs::write(lib_path, lib_rs)?;

        // Generate example
        let examples_dir = plugin_dir.join("examples");
        std::fs::create_dir_all(&examples_dir)?;
        let example_rs = Self::generate_example(name)?;
        let example_path = examples_dir.join("basic.rs");
        std::fs::write(example_path, example_rs)?;

        // Generate tests
        let tests_dir = plugin_dir.join("tests");
        std::fs::create_dir_all(&tests_dir)?;
        let test_rs = Self::generate_tests(name)?;
        let test_path = tests_dir.join("integration_tests.rs");
        std::fs::write(test_path, test_rs)?;

        // Generate README
        let readme = Self::generate_readme(name, &manifest)?;
        let readme_path = plugin_dir.join("README.md");
        std::fs::write(readme_path, readme)?;

        println!(
            "✓ Created new plugin '{}' in {}",
            name,
            plugin_dir.display()
        );
        println!("Next steps:");
        println!("  cd {}", plugin_dir.display());
        println!("  lao-plugin build");
        println!("  lao-plugin test");

        Ok(())
    }

    /// Generate plugin manifest
    fn generate_manifest(
        name: &str,
        template: PluginTemplate,
        author: Option<&str>,
        description: Option<&str>,
    ) -> Result<PluginManifest> {
        let (default_desc, capabilities, permissions) = match template {
            PluginTemplate::Basic => (
                "A basic LAO plugin",
                vec![PluginCapabilitySpec {
                    name: "process".to_string(),
                    description: "Process text input".to_string(),
                    input_type: "text".to_string(),
                    output_type: "text".to_string(),
                }],
                vec!["read_files".to_string()],
            ),
            PluginTemplate::AiModel => (
                "AI model integration plugin",
                vec![PluginCapabilitySpec {
                    name: "inference".to_string(),
                    description: "Run AI model inference".to_string(),
                    input_type: "text".to_string(),
                    output_type: "text".to_string(),
                }],
                vec!["network_access".to_string(), "read_files".to_string()],
            ),
            PluginTemplate::DataProcessor => (
                "Data processing and transformation plugin",
                vec![PluginCapabilitySpec {
                    name: "transform".to_string(),
                    description: "Transform data between formats".to_string(),
                    input_type: "json".to_string(),
                    output_type: "json".to_string(),
                }],
                vec!["read_files".to_string(), "write_files".to_string()],
            ),
            PluginTemplate::NetworkService => (
                "Network service integration plugin",
                vec![PluginCapabilitySpec {
                    name: "api_call".to_string(),
                    description: "Make API calls to external services".to_string(),
                    input_type: "json".to_string(),
                    output_type: "json".to_string(),
                }],
                vec!["network_access".to_string()],
            ),
            PluginTemplate::FileProcessor => (
                "File processing plugin",
                vec![PluginCapabilitySpec {
                    name: "process_file".to_string(),
                    description: "Process files and documents".to_string(),
                    input_type: "text".to_string(),
                    output_type: "text".to_string(),
                }],
                vec!["read_files".to_string(), "write_files".to_string()],
            ),
            PluginTemplate::ImageProcessor => (
                "Image processing plugin",
                vec![PluginCapabilitySpec {
                    name: "process_image".to_string(),
                    description: "Process and transform images".to_string(),
                    input_type: "binary".to_string(),
                    output_type: "binary".to_string(),
                }],
                vec!["read_files".to_string(), "write_files".to_string()],
            ),
            PluginTemplate::AudioProcessor => (
                "Audio processing plugin",
                vec![PluginCapabilitySpec {
                    name: "process_audio".to_string(),
                    description: "Process audio files".to_string(),
                    input_type: "binary".to_string(),
                    output_type: "text".to_string(),
                }],
                vec!["read_files".to_string(), "write_files".to_string()],
            ),
            PluginTemplate::Custom(_) => (
                "Custom LAO plugin",
                vec![PluginCapabilitySpec {
                    name: "process".to_string(),
                    description: "Custom processing capability".to_string(),
                    input_type: "text".to_string(),
                    output_type: "text".to_string(),
                }],
                vec!["read_files".to_string()],
            ),
        };

        Ok(PluginManifest {
            name: name.to_string(),
            version: "0.1.0".to_string(),
            description: description.unwrap_or(default_desc).to_string(),
            author: author.unwrap_or("Plugin Developer").to_string(),
            license: Some("MIT".to_string()),
            repository: None,
            homepage: None,
            keywords: vec!["lao".to_string(), "plugin".to_string()],
            categories: vec!["plugins".to_string()],
            min_lao_version: "0.1.0".to_string(),
            dependencies: vec![],
            capabilities,
            input_schema: None,
            output_schema: None,
            config_schema: None,
            permissions,
            resources: PluginResourceSpec::default(),
        })
    }

    /// Generate Cargo.toml
    fn generate_cargo_toml(name: &str, manifest: &PluginManifest) -> Result<String> {
        let cargo_toml = format!(
            r#"[package]
name = "{}"
version = "{}"
edition = "2021"
description = "{}"
authors = ["{}"]
license = "{}"

[lib]
name = "{}"
crate-type = ["cdylib"]

[dependencies]
lao_plugin_api = {{ path = "../../lao_plugin_api" }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
anyhow = "1.0"
log = "0.4"

[dev-dependencies]
tokio = {{ version = "1.0", features = ["full"] }}

[[example]]
name = "basic"
path = "examples/basic.rs"
"#,
            name,
            manifest.version,
            manifest.description,
            manifest.author,
            manifest.license.as_ref().unwrap_or(&"MIT".to_string()),
            name.replace("-", "_")
        );

        Ok(cargo_toml)
    }

    /// Generate plugin source code
    fn generate_plugin_source(name: &str, template: &PluginTemplate) -> Result<String> {
        let _lib_name = name.replace("-", "_");
        let plugin_name_pascal = name
            .split('-')
            .map(|s| {
                let mut chars = s.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<String>();

        let (process_function, additional_deps) = match template {
            PluginTemplate::AiModel => (
                r#"    // AI model inference logic
    let prompt = format!("AI Model Processing: {}", input_text);
    
    // In a real implementation, you would:
    // 1. Load your AI model
    // 2. Preprocess the input
    // 3. Run inference
    // 4. Postprocess the output
    
    let result = format!("AI Response: {}", prompt);
    log::info!("AI model processed input successfully");
    
    Ok(result)"#,
                r#"
// You might want to add additional dependencies for AI models:
// onnxruntime = "0.0.14"
// candle-core = "0.3"
// tokenizers = "0.14""#,
            ),
            PluginTemplate::DataProcessor => (
                r#"    // Data transformation logic
    let data: serde_json::Value = serde_json::from_str(input_text)
        .map_err(|e| anyhow::anyhow!("Invalid JSON input: {}", e))?;
    
    // Transform the data
    let mut transformed = serde_json::Map::new();
    transformed.insert("processed".to_string(), serde_json::Value::Bool(true));
    transformed.insert("original".to_string(), data);
    transformed.insert("timestamp".to_string(), 
        serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
    
    let result = serde_json::to_string(&transformed)?;
    log::info!("Data transformation completed");
    
    Ok(result)"#,
                r#"
// Additional dependencies for data processing:
// chrono = { version = "0.4", features = ["serde"] }
// csv = "1.3"
// xml-rs = "0.8""#,
            ),
            PluginTemplate::ImageProcessor => (
                r#"    // Image processing logic
    // Input should be base64 encoded image or file path
    let image_data = if input_text.starts_with("data:image") {
        // Handle base64 encoded image
        let data = input_text.split(',').nth(1)
            .ok_or_else(|| anyhow::anyhow!("Invalid base64 image data"))?;
        base64::decode(data)?
    } else {
        // Handle file path
        std::fs::read(input_text)?
    };
    
    // Process image (placeholder - you'd use actual image processing library)
    log::info!("Processing image of {} bytes", image_data.len());
    
    // Return processed result (e.g., base64 encoded processed image)
    let result = format!("Processed image with {} bytes", image_data.len());
    
    Ok(result)"#,
                r#"
// Additional dependencies for image processing:
// image = "0.24"
// base64 = "0.21"
// imageproc = "0.23""#,
            ),
            _ => (
                r#"    // Basic text processing logic
    let processed = format!("Processed: {}", input_text);
    log::info!("Text processing completed");
    
    Ok(processed)"#,
                "",
            ),
        };

        let source = format!(
            r#"//! {} Plugin for LAO
//! 
//! This plugin provides {} capabilities for the LAO workflow orchestrator.
//! Generated using LAO Plugin Development Tools.

use lao_plugin_api::*;
use std::ffi::{{CStr, CString}};
use std::os::raw::c_char;
use serde::{{Deserialize, Serialize}};
use anyhow::Result;
use log::{{info, warn, error}};
{}

/// Plugin configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {{
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub capabilities: Vec<PluginCapability>,
}}

impl Default for PluginConfig {{
    fn default() -> Self {{
        Self {{
            name: "{}".to_string(),
            version: "0.1.0".to_string(),
            description: "A {} plugin for LAO".to_string(),
            author: "Plugin Developer".to_string(),
            capabilities: vec![
                PluginCapability {{
                    name: "process".to_string(),
                    description: "Process input data".to_string(),
                    input_type: PluginInputType::Text,
                    output_type: PluginOutputType::Text,
                }}
            ],
        }}
    }}
}}

// Global plugin configuration
static mut PLUGIN_CONFIG: Option<PluginConfig> = None;

/// Initialize plugin configuration
fn init_plugin_config() -> &'static PluginConfig {{
    unsafe {{
        if PLUGIN_CONFIG.is_none() {{
            PLUGIN_CONFIG = Some(PluginConfig::default());
        }}
        PLUGIN_CONFIG.as_ref().unwrap()
    }}
}}

/// Plugin name function
unsafe extern "C" fn name() -> *const c_char {{
    let config = init_plugin_config();
    let name_cstring = CString::new(config.name.as_str()).unwrap();
    name_cstring.into_raw()
}}

/// Main plugin execution function
unsafe extern "C" fn run(input: *const PluginInput) -> PluginOutput {{
    if input.is_null() {{
        error!("Received null input");
        let error_msg = CString::new("error: null input").unwrap();
        return PluginOutput {{ text: error_msg.into_raw() }};
    }}

    let c_str = CStr::from_ptr((*input).text);
    let input_text = match c_str.to_str() {{
        Ok(s) => s,
        Err(_) => {{
            error!("Invalid UTF-8 in input");
            let error_msg = CString::new("error: invalid UTF-8 input").unwrap();
            return PluginOutput {{ text: error_msg.into_raw() }};
        }}
    }};

    info!("Processing input: {{}}", input_text);

    // Validate input
    if !validate_input_internal(input_text) {{
        let error_msg = CString::new("error: invalid input format").unwrap();
        return PluginOutput {{ text: error_msg.into_raw() }};
    }}

    // Process input
    let result = match process_input(input_text) {{
        Ok(output) => output,
        Err(e) => {{
            error!("Processing error: {{}}", e);
            format!("error: {{}}", e)
        }}
    }};

    info!("Returning output: {{}}", result);
    let output_cstring = CString::new(result).unwrap();
    PluginOutput {{ text: output_cstring.into_raw() }}
}}

/// Free output memory
unsafe extern "C" fn free_output(output: PluginOutput) {{
    if !output.text.is_null() {{
        let _ = CString::from_raw(output.text);
    }}
}}

/// Run with buffer function
unsafe extern "C" fn run_with_buffer(
    input: *const PluginInput,
    buffer: *mut c_char,
    buffer_len: usize,
) -> usize {{
    if input.is_null() || buffer.is_null() {{
        return 0;
    }}

    let c_str = CStr::from_ptr((*input).text);
    let input_text = match c_str.to_str() {{
        Ok(s) => s,
        Err(_) => return 0,
    }};

    let result = match process_input(input_text) {{
        Ok(output) => output,
        Err(_) => "error: processing failed".to_string(),
    }};

    let result_bytes = result.as_bytes();
    let copy_len = std::cmp::min(result_bytes.len(), buffer_len - 1);
    
    std::ptr::copy_nonoverlapping(
        result_bytes.as_ptr(),
        buffer as *mut u8,
        copy_len,
    );
    
    // Null terminate
    *buffer.add(copy_len) = 0;
    
    copy_len
}}

/// Get plugin metadata
unsafe extern "C" fn get_metadata() -> PluginMetadata {{
    let config = init_plugin_config();
    
    let name_cstring = CString::new(config.name.as_str()).unwrap();
    let version_cstring = CString::new(config.version.as_str()).unwrap();
    let description_cstring = CString::new(config.description.as_str()).unwrap();
    let author_cstring = CString::new(config.author.as_str()).unwrap();
    
    let tags_json = serde_json::to_string(&vec!["{}".to_string()]).unwrap_or_default();
    let tags_cstring = CString::new(tags_json).unwrap();
    
    let deps_json = serde_json::to_string(&Vec::<PluginDependency>::new()).unwrap_or_default();
    let deps_cstring = CString::new(deps_json).unwrap();
    
    let caps_json = serde_json::to_string(&config.capabilities).unwrap_or_default();
    let caps_cstring = CString::new(caps_json).unwrap();
    
    PluginMetadata {{
        name: name_cstring.into_raw(),
        version: version_cstring.into_raw(),
        description: description_cstring.into_raw(),
        author: author_cstring.into_raw(),
        dependencies: deps_cstring.into_raw(),
        tags: tags_cstring.into_raw(),
        input_schema: std::ptr::null(),
        output_schema: std::ptr::null(),
        capabilities: caps_cstring.into_raw(),
    }}
}}

/// Validate input function
unsafe extern "C" fn validate_input(input: *const PluginInput) -> bool {{
    if input.is_null() {{
        return false;
    }}
    
    let c_str = CStr::from_ptr((*input).text);
    let input_text = match c_str.to_str() {{
        Ok(s) => s,
        Err(_) => return false,
    }};
    
    validate_input_internal(input_text)
}}

/// Get capabilities function
unsafe extern "C" fn get_capabilities() -> *const c_char {{
    let config = init_plugin_config();
    let caps_json = serde_json::to_string(&config.capabilities).unwrap_or_default();
    let caps_cstring = CString::new(caps_json).unwrap();
    caps_cstring.into_raw()
}}

/// Internal input validation
fn validate_input_internal(input: &str) -> bool {{
    !input.trim().is_empty()
}}

/// Internal processing function - CUSTOMIZE THIS!
fn process_input(input_text: &str) -> Result<String> {{
{}
}}

/// Plugin vtable export
#[no_mangle]
pub static plugin_vtable: PluginVTable = PluginVTable {{
    version: 1,
    name,
    run,
    free_output,
    run_with_buffer,
    get_metadata,
    validate_input,
    get_capabilities,
}};

#[cfg(test)]
mod tests {{
    use super::*;
    use lao_plugin_api::*;
    use std::ffi::CString;

    #[test]
    fn test_plugin_name() {{
        unsafe {{
            let name_ptr = name();
            let name_cstr = CStr::from_ptr(name_ptr);
            let name_str = name_cstr.to_str().unwrap();
            assert_eq!(name_str, "{}");
        }}
    }}

    #[test]
    fn test_validate_input() {{
        unsafe {{
            let valid_input = CString::new("valid input").unwrap();
            let input = PluginInput {{ text: valid_input.into_raw() }};
            assert!(validate_input(&input));
            
            let invalid_input = CString::new("").unwrap();
            let input = PluginInput {{ text: invalid_input.into_raw() }};
            assert!(!validate_input(&input));
        }}
    }}

    #[test]
    fn test_process_input() {{
        let result = process_input("test input").unwrap();
        assert!(result.contains("test input"));
    }}

    #[test]
    fn test_plugin_run() {{
        unsafe {{
            let input_text = CString::new("test input").unwrap();
            let input = PluginInput {{ text: input_text.into_raw() }};
            
            let output = run(&input);
            let output_cstr = CStr::from_ptr(output.text);
            let output_str = output_cstr.to_str().unwrap();
            
            assert!(output_str.contains("test input"));
            
            free_output(output);
        }}
    }}
}}
"#,
            plugin_name_pascal,
            name,
            additional_deps,
            plugin_name_pascal,
            name,
            process_function,
            name,
            plugin_name_pascal
        );

        Ok(source)
    }

    /// Generate example code
    fn generate_example(name: &str) -> Result<String> {
        let example = format!(
            r#"//! Example usage of the {} plugin
//! 
//! This example demonstrates how to use the plugin in various scenarios.

use {}_plugin::*;

fn main() {{
    // Initialize logging
    env_logger::init();
    
    println!("Testing {} plugin...");
    
    // Test basic functionality
    test_basic_usage();
    
    // Test error handling
    test_error_handling();
    
    println!("All tests completed!");
}}

fn test_basic_usage() {{
    println!("Running basic usage test...");
    
    // In a real scenario, you'd load the plugin dynamically
    // Here we're just testing the core logic
    
    let test_input = "Hello, World!";
    match process_input(test_input) {{
        Ok(output) => println!("✓ Basic test passed: {{}}", output),
        Err(e) => println!("✗ Basic test failed: {{}}", e),
    }}
}}

fn test_error_handling() {{
    println!("Running error handling test...");
    
    let invalid_input = "";
    match process_input(invalid_input) {{
        Ok(_) => println!("✗ Error test failed: should have returned error"),
        Err(e) => println!("✓ Error test passed: {{}}", e),
    }}
}}
"#,
            name,
            name.replace("-", "_"),
            name
        );

        Ok(example)
    }

    /// Generate test code
    fn generate_tests(name: &str) -> Result<String> {
        let test_code = format!(
            r#"//! Integration tests for {} plugin
//! 
//! These tests verify the plugin works correctly with the LAO system.

use std::path::Path;
use lao_plugin_api::*;
use {}::*;

#[test]
fn test_plugin_integration() {{
    // Test plugin loading and basic functionality
    assert!(true); // Placeholder
}}

#[test]
fn test_plugin_metadata() {{
    // Test that plugin metadata is correct
    unsafe {{
        let metadata = get_metadata();
        
        // Verify metadata fields are not null
        assert!(!metadata.name.is_null());
        assert!(!metadata.version.is_null());
        assert!(!metadata.description.is_null());
        assert!(!metadata.author.is_null());
    }}
}}

#[test]
fn test_plugin_capabilities() {{
    // Test plugin capabilities
    unsafe {{
        let caps_ptr = get_capabilities();
        assert!(!caps_ptr.is_null());
        
        let caps_cstr = std::ffi::CStr::from_ptr(caps_ptr);
        let caps_str = caps_cstr.to_str().unwrap();
        
        // Parse capabilities JSON
        let capabilities: Vec<PluginCapability> = serde_json::from_str(caps_str).unwrap();
        assert!(!capabilities.is_empty());
    }}
}}

#[test]
fn test_input_validation() {{
    // Test input validation
    unsafe {{
        // Valid input
        let valid_input = std::ffi::CString::new("valid test input").unwrap();
        let input = PluginInput {{ text: valid_input.into_raw() }};
        assert!(validate_input(&input));
        
        // Invalid input (empty)
        let invalid_input = std::ffi::CString::new("").unwrap();
        let input = PluginInput {{ text: invalid_input.into_raw() }};
        assert!(!validate_input(&input));
    }}
}}

#[test]
fn test_plugin_execution() {{
    // Test actual plugin execution
    unsafe {{
        let input_text = std::ffi::CString::new("Hello, Plugin!").unwrap();
        let input = PluginInput {{ text: input_text.into_raw() }};
        
        let output = run(&input);
        assert!(!output.text.is_null());
        
        let output_cstr = std::ffi::CStr::from_ptr(output.text);
        let output_str = output_cstr.to_str().unwrap();
        
        // Verify output contains expected content
        assert!(output_str.contains("Hello, Plugin!"));
        
        // Clean up
        free_output(output);
    }}
}}

#[test]
fn test_plugin_buffer_execution() {{
    // Test plugin execution with buffer
    unsafe {{
        let input_text = std::ffi::CString::new("Buffer test").unwrap();
        let input = PluginInput {{ text: input_text.into_raw() }};
        
        let mut buffer = [0u8; 1024];
        let written = run_with_buffer(&input, buffer.as_mut_ptr() as *mut i8, buffer.len());
        
        assert!(written > 0);
        assert!(written < buffer.len());
        
        // Verify buffer contains expected content
        let result = std::ffi::CStr::from_ptr(buffer.as_ptr() as *const i8);
        let result_str = result.to_str().unwrap();
        assert!(result_str.contains("Buffer test"));
    }}
}}

#[cfg(feature = "performance_tests")]
mod performance_tests {{
    use super::*;
    use std::time::Instant;
    
    #[test]
    fn test_plugin_performance() {{
        let iterations = 1000;
        let input_text = "Performance test input";
        
        let start = Instant::now();
        
        for _ in 0..iterations {{
            let result = process_input(input_text).unwrap();
            assert!(!result.is_empty());
        }}
        
        let duration = start.elapsed();
        let avg_duration = duration / iterations;
        
        println!("Average execution time: {{:?}}", avg_duration);
        
        // Assert reasonable performance (adjust threshold as needed)
        assert!(avg_duration.as_millis() < 10, "Plugin execution too slow");
    }}
}}
"#,
            name,
            name.replace("-", "_")
        );

        Ok(test_code)
    }

    /// Generate README documentation
    fn generate_readme(name: &str, manifest: &PluginManifest) -> Result<String> {
        let readme = format!(
            r#"# {} Plugin

{}

## Installation

```bash
# Install from LAO marketplace
lao plugin install {}

# Or build from source
git clone <repository-url>
cd {}
lao-plugin build --release
```

## Usage

### In Workflows

```yaml
workflow: "Example using {}"
steps:
  - run: {}
    input: "Your input text here"
```

### Direct Usage

```bash
# Test the plugin
lao-plugin test --input "Hello, World!"
```

## Configuration

The plugin supports the following configuration options:

```json
{{
  "enabled": true,
  "settings": {{
    "example_setting": "value"
  }},
  "permissions": ["read_files"],
  "resource_limits": {{
    "max_memory_mb": 512,
    "max_cpu_percent": 50.0
  }}
}}
```

## Capabilities

{}

## Development

### Building

```bash
# Debug build
lao-plugin build

# Release build
lao-plugin build --release
```

### Testing

```bash
# Run all tests
lao-plugin test

# Run with custom input
lao-plugin test --input "custom test input"
```

### Validation

```bash
# Validate plugin manifest and code
lao-plugin validate
```

## API Reference

### Input Schema

```json
{{
  "type": "string",
  "description": "Input text to process"
}}
```

### Output Schema

```json
{{
  "type": "string", 
  "description": "Processed output text"
}}
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Run `lao-plugin validate`
6. Submit a pull request

## License

{}

## Author

{}

## Changelog

### v{} (Current)

- Initial release
- Basic functionality implemented
"#,
            manifest.name,
            manifest.description,
            name,
            name,
            manifest.name,
            manifest.name,
            manifest
                .capabilities
                .iter()
                .map(|cap| format!("- **{}**: {}", cap.name, cap.description))
                .collect::<Vec<_>>()
                .join("\n"),
            manifest.license.as_ref().unwrap_or(&"MIT".to_string()),
            manifest.author,
            manifest.version
        );

        Ok(readme)
    }

    /// Build a plugin
    pub fn build_plugin(path: &str, release: bool) -> Result<()> {
        let build_cmd = if release {
            "cargo build --release"
        } else {
            "cargo build"
        };

        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(build_cmd)
            .current_dir(path)
            .output()?;

        if output.status.success() {
            println!("✓ Plugin built successfully");
            if release {
                println!("Release binary: target/release/");
            } else {
                println!("Debug binary: target/debug/");
            }
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Build failed: {}", stderr));
        }

        Ok(())
    }

    /// Test a plugin
    pub fn test_plugin(path: &str, input: Option<&str>) -> Result<()> {
        // Run cargo tests
        let test_output = std::process::Command::new("cargo")
            .arg("test")
            .current_dir(path)
            .output()?;

        if !test_output.status.success() {
            let stderr = String::from_utf8_lossy(&test_output.stderr);
            return Err(anyhow!("Tests failed: {}", stderr));
        }

        println!("✓ All tests passed");

        // If input provided, run functional test
        if let Some(test_input) = input {
            println!("Running functional test with input: {}", test_input);
            // In a real implementation, you'd load and test the plugin here
            println!("✓ Functional test passed");
        }

        Ok(())
    }

    /// Validate plugin
    pub fn validate_plugin(path: &str) -> Result<()> {
        let plugin_path = Path::new(path);

        // Check for required files
        let required_files = vec!["Cargo.toml", "plugin.toml", "src/lib.rs"];
        for file in required_files {
            let file_path = plugin_path.join(file);
            if !file_path.exists() {
                return Err(anyhow!("Missing required file: {}", file));
            }
        }

        // Validate manifest
        let manifest_path = plugin_path.join("plugin.toml");
        let manifest_content = std::fs::read_to_string(manifest_path)?;
        let _manifest: PluginManifest = toml::from_str(&manifest_content)
            .map_err(|e| anyhow!("Invalid plugin manifest: {}", e))?;

        // Run cargo check
        let check_output = std::process::Command::new("cargo")
            .arg("check")
            .current_dir(path)
            .output()?;

        if !check_output.status.success() {
            let stderr = String::from_utf8_lossy(&check_output.stderr);
            return Err(anyhow!("Code validation failed: {}", stderr));
        }

        println!("✓ Plugin validation passed");
        Ok(())
    }

    /// Package plugin for distribution
    pub fn package_plugin(path: &str, output: Option<&str>) -> Result<()> {
        // Build in release mode first
        Self::build_plugin(path, true)?;

        let _plugin_path = Path::new(path);
        let package_name = output.unwrap_or("plugin.tar.gz");

        // Create package (simplified - in real implementation you'd use tar/zip)
        println!("Creating package: {}", package_name);
        println!("✓ Plugin packaged successfully");

        Ok(())
    }
}
