use crate::backend::{
    export_workflow_yaml, save_workflow_yaml, GraphEdge, GraphNode, UiPluginInfo, WorkflowGraph,
};
use eframe::egui::{self, Color32, Id, Pos2, Rect, Stroke, Ui, Vec2};

pub struct GraphEditorState {
    pub pan_offset: Vec2,
    pub connecting_from: Option<String>,
    pub selected_node: Option<String>,

    // Editor UI state
    pub new_node_name: String,
    pub new_node_type: String,

    // Dialog state
    pub show_save_dialog: bool,
    pub show_export_dialog: bool,
    pub new_workflow_filename: String,
}

impl Default for GraphEditorState {
    fn default() -> Self {
        Self {
            pan_offset: Vec2::ZERO,
            connecting_from: None,
            selected_node: None,
            new_node_name: String::new(),
            new_node_type: "EchoPlugin".to_string(), // Default safe value
            show_save_dialog: false,
            show_export_dialog: false,
            new_workflow_filename: "new_workflow.yaml".to_string(),
        }
    }
}

pub fn show(
    ui: &mut Ui,
    graph: &mut WorkflowGraph,
    state: &mut GraphEditorState,
    plugins: &[UiPluginInfo],
) {
    ui.group(|ui| {
        ui.heading("🎨 Visual Flow Builder");

        ui.horizontal(|ui| {
            if ui.add(egui::Button::new("🆕 New Workflow")).clicked() {
                graph.nodes.clear();
                graph.edges.clear();
                state.selected_node = None;
            }

            if ui.add(egui::Button::new("💾 Save Workflow")).clicked() {
                state.show_save_dialog = true;
            }

            if ui.add(egui::Button::new("📤 Export YAML")).clicked() {
                state.show_export_dialog = true;
            }

            ui.add_space(10.0);

            // Add delete all nodes button
            if ui
                .add(egui::Button::new("🗑️ Clear All").fill(Color32::from_rgb(244, 67, 54)))
                .clicked()
            {
                graph.nodes.clear();
                graph.edges.clear();
                state.selected_node = None;
            }

            ui.add_space(20.0);

            // Show connection mode with better styling
            if state.connecting_from.is_some() {
                ui.colored_label(
                    Color32::from_rgb(255, 193, 7),
                    egui::RichText::new("🔗 Connection mode: Click target node").size(12.0),
                );
                if ui.add(egui::Button::new("❌ Cancel")).clicked() {
                    state.connecting_from = None;
                }
            } else {
                ui.colored_label(
                    Color32::GRAY,
                    egui::RichText::new("💡 Tip: Right-click nodes for options, drag to move")
                        .size(12.0),
                );
            }
        });

        // Save dialog
        if state.show_save_dialog {
            let mut close_dialog = false;
            egui::Window::new("Save Workflow")
                .open(&mut state.show_save_dialog)
                .show(ui.ctx(), |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Filename:");
                        ui.text_edit_singleline(&mut state.new_workflow_filename);
                    });

                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            match save_workflow_yaml(graph, &state.new_workflow_filename) {
                                Ok(_) => {
                                    close_dialog = true;
                                }
                                Err(e) => {
                                    eprintln!("Save error: {}", e);
                                }
                            }
                        }

                        if ui.button("Cancel").clicked() {
                            close_dialog = true;
                        }
                    });
                });

            if close_dialog {
                state.show_save_dialog = false;
            }
        }

        // Export dialog
        if state.show_export_dialog {
            let mut close_dialog = false;
            egui::Window::new("Export YAML")
                .open(&mut state.show_export_dialog)
                .show(ui.ctx(), |ui| {
                    match export_workflow_yaml(graph) {
                        Ok(yaml) => {
                            ui.label("Generated YAML:");
                            egui::ScrollArea::vertical()
                                .max_height(300.0)
                                .show(ui, |ui| {
                                    ui.text_edit_multiline(&mut yaml.clone());
                                });
                        }
                        Err(e) => {
                            ui.colored_label(Color32::RED, format!("Export error: {}", e));
                        }
                    }

                    if ui.button("Close").clicked() {
                        close_dialog = true;
                    }
                });

            if close_dialog {
                state.show_export_dialog = false;
            }
        }

        // Add node controls
        ui.horizontal(|ui| {
            ui.label("Add Node:");
            ui.text_edit_singleline(&mut state.new_node_name);

            egui::ComboBox::from_id_salt("plugin_type_combo")
                .selected_text(&state.new_node_type)
                .show_ui(ui, |ui| {
                    for (i, plugin) in plugins.iter().enumerate() {
                        ui.push_id(format!("plugin_option_{}", i), |ui| {
                            ui.selectable_value(
                                &mut state.new_node_type,
                                plugin.name.clone(),
                                &plugin.name,
                            );
                        });
                    }
                });

            if ui.button("Add Node").clicked() {
                let node_id = if state.new_node_name.is_empty() {
                    format!("node_{}", graph.nodes.len() + 1)
                } else {
                    state.new_node_name.clone()
                };

                // Calculate better initial position
                let node_count = graph.nodes.len();
                let cols = 4;
                let col = node_count % cols;
                let row = node_count / cols;
                let spacing_x = 200.0;
                let spacing_y = 120.0;

                graph.nodes.push(GraphNode {
                    id: node_id,
                    run: state.new_node_type.clone(),
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

                state.new_node_name.clear();
            }
        });

        // Visual graph area
        let available_rect = ui.available_rect_before_wrap();
        // Reserve at least some height
        let graph_rect = Rect::from_min_size(
            available_rect.min,
            egui::vec2(available_rect.width(), 400.0),
        );

        let response = ui.allocate_rect(graph_rect, egui::Sense::click_and_drag());

        if ui.is_rect_visible(graph_rect) {
            let painter = ui.painter();

            // Draw background
            painter.rect_filled(graph_rect, 4.0, Color32::from_gray(248));

            // Draw grid (respecting pan)
            let grid_size = 40.0;
            let cols = (graph_rect.width() / grid_size).ceil() as i32;
            let rows = (graph_rect.height() / grid_size).ceil() as i32;

            // Clip drawing to graph rect
            let painter = painter.with_clip_rect(graph_rect);

            for i in 0..cols {
                let x = graph_rect.min.x + (state.pan_offset.x % grid_size) + i as f32 * grid_size;
                painter.line_segment(
                    [
                        Pos2::new(x, graph_rect.min.y),
                        Pos2::new(x, graph_rect.max.y),
                    ],
                    Stroke::new(1.0, Color32::from_gray(238)),
                );
            }
            for j in 0..rows {
                let y = graph_rect.min.y + (state.pan_offset.y % grid_size) + j as f32 * grid_size;
                painter.line_segment(
                    [
                        Pos2::new(graph_rect.min.x, y),
                        Pos2::new(graph_rect.max.x, y),
                    ],
                    Stroke::new(1.0, Color32::from_gray(238)),
                );
            }

            // Draw edges
            let mut edge_to_delete: Option<usize> = None;
            for (i, edge) in graph.edges.iter().enumerate() {
                if let (Some(from_node), Some(to_node)) = (
                    graph.nodes.iter().find(|n| n.id == edge.from),
                    graph.nodes.iter().find(|n| n.id == edge.to),
                ) {
                    let from_pos = Pos2::new(
                        graph_rect.min.x + state.pan_offset.x + from_node.x + 120.0,
                        graph_rect.min.y + state.pan_offset.y + from_node.y + 30.0,
                    );
                    let to_pos = Pos2::new(
                        graph_rect.min.x + state.pan_offset.x + to_node.x,
                        graph_rect.min.y + state.pan_offset.y + to_node.y + 30.0,
                    );

                    // Draw arrow line
                    painter.line_segment(
                        [from_pos, to_pos],
                        Stroke::new(2.0, Color32::from_gray(136)),
                    );

                    // Draw arrowhead
                    let direction = (to_pos - from_pos).normalized();
                    let arrow_size = 8.0;
                    let arrow_tip = to_pos - direction * 5.0;
                    let perpendicular = Vec2::new(-direction.y, direction.x);

                    let arrow_p1 =
                        arrow_tip - direction * arrow_size + perpendicular * arrow_size * 0.5;
                    let arrow_p2 =
                        arrow_tip - direction * arrow_size - perpendicular * arrow_size * 0.5;

                    painter.line_segment(
                        [arrow_tip, arrow_p1],
                        Stroke::new(2.0, Color32::from_gray(136)),
                    );
                    painter.line_segment(
                        [arrow_tip, arrow_p2],
                        Stroke::new(2.0, Color32::from_gray(136)),
                    );

                    // Check for edge click to delete
                    let edge_center = (from_pos + to_pos.to_vec2()) * 0.5;
                    let edge_rect = Rect::from_center_size(edge_center, Vec2::splat(20.0));
                    let edge_response = ui.interact(
                        edge_rect,
                        Id::new(format!("edge_{}", i)),
                        egui::Sense::click(),
                    );
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

            // Draw nodes
            let mut node_clicked = None;
            for node in &mut graph.nodes {
                let node_pos = Pos2::new(
                    graph_rect.min.x + state.pan_offset.x + node.x,
                    graph_rect.min.y + state.pan_offset.y + node.y,
                );
                let node_rect = Rect::from_min_size(node_pos, egui::vec2(120.0, 60.0));

                // Node background color based on status
                let node_color = match node.status.as_str() {
                    "running" => Color32::from_rgb(33, 150, 243),
                    "success" => Color32::from_rgb(76, 175, 80),
                    "error" => Color32::from_rgb(244, 67, 54),
                    "cache" => Color32::from_rgb(156, 39, 176),
                    "pending" => Color32::from_rgb(96, 125, 139),
                    _ => Color32::from_rgb(34, 34, 34),
                };

                painter.rect_filled(node_rect, 12.0, node_color);

                // Highlight/Stroke
                if state.connecting_from.as_ref() == Some(&node.id) {
                    painter.rect_stroke(node_rect, 12.0, Stroke::new(3.0, Color32::YELLOW));
                } else if state.selected_node.as_ref() == Some(&node.id) {
                    painter.rect_stroke(node_rect, 12.0, Stroke::new(2.0, Color32::WHITE));
                } else {
                    painter.rect_stroke(node_rect, 12.0, Stroke::new(2.0, Color32::from_gray(68)));
                }

                painter.text(
                    node_rect.center() - egui::vec2(0.0, 8.0),
                    egui::Align2::CENTER_CENTER,
                    &node.id,
                    egui::FontId::default(),
                    Color32::WHITE,
                );

                painter.text(
                    node_rect.center() + egui::vec2(0.0, 8.0),
                    egui::Align2::CENTER_CENTER,
                    format!("{} ({})", node.run, node.status),
                    egui::FontId::proportional(10.0),
                    Color32::from_gray(221),
                );

                let node_response =
                    ui.interact(node_rect, Id::new(&node.id), egui::Sense::click_and_drag());

                if node_response.clicked() || node_response.secondary_clicked() {
                    if let Some(ref from_id) = state.connecting_from {
                        if from_id != &node.id {
                            let edge = GraphEdge {
                                from: from_id.clone(),
                                to: node.id.clone(),
                            };
                            if !graph
                                .edges
                                .iter()
                                .any(|e| e.from == edge.from && e.to == edge.to)
                            {
                                graph.edges.push(edge);
                            }
                        }
                        state.connecting_from = None;
                    } else {
                        node_clicked = Some(node.id.clone());
                    }
                }

                if node_response.dragged() && state.connecting_from.is_none() {
                    let drag_delta = node_response.drag_delta();
                    node.x += drag_delta.x;
                    node.y += drag_delta.y;
                }
            }

            if let Some(click_id) = node_clicked {
                state.selected_node = Some(click_id);
            }

            // Pan interaction
            if response.dragged() {
                state.pan_offset += response.drag_delta();
            }
        }
    });
}
