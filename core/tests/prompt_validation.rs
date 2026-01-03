use lao_orchestrator_core::cross_platform::PathUtils;
use lao_orchestrator_core::plugins::PluginRegistry;
use lao_plugin_api::{PluginInput, PluginOutput};
use serde::Deserialize;
use std::fs;
use std::path::Path;

// Helper function to check if PromptDispatcherPlugin is available
fn check_prompt_dispatcher_available() -> bool {
    let plugin_dir = PathUtils::plugin_dir();
    let reg = PluginRegistry::dynamic_registry(plugin_dir.to_str().unwrap_or("plugins"));

    if reg.get("PromptDispatcherPlugin").is_none() {
        println!("⚠️  PromptDispatcherPlugin not found, skipping test");
        return false;
    }
    true
}

#[derive(Deserialize)]
struct PromptPair {
    prompt: String,
    workflow: String,
}

fn normalize_yaml(yaml: &str) -> serde_yaml::Value {
    serde_yaml::from_str(yaml).unwrap_or(serde_yaml::Value::Null)
}

#[test]
fn test_missing_plugin_manifest() {
    let plugin_dir = "../plugins/EchoPlugin";
    let manifest_path = std::path::Path::new(plugin_dir).join("plugin.yaml");
    let orig = std::fs::read_to_string(&manifest_path).ok();
    // Temporarily remove manifest
    if manifest_path.exists() {
        std::fs::rename(&manifest_path, manifest_path.with_extension("bak")).unwrap();
    }
    let plugin_dir = PathUtils::plugin_dir();
    let mut registry = lao_orchestrator_core::plugins::PluginRegistry::dynamic_registry(
        plugin_dir.to_str().unwrap_or("plugins"),
    );
    assert!(
        registry.get("Echo").is_none(),
        "Plugin should not load without manifest"
    );
    // Restore manifest
    if let Some(orig) = orig {
        std::fs::write(&manifest_path, orig).unwrap();
    } else if manifest_path.with_extension("bak").exists() {
        std::fs::rename(manifest_path.with_extension("bak"), &manifest_path).unwrap();
    }
}

#[test]
fn test_malformed_plugin_manifest() {
    let plugin_dir = "../plugins/EchoPlugin";
    let manifest_path = std::path::Path::new(plugin_dir).join("plugin.yaml");
    let orig = std::fs::read_to_string(&manifest_path).ok();
    std::fs::write(&manifest_path, "not: yaml: [").unwrap();
    let plugin_dir = PathUtils::plugin_dir();
    let mut registry = lao_orchestrator_core::plugins::PluginRegistry::dynamic_registry(
        plugin_dir.to_str().unwrap_or("plugins"),
    );
    assert!(
        registry.get("Echo").is_none(),
        "Plugin should not load with malformed manifest"
    );
    // Restore manifest
    if let Some(orig) = orig {
        std::fs::write(&manifest_path, orig).unwrap();
    }
}

#[test]
fn test_invalid_workflow_step() {
    let workflow = lao_orchestrator_core::Workflow {
        workflow: "Invalid Step".to_string(),
        steps: vec![lao_orchestrator_core::WorkflowStep {
            run: "NonExistentPlugin".to_string(),
            params: serde_yaml::Value::Null,
            retries: None,
            retry_delay: None,
            cache_key: None,
            input_from: None,
            depends_on: None,
            condition: None,
            on_success: None,
            on_failure: None,
        }],
    };
    let dag = lao_orchestrator_core::build_dag(&workflow.steps).unwrap();
    let plugin_dir = PathUtils::plugin_dir();
    let registry = lao_orchestrator_core::plugins::PluginRegistry::dynamic_registry(
        plugin_dir.to_str().unwrap_or("plugins"),
    );
    let errors = lao_orchestrator_core::validate_workflow_types(&dag, &registry);
    assert!(!errors.is_empty(), "Should report error for missing plugin");
}

#[test]
fn test_prompt_to_workflow_failure() {
    // Check if PromptDispatcherPlugin is available
    if !check_prompt_dispatcher_available() {
        return;
    }

    let plugin_dir = PathUtils::plugin_dir();
    let mut registry = lao_orchestrator_core::plugins::PluginRegistry::dynamic_registry(
        plugin_dir.to_str().unwrap_or("plugins"),
    );
    let dispatcher = registry
        .plugins
        .get_mut("PromptDispatcherPlugin")
        .expect("PromptDispatcherPlugin not found");
    let input = lao_plugin_api::PluginInput {
        text: std::ffi::CString::new("nonsense input that should fail")
            .unwrap()
            .into_raw(),
    };
    let result = unsafe { ((*dispatcher.vtable).run)(&input) };
    let c_str = unsafe { std::ffi::CStr::from_ptr(result.text) };
    let output = c_str.to_string_lossy().to_string();
    unsafe { ((*dispatcher.vtable).free_output)(result) };
    assert!(
        output.contains("error") || output.is_empty(),
        "PromptDispatcher should error on nonsense input"
    );
}

#[test]
fn test_prompt_library_pairs() {
    // Check if PromptDispatcherPlugin is available
    if !check_prompt_dispatcher_available() {
        return;
    }

    let path = "./prompt_dispatcher/prompt/prompt_library.json";
    let data = std::fs::read_to_string(path).expect("Failed to read prompt_library.json");
    let pairs: Vec<PromptPair> =
        serde_json::from_str(&data).expect("Failed to parse prompt_library.json");
    let plugin_dir = PathUtils::plugin_dir();
    let mut registry = PluginRegistry::dynamic_registry(plugin_dir.to_str().unwrap_or("plugins"));
    let dispatcher = registry
        .plugins
        .get_mut("PromptDispatcherPlugin")
        .expect("PromptDispatcherPlugin not found");
    let mut failed = 0;
    for (i, pair) in pairs.iter().enumerate() {
        println!("\nTest {}: {}", i + 1, pair.prompt);
        let input = lao_plugin_api::PluginInput {
            text: std::ffi::CString::new(pair.prompt.clone())
                .unwrap()
                .into_raw(),
        };
        let result = unsafe { ((*dispatcher.vtable).run)(&input) };
        let c_str = unsafe { std::ffi::CStr::from_ptr(result.text) };
        let generated = c_str.to_string_lossy().to_string();
        unsafe { ((*dispatcher.vtable).free_output)(result) };
        let expected_norm = normalize_yaml(&pair.workflow);
        let generated_norm = normalize_yaml(&generated);
        if expected_norm != generated_norm {
            println!("  ❌ FAIL");
            println!("  Expected:\n{}", pair.workflow);
            println!("  Got:\n{}", generated);
            failed += 1;
        }
    }
    assert_eq!(failed, 0, "Some prompt pairs failed");
}
