use crate::backend::WorkflowResult;
use eframe::egui::{self, Color32, RichText, Ui};

pub fn show(
    ui: &mut Ui,
    logs: &mut Vec<String>,
    is_running: bool,
    execution_progress: f32,
    workflow_result: &Option<WorkflowResult>,
) {
    ui.group(|ui| {
        ui.heading("📊 Live Logs & Execution Status");

        // Show execution status indicator with better design
        ui.horizontal(|ui| {
            if is_running {
                ui.spinner();
                ui.colored_label(
                    Color32::from_rgb(33, 150, 243),
                    RichText::new("🔄 Workflow Executing").size(14.0),
                );
                ui.add(
                    egui::ProgressBar::new(execution_progress)
                        .show_percentage()
                        .fill(Color32::from_rgb(33, 150, 243)),
                );
            } else if let Some(ref result) = workflow_result {
                if result.success {
                    ui.colored_label(
                        Color32::from_rgb(76, 175, 80),
                        RichText::new("✅ Execution Complete").size(14.0),
                    );
                } else {
                    ui.colored_label(
                        Color32::from_rgb(244, 67, 54),
                        RichText::new("❌ Execution Failed").size(14.0),
                    );
                }
            } else {
                ui.colored_label(Color32::GRAY, RichText::new("⏸️ Ready").size(14.0));
            }
        });

        ui.add_space(10.0);

        // Log controls with better styling
        ui.horizontal(|ui| {
            ui.label(RichText::new("📝 Logs:").size(14.0));
            if ui.add(egui::Button::new("🗑️ Clear")).clicked() {
                logs.clear();
            }
        });

        // Live logs display with improved styling
        egui::ScrollArea::vertical()
            .max_height(200.0)
            .auto_shrink([false, true])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for log in logs.iter() {
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

                if logs.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.colored_label(
                            Color32::GRAY,
                            RichText::new(
                                "No logs yet. Run a workflow to see execution logs here.",
                            )
                            .size(12.0),
                        );
                    });
                }
            });
    });
}
