use base64::{engine::general_purpose::STANDARD, Engine};
use doodle_core::{document::*, font, model::*, storage};
fn exported(p: &Project, mode: &str) -> Vec<u8> {
    let r = font::export(
        p,
        &font::ExportOptions {
            mode: Some(mode.into()),
            ..Default::default()
        },
    )
    .unwrap();
    STANDARD.decode(r.data).unwrap()
}
fn shaped(bytes: &[u8], text: &str, lang: &str) -> Vec<u32> {
    let face = rustybuzz::Face::from_slice(bytes, 0).unwrap();
    let mut b = rustybuzz::UnicodeBuffer::new();
    b.push_str(text);
    b.guess_segment_properties();
    b.set_language(lang.parse().unwrap());
    rustybuzz::shape(&face, &[], b)
        .glyph_infos()
        .iter()
        .map(|g| g.glyph_id)
        .collect()
}
#[test]
fn exact_sequences_and_regional_identity() {
    let text = "👩‍💻❤️e\u{301}";
    assert_eq!(graphemes(text), vec!["👩‍💻", "❤️", "e\u{301}"]);
    let mut p = new_project();
    add_texts(&mut p, &["骨".into()], "ko", "han-ko", false);
    add_texts(&mut p, &["骨".into()], "ja", "han-ja", false);
    assert_eq!(p.glyphs.iter().filter(|g| g.text == "骨").count(), 2);
    assert_eq!(add_texts(&mut p, &["骨".into()], "ja", "han-ja", false), 0);
    p.validate().unwrap();
}
#[test]
fn original_survives_roundtrip_and_edits() {
    let mut p = new_project();
    p.glyphs[0].artwork.shapes[0].color = "#ffffff".into();
    let json = serde_json::to_vec(&p).unwrap();
    let mut p: Project = serde_json::from_slice(&json).unwrap();
    restore(&mut p.glyphs[0], "template").unwrap();
    assert_eq!(p.glyphs[0].artwork.shapes[0].color, "#34352f");
    let raw = p.glyphs[0].text.clone();
    restore(&mut p.glyphs[0], "system").unwrap();
    assert_eq!(p.glyphs[0].text, raw);
    p.glyphs[0].locked = true;
    assert!(restore(&mut p.glyphs[0], "template").is_err());
}
#[test]
fn candidate_cannot_overwrite_blank_or_locked() {
    let mut g = new_project().glyphs.remove(0);
    g.candidates.push(Candidate {
        id: "c".into(),
        name: "candidate".into(),
        artwork: g.artwork.clone(),
        sources: vec![],
    });
    g.status = "blank".into();
    assert!(accept(&mut g, "c").is_err());
    g.status = "edited".into();
    g.locked = true;
    assert!(accept(&mut g, "c").is_err());
    g.locked = false;
    accept(&mut g, "c").unwrap();
}
#[test]
fn true_type_and_color_tables_parse() {
    let p = new_project();
    for mode in ["mono", "color", "bitmap"] {
        let bytes = exported(&p, mode);
        let face = ttf_parser::Face::parse(&bytes, 0).unwrap();
        assert!(face.number_of_glyphs() > 3);
        assert_eq!(face.units_per_em(), 1000);
        assert!(face.glyph_index('A').is_some());
        assert_eq!(shaped(&bytes, "❤️", "en").len(), 1);
        if mode == "color" {
            assert!(face.tables().colr.is_some());
        }
        if mode == "bitmap" {
            assert!(face
                .glyph_raster_image(face.glyph_index('A').unwrap(), 512)
                .is_some());
        }
    }
}
#[test]
fn ligatures_and_localized_forms_shape() {
    let mut p = new_project();
    let art = p.glyphs[0].artwork.clone();
    for (text, locale) in [
        ("骨", "ko"),
        ("骨", "ja"),
        ("fi", "default"),
        ("ffi", "default"),
        ("👩‍💻", "default"),
    ] {
        let mut g = new_glyph(text, locale, "custom", text == "fi" || text == "ffi");
        g.artwork = art.clone();
        g.status = "edited".into();
        p.glyphs.push(g)
    }
    let bytes = exported(&p, "mono");
    assert_ne!(shaped(&bytes, "骨", "ko"), shaped(&bytes, "骨", "ja"));
    assert_eq!(shaped(&bytes, "fi", "en").len(), 1);
    assert_eq!(shaped(&bytes, "ffi", "en").len(), 1);
    assert_eq!(shaped(&bytes, "👩‍💻", "en").len(), 1);
}
#[test]
fn space_width_and_export_exclusions() {
    let mut p = new_project();
    for (text, width) in [(" ", 320.), ("\u{a0}", 700.), ("\u{3000}", 1000.)] {
        let mut g = new_glyph(text, "default", "spaces", false);
        g.status = "blank".into();
        g.artwork.advance = width;
        p.glyphs.push(g);
    }
    p.glyphs.push(new_glyph("\n", "default", "spaces", false));
    let bytes = exported(&p, "mono");
    let face = ttf_parser::Face::parse(&bytes, 0).unwrap();
    assert_eq!(
        face.glyph_hor_advance(face.glyph_index(' ').unwrap()),
        Some(320)
    );
    assert_eq!(
        face.glyph_hor_advance(face.glyph_index('\u{a0}').unwrap()),
        Some(700)
    );
    assert!(face.glyph_index('\n').is_none());
}
#[test]
fn rights_and_missing_original_are_enforced() {
    let mut p = new_project();
    p.glyphs[0].template.as_mut().unwrap().permissions.embed = false;
    assert!(font::export(&p, &Default::default()).is_err());
    p.glyphs[0].template.as_mut().unwrap().original = None;
    assert!(restore(&mut p.glyphs[0], "template").is_err());
}
#[test]
fn atomic_save_backup_roundtrip() {
    let folder = std::env::temp_dir().join(uid());
    let path = folder.join("project.json");
    let mut p = new_project();
    storage::save(&path, &p).unwrap();
    p.name = "승인 프로젝트".into();
    storage::save(&path, &p).unwrap();
    assert_eq!(storage::read(&path).unwrap().name, p.name);
    assert_eq!(
        storage::read(&path.with_extension("backup.json"))
            .unwrap()
            .name,
        "나의 첫 번째 폰트"
    );
    std::fs::remove_dir_all(folder).unwrap();
}
#[test]
fn project_preview_uses_longest_custom_and_locale() {
    let mut p = new_project();
    add_texts(
        &mut p,
        &["hello".into(), "hello world".into()],
        "default",
        "custom",
        true,
    );
    let tokens = preview("hello world A", &p.glyphs, "ko");
    assert_eq!(tokens[0].0, "hello world");
    assert!(tokens[0].1.is_some());
}
#[test]
fn split_export_is_a_zip_with_report() {
    let p = new_project();
    let r = font::export(
        &p,
        &font::ExportOptions {
            split: Some(true),
            ..Default::default()
        },
    )
    .unwrap();
    let bytes = STANDARD.decode(r.data).unwrap();
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    assert!(z.by_name("font-report.json").is_ok());
    assert_eq!(r.reports.len(), 2);
}
#[test]
fn sparse_catalog_does_not_expand_project() {
    let p = new_project();
    assert_eq!(catalog("hangul").len(), 11172);
    assert_eq!(catalog("han-ko").len(), 20992);
    assert_eq!(p.glyphs.len(), 3);
}
#[test]
fn invalid_import_rejected() {
    let mut p = new_project();
    p.glyphs.push(p.glyphs[0].clone());
    assert!(p.validate().is_err());
    let mut p = new_project();
    p.glyphs[0].artwork.shapes[0].color = "red".into();
    assert!(p.validate().is_err());
}

#[test]
fn native_renderer_reads_color_and_bitmap() {
    let p = new_project();
    for mode in ["mono", "color", "bitmap"] {
        let bytes = exported(&p, mode);
        let im = doodle_core::render::text(&bytes, "A❤️😊", "ko", 96.).unwrap();
        assert!(im.pixels().filter(|p| p[3] > 0).count() > 1000);
        if mode != "mono" {
            assert!(im.pixels().any(|p| p[0] > p[1] + 20 && p[3] > 0));
        }
    }
}
#[test]
fn component_export_rights_cannot_be_lost() {
    let mut p = new_project();
    let mut source = templates().remove(0);
    source.permissions.embed = false;
    p.glyphs[0].sources.push(source);
    assert!(font::export(&p, &Default::default()).is_err());
}
#[test]
fn spoofed_control_import_is_rejected() {
    let mut p = new_project();
    let mut g = new_glyph("\n", "default", "spaces", false);
    g.mapping = "unicode".into();
    g.status = "edited".into();
    p.glyphs.push(g);
    assert!(p.validate().is_err());
}
#[test]
fn zero_alpha_and_hidden_layers_do_not_export_shapes() {
    let mut p = new_project();
    for s in &mut p.glyphs[0].artwork.shapes {
        s.visible = false;
    }
    let bytes = exported(&p, "mono");
    let face = ttf_parser::Face::parse(&bytes, 0).unwrap();
    assert!(face
        .glyph_bounding_box(face.glyph_index('A').unwrap())
        .is_none());
}
