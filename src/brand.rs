//! One image shared by the title bar and native window icon.
use eframe::egui;

pub const PNG: &[u8] = include_bytes!("../assets/brand.png");

pub fn rgba() -> image::RgbaImage {
    image::load_from_memory(PNG)
        .expect("bundled Doodle Font mark")
        .to_rgba8()
}

pub fn texture(ctx: &egui::Context) -> egui::TextureHandle {
    let id = egui::Id::new("doodle-brand-texture");
    if let Some(texture) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return texture;
    }
    let icon = rgba();
    let texture = ctx.load_texture(
        "Doodle Font logo",
        egui::ColorImage::from_rgba_unmultiplied(
            [icon.width() as usize, icon.height() as usize],
            icon.as_raw(),
        ),
        egui::TextureOptions::LINEAR,
    );
    ctx.data_mut(|d| d.insert_temp(id, texture.clone()));
    texture
}
