use crate::backend::{GraphEdge, GraphNode, UiPluginInfo};
use eframe::egui::{self, Color32, Ui};
use std::collections::HashMap;

pub enum InspectorAction {
    None,
    DeleteNode,
}

pub fn show(
    ui: &mut Ui,
    node: &mut GraphNode,
    plugins: &[UiPluginInfo],
    edges: &mut Vec<GraphEdge>,
    pipe_source_for_node: &mut HashMap<String, String>,
    connecting_from: &mut Option<String>,
) -> InspectorAction {
    let mut action = InspectorAction::None;

    ui.separator();
    ui.heading("Node Inspector");

    ui.horizontal(|ui| {
        ui.label("ID:");
        ui.label(&node.id);
    });

    ui.horizontal(|ui| {
        ui.label("Run:");
        egui::ComboBox::from_id_salt("node_run_combo")
            .selected_text(&node.run)
            .show_ui(ui, |ui| {
                for (i, plugin) in plugins.iter().enumerate() {
                    ui.push_id(format!("node_plugin_option_{}", i), |ui| {
                        ui.selectable_value(&mut node.run, plugin.name.clone(), &plugin.name);
                    });
                }
            });
    });

    ui.horizontal(|ui| {
        ui.label("Status:");
        let status_color = match node.status.as_str() {
            "running" => Color32::BLUE,
            "success" => Color32::GREEN,
            "error" => Color32::RED,
            "cache" => Color32::BROWN,
            _ => Color32::GRAY,
        };
        ui.colored_label(status_color, &node.status);
    });

    if let Some(ref msg) = node.message {
        ui.horizontal(|ui| {
            ui.label("Message:");
            ui.label(msg);
        });
    }

    if let Some(ref output) = node.output {
        ui.collapsing("node_output", |ui| {
            egui::ScrollArea::vertical()
                .max_height(100.0)
                .show(ui, |ui| {
                    ui.text_edit_multiline(&mut output.clone());
                });
        });
    }

    if let Some(ref error) = node.error {
        ui.collapsing("node_error", |ui| {
            ui.colored_label(Color32::RED, error);
        });
    }

    ui.horizontal(|ui| {
        if ui.add(egui::Button::new("🔗 Connect From")).clicked() {
            *connecting_from = Some(node.id.clone());
        }

        ui.add_space(10.0);

        if ui
            .add(egui::Button::new("🗑️ Delete Node").fill(Color32::from_rgb(244, 67, 54)))
            .clicked()
        {
            action = InspectorAction::DeleteNode;
        }
    });

    ui.separator();
    ui.heading("Piping");
    // Let user pick which predecessor provides input (input_from)
    let incoming: Vec<String> = edges
        .iter()
        .filter(|e| e.to == node.id)
        .map(|e| e.from.clone())
        .collect();
    if !incoming.is_empty() {
        let mut chosen = pipe_source_for_node
            .get(&node.id)
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
        if pipe_source_for_node.get(&node.id) != Some(&chosen) {
            pipe_source_for_node.insert(node.id.clone(), chosen.clone());
            // Move the chosen edge earlier in list to influence export order
            if let Some(pos) = edges
                .iter()
                .position(|e| e.to == node.id && e.from == chosen)
            {
                let edge = edges.remove(pos);
                // Insert at front before other edges to same target
                let insert_pos = edges
                    .iter()
                    .position(|e| e.to == node.id)
                    .unwrap_or(edges.len());
                edges.insert(insert_pos, edge);
            }
        }
        ui.label("Selected source will be used as input_from; others become depends_on.");
    } else {
        ui.label("No incoming connections.");
    }

    action
}
