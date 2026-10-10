//! The editor, AI reference image, prompts and typography engine share one coordinate space.
use crate::model::{Artwork, Glyph};
use serde_json::{json, Value};

pub const EM: f64 = 1000.;
pub const BASELINE: f64 = 800.;
pub const CAP_TOP: f64 = 200.;
pub const MIDLINE: f64 = 500.;
pub const DESCENT_BOTTOM: f64 = 960.;
pub const TOP: f64 = 0.;
pub const SNAPSHOT: u32 = 512;

pub fn horizontal_bounds(advance: f64) -> [f64; 2] {
    // Zero-advance combining glyphs still need a drawing area.
    let right = if advance > 0. { advance.min(EM) } else { EM };
    let inset = (right * 0.2).min(80.);
    [inset, right - inset]
}

pub fn context(glyph: &Glyph) -> Value {
    let [left, right] = horizontal_bounds(glyph.artwork.advance);
    json!({
        "canvas":{"width":EM,"height":EM,"unitsPerEm":EM,"origin":"top-left","xDirection":"right","yDirection":"down"},
        "referenceImage":{"widthPx":SNAPSHOT,"heightPx":SNAPSHOT,"pixelsPerUnit":SNAPSHOT as f64 / EM,"guidesAreNotGlyphStrokes":true},
        "guides":{"capTopY":CAP_TOP,"midlineY":MIDLINE,"baselineY":BASELINE,"descenderBottomY":DESCENT_BOTTOM,"topLimitY":TOP,"midlineIsNotXHeight":true},
        "spacing":{"advance":glyph.artwork.advance,"leftInkX":left,"rightInkLimitX":right,"alignment":"left sidebearing and baseline; never center in the 1000-unit canvas"},
        "lowercase":"Keep the typeface's x-height smaller than capital height. Do not individually enlarge lowercase or punctuation to the cap line.",
        "descenders":{"examples":["g","j","p","q","y"],"present":glyph.text.chars().filter(|c| "gjpqy".contains(*c)).map(|c|c.to_string()).collect::<Vec<_>>(),"rule":"The main body rests on baseline y=800; descender strokes must continue below it (y>800), within y=960. Never lift the whole glyph to put its lowest point on the baseline."},
        "preserve":"When asked to preserve the current shape or position, keep its placement and proportions. Do not re-align the reference artwork. Honor explicit positioning instructions."
    })
}

pub fn guides() -> [(f64, &'static str, [u8; 3]); 4] {
    [
        (CAP_TOP, "대문자 높이", [194, 202, 185]),
        (MIDLINE, "중심선", [216, 219, 210]),
        (BASELINE, "기준선", [219, 151, 127]),
        (DESCENT_BOTTOM, "하강 한계", [145, 166, 190]),
    ]
}

/// Visible ink bounds, including stroke thickness; shared with candidate layout checks.
pub fn ink_bounds(art: &Artwork) -> Result<Option<[f64; 4]>, String> {
    let mut bounds = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for shape in art.shapes.iter().filter(|s| s.visible && s.opacity > 0.) {
        for p in crate::font::polygons(shape)?.iter().flatten() {
            bounds[0] = bounds[0].min(p[0]);
            bounds[1] = bounds[1].min(p[1]);
            bounds[2] = bounds[2].max(p[0]);
            bounds[3] = bounds[3].max(p[1]);
        }
    }
    Ok(bounds[0].is_finite().then_some(bounds))
}

fn preserve_position(prompt: &str) -> bool {
    let p = prompt.to_lowercase();
    // Conservative: explicit editing/position requests take precedence over default placement.
    [
        "유지",
        "그대로",
        "보정",
        "기존",
        "현재",
        "위치",
        "정렬",
        "가운데",
        "중앙",
        "왼쪽",
        "오른쪽",
        "위로 옮",
        "아래로 옮",
        "위로 이동",
        "아래로 이동",
        "keep",
        "preserve",
        "current",
        "existing",
        "position",
        "align",
        "center",
        "move",
        "shift",
    ]
    .iter()
    .any(|word| p.contains(word))
}

/// Normalize only fresh drawings. Existing-art edits and explicit placement are preserved.
/// No vertical translation/resizing is done: that would erase descenders or distort x-height.
pub fn place_api_candidate(glyph: &Glyph, prompt: &str, art: &mut Artwork) -> Result<(), String> {
    if preserve_position(prompt) {
        return Ok(());
    }
    let Some(bounds) = ink_bounds(art)? else {
        return Ok(());
    };
    let [left, right] = horizontal_bounds(glyph.artwork.advance);
    if bounds[2] - bounds[0] > right - left + 1. {
        return Err(
            "AI 그림이 설정한 글자 폭보다 넓어요. 글자 폭과 좌우 여백에 맞춰 다시 만들어 주세요."
                .into(),
        );
    }
    if glyph.text.chars().any(|c| "gjpqy".contains(c)) && bounds[3] < BASELINE + 40. {
        return Err("AI 그림에서 기준선 아래로 내려오는 획이 확인되지 않아요. g·j·p·q·y의 하강 획을 포함해 다시 만들어 주세요.".into());
    }
    if bounds[1] < TOP || bounds[3] > DESCENT_BOTTOM {
        return Err(
            "AI 그림이 작업 영역의 위아래를 벗어났어요. 가이드라인 안에 맞춰 다시 만들어 주세요."
                .into(),
        );
    }
    let dx = left - bounds[0];
    for shape in &mut art.shapes {
        if shape.kind == "path" {
            for p in &mut shape.points {
                p[0] += dx;
            }
        } else {
            shape.x += dx;
        }
    }
    art.advance = glyph.artwork.advance;
    Ok(())
}
