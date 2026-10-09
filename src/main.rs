#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app;
fn main()->eframe::Result {
    let options=eframe::NativeOptions{viewport:eframe::egui::ViewportBuilder::default().with_title("두들 폰트 · Doodle Font").with_inner_size([1480.,940.]).with_min_inner_size([1120.,760.]),..Default::default()};
    eframe::run_native("Doodle Font",options,Box::new(|cc|Ok(Box::new(app::Studio::new(cc)))))
}
