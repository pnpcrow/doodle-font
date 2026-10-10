#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app;
#[cfg(windows)]
mod native_window;
fn main() -> eframe::Result {
    let icon = doodle_core::brand::rgba();
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Doodle Font")
            .with_decorations(false)
            .with_inner_size([1480., 940.])
            .with_icon(eframe::egui::IconData {
                width: icon.width(),
                height: icon.height(),
                rgba: icon.into_raw(),
            })
            .with_min_inner_size([1120., 760.]),
        ..Default::default()
    };
    eframe::run_native(
        "Doodle Font",
        options,
        Box::new(|cc| Ok(Box::new(app::Studio::new(cc)))),
    )
}
