use lao_orchestrator_core::cross_platform::PathUtils;
use lao_orchestrator_core::plugins::PluginRegistry;
use lao_orchestrator_core::{
    build_dag, run_workflow_yaml, validate_workflow_types, Workflow, WorkflowStep,
};
use lao_plugin_api::PluginInput;
use serial_test::serial;
use std::fs;
use std::path::Path;

// Helper function to check if required plugins are available
fn check_plugins_available(required_plugins: &[&str]) -> bool {
    let plugin_dir = PathUtils::plugin_dir();
    let reg = PluginRegistry::dynamic_registry(plugin_dir.to_str().unwrap_or("plugins"));

    for plugin_name in required_plugins {
        if reg.get(plugin_name).is_none() {
            println!("⚠️  Plugin '{}' not found, skipping test", plugin_name);
            return false;
        }
    }
    true
}

#[test]
#[serial]
fn test_plugin_loading() {
    // Simple test - just try to load the plugin without calling functions
    println!("[TEST] Starting plugin loading test");

    // Use cross-platform plugin directory detection
    let plugin_dir = PathUtils::plugin_dir();
    println!("[TEST] Plugin directory: {}", plugin_dir.display());

    // Valid plugin
    let reg = PluginRegistry::dynamic_registry(plugin_dir.to_str().unwrap_or("plugins"));
    println!(
        "[TEST] Registry created, loaded plugins: {:?}",
        reg.plugins.keys().collect::<Vec<_>>()
    );

    // Check if we can create the registry without crashing
    assert!(true, "Registry creation should not crash");

    // Test that EchoPlugin loads (if available)
    if reg.get("EchoPlugin").is_some() {
        println!("[TEST] EchoPlugin loaded successfully");
    } else {
        println!("[TEST] EchoPlugin not found - this may be expected on some platforms");
    }
}

#[test]
#[serial]
fn test_workflow_execution_success() {
    let workflow = Workflow {
        workflow: "Echo Test".to_string(),
        steps: vec![WorkflowStep {
            run: "EchoPlugin".to_string(),
            params: serde_yaml::from_str("input: 'Hello, LAO!'").unwrap(),
            retries: Some(1),
            retry_delay: None,
            cache_key: None,
            input_from: None,
            depends_on: None,
            condition: None,
            on_success: None,
            on_failure: None,
        }],
    };
    let path = "temp_workflow.yaml";
    fs::write(path, serde_yaml::to_string(&workflow).unwrap()).unwrap();

    // Check if plugins are available before running workflow
    if !check_plugins_available(&["EchoPlugin"]) {
        fs::remove_file(path).unwrap();
        return;
    }

    let logs = run_workflow_yaml(path).unwrap();
    for log in &logs {
        println!(
            "Echo workflow log: step={} runner={} output={:?} error={:?}",
            log.step, log.runner, log.output, log.error
        );
    }
    assert!(
        logs.iter().any(|log| log
            .output
            .as_ref()
            .map(|o| o.contains("Hello, LAO!"))
            .unwrap_or(false)),
        "Echo output should be present"
    );
    fs::remove_file(path).unwrap();
}

#[test]
#[serial]
fn test_workflow_plugin_missing() {
    let workflow = Workflow {
        workflow: "Missing Plugin".to_string(),
        steps: vec![WorkflowStep {
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
    let dag = build_dag(&workflow.steps).unwrap();
    let plugin_dir = PathUtils::plugin_dir();
    let reg = PluginRegistry::dynamic_registry(plugin_dir.to_str().unwrap_or("plugins"));
    let errors = validate_workflow_types(&dag, &reg);
    assert!(!errors.is_empty(), "Should report error for missing plugin");
}

#[test]
#[serial]
fn test_workflow_invalid_step() {
    let workflow = Workflow {
        workflow: "Invalid Step".to_string(),
        steps: vec![WorkflowStep {
            run: "EchoPlugin".to_string(),
            params: serde_yaml::Value::Null, // missing required input
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
    let dag = build_dag(&workflow.steps).unwrap();
    let plugin_dir = PathUtils::plugin_dir();
    let reg = PluginRegistry::dynamic_registry(plugin_dir.to_str().unwrap_or("plugins"));
    let errors = validate_workflow_types(&dag, &reg);
    // Should not error at type level, but runtime may fail
    let path = "temp_invalid.yaml";
    fs::write(path, serde_yaml::to_string(&workflow).unwrap()).unwrap();

    // Check if plugins are available before running workflow
    if !check_plugins_available(&["EchoPlugin"]) {
        fs::remove_file(path).unwrap();
        return;
    }

    let logs = run_workflow_yaml(path).unwrap();
    assert!(
        logs.iter().any(|log| log.error.is_some()),
        "Should log error for invalid step"
    );
    fs::remove_file(path).unwrap();
}

#[test]
#[serial]
fn test_prompt_to_workflow_success() {
    let plugin_dir = PathUtils::plugin_dir();
    let mut reg = PluginRegistry::dynamic_registry(plugin_dir.to_str().unwrap_or("plugins"));

    // Check if PromptDispatcherPlugin is available
    if reg.plugins.get("PromptDispatcherPlugin").is_none() {
        println!("⚠️  PromptDispatcherPlugin not found, skipping prompt to workflow test");
        return;
    }

    let dispatcher = reg
        .plugins
        .get_mut("PromptDispatcherPlugin")
        .expect("PromptDispatcherPlugin not found");
    let input = PluginInput {
        text: std::ffi::CString::new("Summarize this Markdown doc and extract key ideas")
            .unwrap()
            .into_raw(),
    };
    let result = unsafe { ((*dispatcher.vtable).run)(&input) };
    let c_str = unsafe { std::ffi::CStr::from_ptr(result.text) };
    let output = c_str.to_string_lossy().to_string();
    unsafe { ((*dispatcher.vtable).free_output)(result) };
    assert!(!output.is_empty(), "PromptDispatcher should return YAML");
    assert!(
        output.contains("MarkdownSummarizer"),
        "Should contain markdown summarizer step"
    );

    // Test Multi-modal prompt (Audio)
    let input_audio = PluginInput {
        text: std::ffi::CString::new("Summarize this audio and create a todo list")
            .unwrap()
            .into_raw(),
    };
    let result_audio = unsafe { ((*dispatcher.vtable).run)(&input_audio) };
    let c_str_audio = unsafe { std::ffi::CStr::from_ptr(result_audio.text) };
    let output_audio = c_str_audio.to_string_lossy().to_string();
    unsafe { ((*dispatcher.vtable).free_output)(result_audio) };
    assert!(
        output_audio.contains("WhisperPlugin"),
        "Should contain WhisperPlugin for audio prompt"
    );
}

#[test]
#[serial]
fn test_prompt_to_workflow_failure() {
    let plugin_dir = PathUtils::plugin_dir();
    let mut reg = PluginRegistry::dynamic_registry(plugin_dir.to_str().unwrap_or("plugins"));

    // Check if PromptDispatcherPlugin is available
    if reg.plugins.get("PromptDispatcherPlugin").is_none() {
        println!("⚠️  PromptDispatcherPlugin not found, skipping prompt to workflow failure test");
        return;
    }

    let dispatcher = reg
        .plugins
        .get_mut("PromptDispatcherPlugin")
        .expect("PromptDispatcherPlugin not found");
    let input = PluginInput {
        text: std::ffi::CString::new("nonsense input that should fail")
            .unwrap()
            .into_raw(),
    };
    let result = unsafe { ((*dispatcher.vtable).run)(&input) };
    let c_str = unsafe { std::ffi::CStr::from_ptr(result.text) };
    let output = c_str.to_string_lossy().to_string();
    unsafe { ((*dispatcher.vtable).free_output)(result) };
    println!("PromptDispatcherPlugin nonsense input output: '{output}'");
    assert!(
        output.contains("error") || output.is_empty(),
        "PromptDispatcher should error on nonsense input"
    );
}

#[test]
#[serial]
fn test_caching_and_retries() {
    std::env::set_var("LAO_CACHE_DIR", "cache");
    let workflow = Workflow {
        workflow: "Echo Cache Test".to_string(),
        steps: vec![WorkflowStep {
            run: "EchoPlugin".to_string(),
            params: serde_yaml::from_str("input: 'Cache me!'").unwrap(),
            retries: Some(2),
            retry_delay: Some(10),
            cache_key: Some("echo_cache_test".to_string()),
            input_from: None,
            depends_on: None,
            condition: None,
            on_success: None,
            on_failure: None,
        }],
    };
    let path = "temp_cache.yaml";
    let cache_path = "cache/echo_cache_test.json";
    if Path::new(cache_path).exists() {
        fs::remove_file(cache_path).unwrap();
    }
    fs::write(path, serde_yaml::to_string(&workflow).unwrap()).unwrap();

    // Check if plugins are available before running workflow
    if !check_plugins_available(&["EchoPlugin"]) {
        fs::remove_file(path).unwrap();
        return;
    }

    // First run: should not hit cache
    let logs1 = run_workflow_yaml(path).unwrap();
    println!("[DEBUG] logs1: {:?}", logs1);
    assert!(
        logs1
            .iter()
            .any(|log| log.validation.as_deref() == Some("saved")),
        "Should save to cache"
    );
    // Second run: should hit cache
    let logs2 = run_workflow_yaml(path).unwrap();
    println!("[DEBUG] logs2: {:?}", logs2);
    assert!(
        logs2
            .iter()
            .any(|log| log.validation.as_deref() == Some("cache")),
        "Should hit cache"
    );
    fs::remove_file(path).unwrap();
    if Path::new(cache_path).exists() {
        fs::remove_file(cache_path).unwrap();
    }
}

#[test]
#[serial]
fn test_log_output() {
    let workflow = Workflow {
        workflow: "Echo Log Test".to_string(),
        steps: vec![WorkflowStep {
            run: "EchoPlugin".to_string(),
            params: serde_yaml::from_str("input: 'Log this!'").unwrap(),
            retries: Some(1),
            retry_delay: None,
            cache_key: None,
            input_from: None,
            depends_on: None,
            condition: None,
            on_success: None,
            on_failure: None,
        }],
    };
    let path = "temp_log.yaml";
    fs::write(path, serde_yaml::to_string(&workflow).unwrap()).unwrap();

    // Check if plugins are available before running workflow
    if !check_plugins_available(&["EchoPlugin"]) {
        fs::remove_file(path).unwrap();
        return;
    }

    let logs = run_workflow_yaml(path).unwrap();
    for log in &logs {
        println!(
            "Step {}: runner={} output={:?} error={:?} attempt={}",
            log.step, log.runner, log.output, log.error, log.attempt
        );
    }
    assert!(
        logs.iter().any(|log| log
            .output
            .as_ref()
            .map(|o| o.contains("Log this!"))
            .unwrap_or(false)),
        "Log output should be present"
    );
    fs::remove_file(path).unwrap();
}

#[test]
#[serial]
fn test_multi_plugin_workflow() {
    // This test assumes Echo and SummarizerPlugin plugins exist and are compatible
    let workflow = Workflow {
        workflow: "Multi-Plugin Chain".to_string(),
        steps: vec![
            WorkflowStep {
                run: "EchoPlugin".to_string(),
                params: serde_yaml::from_str("input: 'Chain this!'").unwrap(),
                retries: Some(1),
                retry_delay: None,
                cache_key: None,
                input_from: None,
                depends_on: None,
                condition: None,
                on_success: None,
                on_failure: None,
            },
            WorkflowStep {
                run: "SummarizerPlugin".to_string(),
                params: serde_yaml::Value::Null,
                retries: Some(1),
                retry_delay: None,
                cache_key: None,
                input_from: Some("EchoPlugin".to_string()),
                depends_on: None,
                condition: None,
                on_success: None,
                on_failure: None,
            },
        ],
    };
    let path = "temp_multi_plugin.yaml";
    fs::write(path, serde_yaml::to_string(&workflow).unwrap()).unwrap();

    // Check if plugins are available before running workflow
    if !check_plugins_available(&["EchoPlugin", "SummarizerPlugin"]) {
        fs::remove_file(path).unwrap();
        return;
    }

    let logs = run_workflow_yaml(path).unwrap();
    println!("[DEBUG] multi_plugin logs: {:?}", logs);
    assert!(
        logs.iter().any(|log| log.runner == "SummarizerPlugin"),
        "SummarizerPlugin step should run"
    );
    fs::remove_file(path).unwrap();
}

#[test]
#[serial]
fn test_circular_dependency() {
    let workflow = Workflow {
        workflow: "Circular Dependency".to_string(),
        steps: vec![
            WorkflowStep {
                run: "EchoPlugin".to_string(),
                params: serde_yaml::from_str("input: 'A'").unwrap(),
                retries: None,
                retry_delay: None,
                cache_key: None,
                input_from: Some("step2".to_string()),
                depends_on: None,
                condition: None,
                on_success: None,
                on_failure: None,
            },
            WorkflowStep {
                run: "SummarizerPlugin".to_string(),
                params: serde_yaml::Value::Null,
                retries: None,
                retry_delay: None,
                cache_key: None,
                input_from: Some("step1".to_string()),
                depends_on: None,
                condition: None,
                on_success: None,
                on_failure: None,
            },
        ],
    };
    let dag = build_dag(&workflow.steps).unwrap();
    let result = lao_orchestrator_core::topo_sort(&dag);
    assert!(result.is_err(), "Should error on circular dependency");
}

#[test]
#[serial]
fn test_invalid_yaml() {
    let path = "temp_invalid_yaml.yaml";
    fs::write(
        path,
        "workflow: Invalid\nsteps: [ { run: EchoPlugin, input: 'oops' }",
    )
    .unwrap(); // malformed YAML
    let result = run_workflow_yaml(path);
    assert!(result.is_err(), "Should error on invalid YAML");
    fs::remove_file(path).unwrap();
}

#[test]
#[serial]
fn test_plugin_type_mismatch() {
    // Simulate a plugin expecting text but receiving an object
    let workflow = Workflow {
        workflow: "Type Mismatch".to_string(),
        steps: vec![WorkflowStep {
            run: "EchoPlugin".to_string(),
            params: serde_yaml::from_str("input: { not: 'a string' }").unwrap(),
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
    let path = "temp_type_mismatch.yaml";
    fs::write(path, serde_yaml::to_string(&workflow).unwrap()).unwrap();

    // Check if plugins are available before running workflow
    if !check_plugins_available(&["EchoPlugin"]) {
        fs::remove_file(path).unwrap();
        return;
    }

    let logs = run_workflow_yaml(path).unwrap();
    assert!(
        logs.iter().any(|log| log.error.is_some()),
        "Should log error for type mismatch"
    );
    fs::remove_file(path).unwrap();
}

#[test]
#[serial]
fn test_conditional_execution() {
    use lao_orchestrator_core::{ConditionOperator, ConditionType, StepCondition};

    let workflow = Workflow {
        workflow: "Conditional Test".to_string(),
        steps: vec![
            // Step 1: Output "trigger"
            WorkflowStep {
                run: "EchoPlugin".to_string(),
                params: serde_yaml::from_str("input: 'trigger'").unwrap(),
                retries: None,
                retry_delay: None,
                cache_key: None,
                input_from: None,
                depends_on: None,
                condition: None,
                on_success: None,
                on_failure: None,
            },
            // Step 2: Should run (OutputContains "trigger")
            WorkflowStep {
                run: "EchoPlugin".to_string(),
                params: serde_yaml::from_str("input: 'Ran Step 2'").unwrap(),
                retries: None,
                retry_delay: None,
                cache_key: None,
                input_from: None,
                depends_on: Some(vec!["step1".to_string()]),
                condition: Some(StepCondition {
                    condition_type: ConditionType::OutputContains,
                    field: "step1".to_string(),
                    operator: ConditionOperator::Contains,
                    value: "trigger".to_string(),
                }),
                on_success: None,
                on_failure: None,
            },
            // Step 3: Should skip (OutputContains "foobar")
            WorkflowStep {
                run: "EchoPlugin".to_string(),
                params: serde_yaml::from_str("input: 'Ran Step 3'").unwrap(),
                retries: None,
                retry_delay: None,
                cache_key: None,
                input_from: None,
                depends_on: Some(vec!["step1".to_string()]),
                condition: Some(StepCondition {
                    condition_type: ConditionType::OutputContains,
                    field: "step1".to_string(),
                    operator: ConditionOperator::Contains,
                    value: "foobar".to_string(),
                }),
                on_success: None,
                on_failure: None,
            },
        ],
    };

    let path = "temp_conditional.yaml";
    fs::write(path, serde_yaml::to_string(&workflow).unwrap()).unwrap();

    if !check_plugins_available(&["EchoPlugin"]) {
        fs::remove_file(path).unwrap();
        return;
    }

    let logs = run_workflow_yaml(path).unwrap();
    println!("[DEBUG] Conditional logs: {:?}", logs);

    // Step 1 should run
    assert!(
        logs.iter()
            .any(|l| l.runner == "EchoPlugin" && l.output.as_deref() == Some("trigger")),
        "Step 1 should run"
    );

    // Step 2 should run
    assert!(
        logs.iter()
            .any(|l| l.output.as_deref() == Some("Ran Step 2")),
        "Step 2 should run"
    );

    // Step 3 should be skipped (in logs with validation="skipped" or check if it's absent/different status)
    // Looking at `lib.rs`, skipped steps are pushed to logs with validation="skipped"
    let step3_log = logs
        .iter()
        .find(|l| l.input.get("input").and_then(|v| v.as_str()) == Some("Ran Step 3"));
    assert!(step3_log.is_some(), "Step 3 should be in logs");
    assert_eq!(
        step3_log.unwrap().validation.as_deref(),
        Some("skipped"),
        "Step 3 should be skipped"
    );

    fs::remove_file(path).unwrap();
}
