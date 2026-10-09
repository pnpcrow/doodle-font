//! Live local generation evidence. Outputs are kept outside the source tree.
use doodle_core::{document, font, local_ai, model::Artwork};
use serde_json::json;
use std::sync::{atomic::AtomicBool, mpsc, Arc};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).is_some_and(|s| s == "--sheet") {
        let before = args.get(2).unwrap();
        let after = args.get(3).unwrap();
        let output = args.get(4).unwrap();
        let labels = std::fs::read("assets/fonts/NotoSansCJKkr-VF.otf").unwrap();
        let mut sheet = image::RgbaImage::from_pixel(820, 1520, image::Rgba([249, 248, 243, 255]));
        for (title, x) in [
            ("요청 글자", 20),
            ("수정 전 · 내장 AI", 190),
            ("수정 후 · 내장 AI", 490),
        ] {
            let label = doodle_core::render::text(&labels, title, "ko", 23.).unwrap();
            image::imageops::overlay(&mut sheet, &label, x, 10);
        }
        for i in 0..6 {
            let old: serde_json::Value =
                serde_json::from_slice(&std::fs::read(format!("{before}/{i}.json")).unwrap())
                    .unwrap();
            let new: serde_json::Value =
                serde_json::from_slice(&std::fs::read(format!("{after}/{i}.json")).unwrap())
                    .unwrap();
            let y = 60 + i as i64 * 240;
            let label =
                doodle_core::render::text(&labels, old["text"].as_str().unwrap(), "ko", 50.)
                    .unwrap();
            image::imageops::overlay(&mut sheet, &label, 50, y + 65);
            for (value, x) in [(&old, 190), (&new, 490)] {
                let art: Artwork =
                    serde_json::from_value(value["result"]["artwork"].clone()).unwrap();
                let drawing = font::raster(&art, 220).unwrap();
                image::imageops::overlay(&mut sheet, &drawing, x, y);
            }
        }
        sheet.save(output).unwrap();
        return;
    }
    if args.get(1).is_some_and(|s| s == "--font-diagnose") {
        for path in [
            "assets/fonts/NotoSansCJKkr-VF.otf",
            "assets/fonts/NotoSerifCJKkr-VF.otf",
            "assets/fonts/SingleDay-Regular.ttf",
        ] {
            let bytes = std::fs::read(path).unwrap();
            let mut f = ttf_parser::Face::parse(&bytes, 0).unwrap();
            for text in ["骨", "直", "令", "曜", "辻", "葛", "逸", "遍", "道"] {
                let mut ids = vec![];
                for language in ["ko", "ja"] {
                    let mut b = rustybuzz::UnicodeBuffer::new();
                    b.push_str(text);
                    b.guess_segment_properties();
                    b.set_language(language.parse().unwrap());
                    let face = rustybuzz::Face::from_slice(&bytes, 0).unwrap();
                    let result = rustybuzz::shape(&face, &[], b);
                    ids.push(result.glyph_infos()[0].glyph_id);
                }
                println!("regional {text}: {ids:?}");
            }
            println!(
                "{path}: {} glyphs, glyf {} gvar {}",
                f.number_of_glyphs(),
                f.tables().glyf.is_some(),
                f.tables().gvar.is_some()
            );
            for weight in [100., 400., 650.] {
                f.set_variation(ttf_parser::Tag::from_bytes(b"wght"), weight);
                for c in ['A', '한', 'O'] {
                    let gid = f.glyph_index(c);
                    println!(
                        "{c}: {gid:?}, weight {weight}, bounds {:?}",
                        gid.and_then(|g| f.glyph_bounding_box(g))
                    );
                }
            }
        }
        return;
    }
    let directory = args.get(1).expect("output directory");
    std::fs::create_dir_all(directory).unwrap();
    let style_suite = args.get(2).is_some_and(|s| s == "--styles");
    let cases = if style_suite {
        vec![("한","ko","선택한 한 글자를 얇고 단정한 명조체로 그려줘."),("가","ko","가 글자를 따뜻한 손글씨로 만들어줘."),("A","default","현재 그림을 유지해서 보정해줘. 두 획 모두 굵기를 70으로 통일하고 글자의 형태와 위치는 그대로 유지해줘.")]
    } else {
        vec![
            ("A", "default", ""),
            ("B", "default", ""),
            ("가", "ko", ""),
            ("한", "ko", ""),
            ("あ", "ja", ""),
            ("永", "ja", ""),
        ]
    };
    let mut failed = false;
    for (i, (text, locale, instruction)) in cases.iter().enumerate() {
        let mut glyph = document::new_glyph(text, locale, "quality", false);
        if style_suite && i == 2 {
            glyph = document::new_project().glyphs.remove(0);
            glyph.artwork.shapes[0].stroke = 5.;
            glyph.artwork.shapes[1].stroke = 70.;
        }
        let prompt = if instruction.is_empty() {
            format!("선택한 '{text}' 글자를 검은색의 굵고 둥근 고딕체로 그려줘. 모든 획의 굵기를 일정하게 하고 정확히 읽을 수 있게 해줘.")
        } else {
            (*instruction).to_owned()
        };
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            while let Ok(s) = rx.recv() {
                eprintln!("{s}");
            }
        });
        let start = std::time::Instant::now();
        let result = local_ai::run(
            json!({"action":"generate","glyph":glyph,"config":{"provider":"local"},"prompt":prompt,"colorMode":"mono","references":[]}),
            Arc::new(AtomicBool::new(false)),
            tx,
        );
        let evidence = match result {
            Ok(result) => {
                let art: Artwork = serde_json::from_value(result["artwork"].clone()).unwrap();
                font::raster(&art, 512)
                    .unwrap()
                    .save(format!("{directory}/{i}.png"))
                    .unwrap();
                json!({"text":text,"prompt":prompt,"seconds":start.elapsed().as_secs_f64(),"result":result})
            }
            Err(error) => {
                failed = true;
                json!({"text":text,"prompt":prompt,"seconds":start.elapsed().as_secs_f64(),"error":error})
            }
        };
        std::fs::write(
            format!("{directory}/{i}.json"),
            serde_json::to_vec_pretty(&evidence).unwrap(),
        )
        .unwrap();
        println!(
            "{}: {:.1}s {}",
            text,
            start.elapsed().as_secs_f64(),
            evidence["error"]
        );
    }
    if failed {
        std::process::exit(1);
    }
}
