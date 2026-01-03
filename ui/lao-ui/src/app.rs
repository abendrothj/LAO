use eframe::egui::{self, Color32, RichText};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::backend::{list_plugins_for_ui, BackendState};
use crate::components::{graph, inspector, logs, toolbar};

pub struct LaoApp {
    state: Arc<Mutex<BackendState>>,

    // UI Logic states
    graph_state: graph::GraphEditorState,
    pipe_source_for_node: HashMap<String, String>,
}

impl LaoApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut state = BackendState::default();

        // Try to load plugins on startup
        if let Ok(plugins) = list_plugins_for_ui() {
            state.plugins = plugins;
        }

        Self {
            state: Arc::new(Mutex::new(state)),
            graph_state: graph::GraphEditorState::default(),
            pipe_source_for_node: HashMap::new(),
        }
    }
}

impl eframe::App for LaoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Set a more professional theme
        ctx.set_visuals(egui::Visuals::dark());

        // Handle keyboard shortcuts
        if ctx.input(|i| i.key_pressed(egui::Key::Delete)) {
            let mut state = self.state.lock().unwrap();
            if let (Some(selected_id), Some(ref mut graph)) =
                (self.graph_state.selected_node.clone(), &mut state.graph)
            {
                graph.nodes.retain(|n| n.id != selected_id);
                graph
                    .edges
                    .retain(|e| e.from != selected_id && e.to != selected_id);
                self.graph_state.selected_node = None;
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            // Header with better styling
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), 60.0),
                egui::Layout::top_down(egui::Align::Center),
                |ui| {
                    ui.heading(
                        RichText::new("⚡ LAO Orchestrator")
                            .size(24.0)
                            .color(Color32::from_rgb(33, 150, 243)),
                    );
                    ui.label(
                        RichText::new("Local AI Workflow Orchestrator")
                            .size(12.0)
                            .color(Color32::GRAY),
                    );
                },
            );

            ui.add_space(10.0);

            // 1. Top Bar / Workflow Management
            toolbar::show(ui, &self.state);

            ui.add_space(15.0);

            // 2. Main Workspace (Graph + Inspector)
            // We need to access state.graph. Since we're borrowing self.state (arc) for toolbar,
            // we can lock it here.

            let mut state = self.state.lock().unwrap();
            let is_running = state.is_running;
            let execution_progress = state.execution_progress;
            let workflow_result = state.workflow_result.clone();
            // Clone plugins so we can use them while graph is borrowed mutably
            let plugins = state.plugins.clone();

            if let Some(ref mut graph) = state.graph {
                // Split view: Left = Graph (bigger), Right = Inspector (if selected)
                if self.graph_state.selected_node.is_some() {
                    ui.columns(2, |columns| {
                        // Left: Visual Graph
                        graph::show(&mut columns[0], graph, &mut self.graph_state, &plugins);

                        // Right: Inspector
                        if let Some(ref selected_id) = self.graph_state.selected_node {
                            // We need to find the node.
                            // We have `graph` and `selected_id`.
                            if let Some(node_idx) =
                                graph.nodes.iter().position(|n| n.id == *selected_id)
                            {
                                let action = inspector::show(
                                    &mut columns[1],
                                    &mut graph.nodes[node_idx],
                                    &plugins,
                                    &mut graph.edges,
                                    &mut self.pipe_source_for_node,
                                    &mut self.graph_state.connecting_from,
                                );

                                match action {
                                    inspector::InspectorAction::DeleteNode => {
                                        // Handle deletion
                                        graph.nodes.remove(node_idx);
                                        let id_to_remove = selected_id.clone();
                                        graph.edges.retain(|e| {
                                            e.from != id_to_remove && e.to != id_to_remove
                                        });
                                        self.graph_state.selected_node = None;
                                    }
                                    inspector::InspectorAction::None => {}
                                }
                            }
                        }
                    });
                } else {
                    // Full width graph
                    graph::show(ui, graph, &mut self.graph_state, &plugins);
                }
            } else {
                // No graph loaded, maybe show a placeholder or just the empty space
                ui.centered_and_justified(|ui| {
                    ui.label("No workflow loaded. Create a new one or load from file.");
                    if ui.button("🆕 Create New Workflow").clicked() {
                        state.graph = Some(crate::backend::WorkflowGraph {
                            nodes: Vec::new(),
                            edges: Vec::new(),
                        });
                    }
                });
            }

            ui.add_space(15.0);

            // 3. Bottom: Logs
            logs::show(
                ui,
                &mut state.live_logs,
                is_running,
                execution_progress,
                &workflow_result,
            );
        });
    }
}
