use doodle_core::{ai, document::*};
use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::mpsc,
};
fn mock(provider: &str, answer: &str) -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (tx, rx) = mpsc::channel();
    let response = if provider == "openai" {
        json!({"choices":[{"message":{"content":answer}}]})
    } else {
        json!({"content":[{"type":"text","text":answer}]})
    }
    .to_string();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(3)))
            .unwrap();
        let mut bytes = vec![];
        let mut buffer = [0u8; 4096];
        loop {
            let n = stream.read(&mut buffer).unwrap();
            if n == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..n]);
            let text = String::from_utf8_lossy(&bytes);
            if let Some(end) = text.find("\r\n\r\n") {
                let length = text[..end]
                    .lines()
                    .find_map(|l| {
                        l.to_lowercase()
                            .strip_prefix("content-length:")
                            .and_then(|s| s.trim().parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                if bytes.len() >= end + 4 + length {
                    break;
                }
            }
        }
        tx.send(String::from_utf8(bytes).unwrap()).unwrap();
        write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",response.len(),response).unwrap();
    });
    (format!("http://{address}/v1"), rx)
}
fn run(v: Value) -> Result<doodle_core::model::Artwork, String> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(ai::generate(v))
}
#[test]
fn both_provider_contracts_preserve_exact_target() {
    for provider in ["openai", "anthropic"] {
        let answer=json!({"advance":1000,"shapes":[{"type":"path","points":[[100,800],[500,100],[900,800]]}]}).to_string();
        let (url, rx) = mock(provider, &answer);
        let mut g = new_glyph("👩‍💻", "ja", "symbols", false);
        g.artwork.shapes.push(path(&[[12., 34.], [45., 67.]]));
        let before = serde_json::to_string(&g).unwrap();
        let result=run(json!({"glyph":g,"config":{"provider":provider,"endpoint":url,"model":"test-model","key":"test-key"},"prompt":"rounded","sendReference":true})).unwrap();
        assert_eq!(result.shapes.len(), 1);
        let req = rx.recv().unwrap();
        assert!(req.contains(if provider == "openai" {
            "/v1/chat/completions"
        } else {
            "/v1/messages"
        }));
        assert!(req.contains("U+200D"));
        assert!(req.contains("ja"));
        assert!(req.contains("reference"));
        assert!(req.contains("image/png"));
        assert!(req.contains(if provider == "openai" {
            "image_url"
        } else {
            "base64"
        }));
        assert_eq!(serde_json::to_string(&g).unwrap(), before);
    }
}
#[test]
fn no_image_is_sent_without_reference_consent() {
    let (url, rx) = mock(
        "openai",
        r#"{"shapes":[{"type":"path","points":[[100,800],[500,100],[900,800]]}]}"#,
    );
    run(json!({"glyph":new_glyph("A","default","latin",false),"config":{"provider":"openai","endpoint":url,"model":"test"},"sendReference":false})).unwrap();
    let request = rx.recv().unwrap();
    assert!(!request.contains("image_url"));
    assert!(!request.contains("image/png"));
}
#[test]
fn canvas_snapshot_preserves_color_on_white() {
    use base64::Engine;
    let project = new_project();
    let picture = ai::canvas_image(&project.glyphs[1].artwork).unwrap();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(picture.trim_start_matches("data:image/png;base64,"))
        .unwrap();
    let image = image::load_from_memory(&bytes).unwrap().into_rgba8();
    assert_eq!(image.dimensions(), (512, 512));
    assert_eq!(image.get_pixel(0, 0).0, [255, 255, 255, 255]);
    assert!(image.pixels().any(|p| p[0] > p[2].saturating_add(50)));
}
#[test]
fn cancelling_active_request_returns_promptly() {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let cancel = Arc::new(AtomicBool::new(false));
    let stop = cancel.clone();
    std::thread::spawn(move || {
        let (_stream, _) = listener.accept().unwrap();
        stop.store(true, Ordering::Relaxed);
        std::thread::sleep(std::time::Duration::from_millis(300));
    });
    let (tx, _rx) = mpsc::channel();
    let start = std::time::Instant::now();
    let result = doodle_core::local_ai::run(
        json!({"action":"assist","glyph":new_glyph("A","default","latin",false),"config":{"provider":"openai","endpoint":format!("http://{address}/v1"),"model":"test"}}),
        cancel,
        tx,
    );
    assert!(result.unwrap_err().contains("취소"));
    assert!(start.elapsed().as_secs_f32() < 2.);
}
#[test]
fn malformed_ai_does_not_panic() {
    let (url, _rx) = mock("openai", r#"{"shapes":[{}]}"#);
    let result = run(
        json!({"glyph":new_glyph("A","default","latin",false),"config":{"provider":"openai","endpoint":url,"model":"test"}}),
    );
    assert!(result.is_err());
}
#[test]
fn empty_invisible_and_off_canvas_candidates_are_rejected() {
    for answer in [
        r#"{"shapes":[]}"#,
        r#"{"shapes":[{"type":"path","points":[]}]}"#,
        r#"{"shapes":[{"type":"path","points":[[100,100],[700,700]],"opacity":0}]}"#,
        r#"{"shapes":[{"type":"path","points":[[100,100],[700,700]],"visible":false}]}"#,
        r#"{"shapes":[{"type":"rect","x":2000,"y":2000,"width":100,"height":100,"fill":true}]}"#,
    ] {
        let (url, _rx) = mock("openai", answer);
        let result = run(
            json!({"glyph":new_glyph("A","default","latin",false),"config":{"provider":"openai","endpoint":url,"model":"test"}}),
        );
        assert!(result.unwrap_err().contains("보이는 그림"));
    }
}
#[test]
fn drawing_action_returns_artwork_and_requires_nonempty_local_schema() {
    let (url, rx) = mock(
        "openai",
        r#"{"shapes":[{"type":"path","points":[[100,800],[500,100],[900,800]]}]}"#,
    );
    let (tx, _status) = mpsc::channel();
    let result = doodle_core::local_ai::run(
        json!({"action":"generate","glyph":new_glyph("☺","default","symbols",false),"config":{"provider":"openai","endpoint":url,"model":"test","local":true}}),
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)), tx,
    ).unwrap();
    assert!(result["artwork"]["shapes"]
        .as_array()
        .is_some_and(|s| !s.is_empty()));
    assert!(result["answer"].is_null());
    let request = rx.recv().unwrap();
    let body: Value = serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(
        body["response_format"]["schema"]["properties"]["shapes"]["minItems"],
        1
    );
}
#[test]
fn explanation_cannot_succeed_as_a_drawing() {
    let (url, _rx) = mock("openai", "둥근 획으로 그리면 좋아요.");
    let result = run(
        json!({"glyph":new_glyph("가","ko","hangul",false),"config":{"provider":"openai","endpoint":url,"model":"test"}}),
    );
    assert!(result.unwrap_err().contains("그림"));
}
#[test]
fn missing_shape_dimensions_are_rejected() {
    let (url, _rx) = mock(
        "openai",
        r##"{"shapes":[{"type":"ellipse","width":600,"color":"#ffcc00"}]}"##,
    );
    let result = run(
        json!({"glyph":new_glyph("A","default","latin",false),"config":{"provider":"openai","endpoint":url,"model":"test"}}),
    );
    assert!(result.unwrap_err().contains("크기"));
}
#[test]
fn protected_targets_never_make_network_request() {
    for state in ["blank", "unsupported"] {
        let mut g = new_glyph("A", "default", "latin", false);
        g.status = state.into();
        let result = run(
            json!({"glyph":g,"config":{"provider":"openai","endpoint":"http://127.0.0.1:1/v1","model":"test"}}),
        );
        assert!(result.unwrap_err().contains("보호"));
    }
}
#[test]
fn component_ai_restrictions_are_preserved() {
    let mut g = new_glyph("A", "default", "latin", false);
    let mut source = templates().remove(0);
    source.permissions.ai = false;
    g.sources.push(source);
    assert!(run(json!({"glyph":g})).unwrap_err().contains("권한"));
}
