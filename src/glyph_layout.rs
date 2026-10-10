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

pub fn vertical_guides(advance: f64) -> [f64; 3] {
    let [left, right] = horizontal_bounds(advance);
    [left, right, advance.min(EM)]
}

/// Balance visible ink within the drawing part of the advance, without scaling it.
pub fn balanced_offset(advance: f64, ink_left: f64, ink_right: f64) -> Result<f64, String> {
    let [left, right] = horizontal_bounds(advance);
    if ink_right - ink_left > right - left + 0.01 {
        return Err("글자의 자연스러운 비율을 유지하면 설정한 폭과 여백을 넘어요. 더 좁은 스타일을 요청하거나 글자 폭 설정을 확인해 주세요.".into());
    }
    Ok((left + right - ink_left - ink_right) / 2.)
}

pub fn context(glyph: &Glyph) -> Value {
    let [left, right] = horizontal_bounds(glyph.artwork.advance);
    let center = (left + right) / 2.;
    let natural = crate::typography::natural_reference(glyph).map(|[x0, y0, x1, y1]| {
        let width = x1 - x0;
        json!({
            "basis":"Bundled sans at normal width and weight, using the same font metrics as local generation. Approximate proportions, not a mandatory font or bounding box.",
            "inkWidth":width.round(),"inkHeight":(y1-y0).round(),
            "topY":y0.round(),"bottomY":y1.round(),
            "centeredLeftX":(center-width/2.).round(),"centeredRightX":(center+width/2.).round(),
            "fitsAdvance":width <= right-left,
            "rule":"Use these dimensions as a natural-size starting point, adjusted for the requested style. Do not force every letter to the same width. I/l/i are narrow, M/W are wider, and punctuation stays small. Never stretch to the outer limits or shrink the whole drawing just to fill a box. If the reference does not fit, request a narrower design or a wider advance instead of squeezing it."
        })
    });
    json!({
        "canvas":{"width":EM,"height":EM,"unitsPerEm":EM,"origin":"top-left","xDirection":"right","yDirection":"down"},
        "referenceImage":{"widthPx":SNAPSHOT,"heightPx":SNAPSHOT,"pixelsPerUnit":SNAPSHOT as f64 / EM,"guidesAreNotGlyphStrokes":true},
        "guides":{"capTopY":CAP_TOP,"midlineY":MIDLINE,"baselineY":BASELINE,"descenderBottomY":DESCENT_BOTTOM,"topLimitY":TOP,"midlineIsNotXHeight":true},
        "spacing":{"advance":glyph.artwork.advance,"leftInkLimitX":left,"rightInkLimitX":right,"centerX":center,"alignment":"For new drawings balance visible ink horizontally around centerX within the available advance, with similar blank space on both sides. Do not anchor ink to the left limit. Do not vertically center: keep the baseline and typeface metrics.","limitsAreNotTargets":true,"rule":"The side limits are minimum margins, NOT edges to touch or a box to fill. Leave the remaining whitespace. Include stroke thickness when measuring ink. Preserve natural width-to-height proportions; never stretch or compress either axis merely to fit. Keep the existing advance unchanged."},
        "naturalReference":natural,
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
    let dx = balanced_offset(glyph.artwork.advance, bounds[0], bounds[2])?;
    if glyph.text.chars().any(|c| "gjpqy".contains(c)) && bounds[3] < BASELINE + 40. {
        return Err("AI 그림에서 기준선 아래로 내려오는 획이 확인되지 않아요. g·j·p·q·y의 하강 획을 포함해 다시 만들어 주세요.".into());
    }
    if bounds[1] < TOP || bounds[3] > DESCENT_BOTTOM {
        return Err(
            "AI 그림이 작업 영역의 위아래를 벗어났어요. 가이드라인 안에 맞춰 다시 만들어 주세요."
                .into(),
        );
    }
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
