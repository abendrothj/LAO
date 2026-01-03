use eframe::egui::{self, Ui, Pos2, Rect, Color32, Stroke, FontId, Vec2};
use std::sync::{Arc, Mutex};
use crate::backend::{BackendState, WorkflowGraph, GraphNode, get_workflow_graph, list_plugins_for_ui, run_workflow_stream, save_workflow_yaml, export_workflow_yaml};

pub struct LaoApp {
    state: Arc<Mutex<BackendState>>,
    
    // UI state
    new_node_name: String,
    new_node_type: String,
    new_workflow_filename: String,
    
    // Visual editor state
    connecting_from: Option<String>,
    // Canvas panning
    pan_offset: Vec2,
    last_pan_drag_id: Option<egui::Id>,
    
    // Piping preference per target node (which incoming edge is used as input_from)
    // We implement this by reordering edges when user selects a pipe source
    pipe_source_for_node: std::collections::HashMap<String, String>,
    show_save_dialog: bool,
    show_export_dialog: bool,
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
            new_node_name: String::new(),
            new_node_type: "EchoPlugin".to_string(),
            new_workflow_filename: "new_workflow.yaml".to_string(),
            connecting_from: None,
            pan_offset: Vec2::ZERO,
            last_pan_drag_id: None,
            pipe_source_for_node: std::collections::HashMap::new(),
            show_save_dialog: false,
            show_export_dialog: false,
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
            if let Some(selected_id) = state.selected_node.clone() {
                if let Some(ref mut graph) = state.graph {
                    graph.nodes.retain(|n| n.id != selected_id);
                    graph.edges.retain(|e| e.from != selected_id && e.to != selected_id);
                    state.selected_node = None;
                }
            }
        }
        
        egui::CentralPanel::default().show(ctx, |ui| {
            // Header with better styling
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), 60.0),
                egui::Layout::top_down(egui::Align::Center),
                |ui| {
                    ui.heading(egui::RichText::new("⚡ LAO Orchestrator").size(24.0).color(Color32::from_rgb(33, 150, 243)));
                    ui.label(egui::RichText::new("Local AI Workflow Orchestrator").size(12.0).color(Color32::GRAY));
                }
            );
            
            ui.add_space(10.0);
            
            // Workflow section with improved layout
            self.show_workflow_section(ui);
            
            ui.add_space(15.0);
            
            // Visual graph editor
            self.show_visual_editor(ui);
            
            ui.add_space(15.0);
            
            // Live logs section
            self.show_live_logs_section(ui);
        });
    }
}

impl LaoApp {
    fn show_workflow_section(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.heading("📋 Workflow Management");
            
            let mut state = self.state.lock().unwrap();
            
            // File path input with better styling
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Workflow File:").size(14.0));
                ui.add(egui::TextEdit::singleline(&mut state.workflow_path)
                    .hint_text("e.g., workflows/test.yaml")
                    .desired_width(ui.available_width() * 0.6)
                    .id_source("workflow_path_input"));
                
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
                        let state_ref = Arc::clone(&self.state);
                        let _ = run_workflow_stream(path, false, state_ref);
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
                        
                        let path = state.workflow_path.clone();
                        let state_ref = Arc::clone(&self.state);
                        let _ = run_workflow_stream(path, true, state_ref);
                    }
                }
            });
            
            // Error display with better styling
            if !state.error.is_empty() {
                ui.add_space(5.0);
                ui.colored_label(Color32::from_rgb(244, 67, 54), 
                    egui::RichText::new(format!("⚠️ {}", state.error)).size(12.0));
            }
            
            // Execution status with improved design
            if state.is_running {
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(egui::RichText::new(format!("Executing workflow... {:.0}%", state.execution_progress * 100.0))
                        .color(Color32::from_rgb(33, 150, 243)));
                });
                ui.add(egui::ProgressBar::new(state.execution_progress)
                    .show_percentage()
                    .fill(Color32::from_rgb(33, 150, 243)));
            }
            
            // Execution results with better presentation
            if let Some(ref result) = state.workflow_result {
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if result.success {
                        ui.colored_label(Color32::from_rgb(76, 175, 80), "✅");
                    } else {
                        ui.colored_label(Color32::from_rgb(244, 67, 54), "❌");
                    }
                    ui.label(egui::RichText::new(&result.final_message).size(14.0));
                });
                
                ui.collapsing("execution_summary", |ui| {
                    ui.horizontal(|ui| {
                        ui.label(format!("Steps: {}", result.total_steps));
                        ui.label(format!("Completed: {}", result.completed_steps));
                        ui.label(format!("Failed: {}", result.failed_steps));
                        ui.label(format!("Time: {:.2}s", result.execution_time));
                    });
                });
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
        });
    }
    
    fn show_visual_editor(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.heading("🎨 Visual Flow Builder");
            
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new("🆕 New Workflow")).clicked() {
                    let mut state = self.state.lock().unwrap();
                    state.graph = Some(WorkflowGraph {
                        nodes: Vec::new(),
                        edges: Vec::new(),
                    });
                }
                
                if ui.add(egui::Button::new("💾 Save Workflow")).clicked() {
                    self.show_save_dialog = true;
                }
                
                if ui.add(egui::Button::new("📤 Export YAML")).clicked() {
                    self.show_export_dialog = true;
                }
                
                ui.add_space(10.0);
                
                // Add delete all nodes button
                if ui.add(egui::Button::new("🗑️ Clear All")
                    .fill(Color32::from_rgb(244, 67, 54)))
                    .clicked() {
                    let mut state = self.state.lock().unwrap();
                    if let Some(ref mut graph) = state.graph {
                        graph.nodes.clear();
                        graph.edges.clear();
                        state.selected_node = None;
                    }
                }
                
                ui.add_space(20.0);
                
                // Show connection mode with better styling
                if self.connecting_from.is_some() {
                    ui.colored_label(Color32::from_rgb(255, 193, 7), 
                        egui::RichText::new("🔗 Connection mode: Click target node").size(12.0));
                    if ui.add(egui::Button::new("❌ Cancel")).clicked() {
                        self.connecting_from = None;
                    }
                } else {
                    ui.colored_label(Color32::GRAY, 
                        egui::RichText::new("💡 Tip: Right-click nodes for options, drag to move, or press Delete to remove selected node").size(12.0));
                }
            });
        
        // Save dialog
        if self.show_save_dialog {
            let mut close_dialog = false;
            egui::Window::new("Save Workflow")
                .open(&mut self.show_save_dialog)
                .show(ui.ctx(), |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Filename:");
                        ui.text_edit_singleline(&mut self.new_workflow_filename);
                    });
                    
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            let state = self.state.lock().unwrap();
                            if let Some(ref graph) = state.graph {
                                match save_workflow_yaml(graph, &self.new_workflow_filename) {
                                    Ok(_) => {
                                        close_dialog = true;
                                        // Could add success message
                                    }
                                    Err(e) => {
                                        // Could add error handling
                                        eprintln!("Save error: {}", e);
                                    }
                                }
                            }
                        }
                        
                        if ui.button("Cancel").clicked() {
                            close_dialog = true;
                        }
                    });
                });
            
            if close_dialog {
                self.show_save_dialog = false;
            }
        }
        
        // Export dialog  
        if self.show_export_dialog {
            let mut close_dialog = false;
            egui::Window::new("Export YAML")
                .open(&mut self.show_export_dialog)
                .show(ui.ctx(), |ui| {
                    let state = self.state.lock().unwrap();
                    if let Some(ref graph) = state.graph {
                        match export_workflow_yaml(graph) {
                            Ok(yaml) => {
                                ui.label("Generated YAML:");
                                egui::ScrollArea::vertical()
                                    .max_height(300.0)
                                    .show(ui, |ui| {
                                        ui.text_edit_multiline(&mut yaml.as_str());
                                    });
                            }
                            Err(e) => {
                                ui.colored_label(Color32::RED, format!("Export error: {}", e));
                            }
                        }
                    }
                    
                    if ui.button("Close").clicked() {
                        close_dialog = true;
                    }
                });
            
            if close_dialog {
                self.show_export_dialog = false;
            }
        }
        
        // Get the plugins list and selected node first
        let (plugins, selected_node_id) = {
            let state = self.state.lock().unwrap();
            (state.plugins.clone(), state.selected_node.clone())
        };
        
        // Handle interaction separately
        let mut node_clicked = None;
        let mut should_remove_node = false;
        
        {
            let mut state = self.state.lock().unwrap();
            
            if let Some(ref mut graph) = state.graph {
                // Add node controls
                ui.horizontal(|ui| {
                    ui.label("Add Node:");
                    ui.text_edit_singleline(&mut self.new_node_name);
                    
                    egui::ComboBox::from_id_salt("plugin_type_combo")
                        .selected_text(&self.new_node_type)
                        .show_ui(ui, |ui| {
                            for (i, plugin) in plugins.iter().enumerate() {
                                ui.push_id(format!("plugin_option_{}", i), |ui| {
                                    ui.selectable_value(&mut self.new_node_type, plugin.name.clone(), &plugin.name);
                                });
                            }
                        });
                    
                    if ui.button("Add Node").clicked() {
                        let node_id = if self.new_node_name.is_empty() {
                            format!("node_{}", graph.nodes.len() + 1)
                        } else {
                            self.new_node_name.clone()
                        };
                        
                        // Calculate better initial position - spread nodes in a more organized way
                        let node_count = graph.nodes.len();
                        let cols = 4; // Number of columns
                        let col = node_count % cols;
                        let row = node_count / cols;
                        let spacing_x = 200.0;
                        let spacing_y = 120.0;
                        
                        graph.nodes.push(GraphNode {
                            id: node_id,
                            run: self.new_node_type.clone(),
                            input_type: None,
                            output_type: None,
                            status: "pending".to_string(),
                            x: 50.0 + (col as f32 * spacing_x),
                            y: 50.0 + (row as f32 * spacing_y),
                            message: None,
                            output: None,
                            error: None,
                            attempt: 0,
                        });
                        
                        self.new_node_name.clear();
                    }
                });
                
                // Visual graph area
                let available_rect = ui.available_rect_before_wrap();
                let graph_rect = Rect::from_min_size(available_rect.min, egui::vec2(800.0, 400.0));
                
                let response = ui.allocate_rect(graph_rect, egui::Sense::click_and_drag());
                
                if ui.is_rect_visible(graph_rect) {
                    let painter = ui.painter();
                    
                    // Draw background
                    painter.rect_filled(graph_rect, 4.0, Color32::from_gray(248));
                    
                    // Draw grid (respecting pan)
                    let grid_size = 40.0;
                    for i in 0..40 {
                        let x = graph_rect.min.x + self.pan_offset.x + i as f32 * grid_size;
                        if x < graph_rect.max.x {
                            painter.line_segment(
                                [Pos2::new(x, graph_rect.min.y), Pos2::new(x, graph_rect.max.y)],
                                Stroke::new(1.0, Color32::from_gray(238))
                            );
                        }
                    }
                    for j in 0..40 {
                        let y = graph_rect.min.y + self.pan_offset.y + j as f32 * grid_size;
                        if y < graph_rect.max.y {
                            painter.line_segment(
                                [Pos2::new(graph_rect.min.x, y), Pos2::new(graph_rect.max.x, y)],
                                Stroke::new(1.0, Color32::from_gray(238))
                            );
                        }
                    }
                    
                    // Draw edges with improved visualization and allow deletion via right-click
                    let mut edge_to_delete: Option<usize> = None;
                    for (i, edge) in graph.edges.iter().enumerate() {
                        if let (Some(from_node), Some(to_node)) = (
                            graph.nodes.iter().find(|n| n.id == edge.from),
                            graph.nodes.iter().find(|n| n.id == edge.to)
                        ) {
                            let from_pos = Pos2::new(
                                graph_rect.min.x + self.pan_offset.x + from_node.x + 120.0,
                                graph_rect.min.y + self.pan_offset.y + from_node.y + 30.0
                            );
                            let to_pos = Pos2::new(
                                graph_rect.min.x + self.pan_offset.x + to_node.x,
                                graph_rect.min.y + self.pan_offset.y + to_node.y + 30.0
                            );
                            
                            // Draw arrow line
                            painter.line_segment(
                                [from_pos, to_pos],
                                Stroke::new(2.0, Color32::from_gray(136))
                            );
                            
                            // Draw arrowhead
                            let direction = (to_pos - from_pos).normalized();
                            let arrow_size = 8.0;
                            let arrow_tip = to_pos - direction * 5.0;
                            let perpendicular = Vec2::new(-direction.y, direction.x);
                            
                            let arrow_p1 = arrow_tip - direction * arrow_size + perpendicular * arrow_size * 0.5;
                            let arrow_p2 = arrow_tip - direction * arrow_size - perpendicular * arrow_size * 0.5;
                            
                            painter.line_segment([arrow_tip, arrow_p1], Stroke::new(2.0, Color32::from_gray(136)));
                            painter.line_segment([arrow_tip, arrow_p2], Stroke::new(2.0, Color32::from_gray(136)));
                            
                            // Check for edge click to delete
                            let edge_center = (from_pos + to_pos.to_vec2()) * 0.5;
                            let edge_rect = Rect::from_center_size(edge_center, Vec2::splat(20.0));
                            let edge_response = ui.interact(edge_rect, egui::Id::new(format!("edge_{}", i)), egui::Sense::click());
                            if edge_response.secondary_clicked() {
                                edge_to_delete = Some(i);
                            }
                        }
                    }
                    if let Some(idx) = edge_to_delete {
                        if idx < graph.edges.len() {
                            graph.edges.remove(idx);
                        }
                    }
                    
                    // Draw nodes and handle interactions
                    for node in &mut graph.nodes {
                        let node_pos = Pos2::new(
                            graph_rect.min.x + self.pan_offset.x + node.x,
                            graph_rect.min.y + self.pan_offset.y + node.y
                        );
                        let node_rect = Rect::from_min_size(node_pos, egui::vec2(120.0, 60.0));
                        
                        // Node background color based on status
                        let node_color = match node.status.as_str() {
                            "running" => Color32::from_rgb(33, 150, 243),   // Blue
                            "success" => Color32::from_rgb(76, 175, 80),    // Green  
                            "error" => Color32::from_rgb(244, 67, 54),      // Red
                            "cache" => Color32::from_rgb(156, 39, 176),     // Purple
                            "pending" => Color32::from_rgb(96, 125, 139),   // Blue Gray
                            _ => Color32::from_rgb(34, 34, 34),             // Dark Gray
                        };
                        
                        painter.rect_filled(node_rect, 12.0, node_color);
                        
                        // Highlight node if it's the connection source
                        if self.connecting_from.as_ref() == Some(&node.id) {
                            painter.rect_stroke(node_rect, 12.0, Stroke::new(3.0, Color32::YELLOW));
                        } else {
                            painter.rect_stroke(node_rect, 12.0, Stroke::new(2.0, Color32::from_gray(68)));
                        }
                        
                        // Node text
                        painter.text(
                            node_rect.center() - egui::vec2(0.0, 8.0),
                            egui::Align2::CENTER_CENTER,
                            &node.id,
                            FontId::default(),
                            Color32::WHITE
                        );
                        
                        painter.text(
                            node_rect.center() + egui::vec2(0.0, 8.0),
                            egui::Align2::CENTER_CENTER,
                            format!("{} ({})", node.run, node.status),
                            FontId::proportional(10.0),
                            Color32::from_gray(221)
                        );
                        
                        // Handle node interaction
                        let node_response = ui.interact(node_rect, egui::Id::new(&node.id), egui::Sense::click_and_drag());
                        
                        // Debug: Check if node is being interacted with
                        if node_response.hovered() {
                            // Highlight hovered node
                            painter.rect_stroke(node_rect, 12.0, Stroke::new(3.0, Color32::YELLOW));
                        }
                        
                        if node_response.clicked() {
                            // Handle connection mode
                            if let Some(ref from_id) = self.connecting_from {
                                if from_id != &node.id {
                                    // Create edge
                                    let edge = crate::backend::GraphEdge {
                                        from: from_id.clone(),
                                        to: node.id.clone(),
                                    };
                                    if !graph.edges.iter().any(|e| e.from == edge.from && e.to == edge.to) {
                                        graph.edges.push(edge);
                                    }
                                }
                                self.connecting_from = None;
                            } else {
                                node_clicked = Some(node.id.clone());
                            }
                        }
                        
                        // Right-click for context menu
                        if node_response.secondary_clicked() {
                            node_clicked = Some(node.id.clone());
                            // For now, just select the node on right-click
                            // Context menu can be added later with proper egui version
                        }
                        
                        if node_response.dragged() && self.connecting_from.is_none() {
                            // Get the drag delta from the node response
                            let drag_delta = node_response.drag_delta();
                            
                            // Apply the drag delta directly to the node position
                            node.x += drag_delta.x;
                            node.y += drag_delta.y;
                        }
                    }
                    
                    // Canvas panning: drag background when not dragging a node
                    if response.dragged() {
                        let delta = response.drag_delta();
                        self.pan_offset += delta;
                    }
                }
                
                // Node inspector
                if let Some(ref selected_id) = selected_node_id {
                    if let Some(selected_node) = graph.nodes.iter_mut().find(|n| n.id == *selected_id) {
                        ui.separator();
                        ui.heading("Node Inspector");
                        
                        ui.horizontal(|ui| {
                            ui.label("ID:");
                            ui.label(&selected_node.id);
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Run:");
                            egui::ComboBox::from_id_salt("node_run_combo")
                                .selected_text(&selected_node.run)
                                .show_ui(ui, |ui| {
                                    for (i, plugin) in plugins.iter().enumerate() {
                                        ui.push_id(format!("node_plugin_option_{}", i), |ui| {
                                            ui.selectable_value(&mut selected_node.run, plugin.name.clone(), &plugin.name);
                                        });
                                    }
                                });
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Status:");
                            let status_color = match selected_node.status.as_str() {
                                "running" => Color32::BLUE,
                                "success" => Color32::GREEN,
                                "error" => Color32::RED,
                                "cache" => Color32::BROWN,
                                _ => Color32::GRAY,
                            };
                            ui.colored_label(status_color, &selected_node.status);
                        });
                        
                        if let Some(ref msg) = selected_node.message {
                            ui.horizontal(|ui| {
                                ui.label("Message:");
                                ui.label(msg);
                            });
                        }
                        
                        if let Some(ref output) = selected_node.output {
                            ui.collapsing("node_output", |ui| {
                                egui::ScrollArea::vertical()
                                    .max_height(100.0)
                                    .show(ui, |ui| {
                                        ui.text_edit_multiline(&mut output.as_str());
                                    });
                            });
                        }
                        
                        if let Some(ref error) = selected_node.error {
                            ui.collapsing("node_error", |ui| {
                                ui.colored_label(Color32::RED, error);
                            });
                        }
                        
                        ui.horizontal(|ui| {
                            if ui.add(egui::Button::new("🔗 Connect From")).clicked() {
                                self.connecting_from = Some(selected_node.id.clone());
                            }
                            
                            ui.add_space(10.0);
                            
                            if ui.add(egui::Button::new("🗑️ Delete Node")
                                .fill(Color32::from_rgb(244, 67, 54)))
                                .clicked() {
                                should_remove_node = true;
                            }
                        });

                        ui.separator();
                        ui.heading("Piping");
                        // Let user pick which predecessor provides input (input_from)
                        let incoming: Vec<String> = graph.edges.iter()
                            .filter(|e| e.to == selected_node.id)
                            .map(|e| e.from.clone())
                            .collect();
                        if !incoming.is_empty() {
                            let mut chosen = self.pipe_source_for_node
                                .get(&selected_node.id)
                                .cloned()
                                .unwrap_or_else(|| incoming[0].clone());
                            egui::ComboBox::from_id_salt("node_pipe_from")
                                .selected_text(&chosen)
                                .show_ui(ui, |ui| {
                                    for pred in &incoming {
                                        ui.selectable_value(&mut chosen, pred.clone(), pred);
                                    }
                                });
                            // Apply choice by reordering edges so chosen is first among incoming
                            if self.pipe_source_for_node.get(&selected_node.id) != Some(&chosen) {
                                self.pipe_source_for_node.insert(selected_node.id.clone(), chosen.clone());
                                // Move the chosen edge earlier in list to influence export order
                                if let Some(pos) = graph.edges.iter().position(|e| e.to == selected_node.id && e.from == chosen) {
                                    let edge = graph.edges.remove(pos);
                                    // Insert at front before other edges to same target
                                    let insert_pos = graph.edges.iter().position(|e| e.to == selected_node.id).unwrap_or(graph.edges.len());
                                    graph.edges.insert(insert_pos, edge);
                                }
                            }
                            ui.label("Selected source will be used as input_from; others become depends_on.");
                        } else {
                            ui.label("No incoming connections.");
                        }
                    }
                }
            }
        }
        
        // Handle state updates after dropping the lock
        {
            let mut state = self.state.lock().unwrap();
            if let Some(clicked_id) = node_clicked {
                state.selected_node = Some(clicked_id);
            }
            
            if should_remove_node {
                if let (Some(ref mut graph), Some(ref selected_id)) = (&mut state.graph, &selected_node_id) {
                    graph.nodes.retain(|n| n.id != *selected_id);
                    graph.edges.retain(|e| e.from != *selected_id && e.to != *selected_id);
                    state.selected_node = None;
                }
            }
        }
        });
    }
    
    fn show_live_logs_section(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.heading("📊 Live Logs & Execution Status");
            
            let (is_running, execution_progress, workflow_result, logs) = {
                let state = self.state.lock().unwrap();
                (
                    state.is_running,
                    state.execution_progress,
                    state.workflow_result.clone(),
                    state.live_logs.clone()
                )
            };
            
            // Show execution status indicator with better design
            ui.horizontal(|ui| {
                if is_running {
                    ui.spinner();
                    ui.colored_label(Color32::from_rgb(33, 150, 243), 
                        egui::RichText::new("🔄 Workflow Executing").size(14.0));
                    ui.add(egui::ProgressBar::new(execution_progress)
                        .show_percentage()
                        .fill(Color32::from_rgb(33, 150, 243)));
                } else if let Some(ref result) = workflow_result {
                    if result.success {
                        ui.colored_label(Color32::from_rgb(76, 175, 80), 
                            egui::RichText::new("✅ Execution Complete").size(14.0));
                    } else {
                        ui.colored_label(Color32::from_rgb(244, 67, 54), 
                            egui::RichText::new("❌ Execution Failed").size(14.0));
                    }
                } else {
                    ui.colored_label(Color32::GRAY, 
                        egui::RichText::new("⏸️ Ready").size(14.0));
                }
            });
            
            ui.add_space(10.0);
            
            // Log controls with better styling
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("📝 Logs:").size(14.0));
                if ui.add(egui::Button::new("🗑️ Clear")).clicked() {
                    let mut state = self.state.lock().unwrap();
                    state.live_logs.clear();
                }
            });
            
            // Live logs display with improved styling
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .auto_shrink([false, true])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for log in &logs {
                        // Color code based on log content with better colors
                        let (color, icon) = if log.contains("✓ DONE") {
                            (Color32::from_rgb(76, 175, 80), "✅")
                        } else if log.contains("✗ ERROR") {
                            (Color32::from_rgb(244, 67, 54), "❌")
                        } else if log.contains("running") {
                            (Color32::from_rgb(33, 150, 243), "🔄")
                        } else if log.contains("success") || log.contains("cache") {
                            (Color32::from_rgb(76, 175, 80), "✅")
                        } else if log.contains("error") || log.contains("failed") {
                            (Color32::from_rgb(244, 67, 54), "❌")
                        } else {
                            (Color32::WHITE, "ℹ️")
                        };
                        
                        ui.horizontal(|ui| {
                            ui.label(icon);
                            ui.colored_label(color, log);
                        });
                    }
                    
                    // Show empty state with better styling
                    if logs.is_empty() {
                        ui.centered_and_justified(|ui| {
                            ui.colored_label(Color32::GRAY, 
                                egui::RichText::new("No logs yet. Run a workflow to see execution logs here.").size(12.0));
                        });
                    }
                });
        });
    }
}