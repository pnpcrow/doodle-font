//! Local AI chooses style; a font shaper preserves the exact text and its counters.
use crate::{
    document::{new_shape, uid},
    model::*,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

static SANS: &[u8] = include_bytes!("../assets/fonts/NotoSansCJKkr-VF.otf");
static SERIF: &[u8] = include_bytes!("../assets/fonts/NotoSerifCJKkr-VF.otf");
static HAND: &[u8] = include_bytes!("../assets/fonts/SingleDay-Regular.ttf");
pub const OFL: &str = include_str!("../assets/fonts/Noto-OFL.txt");
pub const HAND_OFL: &str = include_str!("../assets/fonts/SingleDay-OFL.txt");

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Style {
    pub family: String,
    pub weight: u16,
    pub width: u16,
    pub slant: i16,
    pub roundness: u16,
    pub tracking: i16,
    pub color: String,
    pub source: String,
    pub stroke: u16,
}
impl Default for Style {
    fn default() -> Self {
        Self {
            family: "sans".into(),
            weight: 500,
            width: 100,
            slant: 0,
            roundness: 0,
            tracking: 0,
            color: "#242424".into(),
            source: "glyph".into(),
            stroke: 48,
        }
    }
}
impl Style {
    pub fn validate(&self) -> Result<(), String> {
        check(
            ["sans", "serif", "handwritten"].contains(&self.family.as_str())
                && ["glyph", "current"].contains(&self.source.as_str())
                && (200..=850).contains(&self.weight)
                && (75..=125).contains(&self.width)
                && (-15..=15).contains(&self.slant)
                && self.roundness <= 100
                && (-20..=100).contains(&self.tracking)
                && (12..=120).contains(&self.stroke)
                && self.color.len() == 7
                && self.color.starts_with('#')
                && self.color[1..].bytes().all(|b| b.is_ascii_hexdigit()),
            "AI 글자 스타일의 범위가 올바르지 않아요. 다시 요청해 주세요.",
        )
    }
}
pub fn schema() -> Value {
    json!({"type":"object","additionalProperties":false,
    "required":["family","weight","width","slant","roundness","tracking","color","source","stroke"],
    "properties":{
        "family":{"type":"string","enum":["sans","serif","handwritten"]},
        "weight":{"type":"integer","minimum":200,"maximum":850},
        "width":{"type":"integer","minimum":75,"maximum":125},
        "slant":{"type":"integer","minimum":-15,"maximum":15},
        "roundness":{"type":"integer","minimum":0,"maximum":100},
        "tracking":{"type":"integer","minimum":-20,"maximum":100},
        "color":{"type":"string","pattern":"^#[0-9a-fA-F]{6}$"},
        "source":{"type":"string","enum":["glyph","current"]},
        "stroke":{"type":"integer","minimum":12,"maximum":120}
    }})
}
pub fn is_text(glyph: &Glyph) -> bool {
    glyph.text.chars().any(|c| {
        c.is_alphanumeric()
            || c.is_ascii_punctuation()
            || ('\u{2000}'..='\u{206f}').contains(&c)
            || ('\u{3001}'..='\u{303f}').contains(&c)
    }) && !glyph.text.contains('\u{200d}')
        && !glyph.text.contains('\u{fe0f}')
}
pub struct Generated {
    pub artwork: Artwork,
    pub sources: Vec<Template>,
    pub note: String,
}
fn source(bytes: &[u8], family: &str) -> Template {
    let face = ttf_parser::Face::parse(bytes, 0).expect("bundled font");
    let copyright = face
        .names()
        .into_iter()
        .find(|n| n.name_id == 0)
        .and_then(|n| n.to_string())
        .unwrap_or_default();
    Template {
        id:format!("builtin-typeface-{family}-1"), name:family.into(), provider:if family == "Single Day" { "DXKorea" } else { "Noto / Adobe / Google" }.into(),
        version:face.names().into_iter().find(|n| n.name_id == 5).and_then(|n| n.to_string()).unwrap_or_else(||"pinned".into()),
        source:if family == "Single Day" { "https://github.com/google/fonts/tree/51303ca9e8ac9dcea7b12d307ba568fd0e6fcfca/ofl/singleday" } else { "https://github.com/notofonts/noto-cjk/tree/f8d157532fbfaeda587e826d4cd5b21a49186f7c" }.into(),
        license:"SIL Open Font License 1.1".into(), attribution:copyright,
        permissions:Permissions{display:true,edit:true,embed:true,redistribute:true,ai:true}, original:None,
    }
}

#[derive(Default)]
struct Curves {
    contours: Vec<Vec<[f64; 2]>>,
    current: Vec<[f64; 2]>,
}
impl Curves {
    fn finish(&mut self) {
        if self.current.len() > 1 && self.current.first() == self.current.last() {
            self.current.pop();
        }
        if self.current.len() >= 3 {
            self.contours.push(std::mem::take(&mut self.current));
        }
        self.current.clear();
    }
    fn cubic(&mut self, p: [[f64; 2]; 4], depth: u8) {
        let distance = |q: [f64; 2]| {
            let dx = p[3][0] - p[0][0];
            let dy = p[3][1] - p[0][1];
            ((q[0] - p[0][0]) * dy - (q[1] - p[0][1]) * dx).abs() / dx.hypot(dy).max(0.01)
        };
        if depth >= 12 || (distance(p[1]) <= 0.5 && distance(p[2]) <= 0.5 && p[0] != p[3]) {
            self.current.push(p[3]);
            return;
        }
        let mid = |a: [f64; 2], b: [f64; 2]| [(a[0] + b[0]) / 2., (a[1] + b[1]) / 2.];
        let a = mid(p[0], p[1]);
        let b = mid(p[1], p[2]);
        let c = mid(p[2], p[3]);
        let d = mid(a, b);
        let e = mid(b, c);
        let f = mid(d, e);
        self.cubic([p[0], a, d, f], depth + 1);
        self.cubic([f, e, c, p[3]], depth + 1);
    }
}
impl ttf_parser::OutlineBuilder for Curves {
    fn move_to(&mut self, x: f32, y: f32) {
        self.finish();
        self.current.push([x as f64, y as f64]);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.current.push([x as f64, y as f64]);
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let p = *self.current.last().unwrap();
        let c = [x1 as f64, y1 as f64];
        let q = [x as f64, y as f64];
        self.cubic(
            [
                p,
                [
                    p[0] + (c[0] - p[0]) * 2. / 3.,
                    p[1] + (c[1] - p[1]) * 2. / 3.,
                ],
                [
                    q[0] + (c[0] - q[0]) * 2. / 3.,
                    q[1] + (c[1] - q[1]) * 2. / 3.,
                ],
                q,
            ],
            0,
        );
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.cubic(
            [
                *self.current.last().unwrap(),
                [x1 as f64, y1 as f64],
                [x2 as f64, y2 as f64],
                [x as f64, y as f64],
            ],
            0,
        );
    }
    fn close(&mut self) {
        self.finish();
    }
}
fn rounded(contour: &[[f64; 2]], amount: f64) -> Vec<[f64; 2]> {
    if amount <= 0. {
        return contour.to_vec();
    }
    let mut out = vec![];
    for i in 0..contour.len() {
        let prev = contour[(i + contour.len() - 1) % contour.len()];
        let p = contour[i];
        let next = contour[(i + 1) % contour.len()];
        let l = (p[0] - prev[0]).hypot(p[1] - prev[1]);
        let r = (p[0] - next[0]).hypot(p[1] - next[1]);
        let cut = amount.min(l * 0.22).min(r * 0.22);
        if cut < 0.1 {
            out.push(p);
            continue;
        }
        let a = [
            p[0] + (prev[0] - p[0]) * cut / l,
            p[1] + (prev[1] - p[1]) * cut / l,
        ];
        let b = [
            p[0] + (next[0] - p[0]) * cut / r,
            p[1] + (next[1] - p[1]) * cut / r,
        ];
        for j in 0..=4 {
            let t = j as f64 / 4.;
            out.push([
                (1. - t).powi(2) * a[0] + 2. * (1. - t) * t * p[0] + t * t * b[0],
                (1. - t).powi(2) * a[1] + 2. * (1. - t) * t * p[1] + t * t * b[1],
            ]);
        }
    }
    out
}
pub fn generate(glyph: &Glyph, style: &Style, mono: bool) -> Result<Generated, String> {
    style.validate()?;
    if style.source == "current" && !glyph.artwork.shapes.is_empty() {
        let mut art = glyph.artwork.clone();
        for s in &mut art.shapes {
            s.id = uid();
            if !s.fill && s.kind != "image" && s.kind != "outline" {
                s.stroke = style.stroke as f64;
            }
            if s.kind != "image" {
                s.color = if mono {
                    "#242424".into()
                } else {
                    style.color.clone()
                };
            }
            if s.kind == "path" {
                for p in &mut s.points {
                    p[0] = 500.
                        + (p[0] - 500.) * style.width as f64 / 100.
                        + (800. - p[1]) * (style.slant as f64).to_radians().tan();
                }
            } else {
                s.x = 500. + (s.x - 500.) * style.width as f64 / 100.;
                s.width *= style.width as f64 / 100.;
            }
        }
        crate::ai::validate_candidate(&art)?;
        return Ok(Generated {
            artwork: art,
            sources: vec![],
            note: "현재 그림의 형태를 유지하고 획 굵기를 통일했어요.".into(),
        });
    }
    let (mut bytes, mut name) = match style.family.as_str() {
        "serif" => (SERIF, "Noto Serif CJK"),
        "handwritten" => (HAND, "Single Day"),
        _ => (SANS, "Noto Sans CJK"),
    };
    let supports = |bytes: &[u8]| {
        ttf_parser::Face::parse(bytes, 0).is_ok_and(|f| {
            glyph.text.chars().all(|c| {
                c.is_whitespace()
                    || matches!(c, '\u{fe0e}' | '\u{fe0f}')
                    || f.glyph_index(c).is_some()
            })
        })
    };
    let mut fallback = false;
    if !supports(bytes) && style.family == "handwritten" {
        bytes = SANS;
        name = "Noto Sans CJK";
        fallback = true;
    }
    check(supports(bytes),"이 문자는 내장 자형에 없어요. 직접 그린 기준 그림을 준비해 ‘현재 그림을 유지해서 보정’해 주세요.")?;
    let mut face =
        ttf_parser::Face::parse(bytes, 0).map_err(|e| format!("내장 글꼴 오류: {e:?}"))?;
    face.set_variation(ttf_parser::Tag::from_bytes(b"wght"), style.weight as f32);
    let mut hb = rustybuzz::Face::from_slice(bytes, 0).ok_or("내장 글꼴을 읽지 못했어요.")?;
    hb.set_variations(&[rustybuzz::Variation {
        tag: ttf_parser::Tag::from_bytes(b"wght"),
        value: style.weight as f32,
    }]);
    let mut buffer = rustybuzz::UnicodeBuffer::new();
    buffer.push_str(&glyph.text);
    buffer.guess_segment_properties();
    if let Ok(language) = glyph.locale.parse() {
        buffer.set_language(language);
    }
    let shaped = rustybuzz::shape(&hb, &[], buffer);
    check(
        shaped.glyph_infos().iter().all(|g| g.glyph_id != 0),
        "일부 문자의 정확한 자형을 찾지 못했어요.",
    )?;
    let unit = 1000. / face.units_per_em() as f64;
    let mut contours = vec![];
    let mut pen = 0.;
    for (info, position) in shaped.glyph_infos().iter().zip(shaped.glyph_positions()) {
        let mut builder = Curves::default();
        face.outline_glyph(ttf_parser::GlyphId(info.glyph_id as u16), &mut builder);
        builder.finish();
        for contour in builder.contours {
            contours.push(
                contour
                    .into_iter()
                    .map(|p| {
                        [
                            pen + (p[0] + position.x_offset as f64) * unit,
                            800. - (p[1] + position.y_offset as f64) * unit,
                        ]
                    })
                    .collect::<Vec<_>>(),
            );
        }
        pen += position.x_advance as f64 * unit + style.tracking as f64;
    }
    check(!contours.is_empty(), "그릴 수 있는 글자 윤곽이 없어요.")?;
    let min_x = contours
        .iter()
        .flatten()
        .map(|p| p[0])
        .fold(f64::INFINITY, f64::min);
    let max_x = contours
        .iter()
        .flatten()
        .map(|p| p[0])
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = contours
        .iter()
        .flatten()
        .map(|p| p[1])
        .fold(f64::INFINITY, f64::min);
    let max_y = contours
        .iter()
        .flatten()
        .map(|p| p[1])
        .fold(f64::NEG_INFINITY, f64::max);
    // One scale for all contours preserves component proportions and small counters.
    let scale = (820. / (max_x - min_x))
        .min(760. / (max_y - min_y))
        .min(0.82);
    let center = (min_x + max_x) / 2.;
    let mut final_contours = vec![];
    for contour in contours {
        let points: Vec<_> = contour
            .into_iter()
            .map(|p| {
                let y = 800. + (p[1] - 800.) * scale;
                [
                    500. + (p[0] - center) * scale * style.width as f64 / 100.
                        + (800. - y) * (style.slant as f64).to_radians().tan(),
                    y,
                ]
            })
            .collect();
        final_contours.push(rounded(&points, style.roundness as f64 * 0.09));
    }
    // Keep every stroke inside the canvas after width/slant changes.
    let lo = final_contours
        .iter()
        .flatten()
        .map(|p| p[0])
        .fold(f64::INFINITY, f64::min);
    let hi = final_contours
        .iter()
        .flatten()
        .map(|p| p[0])
        .fold(f64::NEG_INFINITY, f64::max);
    let fit = (900. / (hi - lo)).min(1.);
    let center = (lo + hi) / 2.;
    for p in final_contours.iter_mut().flatten() {
        p[0] = 500. + (p[0] - center) * fit;
    }
    let mut shape = new_shape("outline");
    shape.name = format!("{} · 글자 형태", glyph.text);
    shape.x = 0.;
    shape.y = 0.;
    shape.width = 1000.;
    shape.height = 1000.;
    shape.fill = true;
    shape.stroke = 0.;
    shape.color = if mono {
        "#242424".into()
    } else {
        style.color.clone()
    };
    shape.contours = final_contours;
    let artwork = Artwork {
        shapes: vec![shape],
        advance: 1000.,
    };
    crate::ai::validate_candidate(&artwork)?;
    Ok(Generated {
        artwork,
        sources: vec![source(bytes, name)],
        note: format!(
            "글자 형태를 보존해 {} 스타일로 만들었어요.{}",
            match style.family.as_str() {
                "serif" => "명조",
                "handwritten" => "손글씨",
                _ => "고딕",
            },
            if fallback {
                " 이 문자는 손글씨 자형이 없어 고딕 자형을 사용했어요."
            } else {
                ""
            }
        ),
    })
}
