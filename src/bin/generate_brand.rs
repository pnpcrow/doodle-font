//! Run with `cargo run --bin generate_brand` to regenerate the shared brand assets.
#[path = "../brand_art.rs"]
mod brand_art;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use image::codecs::ico::{IcoEncoder, IcoFrame};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    let sizes = [16, 20, 24, 32, 40, 48, 64, 128, 256];
    let mut frames = Vec::new();
    for size in sizes {
        let pixmap = brand_art::render(size);
        // tiny-skia's PNG encoder unpremultiplies antialiased transparent edges.
        let png = pixmap.encode_png()?;
        if size == 256 {
            std::fs::write(root.join("brand.png"), &png)?;
        }
        frames.push(IcoFrame::with_encoded(
            png,
            size,
            size,
            image::ExtendedColorType::Rgba8,
        )?);
    }
    IcoEncoder::new(std::fs::File::create(root.join("icon.ico"))?).encode_images(&frames)?;
    println!("Generated shared d mark at {} sizes", sizes.len());
    Ok(())
}
