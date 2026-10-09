use base64::{engine::general_purpose::STANDARD, Engine};
use doodle_core::{document::*, font, storage};
fn main() {
    let folder = std::path::Path::new("artifacts/verification");
    std::fs::create_dir_all(folder).unwrap();
    let mut p = new_project();
    let art = p.glyphs[0].artwork.clone();
    for (text, locale) in [
        ("骨", "ko"),
        ("骨", "ja"),
        ("fi", "default"),
        ("👩‍💻", "default"),
        (" ", "default"),
    ] {
        let mut g = new_glyph(text, locale, "custom", text == "fi");
        g.artwork = art.clone();
        g.status = "edited".into();
        if text == " " {
            g.status = "blank".into();
            g.artwork.advance = 340.;
        }
        p.glyphs.push(g);
    }
    storage::save(&folder.join("demo.doodle.json"), &p).unwrap();
    for mode in ["mono", "color", "bitmap"] {
        let start = std::time::Instant::now();
        let r = font::export(
            &p,
            &font::ExportOptions {
                mode: Some(mode.into()),
                ..Default::default()
            },
        )
        .unwrap();
        let data = STANDARD.decode(r.data).unwrap();
        std::fs::write(folder.join(format!("demo-{mode}.ttf")), &data).unwrap();
        doodle_core::render::text(&data, "A❤️😊 fi 骨", "ja", 160.)
            .unwrap()
            .save(folder.join(format!("render-{mode}.png")))
            .unwrap();
        println!(
            "{mode}: {} bytes, {} glyphs, {:?}",
            data.len(),
            r.reports[0].glyph_count,
            start.elapsed()
        );
    }
    let start = std::time::Instant::now();
    let all = catalog("hangul");
    let han = catalog("han-ko");
    println!(
        "Catalog: {} entries in {:?}; sparse project: {} glyphs",
        all.len() + han.len(),
        start.elapsed(),
        new_project().glyphs.len()
    );
    if std::env::args().any(|a| a == "--large") {
        let mut p = new_project();
        p.glyphs.clear();
        let start = std::time::Instant::now();
        for (group, locale) in [("hangul", "ko"), ("han-ko", "ko"), ("han-ja", "ja")] {
            add_texts(&mut p, &catalog(group), locale, group, false);
        }
        for g in &mut p.glyphs {
            g.status = "blank".into();
        }
        let model_ms = start.elapsed().as_secs_f64() * 1000.;
        let start = std::time::Instant::now();
        let r = font::export(
            &p,
            &font::ExportOptions {
                mode: Some("mono".into()),
                ..Default::default()
            },
        )
        .unwrap();
        let export_ms = start.elapsed().as_secs_f64() * 1000.;
        let bytes = STANDARD.decode(r.data).unwrap();
        std::fs::write(folder.join("large-font.ttf"), &bytes).unwrap();
        let report = serde_json::json!({"fixture":"blank glyph metrics + regional alternatives; no bitmap data","projectGlyphs":p.glyphs.len(),"outputGlyphs":r.reports[0].glyph_count,"createModelMs":model_ms,"exportMs":export_ms,"fontBytes":bytes.len()});
        std::fs::write(
            folder.join("benchmark.json"),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
        storage::save(&folder.join("large-project.json"), &p).unwrap();
        println!("{report}");
    }
}
