use lao_orchestrator_core::{
    load_workflow_yaml, run_workflow_yaml_parallel_with_callback, run_workflow_yaml_with_callback,
    StepEvent,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub run: String,
    pub input_type: Option<String>,
    pub output_type: Option<String>,
    pub status: String,
    pub x: f32,
    pub y: f32,
    pub message: Option<String>,
    pub output: Option<String>,
    pub error: Option<String>,
    pub attempt: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiPluginInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub tags: Vec<String>,
}

pub struct BackendState {
    pub workflow_path: String,
    pub graph: Option<WorkflowGraph>,
    pub error: String,
    pub plugins: Vec<UiPluginInfo>,
    pub live_logs: Vec<String>,
    #[allow(dead_code)]
    pub selected_node: Option<String>,
    pub is_running: bool,
    pub execution_progress: f32,
    pub workflow_result: Option<WorkflowResult>,
    #[allow(dead_code)]
    pub multimodal_files: Vec<UploadedFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadedFile {
    pub name: String,
    pub path: String,
    pub file_type: String, // "audio", "image", "video", "text", "binary"
    pub size: usize,
    pub upload_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResult {
    pub success: bool,
    pub total_steps: usize,
    pub completed_steps: usize,
    pub failed_steps: usize,
    pub execution_time: f32,
    pub final_message: String,
}

impl Default for BackendState {
    fn default() -> Self {
        Self {
            workflow_path: String::new(),
            graph: None,
            error: String::new(),
            plugins: Vec::new(),
            live_logs: Vec::new(),
            selected_node: None,
            is_running: false,
            execution_progress: 0.0,
            workflow_result: None,
            multimodal_files: Vec::new(),
        }
    }
}

#[allow(dead_code)]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

pub fn get_workflow_graph(path: &str) -> Result<WorkflowGraph, String> {
    let workflow = load_workflow_yaml(path)?;
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    for (_i, step) in workflow.steps.iter().enumerate() {
        let id = step.run.clone();
        nodes.push(GraphNode {
            id: id.clone(),
            run: step.run.clone(),
            input_type: None,
            output_type: None,
            status: "pending".to_string(),
            x: 100.0 + (_i as f32 * 150.0),
            y: 100.0,
            message: None,
            output: None,
            error: None,
            attempt: 0,
        });

        if let Some(ref from) = step.input_from {
            edges.push(GraphEdge {
                from: from.clone(),
                to: id.clone(),
            });
        }

        if let Some(ref deps) = step.depends_on {
            for d in deps {
                edges.push(GraphEdge {
                    from: d.clone(),
                    to: id.clone(),
                });
            }
        }
    }

    Ok(WorkflowGraph { nodes, edges })
}

pub fn list_plugins_for_ui() -> Result<Vec<UiPluginInfo>, String> {
    let plugins_dir = resolve_plugins_dir();
    let mut out: Vec<UiPluginInfo> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&plugins_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let manifest = p.join("plugin.yaml");
                if manifest.exists() {
                    if let Ok(txt) = std::fs::read_to_string(&manifest) {
                        if let Ok(val) = serde_yaml::from_str::<serde_yaml::Value>(&txt) {
                            let name = val
                                .get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            if !name.is_empty() && !out.iter().any(|i| i.name == name) {
                                let version = val
                                    .get("version")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let description = val
                                    .get("description")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let author = val
                                    .get("maintainer")
                                    .or_else(|| val.get("author"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let tags = val
                                    .get("tags")
                                    .and_then(|v| v.as_sequence())
                                    .map(|seq| {
                                        seq.iter()
                                            .filter_map(|e| e.as_str().map(|s| s.to_string()))
                                            .collect::<Vec<_>>()
                                    })
                                    .unwrap_or_default();
                                out.push(UiPluginInfo {
                                    name,
                                    version,
                                    description,
                                    author,
                                    tags,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // Fallback: scan shared libs for names if no manifests found
    if out.is_empty() {
        if let Ok(files) = std::fs::read_dir(&plugins_dir) {
            for f in files.flatten() {
                let path = f.path();
                if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    if matches!(ext, "so" | "dll" | "dylib") {
                        if let Some(fname) = path.file_stem().and_then(|s| s.to_str()) {
                            let base = fname.strip_prefix("lib").unwrap_or(fname);
                            if !out.iter().any(|i| i.name.eq_ignore_ascii_case(base)) {
                                out.push(UiPluginInfo {
                                    name: base.to_string(),
                                    version: String::new(),
                                    description: String::new(),
                                    author: String::new(),
                                    tags: Vec::new(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(out)
}

fn resolve_plugins_dir() -> String {
    if let Ok(dir) = std::env::var("LAO_PLUGINS_DIR") {
        if std::path::Path::new(&dir).exists() {
            return dir;
        }
    }

    let candidates = ["plugins/", "./plugins/", "../plugins/", "../../plugins/"];

    for candidate in &candidates {
        if std::path::Path::new(candidate).exists() {
            return candidate.to_string();
        }
    }

    "plugins/".to_string()
}

pub fn run_workflow_stream(
    path: String,
    parallel: bool,
    state: Arc<Mutex<BackendState>>,
) -> Result<(), String> {
    std::thread::spawn(move || {
        let start_time = std::time::Instant::now();
        let mut total_steps = 0;
        let mut completed_steps = 0;
        let mut failed_steps = 0;

        // Initialize execution state
        {
            let mut state_guard = state.lock().unwrap();
            state_guard.is_running = true;
            state_guard.execution_progress = 0.0;
            state_guard.workflow_result = None;
            state_guard.error.clear();

            // Count total steps for progress tracking
            if let Some(ref graph) = state_guard.graph {
                total_steps = graph.nodes.len();
            }
        }

        let emit = |event: StepEvent| {
            if let Ok(mut state_guard) = state.lock() {
                // Update node status in graph
                if let Some(ref mut graph) = state_guard.graph {
                    if let Some(node) = graph.nodes.iter_mut().find(|n| n.id == event.step_id) {
                        node.status = event.status.clone();
                        node.message = event.message.clone();
                        node.output = event.output.clone();
                        node.error = event.error.clone();
                        node.attempt = event.attempt;
                    }
                }

                // Add to live logs
                let log_message = format!(
                    "[{}] {}: {} (attempt {}){}",
                    event.step_id,
                    event.runner,
                    event.status,
                    event.attempt,
                    event
                        .message
                        .map(|m| format!(" - {}", m))
                        .unwrap_or_default()
                );
                state_guard.live_logs.push(log_message);

                // Limit log size
                if state_guard.live_logs.len() > 200 {
                    state_guard.live_logs.remove(0);
                }

                // Update progress
                if event.status == "success" || event.status == "cache" {
                    completed_steps += 1;
                    state_guard.execution_progress = completed_steps as f32 / total_steps as f32;
                } else if event.status == "error" {
                    failed_steps += 1;
                }
            }
        };

        let result = if parallel {
            run_workflow_yaml_parallel_with_callback(&path, emit)
        } else {
            run_workflow_yaml_with_callback(&path, emit)
        };

        let execution_time = start_time.elapsed().as_secs_f32();

        // Update final state
        if let Ok(mut state_guard) = state.lock() {
            state_guard.is_running = false;
            state_guard.execution_progress = 1.0;

            let workflow_result = match result {
                Ok(logs) => {
                    let final_message = format!(
                        "Workflow completed successfully with {} steps in {:.2}s",
                        logs.len(),
                        execution_time
                    );
                    state_guard
                        .live_logs
                        .push(format!("✓ DONE: {}", final_message));
                    WorkflowResult {
                        success: true,
                        total_steps,
                        completed_steps: logs.len(),
                        failed_steps: 0,
                        execution_time,
                        final_message,
                    }
                }
                Err(err) => {
                    let final_message = format!("Workflow failed: {}", err);
                    state_guard
                        .live_logs
                        .push(format!("✗ ERROR: {}", final_message));
                    state_guard.error = err;
                    WorkflowResult {
                        success: false,
                        total_steps,
                        completed_steps,
                        failed_steps,
                        execution_time,
                        final_message,
                    }
                }
            };

            state_guard.workflow_result = Some(workflow_result);
        }
    });

    Ok(())
}

pub fn save_workflow_yaml(graph: &WorkflowGraph, filename: &str) -> Result<(), String> {
    // Build dependency info from edges
    let mut incoming: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for e in &graph.edges {
        incoming
            .entry(e.to.clone())
            .or_default()
            .push(e.from.clone());
    }

    let workflow = lao_orchestrator_core::Workflow {
        workflow: filename.trim_end_matches(".yaml").to_string(),
        steps: graph
            .nodes
            .iter()
            .map(|node| {
                let deps = incoming.get(&node.id).cloned().unwrap_or_default();
                // input_from = first predecessor if any
                let input_from = deps.first().cloned();
                // remaining predecessors are depends_on
                let depends_on = if deps.len() > 1 {
                    Some(deps[1..].to_vec())
                } else {
                    None
                };

                lao_orchestrator_core::WorkflowStep {
                    run: node.run.clone(),
                    params: serde_yaml::Value::Null, // Could be enhanced to support parameters
                    retries: None,
                    retry_delay: None,
                    cache_key: None,
                    input_from,
                    depends_on,
                    condition: None,
                    on_success: None,
                    on_failure: None,
                }
            })
            .collect(),
    };

    let yaml_content = serde_yaml::to_string(&workflow).map_err(|e| e.to_string())?;
    std::fs::write(format!("../workflows/{}", filename), yaml_content)
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn export_workflow_yaml(graph: &WorkflowGraph) -> Result<String, String> {
    let mut yaml = String::new();
    yaml.push_str("workflow: generated_workflow\n");
    yaml.push_str("steps:\n");

    // Create a map of node incoming edges (predecessors)
    let mut incoming: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for edge in &graph.edges {
        incoming
            .entry(edge.to.clone())
            .or_default()
            .push(edge.from.clone());
    }

    // Create a map of node ID to step index for proper step naming
    let mut node_to_step: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for (index, node) in graph.nodes.iter().enumerate() {
        node_to_step.insert(node.id.clone(), index);
    }

    for node in graph.nodes.iter() {
        yaml.push_str(&format!("- run: {}\n", node.run));

        // Add input_from and depends_on if this node has predecessors
        if let Some(preds) = incoming.get(&node.id) {
            if !preds.is_empty() {
                // input_from = first predecessor (export order is influenced by UI piping selection)
                if let Some(first) = preds.first() {
                    if let Some(&idx) = node_to_step.get(first) {
                        yaml.push_str(&format!("  input_from: step{}\n", idx + 1));
                    }
                }
                if preds.len() > 1 {
                    let step_deps: Vec<String> = preds[1..]
                        .iter()
                        .filter_map(|dep_id| node_to_step.get(dep_id))
                        .map(|&dep_index| format!("step{}", dep_index + 1))
                        .collect();
                    if !step_deps.is_empty() {
                        yaml.push_str(&format!("  depends_on: [{}]\n", step_deps.join(", ")));
                    }
                }
            }
        }

        // Only add fields that have meaningful values
        if let Some(ref input_type) = node.input_type {
            yaml.push_str(&format!("  input_type: {}\n", input_type));
        }
        if let Some(ref output_type) = node.output_type {
            yaml.push_str(&format!("  output_type: {}\n", output_type));
        }
        if node.status != "pending" {
            yaml.push_str(&format!("  status: {}\n", node.status));
        }
    }

    Ok(yaml)
}

// Handle file upload for multi-modal input
#[allow(dead_code)]
pub fn handle_file_upload(file_path: &str, original_name: &str) -> Result<UploadedFile, String> {
    let metadata = std::fs::metadata(file_path).map_err(|e| e.to_string())?;
    let size = metadata.len() as usize;

    // Determine file type based on extension
    let file_type = match std::path::Path::new(original_name)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
        .as_deref()
    {
        Some("wav") | Some("mp3") | Some("flac") | Some("m4a") => "audio",
        Some("jpg") | Some("jpeg") | Some("png") | Some("gif") | Some("bmp") => "image",
        Some("mp4") | Some("avi") | Some("mov") | Some("mkv") | Some("webm") => "video",
        Some("txt") | Some("md") | Some("json") | Some("yaml") | Some("yml") => "text",
        _ => "binary",
    };

    // Create uploads directory if it doesn't exist
    let uploads_dir = "../uploads";
    std::fs::create_dir_all(uploads_dir).map_err(|e| e.to_string())?;

    // Copy file to uploads directory with timestamp
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let new_path = format!("{}/{}_{}", uploads_dir, timestamp, original_name);
    std::fs::copy(file_path, &new_path).map_err(|e| e.to_string())?;

    Ok(UploadedFile {
        name: original_name.to_string(),
        path: new_path,
        file_type: file_type.to_string(),
        size,
        upload_time: chrono::Utc::now()
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string(),
    })
}

// Get supported file types for upload
#[allow(dead_code)]
pub fn get_supported_file_types() -> Vec<&'static str> {
    vec![
        "audio/*", "image/*", "video/*", "text/*", ".wav", ".mp3", ".flac", ".m4a", ".jpg",
        ".jpeg", ".png", ".gif", ".bmp", ".mp4", ".avi", ".mov", ".mkv", ".webm", ".txt", ".md",
        ".json", ".yaml", ".yml", ".pdf",
    ]
}
