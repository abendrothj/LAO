use eframe::egui;

mod app;
mod backend;
mod components;
// mod ui_old; // Not compiling ui_old to avoid duplicate symbol errors or unused code warnings if possible, but user asked to keep it.
// Actually, if I include `mod ui_old;`, it will try to compile it.
// `ui_old.rs` has `LaoApp` struct which might conflict if I import it, but I am not importing it.
// However, `ui_old.rs` implementation of `eframe::App` for `LaoApp` might conflict if I don't rename the struct in `ui_old.rs`.
// Good catch. `app.rs` defines `LaoApp`. `ui_old.rs` defines `LaoApp`.
// Rust allows multiple structs with same name in different modules.
// But if I `mod ui_old`, it compiles.
// User said: "Don't delete ui.rs yet. Rename it to ui_old.rs so you have a reference while copying."
// It doesn't strictly say "make it part of the build".
// If I `mod ui_old`, it compiles `ui_old.rs`.
// Compiling it might be useful for reference but also might cause errors if `ui_old` logic is broken or dependencies change.
// I'll skip `mod ui_old` in `main.rs` so it's just a file on disk, not compiled.
// This is safer.

use app::LaoApp;

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("LAO Orchestrator"),
        ..Default::default()
    };

    eframe::run_native(
        "LAO Orchestrator",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(Box::new(LaoApp::new(cc)))
        }),
    )
}
