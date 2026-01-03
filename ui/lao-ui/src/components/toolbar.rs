use crate::backend::{get_workflow_graph, run_workflow_stream, BackendState};
use eframe::egui::{self, Color32, RichText, Ui};
use std::sync::{Arc, Mutex};

pub fn show(ui: &mut Ui, state_arc: &Arc<Mutex<BackendState>>) {
    ui.group(|ui| {
        ui.heading("📋 Workflow Management");

        let mut state = state_arc.lock().unwrap();

        // File path input with better styling
        ui.horizontal(|ui| {
            ui.label(RichText::new("Workflow File:").size(14.0));
            ui.add(
                egui::TextEdit::singleline(&mut state.workflow_path)
                    .hint_text("e.g., workflows/test.yaml")
                    .desired_width(ui.available_width() * 0.6)
                    .id_source("workflow_path_input"),
            );

            ui.add_space(10.0);

            // Action buttons with icons and better styling
            if ui.add(egui::Button::new("📁 Load")).clicked() {
                match get_workflow_graph(&state.workflow_path) {
                    Ok(graph) => {
                        state.graph = Some(graph);
                        state.error.clear();
                    }
                    Err(e) => {
                        state.error = e;
                        state.graph = None;
                    }
                }
            }

            ui.add_space(5.0);

            if ui.add(egui::Button::new("▶️ Run")).clicked() {
                if !state.workflow_path.is_empty() && !state.is_running {
                    if let Some(ref graph) = state.graph {
                        // Reset node statuses before execution
                        let mut graph_clone = graph.clone();
                        for node in &mut graph_clone.nodes {
                            node.status = "pending".to_string();
                            node.message = None;
                            node.output = None;
                            node.error = None;
                            node.attempt = 0;
                        }
                        state.graph = Some(graph_clone);
                    }

                    let path = state.workflow_path.clone();
                    // We need to clone the arc to pass it to the stream
                    // But we have the lock right now.
                    // run_workflow_stream spawns a thread, so it needs the Arc.
                    let state_ref = Arc::clone(state_arc);
                    // Drop lock before running
                    drop(state);
                    let _ = run_workflow_stream(path, false, state_ref);
                    return; // Return early since we dropped state and don't want to use it again
                }
            }
            // Re-acquire lock if needed or just continue if we didn't drop

            // To handle the drop logic cleanly, we might want to split the "Run" logic
            // from the drawing logic or just handle the lock dropping carefully.
            // But wait, the `state` variable is defining the scope of the lock.
            // If I drop `state`, I can't look at it anymore in this function.
            // So for "Run Parallel" distinct button...
        });

        // We already have the lock (unless we returned).
        // Let's re-structure to avoid early return issues if we want to show more UI.
        // Actually, we can just use a flag: `should_run`
    });

    // Re-implemented properly below to handle lock
    ui.group(|ui| {
        ui.heading("📋 Workflow Management");

        let mut should_run = false;
        let mut should_run_parallel = false;
        // We need to read state to draw UI, then maybe run commands

        // Scope for lock
        {
            let mut state = state_arc.lock().unwrap();
            // File path input with better styling
            ui.horizontal(|ui| {
                ui.label(RichText::new("Workflow File:").size(14.0));
                ui.add(
                    egui::TextEdit::singleline(&mut state.workflow_path)
                        .hint_text("e.g., workflows/test.yaml")
                        .desired_width(ui.available_width() * 0.6)
                        .id_source("workflow_path_input"),
                );

                ui.add_space(10.0);

                // Action buttons with icons and better styling
                if ui.add(egui::Button::new("📁 Load")).clicked() {
                    match get_workflow_graph(&state.workflow_path) {
                        Ok(graph) => {
                            state.graph = Some(graph);
                            state.error.clear();
                        }
                        Err(e) => {
                            state.error = e;
                            state.graph = None;
                        }
                    }
                }

                ui.add_space(5.0);

                if ui.add(egui::Button::new("▶️ Run")).clicked() {
                    if !state.workflow_path.is_empty() && !state.is_running {
                        if let Some(ref graph) = state.graph {
                            // Reset node statuses before execution
                            let mut graph_clone = graph.clone();
                            for node in &mut graph_clone.nodes {
                                node.status = "pending".to_string();
                                node.message = None;
                                node.output = None;
                                node.error = None;
                                node.attempt = 0;
                            }
                            state.graph = Some(graph_clone);
                        }
                        should_run = true;
                    }
                }

                if ui.add(egui::Button::new("⚡ Run Parallel")).clicked() {
                    if !state.workflow_path.is_empty() && !state.is_running {
                        if let Some(ref graph) = state.graph {
                            // Reset node statuses before execution
                            let mut graph_clone = graph.clone();
                            for node in &mut graph_clone.nodes {
                                node.status = "pending".to_string();
                                node.message = None;
                                node.output = None;
                                node.error = None;
                                node.attempt = 0;
                            }
                            state.graph = Some(graph_clone);
                        }
                        should_run_parallel = true;
                    }
                }
            });

            // Error display with better styling
            if !state.error.is_empty() {
                ui.add_space(5.0);
                ui.colored_label(
                    Color32::from_rgb(244, 67, 54),
                    RichText::new(format!("⚠️ {}", state.error)).size(12.0),
                );
            }

            // Graph info with better organization
            if let Some(ref graph) = state.graph {
                ui.add_space(10.0);
                ui.collapsing("workflow_details", |ui| {
                    ui.horizontal(|ui| {
                        ui.label(format!("Nodes: {}", graph.nodes.len()));
                        ui.label(format!("Connections: {}", graph.edges.len()));
                    });

                    ui.separator();

                    for node in &graph.nodes {
                        let status_color = match node.status.as_str() {
                            "running" => Color32::from_rgb(33, 150, 243),
                            "success" => Color32::from_rgb(76, 175, 80),
                            "error" => Color32::from_rgb(244, 67, 54),
                            "cache" => Color32::from_rgb(156, 39, 176),
                            _ => Color32::GRAY,
                        };

                        ui.horizontal(|ui| {
                            ui.colored_label(status_color, "●");
                            ui.label(format!("{} ({})", node.id, node.run));
                            ui.label(format!("[{}]", node.status));
                        });
                    }

                    if !graph.edges.is_empty() {
                        ui.separator();
                        ui.label("Connections:");
                        for edge in &graph.edges {
                            ui.label(format!("  {} → {}", edge.from, edge.to));
                        }
                    }
                });
            }
        } // End lock scope

        if should_run {
            let state = state_arc.lock().unwrap();
            let path = state.workflow_path.clone();
            drop(state); // Drop lock before async call
            let _ = run_workflow_stream(path, false, Arc::clone(state_arc));
        }

        if should_run_parallel {
            let state = state_arc.lock().unwrap();
            let path = state.workflow_path.clone();
            drop(state);
            let _ = run_workflow_stream(path, true, Arc::clone(state_arc));
        }
    });
}
