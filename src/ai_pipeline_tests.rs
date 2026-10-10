use super::*;
use crate::document::*;
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::mpsc,
    time::Duration,
};

fn payload(provider: &str) -> Value {
    json!({"glyph":new_project().glyphs[0],"action":"generate","decision":"needle","prompt":"Set the stroke width to 70.","colorMode":"color","sendReference":false,"config":{"provider":provider,"key":"secret-not-for-router","model":"test","endpoint":"http://127.0.0.1:1/v1"}})
}
fn reply() -> Value {
    json!({"success":true,"confidence":0.95,"function_calls":[{"name":"set_stroke","arguments":{"width":70}}],"suppressed_calls":[],"validation":{"ungrounded":[],"negation":false}})
}
fn candidate(p: &Value, r: &Value) -> Option<Value> {
    let g = serde_json::from_value(p["glyph"].clone()).unwrap();
    plan(p, &g, Some(r)).candidate
}
#[test]
fn disabled_generation_never_calls_the_decision_model_or_provider() {
    let (tx, rx) = mpsc::channel();
    let result = run_using(
        payload("off"),
        Arc::new(AtomicBool::new(false)),
        tx,
        |_, _| {
            panic!("AI disabled must skip Needle too");
        },
    );
    assert!(result.unwrap_err().contains("꺼져"));
    assert!(rx.try_recv().is_err());
}

#[test]
fn common_quick_candidate_preserves_geometry_provenance_and_never_calls_provider() {
    for provider in ["local", "openai", "anthropic"] {
        let mut p = payload(provider);
        p["glyph"]["sources"] = json!([new_project().templates[0]]);
        let (tx, _rx) = mpsc::channel();
        let result = run_using(
            p.clone(),
            Arc::new(AtomicBool::new(false)),
            tx,
            |prompt, _| {
                assert_eq!(prompt, "Set the stroke width to 70.");
                assert!(!prompt.contains("secret"));
                Ok(reply())
            },
        )
        .unwrap();
        let before: Glyph = serde_json::from_value(p["glyph"].clone()).unwrap();
        let after: Artwork = serde_json::from_value(result["artwork"].clone()).unwrap();
        for (a, b) in before.artwork.shapes.iter().zip(after.shapes) {
            assert_eq!(a.points, b.points);
            assert_eq!(a.color, b.color);
            assert_eq!(b.stroke, 70.);
        }
        assert_eq!(result["sources"], p["glyph"]["sources"]);
        assert!(result["decisionNote"]
            .as_str()
            .unwrap()
            .contains("호출하지"));
    }
}
#[test]
fn untrusted_decisions_never_override_intent_or_protected_content() {
    let original = payload("local");
    for prompt in [
        "Do not set the stroke width to 70.",
        "Set the stroke width to 70 and make it red.",
        "Set the stroke width to 17.",
        "현재 그림은 바꾸지 마",
        "A를 그려줘",
    ] {
        let mut p = original.clone();
        p["prompt"] = json!(prompt);
        assert!(candidate(&p, &reply()).is_none());
    }
    for (field, value) in [
        ("confidence", json!(0.2)),
        ("confidence", Value::Null),
        ("success", json!(false)),
        ("suppressed_calls", json!([{}])),
        (
            "validation",
            json!({"ungrounded":["set_stroke.width"],"negation":false}),
        ),
        ("validation", json!({"ungrounded":[],"negation":true})),
    ] {
        let mut r = reply();
        r[field] = value;
        assert!(candidate(&original, &r).is_none());
    }
    for modify in [0, 1, 2, 3, 4, 5] {
        let mut p = original.clone();
        match modify {
            0 => p["action"] = json!("assist"),
            1 => p["glyph"]["locked"] = json!(true),
            2 => p["glyph"]["status"] = json!("blank"),
            3 => p["glyph"]["artwork"]["shapes"][0]["locked"] = json!(true),
            4 => p["glyph"]["artwork"]["shapes"][0]["fill"] = json!(true),
            _ => p["glyph"]["artwork"]["shapes"] = json!([]),
        }
        assert!(candidate(&p, &reply()).is_none());
    }
}
#[test]
fn grounded_color_respects_monochrome_and_korean_negation() {
    let mut p = payload("local");
    p["prompt"] = json!("그림을 빨간색으로 바꿔줘.");
    let mut r = reply();
    r["function_calls"] = json!([{"name":"set_color","arguments":{"color":"red"}}]);
    assert_eq!(
        candidate(&p, &r).unwrap()["artwork"]["shapes"][0]["color"],
        "#ff0000"
    );
    p["colorMode"] = json!("mono");
    assert!(candidate(&p, &r).is_none());
    p["colorMode"] = json!("color");
    p["prompt"] = json!("그림을 빨간색으로 바꾸지 마.");
    assert!(candidate(&p, &r).is_none());
}
fn mock(provider: &str) -> (String, mpsc::Receiver<Value>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (tx, rx) = mpsc::channel();
    let response = if provider == "openai" {
        json!({"choices":[{"message":{"content":"원래 요청을 받았습니다."}}]})
    } else {
        json!({"content":[{"type":"text","text":"원래 요청을 받았습니다."}]})
    }
    .to_string();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut bytes = vec![];
        let mut buffer = [0; 4096];
        loop {
            let n = stream.read(&mut buffer).unwrap();
            assert!(n > 0);
            bytes.extend_from_slice(&buffer[..n]);
            let text = String::from_utf8_lossy(&bytes);
            if let Some((head, body)) = text.split_once("\r\n\r\n") {
                let length = head
                    .lines()
                    .find_map(|s| {
                        s.to_lowercase()
                            .strip_prefix("content-length:")
                            .and_then(|v| v.trim().parse::<usize>().ok())
                    })
                    .unwrap();
                if body.len() >= length {
                    tx.send(serde_json::from_str(body).unwrap()).unwrap();
                    break;
                }
            }
        }
        write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",response.len(),response).unwrap();
    });
    (format!("http://{address}/v1"), rx)
}
#[test]
fn off_failed_and_uncertain_decisions_preserve_both_api_contracts_and_image_consent() {
    for provider in ["openai", "anthropic"] {
        for scenario in ["off", "failed", "uncertain"] {
            let (url, rx) = mock(provider);
            let mut p = payload(provider);
            p["action"] = json!("assist");
            p["prompt"] = json!("원래 A 그림을 설명해줘.");
            p["config"]["endpoint"] = json!(url);
            if scenario == "off" {
                p["decision"] = json!("off");
            }
            let (tx, _status) = mpsc::channel();
            let result = run_using(p, Arc::new(AtomicBool::new(false)), tx, |prompt, _| {
                assert_ne!(scenario, "off", "disabled option must not start Needle");
                assert_eq!(prompt, "원래 A 그림을 설명해줘.");
                if scenario == "failed" {
                    Err("missing model".into())
                } else {
                    let mut r = reply();
                    r["confidence"] = json!(0.1);
                    Ok(r)
                }
            })
            .unwrap();
            assert_eq!(result["answer"], "원래 요청을 받았습니다.");
            assert_eq!(result.get("decisionNote").is_none(), scenario == "off");
            let request = rx.recv_timeout(Duration::from_secs(3)).unwrap();
            let content = request["messages"][0]["content"].as_str().unwrap();
            assert!(content.contains("원래 A 그림을 설명해줘."));
            assert!(!request.to_string().contains("image/png"));
        }
    }
}
#[test]
fn cancellation_during_decision_never_starts_generation() {
    let (tx, _rx) = mpsc::channel();
    let result = run_using(
        payload("local"),
        Arc::new(AtomicBool::new(false)),
        tx,
        |_, cancel| {
            cancel.store(true, Ordering::Relaxed);
            Ok(reply())
        },
    );
    assert!(result.unwrap_err().contains("취소"));
}
#[test]
fn options_persist_independently_without_credentials() {
    let folder = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
    let path = folder.join("ai-options.json");
    assert_eq!(crate::needle::Options::load(&path).decision, Mode::Off);
    crate::needle::Options {
        decision: Mode::Needle,
    }
    .save(&path)
    .unwrap();
    assert_eq!(crate::needle::Options::load(&path).decision, Mode::Needle);
    let data: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(data, json!({"decision":"needle"}));
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(folder).unwrap();
}

#[test]
fn needle_motion_uses_one_existing_inference_and_cannot_inject_completion_or_frames() {
    let mut p = payload("local");
    p["prompt"] = json!("Set the color to red.");
    let mut r = reply();
    r["function_calls"] = json!([{"name":"set_color","arguments":{"color":"red"}}]);
    let (tx, rx) = mpsc::channel();
    let result = run_using(p.clone(), Arc::new(AtomicBool::new(false)), tx, |_, _| {
        Ok(r.clone())
    })
    .unwrap();
    assert_eq!(result["artwork"]["shapes"][0]["color"], "#ff0000");
    let events = rx.try_iter().collect::<Vec<_>>();
    assert!(events.contains(&Progress::Animation(Mood::Thinking)));
    assert!(events.contains(&Progress::Animation(Mood::Coloring)));
    assert!(!events.contains(&Progress::Animation(Mood::Done)));
    for confidence in [0.0, 0.79, 1.01] {
        r["confidence"] = json!(confidence);
        assert_eq!(animation_hint(&p, Some(&r)), None);
    }
    r["confidence"] = json!(0.99);
    for name in [
        "done",
        "needs_help",
        "animate_frames",
        "set_provider",
        "run_shell",
    ] {
        r["function_calls"] = json!([{"name":name,"arguments":{}}]);
        assert_eq!(animation_hint(&p, Some(&r)), None);
    }
    for (task, expected) in [
        ("analyze", Some(Mood::Inspecting)),
        ("generate", Some(Mood::Drawing)),
        ("other", Some(Mood::Thinking)),
        ("done", None),
    ] {
        r["function_calls"] = json!([{"name":"use_assistant","arguments":{"task":task}}]);
        assert_eq!(animation_hint(&p, Some(&r)), expected);
    }
    assert_eq!(animation_hint(&p, None), None);
}

#[test]
fn absent_disabled_or_uncertain_needle_keeps_activity_animation_without_extra_calls() {
    for scenario in ["off", "missing", "uncertain"] {
        let (url, _request) = mock("openai");
        let mut p = payload("openai");
        p["config"]["endpoint"] = json!(url);
        p["action"] = json!("assist");
        if scenario == "off" {
            p["decision"] = json!("off");
        }
        let (tx, rx) = mpsc::channel();
        run_using(p, Arc::new(AtomicBool::new(false)), tx, |_, _| {
            assert_ne!(scenario, "off");
            if scenario == "missing" {
                Err("model missing".into())
            } else {
                let mut r = reply();
                r["confidence"] = json!(0.1);
                Ok(r)
            }
        })
        .unwrap();
        assert!(rx
            .try_iter()
            .any(|event| event == Progress::Animation(Mood::Inspecting)));
    }
}
