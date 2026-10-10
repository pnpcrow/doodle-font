#[path = "../src/brand_art.rs"]
mod brand_art;

#[test]
fn header_native_window_and_all_executable_icon_sizes_share_the_canonical_mark() {
    let png = doodle_core::brand::rgba();
    let expected = image::load_from_memory(&brand_art::render(256).encode_png().unwrap())
        .unwrap()
        .to_rgba8();
    assert_eq!(png, expected, "regenerate assets with generate_brand");
    let ico = include_bytes!("../assets/icon.ico");
    let u16_at = |i| u16::from_le_bytes(ico[i..i + 2].try_into().unwrap());
    let u32_at = |i| u32::from_le_bytes(ico[i..i + 4].try_into().unwrap()) as usize;
    assert_eq!(u16_at(2), 1);
    let sizes = [16, 20, 24, 32, 40, 48, 64, 128, 256];
    assert_eq!(usize::from(u16_at(4)), sizes.len());
    for (i, size) in sizes.into_iter().enumerate() {
        let entry = 6 + i * 16;
        let offset = u32_at(entry + 12);
        let count = u32_at(entry + 8);
        let frame = image::load_from_memory(&ico[offset..offset + count])
            .unwrap()
            .to_rgba8();
        let rendered = image::load_from_memory(&brand_art::render(size).encode_png().unwrap())
            .unwrap()
            .to_rgba8();
        assert_eq!(frame, rendered, "icon size {size}");
        assert_eq!(frame.get_pixel(0, 0)[3], 0, "rounded transparent corner");
    }
}
