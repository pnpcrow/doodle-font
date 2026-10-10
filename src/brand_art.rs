//! Canonical Doodle Font mark, expressed as curves in a 256-unit square.
//! The header, native window and executable assets all derive from this geometry.
pub fn render(size: u32) -> tiny_skia::Pixmap {
    use tiny_skia::{FillRule, LineCap, Paint, PathBuilder, Pixmap, Stroke, Transform};
    let mut image = Pixmap::new(size, size).expect("nonzero icon size");
    let transform = Transform::from_scale(size as f32 / 256., size as f32 / 256.);
    let mut background = PathBuilder::new();
    background.move_to(76., 8.);
    background.line_to(180., 8.);
    background.cubic_to(224., 8., 248., 32., 248., 76.);
    background.line_to(248., 180.);
    background.cubic_to(248., 224., 224., 248., 180., 248.);
    background.line_to(76., 248.);
    background.cubic_to(32., 248., 8., 224., 8., 180.);
    background.line_to(8., 76.);
    background.cubic_to(8., 32., 32., 8., 76., 8.);
    background.close();
    let mut paint = Paint::default();
    paint.set_color_rgba8(190, 83, 51, 255);
    image.fill_path(
        &background.finish().unwrap(),
        &paint,
        FillRule::Winding,
        transform,
        None,
    );
    let mut letter = PathBuilder::new();
    letter.move_to(166., 61.);
    letter.line_to(166., 188.);
    letter.move_to(166., 139.);
    letter.cubic_to(164., 112., 147., 100., 128., 100.);
    letter.cubic_to(101., 100., 84., 119., 84., 146.);
    letter.cubic_to(84., 174., 101., 191., 128., 191.);
    letter.cubic_to(147., 191., 164., 179., 166., 157.);
    paint.set_color_rgba8(255, 255, 255, 255);
    image.stroke_path(
        &letter.finish().unwrap(),
        &paint,
        &Stroke {
            width: 13.,
            line_cap: LineCap::Round,
            ..Default::default()
        },
        transform,
        None,
    );
    image
}
