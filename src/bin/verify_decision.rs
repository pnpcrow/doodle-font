//! Real optional-model probe; no external AI endpoint or API key is used.
use doodle_core::{ai_install, ai_pipeline, document, needle};
use serde_json::json;
use std::{
    sync::{atomic::AtomicBool, mpsc, Arc},
    time::Instant,
};
fn main() {
    let root = needle::root();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        while let Ok(p) = rx.recv() {
            let p: ai_install::Progress = p;
            eprintln!("{}", p.message);
        }
    });
    ai_install::install_package(
        root.clone(),
        ai_install::Package::Needle,
        Arc::new(AtomicBool::new(false)),
        tx,
    )
    .expect("decision model install/verify");
    let mut evidence = vec![];
    for prompt in [
        "Set the stroke width to 70.",
        "현재 그림의 획 굵기를 70으로 통일해줘.",
        "그림을 검은색으로 바꿔줘.",
        "A를 굵고 둥근 고딕체로 그려줘.",
    ] {
        let start = Instant::now();
        let raw = needle::infer(prompt, Arc::new(AtomicBool::new(false)));
        let glyph = document::new_project().glyphs.remove(0);
        let payload =
            json!({"glyph":glyph,"action":"generate","prompt":prompt,"colorMode":"color"});
        let result = ai_pipeline::plan(&payload, &glyph, raw.as_ref().ok());
        println!(
            "{prompt}: quick={} {:.2}s",
            result.candidate.is_some(),
            start.elapsed().as_secs_f64()
        );
        let hint = ai_pipeline::animation_hint(&payload, raw.as_ref().ok());
        println!("  motion={hint:?}");
        evidence.push(json!({"prompt":prompt,"seconds":start.elapsed().as_secs_f64(),"raw":raw.as_ref().ok(),"error":raw.as_ref().err(),"quick":result.candidate.is_some(),"note":result.note,"motion":format!("{hint:?}")}));
    }
    let (tx, rx) = mpsc::channel();
    let result=ai_pipeline::run(json!({"glyph":document::new_project().glyphs[0],"action":"generate","decision":"needle","prompt":"Set the stroke width to 70.","colorMode":"color","config":{"provider":"openai","endpoint":"http://127.0.0.1:1/v1","model":"unreachable"}}),Arc::new(AtomicBool::new(false)),tx).expect("fast edit must skip unavailable external API");
    assert_eq!(result["artwork"]["shapes"][0]["stroke"], 70.);
    let progress = rx.try_iter().collect::<Vec<_>>();
    assert!(
        progress.contains(&doodle_core::ai_progress::Progress::Animation(
            doodle_core::companion::Mood::Thinking
        ))
    );
    assert!(
        progress.contains(&doodle_core::ai_progress::Progress::Animation(
            doodle_core::companion::Mood::Drawing
        ))
    );
    let cancel = Arc::new(AtomicBool::new(false));
    let flag = cancel.clone();
    let worker = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(100));
        flag.store(true, std::sync::atomic::Ordering::Relaxed);
    });
    let cancelled = needle::infer("Set the stroke width to 70.", cancel);
    worker.join().unwrap();
    assert!(cancelled.unwrap_err().contains("취소"));
    assert!(needle::infer_at(
        &root,
        "Set the stroke width to 70.",
        Arc::new(AtomicBool::new(false)),
        std::time::Duration::from_millis(30)
    )
    .unwrap_err()
    .contains("시간"));
    std::fs::create_dir_all("artifacts").unwrap();
    std::fs::write(
        "artifacts/decision-live.json",
        serde_json::to_vec_pretty(&evidence).unwrap(),
    )
    .unwrap();
}
