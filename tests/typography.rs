use base64::{engine::general_purpose::STANDARD, Engine};
use doodle_core::{
    document::*,
    font,
    model::*,
    typography::{self, Style},
};

fn make(text: &str, locale: &str, style: &Style) -> typography::Generated {
    typography::generate(&new_glyph(text, locale, "quality", false), style, true).unwrap()
}
fn ink(art: &Artwork) -> usize {
    font::raster(art, 512)
        .unwrap()
        .pixels()
        .filter(|p| p[3] > 128)
        .count()
}

#[test]
fn multilingual_shapes_are_distinct_visible_and_have_no_missing_glyphs() {
    let mut seen = std::collections::HashSet::new();
    for (text, locale) in [
        ("A", "default"),
        ("B", "default"),
        ("g", "default"),
        ("é", "default"),
        ("e\u{301}", "default"),
        ("가", "ko"),
        ("한", "ko"),
        ("힣", "ko"),
        ("뷁", "ko"),
        ("あ", "ja"),
        ("ガ", "ja"),
        ("永", "ja"),
        ("鬱", "ja"),
        ("曜", "ko"),
        ("曜", "ja"),
        ("?", "default"),
        ("。", "ja"),
    ] {
        let generated = make(
            text,
            locale,
            &Style {
                weight: 650,
                roundness: 80,
                ..Default::default()
            },
        );
        generated.artwork.validate().unwrap();
        assert!(ink(&generated.artwork) > 1500, "{text}");
        assert!(
            generated.artwork.shapes[0]
                .contours
                .iter()
                .flatten()
                .all(|p| p[0] >= 0. && p[0] <= 1000. && p[1] >= 0. && p[1] <= 1000.),
            "clipped: {text}"
        );
        // Precomposed/decomposed accented text should intentionally share a drawing.
        if text != "e\u{301}" {
            assert!(
                seen.insert(serde_json::to_string(&generated.artwork.shapes[0].contours).unwrap()),
                "duplicate shape: {text}/{locale}"
            );
        }
    }
}
#[test]
fn weight_width_slant_and_family_actually_change_the_drawing() {
    let normal = make("한", "ko", &Style::default());
    let thin = make(
        "한",
        "ko",
        &Style {
            weight: 250,
            ..Default::default()
        },
    );
    let bold = make(
        "한",
        "ko",
        &Style {
            weight: 750,
            ..Default::default()
        },
    );
    assert!(ink(&bold.artwork) > ink(&thin.artwork) * 2);
    for style in [
        Style {
            width: 80,
            ..Default::default()
        },
        Style {
            slant: 12,
            ..Default::default()
        },
        Style {
            family: "serif".into(),
            ..Default::default()
        },
        Style {
            family: "handwritten".into(),
            ..Default::default()
        },
    ] {
        let changed = make("한", "ko", &style);
        assert_ne!(
            serde_json::to_value(&changed.artwork.shapes[0].contours).unwrap(),
            serde_json::to_value(&normal.artwork.shapes[0].contours).unwrap()
        );
    }
}
#[test]
fn local_style_refines_existing_strokes_without_inventing_new_geometry() {
    let mut glyph = new_project().glyphs.remove(0);
    glyph.artwork.shapes[0].stroke = 5.;
    glyph.artwork.shapes[1].stroke = 70.;
    let result = typography::generate(
        &glyph,
        &Style {
            source: "current".into(),
            stroke: 48,
            ..Default::default()
        },
        true,
    )
    .unwrap();
    assert_eq!(result.artwork.shapes.len(), 2);
    for (before, after) in glyph.artwork.shapes.iter().zip(&result.artwork.shapes) {
        assert_eq!(before.points, after.points);
        assert_eq!(after.stroke, 48.);
    }
}
#[test]
fn outline_counters_survive_raster_export_and_license_provenance() {
    let generated = make("O", "default", &Style::default());
    let image = font::raster(&generated.artwork, 512).unwrap();
    let shape = &generated.artwork.shapes[0];
    let points: Vec<_> = shape.contours.iter().flatten().collect();
    let cx = (points.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min)
        + points
            .iter()
            .map(|p| p[0])
            .fold(f64::NEG_INFINITY, f64::max))
        / 2.;
    let cy = (points.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min)
        + points
            .iter()
            .map(|p| p[1])
            .fold(f64::NEG_INFINITY, f64::max))
        / 2.;
    assert_eq!(
        image.get_pixel((cx * 0.512) as u32, (cy * 0.512) as u32)[3],
        0,
        "O counter must remain transparent"
    );
    let mut p = new_project();
    p.glyphs.clear();
    let mut g = new_glyph("O", "default", "latin", false);
    g.candidates.push(Candidate {
        id: "local".into(),
        name: "local".into(),
        artwork: generated.artwork,
        sources: generated.sources,
    });
    accept(&mut g, "local").unwrap();
    assert_eq!(g.sources.len(), 1);
    p.glyphs.push(g);
    let copy: Project = serde_json::from_slice(&serde_json::to_vec(&p).unwrap()).unwrap();
    copy.validate().unwrap();
    for mode in ["mono", "color", "bitmap"] {
        let result = font::export(
            &copy,
            &font::ExportOptions {
                mode: Some(mode.into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(result.reports[0]
            .license_text
            .as_ref()
            .unwrap()
            .contains("SIL OPEN FONT LICENSE"));
        let bytes = STANDARD.decode(result.data).unwrap();
        let face = ttf_parser::Face::parse(&bytes, 0).unwrap();
        assert!(face.names().into_iter().any(|n| n.name_id == 13
            && n.to_string()
                .is_some_and(|s| s.contains("SIL OPEN FONT LICENSE"))));
        let rendered = doodle_core::render::text(&bytes, "O", "default", 512.).unwrap();
        let colored: Vec<_> = rendered
            .enumerate_pixels()
            .filter(|(_, _, p)| p[3] > 128)
            .collect();
        let x = (colored.iter().map(|(x, _, _)| *x).min().unwrap()
            + colored.iter().map(|(x, _, _)| *x).max().unwrap())
            / 2;
        let y = (colored.iter().map(|(_, y, _)| *y).min().unwrap()
            + colored.iter().map(|(_, y, _)| *y).max().unwrap())
            / 2;
        assert_eq!(rendered.get_pixel(x, y)[3], 0, "{mode} O counter filled");
    }
}
#[test]
fn unsupported_text_and_invalid_styles_fail_instead_of_substituting_random_letters() {
    assert!(typography::generate(
        &new_glyph("\u{10ffff}", "default", "quality", false),
        &Style::default(),
        true
    )
    .is_err());
    assert!(typography::generate(
        &new_glyph("가", "ko", "hangul", false),
        &Style {
            weight: 0,
            ..Default::default()
        },
        true
    )
    .is_err());
}

#[test]
fn bundled_sans_covers_the_full_hangul_syllable_range() {
    let bytes = include_bytes!("../assets/fonts/NotoSansCJKkr-VF.otf");
    let face = ttf_parser::Face::parse(bytes, 0).unwrap();
    for codepoint in 0xac00..=0xd7a3 {
        assert!(face
            .glyph_index(char::from_u32(codepoint).unwrap())
            .is_some());
    }
}
