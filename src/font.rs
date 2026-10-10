// Native OpenType compiler. No system-font outlines or external executables.
use crate::model::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
mod formats;
#[derive(Default, Deserialize)]
pub struct ExportOptions {
    pub format: Option<String>,
    pub mode: Option<String>,
    pub locale: Option<String>,
    pub group: Option<String>,
    pub ids: Option<Vec<String>>,
    pub split: Option<bool>,
}
#[derive(Serialize)]
pub struct ExportResult {
    pub filename: String,
    pub data: String,
    pub reports: Vec<Report>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub family: String,
    pub format: String,
    pub glyph_count: usize,
    pub authored_count: usize,
    pub tables: Vec<String>,
    pub mode: String,
    pub warnings: Vec<String>,
    pub attribution: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license_text: Option<String>,
}
#[derive(Default, Clone)]
struct B(Vec<u8>);
impl B {
    fn u16(&mut self, v: u16) {
        self.0.extend(v.to_be_bytes())
    }
    fn i16(&mut self, v: i16) {
        self.u16(v as u16)
    }
    fn u32(&mut self, v: u32) {
        self.0.extend(v.to_be_bytes())
    }
    fn put(&mut self, pos: usize, v: u32) {
        self.0[pos..pos + 4].copy_from_slice(&v.to_be_bytes())
    }
    fn bytes(&mut self, v: &[u8]) {
        self.0.extend(v)
    }
    fn zeros(&mut self, n: usize) {
        self.0.resize(self.0.len() + n, 0)
    }
}
fn coverage(ids: &[u16]) -> B {
    let mut b = B::default();
    b.u16(1);
    b.u16(ids.len() as u16);
    for id in ids {
        b.u16(*id)
    }
    b
}
fn offset(v: usize) -> Result<u16, String> {
    u16::try_from(v)
        .map_err(|_| "글리프 치환 규칙이 너무 많아요. 문자 집합별로 분할해 주세요.".into())
}
fn ellipse(x: f64, y: f64, w: f64, h: f64, n: usize) -> Vec<[f64; 2]> {
    (0..n)
        .map(|i| {
            let t = i as f64 * std::f64::consts::TAU / n as f64;
            [x + w / 2. + w / 2. * t.cos(), y + h / 2. + h / 2. * t.sin()]
        })
        .collect()
}
pub fn decode_image(src: &str) -> Result<image::RgbaImage, String> {
    let bytes = STANDARD
        .decode(
            src.strip_prefix("data:image/png;base64,")
                .ok_or("PNG 원본이 아니에요.")?,
        )
        .map_err(|_| "이미지 인코딩 오류")?;
    let mut reader =
        image::ImageReader::with_format(std::io::Cursor::new(bytes), image::ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    reader
        .decode()
        .map(|im| im.to_rgba8())
        .map_err(|_| "4096 × 4096 이하 PNG를 사용해 주세요.".into())
}
pub fn polygons(s: &Shape) -> Result<Vec<Vec<[f64; 2]>>, String> {
    if s.kind == "outline" {
        return Ok(s
            .contours
            .iter()
            .map(|contour| {
                contour
                    .iter()
                    .map(|p| [s.x + p[0] * s.width / 1000., s.y + p[1] * s.height / 1000.])
                    .collect()
            })
            .collect());
    }
    let mut out = vec![];
    if s.kind == "image" {
        let im = decode_image(s.src.as_deref().unwrap_or(""))?;
        let im = image::imageops::resize(&im, 64, 64, image::imageops::FilterType::Triangle);
        for y in 0..64 {
            let mut start = None;
            for x in 0..=64 {
                let on = x < 64 && im.get_pixel(x, y)[3] >= 96;
                if on && start.is_none() {
                    start = Some(x)
                }
                if !on {
                    if let Some(a) = start.take() {
                        let xx = s.x + a as f64 / 64. * s.width;
                        let yy = s.y + y as f64 / 64. * s.height;
                        let w = (x - a) as f64 / 64. * s.width;
                        let h = s.height / 64.;
                        out.push(vec![[xx, yy], [xx + w, yy], [xx + w, yy + h], [xx, yy + h]]);
                    }
                }
            }
        }
        return Ok(out);
    }
    let mut pts = match s.kind.as_str() {
        "rect" => vec![
            [s.x, s.y],
            [s.x + s.width, s.y],
            [s.x + s.width, s.y + s.height],
            [s.x, s.y + s.height],
        ],
        "ellipse" => ellipse(s.x, s.y, s.width, s.height, 48),
        _ => s.points.clone(),
    };
    if pts.is_empty() {
        return Ok(out);
    }
    if s.fill {
        out.push(pts);
        return Ok(out);
    }
    if s.kind != "path" {
        pts.push(pts[0]);
    }
    let r = s.stroke / 2.;
    if r <= 0. {
        return Ok(out);
    }
    for p in &pts {
        out.push(ellipse(p[0] - r, p[1] - r, r * 2., r * 2., 12));
    }
    for pair in pts.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let len = dx.hypot(dy);
        if len < 0.001 {
            continue;
        }
        let nx = -dy / len * r;
        let ny = dx / len * r;
        out.push(vec![
            [a[0] + nx, a[1] + ny],
            [b[0] + nx, b[1] + ny],
            [b[0] - nx, b[1] - ny],
            [a[0] - nx, a[1] - ny],
        ]);
    }
    Ok(out)
}
fn shapes(g: &Glyph) -> Vec<&Shape> {
    if g.status == "blank" {
        vec![]
    } else {
        g.artwork
            .shapes
            .iter()
            .filter(|s| s.visible && s.opacity > 0.)
            .collect()
    }
}
struct Outline {
    data: Vec<u8>,
    bounds: [i16; 4],
    points: u16,
    contours: u16,
}
fn outline(shapes: &[&Shape], cff_width: Option<u16>) -> Result<Outline, String> {
    let mut contours = Vec::new();
    for s in shapes {
        for mut pts in polygons(s)? {
            if pts.len() < 3 {
                continue;
            }
            let area: f64 = pts
                .iter()
                .zip(pts.iter().cycle().skip(1))
                .take(pts.len())
                .map(|(a, b)| a[0] * b[1] - b[0] * a[1])
                .sum();
            if s.kind != "outline" && area < 0. {
                pts.reverse();
            }
            contours.push(pts);
        }
    }
    let count: usize = contours.iter().map(Vec::len).sum();
    check(
        count < 65000 && contours.len() < 32000,
        "윤곽선이 너무 복잡해요. 선 보정이나 레이어 단순화를 적용해 주세요.",
    )?;
    if count == 0 {
        return Ok(Outline {
            data: if let Some(width) = cff_width {
                formats::charstring(&[], &[], width)?
            } else {
                vec![]
            },
            bounds: [0; 4],
            points: 0,
            contours: 0,
        });
    }
    let pts: Vec<[i16; 2]> = contours
        .iter()
        .flatten()
        .map(|p| [p[0].round() as i16, (800. - p[1]).round() as i16])
        .collect();
    let bounds = [
        pts.iter().map(|p| p[0]).min().unwrap(),
        pts.iter().map(|p| p[1]).min().unwrap(),
        pts.iter().map(|p| p[0]).max().unwrap(),
        pts.iter().map(|p| p[1]).max().unwrap(),
    ];
    if let Some(width) = cff_width {
        let lengths: Vec<usize> = contours.iter().map(Vec::len).collect();
        let data = formats::charstring(&pts, &lengths, width)?;
        check(
            data.len() <= 65535,
            "OTF 윤곽선이 너무 복잡해요. 레이어를 단순화하거나 TTF·WOFF 형식을 선택해 주세요.",
        )?;
        return Ok(Outline {
            data,
            bounds,
            points: count as u16,
            contours: contours.len() as u16,
        });
    }
    let mut b = B::default();
    b.i16(contours.len() as i16);
    for v in bounds {
        b.i16(v)
    }
    let mut end = 0;
    for c in &contours {
        end += c.len();
        b.u16((end - 1) as u16)
    }
    b.u16(0);
    b.bytes(&vec![1; pts.len()]);
    for axis in 0..2 {
        let mut prev = 0;
        for pt in &pts {
            b.i16(pt[axis] - prev);
            prev = pt[axis];
        }
    }
    if b.0.len() % 2 == 1 {
        b.0.push(0)
    }
    Ok(Outline {
        data: b.0,
        bounds,
        points: count as u16,
        contours: contours.len() as u16,
    })
}
pub fn raster(art: &Artwork, size: u32) -> Result<image::RgbaImage, String> {
    let mut pix = tiny_skia::Pixmap::new(size, size).ok_or("이미지 크기 오류")?;
    let scale = size as f32 / 1000.;
    for s in art.shapes.iter().filter(|s| s.visible && s.opacity > 0.) {
        if s.kind == "image" {
            let im = decode_image(s.src.as_deref().unwrap_or(""))?;
            let resized = image::imageops::resize(
                &im,
                ((s.width as f32 * scale).round() as u32).clamp(1, 4096),
                ((s.height as f32 * scale).round() as u32).clamp(1, 4096),
                image::imageops::FilterType::Triangle,
            );
            let mut overlay = tiny_skia::Pixmap::new(resized.width(), resized.height())
                .ok_or("이미지 크기 오류")?;
            for (d, p) in overlay
                .data_mut()
                .as_chunks_mut::<4>()
                .0
                .iter_mut()
                .zip(resized.pixels())
            {
                let a = p[3] as u16;
                d[0] = (p[0] as u16 * a / 255) as u8;
                d[1] = (p[1] as u16 * a / 255) as u8;
                d[2] = (p[2] as u16 * a / 255) as u8;
                d[3] = p[3];
            }
            pix.draw_pixmap(
                (s.x as f32 * scale).round() as i32,
                (s.y as f32 * scale).round() as i32,
                overlay.as_ref(),
                &tiny_skia::PixmapPaint {
                    opacity: s.opacity as f32,
                    ..Default::default()
                },
                tiny_skia::Transform::identity(),
                None,
            );
        } else {
            let mut pb = tiny_skia::PathBuilder::new();
            for mut pts in polygons(s)? {
                if pts.len() < 3 {
                    continue;
                }
                let area: f64 = pts
                    .iter()
                    .zip(pts.iter().cycle().skip(1))
                    .take(pts.len())
                    .map(|(a, b)| a[0] * b[1] - b[0] * a[1])
                    .sum();
                if s.kind != "outline" && area < 0. {
                    pts.reverse();
                }
                pb.move_to(pts[0][0] as f32, pts[0][1] as f32);
                for p in pts.iter().skip(1) {
                    pb.line_to(p[0] as f32, p[1] as f32)
                }
                pb.close();
            }
            if let Some(path) = pb.finish() {
                let mut paint = tiny_skia::Paint::default();
                let c = color(s);
                paint.set_color_rgba8(c[0], c[1], c[2], c[3]);
                pix.fill_path(
                    &path,
                    &paint,
                    tiny_skia::FillRule::Winding,
                    tiny_skia::Transform::from_scale(scale, scale),
                    None,
                );
            }
        }
    }
    let mut rgba = pix.data().to_vec();
    for p in rgba.as_chunks_mut::<4>().0.iter_mut() {
        if p[3] > 0 {
            let a = p[3] as u16;
            for v in &mut p[..3] {
                *v = (*v as u16 * 255 / a).min(255) as u8;
            }
        }
    }
    image::RgbaImage::from_raw(size, size, rgba).ok_or("이미지 생성 오류".into())
}
fn color(s: &Shape) -> [u8; 4] {
    [
        u8::from_str_radix(&s.color[1..3], 16).unwrap_or(0),
        u8::from_str_radix(&s.color[3..5], 16).unwrap_or(0),
        u8::from_str_radix(&s.color[5..7], 16).unwrap_or(0),
        (s.opacity * 255.).round() as u8,
    ]
}
fn cmap_table(map: &BTreeMap<u32, u16>, uvs: &BTreeMap<u32, Vec<(u32, u16)>>) -> B {
    let mut c12 = B::default();
    c12.u16(12);
    c12.u16(0);
    c12.u32((16 + map.len() * 12) as u32);
    c12.u32(0);
    c12.u32(map.len() as u32);
    for (cp, g) in map {
        c12.u32(*cp);
        c12.u32(*cp);
        c12.u32(*g as u32)
    }
    let mut c14 = B::default();
    if !uvs.is_empty() {
        c14.u16(14);
        c14.u32(0);
        c14.u32(uvs.len() as u32);
        let mut data = B::default();
        for (v, items) in uvs {
            c14.bytes(&v.to_be_bytes()[1..]);
            c14.u32(0);
            c14.u32((10 + uvs.len() * 11 + data.0.len()) as u32);
            data.u32(items.len() as u32);
            for (cp, g) in items {
                data.bytes(&cp.to_be_bytes()[1..]);
                data.u16(*g)
            }
        }
        c14.bytes(&data.0);
        let len = c14.0.len();
        c14.put(2, len as u32);
    }
    let n = if uvs.is_empty() { 2 } else { 3 };
    let mut b = B::default();
    b.u16(0);
    b.u16(n);
    b.u16(0);
    b.u16(4);
    b.u32((4 + n as usize * 8) as u32);
    if !uvs.is_empty() {
        b.u16(0);
        b.u16(5);
        b.u32((4 + n as usize * 8 + c12.0.len()) as u32)
    }
    b.u16(3);
    b.u16(10);
    b.u32((4 + n as usize * 8) as u32);
    b.bytes(&c12.0);
    b.bytes(&c14.0);
    b
}
fn lookup(kind: u16, subs: Vec<B>) -> Result<B, String> {
    let mut b = B::default();
    b.u16(kind);
    b.u16(0);
    b.u16(offset(subs.len())?);
    let mut off = 6 + subs.len() * 2;
    for s in &subs {
        b.u16(offset(off)?);
        off += s.0.len();
    }
    for s in subs {
        b.bytes(&s.0)
    }
    Ok(b)
}
fn gsub(
    ligatures: &[(Vec<u16>, u16)],
    locals: &BTreeMap<String, Vec<(u16, u16)>>,
) -> Result<Option<B>, String> {
    if ligatures.is_empty() && locals.is_empty() {
        return Ok(None);
    }
    let mut lookups = vec![];
    let mut features: Vec<(&str, u16)> = vec![];
    let mut local_features = BTreeMap::new();
    for (locale, pairs) in locals {
        if pairs.is_empty() {
            continue;
        }
        let mut pairs = pairs.clone();
        pairs.sort();
        let mut s = B::default();
        s.u16(2);
        s.u16(offset(6 + pairs.len() * 2)?);
        s.u16(pairs.len() as u16);
        for (_, to) in &pairs {
            s.u16(*to)
        }
        s.bytes(&coverage(&pairs.iter().map(|p| p.0).collect::<Vec<_>>()).0);
        let index = lookups.len() as u16;
        lookups.push(lookup(1, vec![s])?);
        local_features.insert(locale.clone(), features.len() as u16);
        features.push(("locl", index));
    }
    let rlig_index = if ligatures.is_empty() {
        None
    } else {
        let mut grouped: BTreeMap<u16, Vec<&(Vec<u16>, u16)>> = BTreeMap::new();
        for l in ligatures {
            grouped.entry(l.0[0]).or_default().push(l)
        }
        let mut subs = vec![];
        for (first, mut ls) in grouped {
            ls.sort_by_key(|l| std::cmp::Reverse(l.0.len()));
            let mut set = B::default();
            set.u16(ls.len() as u16);
            let mut off = 2 + ls.len() * 2;
            for l in &ls {
                set.u16(offset(off)?);
                off += 4 + (l.0.len() - 1) * 2;
            }
            for l in ls {
                set.u16(l.1);
                set.u16(l.0.len() as u16);
                for id in l.0.iter().skip(1) {
                    set.u16(*id)
                }
            }
            let mut sub = B::default();
            sub.u16(1);
            sub.u16(offset(8 + set.0.len())?);
            sub.u16(1);
            sub.u16(8);
            sub.bytes(&set.0);
            sub.bytes(&coverage(&[first]).0);
            subs.push(sub);
        }
        let idx = features.len() as u16;
        features.push(("rlig", lookups.len() as u16));
        lookups.push(lookup(4, subs)?);
        Some(idx)
    };
    fn lang(rlig: Option<u16>, loc: Option<u16>) -> B {
        let mut b = B::default();
        b.u16(0);
        b.u16(0xffff);
        let ids: Vec<_> = rlig.into_iter().chain(loc).collect();
        b.u16(ids.len() as u16);
        for i in ids {
            b.u16(i)
        }
        b
    }
    let mut scripts = B::default();
    let tags = ["DFLT", "hang", "hani", "latn"];
    scripts.u16(tags.len() as u16);
    let mut contents = vec![];
    for tag in tags {
        let locales = if tag == "hani" {
            vec![
                ("JAN ", local_features.get("ja").copied()),
                ("KOR ", local_features.get("ko").copied()),
            ]
        } else {
            vec![]
        };
        let default = lang(rlig_index, None);
        let mut s = B::default();
        s.u16((4 + locales.len() * 6) as u16);
        s.u16(locales.len() as u16);
        let mut offset = 4 + locales.len() * 6 + default.0.len();
        let mut langs = vec![];
        for (tag, index) in locales {
            let l = lang(rlig_index, index);
            s.bytes(tag.as_bytes());
            s.u16(offset as u16);
            offset += l.0.len();
            langs.push(l)
        }
        s.bytes(&default.0);
        for l in langs {
            s.bytes(&l.0)
        }
        contents.push((tag, s));
    }
    let mut off = 2 + contents.len() * 6;
    for (tag, s) in &contents {
        scripts.bytes(tag.as_bytes());
        scripts.u16(offset(off)?);
        off += s.0.len();
    }
    for (_, s) in contents {
        scripts.bytes(&s.0)
    }
    let mut fs = B::default();
    fs.u16(features.len() as u16);
    for (i, (tag, _)) in features.iter().enumerate() {
        fs.bytes(tag.as_bytes());
        fs.u16((2 + features.len() * 6 + i * 6) as u16)
    }
    for (_, idx) in features {
        fs.u16(0);
        fs.u16(1);
        fs.u16(idx)
    }
    let mut ll = B::default();
    ll.u16(lookups.len() as u16);
    let mut off = 2 + lookups.len() * 2;
    for l in &lookups {
        ll.u16(offset(off)?);
        off += l.0.len()
    }
    for l in lookups {
        ll.bytes(&l.0)
    }
    let mut b = B::default();
    b.u32(0x00010000);
    b.u16(10);
    b.u16(offset(10 + scripts.0.len())?);
    b.u16(offset(10 + scripts.0.len() + fs.0.len())?);
    b.bytes(&scripts.0);
    b.bytes(&fs.0);
    b.bytes(&ll.0);
    Ok(Some(b))
}
fn checksum(bytes: &[u8]) -> u32 {
    bytes.chunks(4).fold(0, |sum, part| {
        let mut v = [0; 4];
        v[..part.len()].copy_from_slice(part);
        sum.wrapping_add(u32::from_be_bytes(v))
    })
}
fn sfnt(tables: BTreeMap<&str, B>, cff: bool) -> Vec<u8> {
    let n = tables.len() as u16;
    let power = 1u16 << n.ilog2();
    let mut b = B::default();
    b.u32(if cff { 0x4f54544f } else { 0x00010000 });
    b.u16(n);
    b.u16(power * 16);
    b.u16(n.ilog2() as u16);
    b.u16(n * 16 - power * 16);
    let mut off = 12 + tables.len() * 16;
    let mut head = 0;
    for (tag, t) in &tables {
        b.bytes(tag.as_bytes());
        b.u32(checksum(&t.0));
        b.u32(off as u32);
        b.u32(t.0.len() as u32);
        if *tag == "head" {
            head = off
        }
        off += (t.0.len() + 3) & !3;
    }
    for t in tables.values() {
        b.bytes(&t.0);
        while b.0.len() % 4 != 0 {
            b.0.push(0)
        }
    }
    let adjust = 0xB1B0AFBAu32.wrapping_sub(checksum(&b.0));
    b.put(head + 8, adjust);
    b.0
}
pub fn compile(
    p: &Project,
    selected: &[&Glyph],
    mode: &str,
    suffix: &str,
) -> Result<(Vec<u8>, Report), String> {
    compile_format(p, selected, mode, suffix, "ttf")
}
fn compile_format(
    p: &Project,
    selected: &[&Glyph],
    mode: &str,
    suffix: &str,
    format: &str,
) -> Result<(Vec<u8>, Report), String> {
    let cff = format == "otf";
    check(
        ["mono", "color", "bitmap"].contains(&mode),
        "출력 형식을 확인해 주세요.",
    )?;
    let mut warnings = vec![];
    let bitmap = mode == "bitmap"
        || (mode == "color"
            && selected
                .iter()
                .any(|g| shapes(g).iter().any(|s| s.kind == "image")));
    if bitmap {
        warnings.push("sbix 비트맵 출력: 글리프 그림은 1000 × 1000 작업 영역으로 잘립니다. 외부 앱의 컬러 폰트 지원을 확인하세요.".into())
    }
    let mode = if bitmap { "bitmap" } else { mode };
    let mut outlines = vec![outline(&[], cff.then_some(1000))?];
    let mut advances = vec![1000u16];
    let mut map = BTreeMap::new();
    let mut texts: BTreeMap<&str, Vec<(&Glyph, u16)>> = BTreeMap::new();
    let mut colors = vec![];
    let mut bases = vec![];
    let mut layers = vec![];
    let mut bitmaps: BTreeMap<u16, Vec<u8>> = BTreeMap::new();
    for g in selected {
        let gid = outlines.len();
        check(
            gid < 65535,
            "글리프 수가 65,535개를 넘어요. 분할 출력해 주세요.",
        )?;
        let gid = gid as u16;
        let ss = shapes(g);
        outlines.push(outline(
            &ss,
            cff.then_some(g.artwork.advance.round() as u16),
        )?);
        advances.push(g.artwork.advance.round() as u16);
        texts.entry(&g.text).or_default().push((g, gid));
        if mode == "color" && !ss.is_empty() {
            bases.push((gid, layers.len() as u16, ss.len() as u16));
            for s in ss {
                check(
                    outlines.len() < 65535 && layers.len() < 65535,
                    "컬러 레이어가 너무 많아요. 분할 출력해 주세요.",
                )?;
                let c = color(s);
                let idx = colors.iter().position(|v| v == &c).unwrap_or_else(|| {
                    colors.push(c);
                    colors.len() - 1
                });
                layers.push((outlines.len() as u16, idx as u16));
                outlines.push(outline(
                    &[s],
                    cff.then_some(g.artwork.advance.round() as u16),
                )?);
                advances.push(g.artwork.advance.round() as u16);
            }
        }
        if bitmap && g.status != "blank" {
            let im = raster(&g.artwork, 512)?;
            let mut out = std::io::Cursor::new(vec![]);
            im.write_to(&mut out, image::ImageFormat::Png)
                .map_err(|e| e.to_string())?;
            bitmaps.insert(gid, out.into_inner());
        }
    }
    for items in texts.values_mut() {
        items.sort_by_key(|(g, _)| match g.locale.as_str() {
            "default" => 0,
            "ko" => 1,
            _ => 2,
        });
    }
    for (raw, items) in &texts {
        if raw.chars().count() == 1 {
            map.insert(raw.chars().next().unwrap() as u32, items[0].1);
        }
    }
    let mut missing = vec![];
    for g in selected {
        for cp in g.text.chars().map(|c| c as u32) {
            if let std::collections::btree_map::Entry::Vacant(e) = map.entry(cp) {
                check(
                    outlines.len() < 65535,
                    "입력 부품까지 포함하면 65,535개를 넘어요.",
                )?;
                e.insert(outlines.len() as u16);
                outlines.push(outline(&[], cff.then_some(0))?);
                advances.push(0);
                if cp != 0x200d && !(0xfe00..=0xfe0f).contains(&cp) {
                    missing.push(format!("U+{cp:04X}"));
                }
            }
        }
    }
    if !missing.is_empty() {
        warnings.push(format!(
            "문자열 치환용 빈 입력 글리프: {}. 단독 표시가 필요하면 각 문자도 작성하세요.",
            missing.into_iter().take(20).collect::<Vec<_>>().join(", ")
        ))
    }
    let mut uvs: BTreeMap<u32, Vec<(u32, u16)>> = BTreeMap::new();
    let mut ligatures = vec![];
    let mut locals: BTreeMap<String, Vec<(u16, u16)>> = BTreeMap::new();
    for (raw, items) in &texts {
        let cps: Vec<u32> = raw.chars().map(|c| c as u32).collect();
        let default = items[0].1;
        if cps.len() > 1 {
            ligatures.push((cps.iter().map(|cp| map[cp]).collect(), default));
            if cps.len() == 2
                && ((0xfe00..=0xfe0f).contains(&cps[1]) || (0xe0100..=0xe01ef).contains(&cps[1]))
            {
                uvs.entry(cps[1]).or_default().push((cps[0], default));
            }
        }
        for (g, id) in items.iter().skip(1) {
            locals
                .entry(g.locale.clone())
                .or_default()
                .push((default, *id));
        }
    }
    let n = outlines.len();
    let mut tables = BTreeMap::new();
    let mut glyf = B::default();
    let mut loca = B::default();
    let mut hmtx = B::default();
    for (o, a) in outlines.iter().zip(&advances) {
        if !cff {
            loca.u32(glyf.0.len() as u32);
            glyf.bytes(&o.data);
        }
        hmtx.u16(*a);
        hmtx.i16(o.bounds[0]);
    }
    if !cff {
        loca.u32(glyf.0.len() as u32);
        tables.insert("glyf", glyf);
        tables.insert("loca", loca);
    }
    tables.insert("hmtx", hmtx);
    let xmin = outlines.iter().map(|o| o.bounds[0]).min().unwrap();
    let ymin = outlines.iter().map(|o| o.bounds[1]).min().unwrap();
    let xmax = outlines.iter().map(|o| o.bounds[2]).max().unwrap();
    let ymax = outlines.iter().map(|o| o.bounds[3]).max().unwrap();
    let mut head = B::default();
    head.u32(0x10000);
    head.u32(0x10000);
    head.u32(0);
    head.u32(0x5f0f3cf5);
    head.u16(if cff {
        1
    } else if format == "woff2" {
        3 | (1 << 11)
    } else {
        3
    });
    head.u16(1000);
    let timestamp = p.updated_at.parse::<u64>().unwrap_or(0) + 2_082_844_800;
    head.bytes(&timestamp.to_be_bytes());
    head.bytes(&timestamp.to_be_bytes());
    for v in [xmin, ymin, xmax, ymax] {
        head.i16(v)
    }
    head.u16(0);
    head.u16(8);
    head.i16(2);
    head.i16(if cff { 0 } else { 1 });
    head.i16(0);
    tables.insert("head", head);
    let mut hhea = B::default();
    hhea.u32(0x10000);
    hhea.i16(1000);
    hhea.i16(-250);
    hhea.i16(0);
    hhea.u16(*advances.iter().max().unwrap());
    hhea.i16(xmin);
    hhea.i16(
        advances
            .iter()
            .zip(&outlines)
            .map(|(a, o)| *a as i16 - o.bounds[2])
            .min()
            .unwrap(),
    );
    hhea.i16(xmax);
    hhea.i16(1);
    hhea.i16(0);
    hhea.i16(0);
    hhea.zeros(8);
    hhea.i16(0);
    hhea.u16(n as u16);
    tables.insert("hhea", hhea);
    let mut maxp = B::default();
    maxp.u32(if cff { 0x5000 } else { 0x10000 });
    maxp.u16(n as u16);
    if !cff {
        maxp.u16(outlines.iter().map(|o| o.points).max().unwrap());
        maxp.u16(outlines.iter().map(|o| o.contours).max().unwrap());
        maxp.u16(0);
        maxp.u16(0);
        maxp.u16(2);
        maxp.zeros(16);
    }
    tables.insert("maxp", maxp);
    let mut os = B::default();
    os.u16(4);
    os.i16(800);
    os.u16(400);
    os.u16(5);
    os.u16(0);
    for v in [650, 600, 0, 75, 650, 600, 0, 350, 50, 300, 0] {
        os.i16(v)
    }
    os.zeros(10);
    os.zeros(16);
    os.bytes(b"DDLF");
    os.u16(0x40);
    os.u16(map.keys().next().copied().unwrap_or(0).min(65535) as u16);
    os.u16(map.keys().last().copied().unwrap_or(0).min(65535) as u16);
    os.i16(1000);
    os.i16(-250);
    os.i16(0);
    os.u16(ymax.max(1000) as u16);
    os.u16((-ymin).max(250) as u16);
    os.zeros(8);
    os.i16(500);
    os.i16(800);
    os.u16(0);
    os.u16(32);
    os.u16(2);
    tables.insert("OS/2", os);
    let family = format!("{}{suffix}", p.family);
    let ps: String = family
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .take(50)
        .collect();
    let ps = if ps.is_empty() {
        "DoodleFont".into()
    } else {
        ps
    };
    if cff {
        tables.insert(
            "CFF ",
            formats::cff(
                &format!("{ps}-Regular"),
                &outlines,
                [xmin, ymin, xmax, ymax],
            )?,
        );
    }
    let attribution = selected
        .iter()
        .flat_map(|g| {
            g.template
                .iter()
                .chain(&g.sources)
                .map(|t| format!("{} / {}", t.attribution, t.license))
        })
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join("; ");
    let has_ofl = selected
        .iter()
        .flat_map(|g| g.template.iter().chain(&g.sources))
        .any(|t| t.license == "SIL Open Font License 1.1");
    let license_text = has_ofl.then(|| format!("{attribution}\n\n{}", crate::typography::OFL));
    let mut names = vec![
        (0, attribution.clone()),
        (1, family.clone()),
        (2, "Regular".into()),
        (3, format!("{}{suffix}", p.id)),
        (4, format!("{family} Regular")),
        (5, "Version 1.000".into()),
        (6, format!("{ps}-Regular")),
    ];
    if let Some(license) = &license_text {
        names.push((13, license.clone()));
        names.push((
            14,
            "https://openfontlicense.org/open-font-license-official-text/".into(),
        ));
    }
    let mut name = B::default();
    name.u16(0);
    name.u16(names.len() as u16);
    name.u16((6 + names.len() * 12) as u16);
    let mut strings = B::default();
    for (id, s) in names {
        let data: Vec<u8> = s.encode_utf16().flat_map(u16::to_be_bytes).collect();
        name.u16(3);
        name.u16(1);
        name.u16(0x409);
        name.u16(id);
        name.u16(offset(data.len())?);
        name.u16(offset(strings.0.len())?);
        strings.bytes(&data)
    }
    name.bytes(&strings.0);
    tables.insert("name", name);
    let mut post = B::default();
    post.u32(0x30000);
    post.zeros(28);
    tables.insert("post", post);
    tables.insert("cmap", cmap_table(&map, &uvs));
    if let Some(gsub) = gsub(&ligatures, &locals)? {
        tables.insert("GSUB", gsub);
    }
    if !bases.is_empty() {
        let mut colr = B::default();
        colr.u16(0);
        colr.u16(bases.len() as u16);
        colr.u32(14);
        colr.u32((14 + bases.len() * 6) as u32);
        colr.u16(layers.len() as u16);
        for (a, b, c) in bases {
            colr.u16(a);
            colr.u16(b);
            colr.u16(c)
        }
        for (a, b) in layers {
            colr.u16(a);
            colr.u16(b)
        }
        tables.insert("COLR", colr);
        let mut cpal = B::default();
        cpal.u16(0);
        cpal.u16(colors.len() as u16);
        cpal.u16(1);
        cpal.u16(colors.len() as u16);
        cpal.u32(14);
        cpal.u16(0);
        for [r, g, b, a] in colors {
            cpal.bytes(&[b, g, r, a])
        }
        tables.insert("CPAL", cpal);
    }
    if !bitmaps.is_empty() {
        let mut strike = B::default();
        strike.u16(512);
        strike.u16(72);
        let mut data = B::default();
        for id in 0..n {
            strike.u32((4 + (n + 1) * 4 + data.0.len()) as u32);
            if let Some(png) = bitmaps.get(&(id as u16)) {
                data.i16(0);
                data.i16(-102);
                data.bytes(b"png ");
                data.bytes(png)
            }
        }
        strike.u32((4 + (n + 1) * 4 + data.0.len()) as u32);
        strike.bytes(&data.0);
        let mut sbix = B::default();
        sbix.u16(1);
        sbix.u16(1);
        sbix.u32(1);
        sbix.u32(12);
        sbix.bytes(&strike.0);
        tables.insert("sbix", sbix);
    }
    let tags = tables.keys().map(|s| s.to_string()).collect();
    let bytes = sfnt(tables, cff);
    ttf_parser::Face::parse(&bytes, 0).map_err(|e| format!("출력 폰트 검증 실패: {e:?}"))?;
    let bytes = match format {
        "woff" => formats::woff(&bytes)?,
        "woff2" => formats::woff2(&bytes)?,
        _ => bytes,
    };
    Ok((
        bytes,
        Report {
            family,
            format: format.into(),
            glyph_count: n,
            authored_count: selected.len(),
            tables: tags,
            mode: mode.into(),
            warnings,
            attribution,
            license_text,
        },
    ))
}
pub fn export(p: &Project, options: &ExportOptions) -> Result<ExportResult, String> {
    use std::io::Write;
    p.validate()?;
    let format = options.format.as_deref().unwrap_or("ttf");
    check(
        ["ttf", "otf", "woff", "woff2"].contains(&format),
        "지원하지 않는 폰트 파일 포맷이에요.",
    )?;
    let locale = options.locale.as_deref().unwrap_or("combined");
    let group = options.group.as_deref().unwrap_or("all");
    let ids: Option<HashSet<&str>> = options
        .ids
        .as_ref()
        .map(|v| v.iter().map(String::as_str).collect());
    let mut selected: Vec<&Glyph> = p
        .glyphs
        .iter()
        .filter(|g| {
            ["edited", "blank"].contains(&g.status.as_str())
                && g.mapping != "control"
                && (locale == "combined" || g.locale == "default" || g.locale == locale)
                && (group == "all" || g.group == group)
                && ids.as_ref().is_none_or(|ids| ids.contains(g.id.as_str()))
        })
        .collect();
    if locale != "combined" {
        selected.sort_by_key(|g| g.locale != locale);
        let mut seen = HashSet::new();
        selected.retain(|g| seen.insert(&g.text));
    }
    check(
        !selected.is_empty(),
        "출력할 작성 글리프가 없어요. 글자를 그리거나 빈칸으로 지정해 주세요.",
    )?;
    for g in &selected {
        if g.template
            .iter()
            .chain(&g.sources)
            .any(|t| !t.permissions.embed)
        {
            return Err(format!("'{}' 템플릿에 폰트 포함 권한이 없어요.", g.name));
        }
    }
    let mut groups: BTreeMap<&str, Vec<&Glyph>> = BTreeMap::new();
    for g in selected {
        groups
            .entry(if options.split == Some(true) {
                &g.group
            } else {
                "all"
            })
            .or_default()
            .push(g);
    }
    let mut reports = vec![];
    let mut files = vec![];
    for (group, gs) in groups {
        let suffix = if group == "all" {
            String::new()
        } else {
            format!("-{group}")
        };
        let (bytes, report) = compile_format(
            p,
            &gs,
            options.mode.as_deref().unwrap_or("color"),
            &suffix,
            format,
        )?;
        let safe: String = p
            .family
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        files.push((format!("{safe}{suffix}.{format}"), bytes));
        reports.push(report);
    }
    let (filename, bytes) = if files.len() == 1 && options.split != Some(true) {
        files.pop().unwrap()
    } else {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(vec![]));
        for (name, bytes) in files {
            zip.start_file(name, zip::write::SimpleFileOptions::default())
                .map_err(|e| e.to_string())?;
            zip.write_all(&bytes).map_err(|e| e.to_string())?;
        }
        zip.start_file("font-report.json", zip::write::SimpleFileOptions::default())
            .map_err(|e| e.to_string())?;
        zip.write_all(&serde_json::to_vec_pretty(&reports).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        for (i, report) in reports.iter().enumerate() {
            if let Some(license) = &report.license_text {
                zip.start_file(
                    format!("OFL-{}.txt", i + 1),
                    zip::write::SimpleFileOptions::default(),
                )
                .map_err(|e| e.to_string())?;
                zip.write_all(license.as_bytes())
                    .map_err(|e| e.to_string())?;
            }
        }
        (
            "Doodle-Font-fonts.zip".into(),
            zip.finish().map_err(|e| e.to_string())?.into_inner(),
        )
    };
    Ok(ExportResult {
        filename,
        data: STANDARD.encode(bytes),
        reports,
    })
}
