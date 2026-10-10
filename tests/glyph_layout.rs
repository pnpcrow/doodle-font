use doodle_core::{
    document::*,
    glyph_layout as layout,
    model::*,
    typography::{self, Style},
};

fn generated(text: &str, family: &str, width: u16, advance: f64) -> Artwork {
    let mut glyph = new_glyph(text, "default", "latin", false);
    glyph.artwork.advance = advance;
    typography::generate(
        &glyph,
        &Style {
            family: family.into(),
            width,
            ..Default::default()
        },
        true,
    )
    .unwrap_or_else(|e| panic!("{text}/{family}: {e}"))
    .artwork
}
fn bounds(art: &Artwork) -> [f64; 4] {
    layout::ink_bounds(art).unwrap().unwrap()
}

#[test]
fn families_balance_side_margins_and_keep_cap_height_and_descenders() {
    for family in ["sans", "serif", "handwritten"] {
        for text in ["A", "H", "I", "a", "x", "n", "g", "j", "p", "q", "y"] {
            let art = generated(text, family, 100, 1000.);
            let b = bounds(&art);
            assert!((b[0] + b[2] - 1000.).abs() < 0.01, "{family}/{text}: {b:?}");
            assert!(b[2] <= 920.01 && b[1] >= 0. && b[3] <= 960.01);
            if ["A", "H", "I"].contains(&text) {
                assert!((b[1] - 200.).abs() < 35., "cap line {family}/{text}: {b:?}");
            }
            if ["a", "x", "n"].contains(&text) {
                assert!(b[1] > 280., "lowercase enlarged: {family}/{text}: {b:?}");
                // Handwriting retains its small natural baseline overshoot.
                assert!(b[3] <= 840., "unexpected descent: {family}/{text}: {b:?}");
            }
            if ["g", "j", "p", "q", "y"].contains(&text) {
                assert!(b[3] > 840., "descender lost: {family}/{text}: {b:?}");
            }
        }
        let i = bounds(&generated("I", family, 100, 1000.));
        let m = bounds(&generated("M", family, 100, 1000.));
        assert!(
            i[2] - i[0] < m[2] - m[0],
            "narrow glyph individually enlarged"
        );
    }
}

#[test]
fn width_changes_do_not_move_baseline_or_tail_and_advance_is_preserved() {
    for text in ["H", "a", "g", "j", "p", "q", "y", "fi"] {
        let normal = generated(text, "sans", 100, 1000.);
        let narrow = generated(text, "sans", 75, 1000.);
        let wide = generated(text, "sans", 125, 1000.);
        for art in [&normal, &narrow, &wide] {
            let b = bounds(art);
            assert_eq!(art.advance, 1000.);
            assert!((b[0] + b[2] - 1000.).abs() < 0.01);
            assert_eq!(b[1], bounds(&normal)[1]);
            assert_eq!(b[3], bounds(&normal)[3]);
        }
        let width = |art: &Artwork| bounds(art)[2] - bounds(art)[0];
        assert!((width(&wide) / width(&normal) - 1.25).abs() < 0.001);
        assert!((width(&narrow) / width(&normal) - 0.75).abs() < 0.001);
    }
}

#[test]
fn changing_advance_only_translates_and_overflow_is_not_squeezed() {
    for text in ["A", "I", "g", "가", "。"] {
        let large = generated(text, "sans", 100, 1000.);
        let b = bounds(&large);
        let advance = b[2] - b[0] + 170.;
        let small = generated(text, "sans", 100, advance);
        let a = bounds(&small);
        assert!((a[2] - a[0] - (b[2] - b[0])).abs() < 0.001);
        assert_eq!([a[1], a[3]], [b[1], b[3]]);
        assert!((a[0] + a[2] - advance).abs() < 0.001);
        assert_eq!(small.advance, advance);

        let mut glyph = new_glyph(text, "default", "test", false);
        glyph.artwork.advance = 50.;
        let result = typography::generate(&glyph, &Style::default(), true);
        assert!(result.err().unwrap().contains("자연스러운 비율"));
    }
}

#[test]
fn api_placement_is_consistent_and_never_lifts_descenders() {
    let mut glyph = new_glyph("g", "default", "latin", false);
    glyph.artwork.advance = 700.;
    let mut outputs = vec![];
    for dx in [0., 230.] {
        let mut s = path(&[[100. + dx, 400.], [300. + dx, 800.], [220. + dx, 930.]]);
        s.stroke = 30.;
        let mut art = Artwork {
            advance: 1000.,
            shapes: vec![s],
        };
        let ys: Vec<_> = art.shapes[0].points.iter().map(|p| p[1]).collect();
        layout::place_api_candidate(
            &glyph,
            "g를 고딕체로 그려줘. 아래로 내려오는 획도 살려줘.",
            &mut art,
        )
        .unwrap();
        assert_eq!(art.advance, 700.);
        assert_eq!(
            art.shapes[0]
                .points
                .iter()
                .map(|p| p[1])
                .collect::<Vec<_>>(),
            ys
        );
        assert!((bounds(&art)[0] + bounds(&art)[2] - 700.).abs() < 0.01);
        assert!(bounds(&art)[3] > layout::BASELINE);
        outputs.push(art.shapes[0].points.clone());
    }
    assert_eq!(outputs[0], outputs[1]);
}

#[test]
fn api_balance_moves_all_layers_together_without_resizing_ink_or_strokes() {
    let mut glyph = new_glyph("A", "default", "latin", false);
    glyph.artwork.advance = 800.;
    let mut ellipse = new_shape("ellipse");
    ellipse.x = 100.;
    ellipse.y = 220.;
    ellipse.width = 60.;
    ellipse.height = 80.;
    ellipse.fill = true;
    let mut rect = new_shape("rect");
    rect.x = 200.;
    rect.y = 650.;
    rect.width = 80.;
    rect.height = 100.;
    let mut art = Artwork {
        advance: 1000.,
        shapes: vec![ellipse, rect, path(&[[100., 800.], [380., 210.]])],
    };
    let before = art.clone();
    let b = bounds(&before);
    layout::place_api_candidate(&glyph, "새 A를 그려줘", &mut art).unwrap();
    let a = bounds(&art);
    let dx = a[0] - b[0];
    assert!((a[0] + a[2] - 800.).abs() < 0.001);
    assert!((a[2] - a[0] - (b[2] - b[0])).abs() < 0.001);
    assert_eq!([a[1], a[3]], [b[1], b[3]]);
    for (old, new) in before.shapes.iter().zip(&art.shapes) {
        assert_eq!(
            (old.width, old.height, old.stroke),
            (new.width, new.height, new.stroke)
        );
        if old.kind == "path" {
            for (p, q) in old.points.iter().zip(&new.points) {
                assert!((q[0] - p[0] - dx).abs() < 0.001);
                assert_eq!(p[1], q[1]);
            }
        } else {
            assert!((new.x - old.x - dx).abs() < 0.001);
            assert_eq!(old.y, new.y);
        }
    }
    // An overflowing result is reported, not silently compressed into the cell.
    glyph.artwork.advance = 100.;
    let before_rejection = serde_json::to_value(&art).unwrap();
    assert!(layout::place_api_candidate(&glyph, "새 A를 그려줘", &mut art).is_err());
    assert_eq!(serde_json::to_value(&art).unwrap(), before_rejection);
}

#[test]
fn prompt_gives_character_specific_proportions_instead_of_a_fill_target() {
    let context = |text| layout::context(&new_glyph(text, "default", "test", false));
    let a = context("A");
    let i = context("I");
    let m = context("M");
    let g = context("g");
    let dot = context(".");
    assert!(
        i["naturalReference"]["inkWidth"].as_f64().unwrap()
            < m["naturalReference"]["inkWidth"].as_f64().unwrap() * 0.6
    );
    assert!(dot["naturalReference"]["inkHeight"].as_f64().unwrap() < 200.);
    assert!((a["naturalReference"]["topY"].as_f64().unwrap() - 200.).abs() < 5.);
    assert!(g["naturalReference"]["bottomY"].as_f64().unwrap() > 840.);
    assert_eq!(a["spacing"]["limitsAreNotTargets"], true);
    assert_eq!(a["spacing"]["centerX"].as_f64(), Some(500.));
    assert!(a["naturalReference"]["centeredLeftX"].as_f64().unwrap() > 150.);

    let mut narrow = new_glyph("M", "default", "test", false);
    narrow.artwork.advance = 200.;
    let narrow = layout::context(&narrow);
    assert_eq!(narrow["naturalReference"]["fitsAdvance"], false);
    assert_eq!(
        narrow["naturalReference"]["inkWidth"],
        m["naturalReference"]["inkWidth"]
    );
    assert!(context("\u{10ffff}")["naturalReference"].is_null());
}

#[test]
fn api_rejects_missing_tails_but_explicit_position_preservation_takes_precedence() {
    let glyph = new_glyph("g", "default", "latin", false);
    let mut art = Artwork {
        advance: 900.,
        shapes: vec![path(&[[300., 400.], [500., 700.]])],
    };
    assert!(layout::place_api_candidate(&glyph, "g를 그려줘", &mut art)
        .unwrap_err()
        .contains("하강"));
    let before = serde_json::to_value(&art).unwrap();
    for prompt in [
        "현재 그림의 형태와 위치는 그대로 유지해 줘",
        "Keep the current drawing position",
        "가운데 정렬해줘",
    ] {
        layout::place_api_candidate(&glyph, prompt, &mut art).unwrap();
        assert_eq!(serde_json::to_value(&art).unwrap(), before);
    }
}

#[test]
fn shared_guides_are_present_in_the_actual_ai_image_and_context() {
    use base64::Engine;
    let mut glyph = new_glyph("gjpqy", "default", "latin", true);
    glyph.artwork.advance = 700.;
    let context = layout::context(&glyph);
    assert_eq!(context["canvas"]["width"].as_f64(), Some(1000.));
    assert_eq!(
        context["referenceImage"]["pixelsPerUnit"].as_f64(),
        Some(0.512)
    );
    assert_eq!(context["spacing"]["advance"].as_f64(), Some(700.));
    assert_eq!(
        context["descenders"]["present"].as_array().unwrap().len(),
        5
    );
    let url = doodle_core::ai::canvas_image(&glyph.artwork).unwrap();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(url.split_once(',').unwrap().1)
        .unwrap();
    let image = image::load_from_memory(&bytes).unwrap().to_rgba8();
    for (y, _, [r, g, b]) in layout::guides() {
        let pixel = (y * 0.512).round() as u32;
        assert_eq!(image.get_pixel(10, pixel).0, [r, g, b, 255]);
    }
    for x in [80., 620., 700.] {
        assert_eq!(
            image.get_pixel((x * 0.512_f64).round() as u32, 10).0,
            [161, 181, 156, 255]
        );
    }
}
