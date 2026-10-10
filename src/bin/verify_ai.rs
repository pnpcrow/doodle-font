use doodle_core::{document, local_ai};
use serde_json::json;
use std::sync::{atomic::AtomicBool, mpsc, Arc};
fn main() {
    if std::env::args().any(|a| a == "--diagnose") {
        println!(
            "{}",
            json!({"root":local_ai::root(),"installed":local_ai::installed(),"missing":doodle_core::ai_install::missing(&local_ai::root())})
        );
        return;
    }
    let project = document::new_project();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        while let Ok(s) = rx.recv() {
            eprintln!("{s}");
        }
    });
    let start = std::time::Instant::now();
    let action = if std::env::args().any(|a| a == "--generate") {
        "generate"
    } else {
        "assist"
    };
    let prompt = if action == "generate" {
        "참고 그림처럼 노란 원 안에 검은 눈 두 개와 웃는 입이 있는 얼굴을 만들어 줘."
    } else {
        "그림에 어떤 도형과 색이 보이는지 한국어 두 문장으로 설명해 줘."
    };
    let result=local_ai::run(json!({"glyph":project.glyphs[1],"action":action,"config":{"provider":"local"},"prompt":prompt,"references":project.templates}),Arc::new(AtomicBool::new(false)),tx).expect("live local text + image inference");
    assert!(result["embedding"].is_null());
    assert!(
        result["answer"].as_str().is_some_and(|s| !s.is_empty()) || result["artwork"].is_object()
    );
    std::fs::create_dir_all("artifacts").unwrap();
    if action == "generate" {
        let art: doodle_core::model::Artwork =
            serde_json::from_value(result["artwork"].clone()).unwrap();
        assert!(!art.shapes.is_empty());
        doodle_core::font::raster(&art, 512)
            .unwrap()
            .save("artifacts/local-ai-generated.png")
            .unwrap();
    }
    std::fs::write(
        format!("artifacts/local-ai-{action}.json"),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
    println!(
        "Local multimodal inference passed in {:.1}s: {}",
        start.elapsed().as_secs_f64(),
        result["answer"]
    );
}
