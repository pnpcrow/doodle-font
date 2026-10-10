//! Shared input decision and preflight for local Gemma and both external providers.
use crate::{model::*, needle::Mode};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::Sender,
    Arc,
};

pub struct Decision {
    pub candidate: Option<Value>,
    pub note: &'static str,
}
const DELEGATE: &str = "선택한 AI가 원래 요청을 처리했어요.";
const FALLBACK: &str = "앞단 판단이 불확실하거나 사용할 수 없어 선택한 AI로 넘겼어요.";

pub fn run(
    payload: Value,
    cancel: Arc<AtomicBool>,
    status: Sender<String>,
) -> Result<Value, String> {
    run_using(payload, cancel, status, crate::needle::infer)
}
fn run_using(
    payload: Value,
    cancel: Arc<AtomicBool>,
    status: Sender<String>,
    decide: impl FnOnce(&str, Arc<AtomicBool>) -> Result<Value, String>,
) -> Result<Value, String> {
    let mode = match payload.get("decision") {
        Some(v) => serde_json::from_value::<Mode>(v.clone())
            .map_err(|_| "판단 AI 옵션을 확인해 주세요.")?,
        None => Mode::Off,
    };
    let glyph = crate::ai::validate_target(&payload, payload["action"] != "assist")?;
    check(!cancel.load(Ordering::Relaxed), "AI 작업을 취소했어요.")?;
    let decision = if mode == Mode::Needle {
        let _ = status.send("입력 판단 중 · Needle 3".into());
        let reply = decide(payload["prompt"].as_str().unwrap_or(""), cancel.clone());
        plan(&payload, &glyph, reply.as_ref().ok())
    } else {
        Decision {
            candidate: None,
            note: "",
        }
    };
    // The model cannot change mode, provider, consent, glyph identity, or protected state.
    let _ = status.send("AI 실행 전 요청 확인 중".into());
    check(!cancel.load(Ordering::Relaxed), "AI 작업을 취소했어요.")?;
    crate::ai::validate_target(&payload, payload["action"] != "assist")?;
    let mut result = if let Some(candidate) = decision.candidate {
        let _ = status.send("간단한 편집 후보를 구성하는 중".into());
        candidate
    } else {
        if mode == Mode::Needle {
            let _ = status.send("선택한 AI로 요청 전달 중".into());
        }
        crate::local_ai::run(payload, cancel.clone(), status)?
    };
    check(!cancel.load(Ordering::Relaxed), "AI 작업을 취소했어요.")?;
    if mode == Mode::Needle {
        result["decisionNote"] = json!(decision.note);
    }
    Ok(result)
}
#[cfg(test)]
#[path = "ai_pipeline_tests.rs"]
mod tests;

/// A well-formed model call is advisory. Only a complete, explicitly grounded simple
/// command can bypass the selected provider. Everything else preserves the original input.
pub fn plan(payload: &Value, glyph: &Glyph, reply: Option<&Value>) -> Decision {
    let fallback = || Decision {
        candidate: None,
        note: FALLBACK,
    };
    let Some(reply) = reply else {
        return fallback();
    };
    if reply["success"] != true
        || !reply["confidence"]
            .as_f64()
            .is_some_and(|n| (0.8..=1.).contains(&n))
        || !reply["suppressed_calls"]
            .as_array()
            .is_some_and(Vec::is_empty)
        || !reply["validation"]["ungrounded"]
            .as_array()
            .is_some_and(Vec::is_empty)
        || reply["validation"]["negation"] != false
    {
        return fallback();
    }
    let Some(calls) = reply["function_calls"].as_array().filter(|a| a.len() == 1) else {
        return fallback();
    };
    let call = &calls[0];
    if call["name"] == "use_assistant" || payload["action"] == "assist" {
        return Decision {
            candidate: None,
            note: DELEGATE,
        };
    }
    if crate::ai::validate_target(payload, true).is_err()
        || glyph.artwork.shapes.is_empty()
        || glyph
            .artwork
            .shapes
            .iter()
            .any(|s| s.locked || s.kind == "image")
    {
        return fallback();
    }
    let prompt = payload["prompt"].as_str().unwrap_or("");
    let mut artwork = glyph.artwork.clone();
    match simple_command(prompt) {
        Some(Command::Stroke(width))
            if call["name"] == "set_stroke"
                && call["arguments"]["width"].as_f64() == Some(width)
                && artwork.shapes.iter().all(|s| s.kind == "path" && !s.fill) =>
        {
            for shape in &mut artwork.shapes {
                if shape.visible {
                    shape.stroke = width;
                }
            }
        }
        Some(Command::Color(name, hex))
            if call["name"] == "set_color"
                && call["arguments"]["color"] == name
                && (payload["colorMode"] != "mono" || name == "black" || name == "white") =>
        {
            for shape in &mut artwork.shapes {
                if shape.visible {
                    shape.color = hex.into();
                }
            }
        }
        _ => return fallback(),
    }
    if crate::ai::validate_candidate(&artwork).is_err() {
        return fallback();
    }
    Decision {
        candidate: Some(json!({"artwork":artwork,"sources":glyph.sources})),
        note:
            "Needle 3의 판단을 확인해 간단한 편집 후보를 만들었어요. 생성 AI는 호출하지 않았습니다.",
    }
}
enum Command {
    Stroke(f64),
    Color(&'static str, &'static str),
}
fn simple_command(prompt: &str) -> Option<Command> {
    let p = prompt.trim().trim_end_matches(['.', '!']).to_lowercase();
    for (prefix, suffix) in [
        ("set the stroke width to ", ""),
        ("set stroke width to ", ""),
        ("change the stroke width to ", ""),
        ("현재 그림의 획 굵기를 ", "으로 통일해줘"),
        ("획 굵기를 ", "으로 바꿔줘"),
    ] {
        if let Some(number) = p.strip_prefix(prefix).and_then(|s| s.strip_suffix(suffix)) {
            if let Ok(n) = number.trim().parse::<f64>() {
                if (1.0..=300.0).contains(&n) {
                    return Some(Command::Stroke(n));
                }
            }
        }
    }
    for (name, korean, hex) in [
        ("black", "검은색", "#000000"),
        ("white", "흰색", "#ffffff"),
        ("red", "빨간색", "#ff0000"),
        ("blue", "파란색", "#0000ff"),
        ("green", "초록색", "#008000"),
    ] {
        if p == format!("set the color to {name}")
            || p == format!("change the color to {name}")
            || p == format!("그림을 {korean}으로 바꿔줘")
        {
            return Some(Command::Color(name, hex));
        }
    }
    None
}
