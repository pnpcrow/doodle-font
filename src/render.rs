//! Render and shape actual font bytes, including our COLR v0 and sbix exports.
use crate::i18n::{format as tf, text as tr};
use tiny_skia::{PathBuilder, Pixmap, Transform};
use ttf_parser::{colr, GlyphId, OutlineBuilder};
struct Path(PathBuilder);
impl OutlineBuilder for Path {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to(x, y)
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to(x, y)
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.0.quad_to(x1, y1, x, y)
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.0.cubic_to(x1, y1, x2, y2, x, y)
    }
    fn close(&mut self) {
        self.0.close()
    }
}
struct Painter<'a, 'b, 'c> {
    face: &'a ttf_parser::Face<'b>,
    pix: &'c mut Pixmap,
    path: Option<tiny_skia::Path>,
    transform: Transform,
    stack: Vec<Transform>,
}
impl<'b> colr::Painter<'b> for Painter<'_, 'b, '_> {
    fn outline_glyph(&mut self, id: GlyphId) {
        let mut b = Path(PathBuilder::new());
        self.face.outline_glyph(id, &mut b);
        self.path = b.0.finish();
    }
    fn paint(&mut self, paint: colr::Paint<'b>) {
        if let (Some(path), colr::Paint::Solid(c)) = (&self.path, paint) {
            let mut p = tiny_skia::Paint::default();
            p.set_color_rgba8(c.red, c.green, c.blue, c.alpha);
            self.pix
                .fill_path(path, &p, tiny_skia::FillRule::Winding, self.transform, None);
        }
    }
    fn push_clip(&mut self) {}
    fn push_clip_box(&mut self, _: colr::ClipBox) {}
    fn pop_clip(&mut self) {}
    fn push_layer(&mut self, _: colr::CompositeMode) {}
    fn pop_layer(&mut self) {}
    fn push_transform(&mut self, t: ttf_parser::Transform) {
        self.stack.push(self.transform);
        self.transform = self
            .transform
            .pre_concat(Transform::from_row(t.a, t.b, t.c, t.d, t.e, t.f));
    }
    fn pop_transform(&mut self) {
        if let Some(t) = self.stack.pop() {
            self.transform = t;
        }
    }
}
pub fn text(bytes: &[u8], text: &str, locale: &str, px: f32) -> Result<image::RgbaImage, String> {
    use colr::Painter as _;
    let face = ttf_parser::Face::parse(bytes, 0)
        .map_err(|e| tf("폰트 읽기 오류: {0}", &[format!("{:?}", e)]))?;
    let hb = rustybuzz::Face::from_slice(bytes, 0).ok_or(tr("폰트 셰이핑 오류"))?;
    let mut b = rustybuzz::UnicodeBuffer::new();
    b.push_str(text);
    b.guess_segment_properties();
    if let Ok(lang) = locale.parse() {
        b.set_language(lang)
    }
    if text.chars().any(|c| ('\u{3400}'..='\u{9fff}').contains(&c)) {
        b.set_script(rustybuzz::script::HAN);
    }
    let shaped = rustybuzz::shape(&hb, &[], b);
    let scale = px / face.units_per_em() as f32;
    let width = ((shaped
        .glyph_positions()
        .iter()
        .map(|p| p.x_advance)
        .sum::<i32>() as f32
        * scale)
        .abs()
        + 1.)
        .ceil()
        .clamp(1., 4096.) as u32;
    let height = (px * 1.5).ceil() as u32;
    let mut pix = Pixmap::new(width, height).ok_or(tr("미리보기 크기가 너무 커요."))?;
    let baseline = px * 1.05;
    let mut x = 0.;
    for (info, pos) in shaped.glyph_infos().iter().zip(shaped.glyph_positions()) {
        let gid = GlyphId(info.glyph_id as u16);
        let ox = x + pos.x_offset as f32 * scale;
        let oy = baseline - pos.y_offset as f32 * scale;
        if let Some(bitmap) = face.glyph_raster_image(gid, px as u16) {
            if bitmap.format == ttf_parser::RasterImageFormat::PNG {
                let decoded =
                    image::load_from_memory_with_format(bitmap.data, image::ImageFormat::Png)
                        .map_err(|e| e.to_string())?
                        .to_rgba8();
                let ratio = px / bitmap.pixels_per_em as f32;
                let im = image::imageops::resize(
                    &decoded,
                    ((decoded.width() as f32 * ratio).round() as u32).max(1),
                    ((decoded.height() as f32 * ratio).round() as u32).max(1),
                    image::imageops::FilterType::Triangle,
                );
                let mut layer =
                    Pixmap::new(im.width(), im.height()).ok_or(tr("이미지 크기 오류"))?;
                for (d, p) in layer
                    .data_mut()
                    .as_chunks_mut::<4>()
                    .0
                    .iter_mut()
                    .zip(im.pixels())
                {
                    for i in 0..3 {
                        d[i] = (p[i] as u16 * p[3] as u16 / 255) as u8;
                    }
                    d[3] = p[3];
                }
                pix.draw_pixmap(
                    (ox + bitmap.x as f32 * ratio).round() as i32,
                    (oy - (bitmap.y as f32 + bitmap.height as f32) * ratio).round() as i32,
                    layer.as_ref(),
                    &Default::default(),
                    Transform::identity(),
                    None,
                );
            }
        } else {
            let transform = Transform::from_row(scale, 0., 0., -scale, ox, oy);
            let mut painter = Painter {
                face: &face,
                pix: &mut pix,
                path: None,
                transform,
                stack: vec![],
            };
            if face
                .paint_color_glyph(
                    gid,
                    0,
                    ttf_parser::RgbaColor::new(48, 51, 46, 255),
                    &mut painter,
                )
                .is_none()
            {
                painter.outline_glyph(gid);
                painter.paint(colr::Paint::Solid(ttf_parser::RgbaColor::new(
                    48, 51, 46, 255,
                )));
            }
        }
        x += pos.x_advance as f32 * scale;
    }
    let mut rgba = pix.data().to_vec();
    for p in rgba.as_chunks_mut::<4>().0.iter_mut() {
        if p[3] > 0 {
            let a = p[3] as u16;
            for c in &mut p[..3] {
                *c = (*c as u16 * 255 / a).min(255) as u8;
            }
        }
    }
    image::RgbaImage::from_raw(width, height, rgba).ok_or(tr("미리보기 생성 오류").into())
}
