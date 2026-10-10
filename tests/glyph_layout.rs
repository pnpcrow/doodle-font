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
fn families_share_left_bearing_cap_line_and_keep_lowercase_descenders() {
    for family in ["sans", "serif", "handwritten"] {
        for text in ["A", "H", "I", "a", "x", "n", "g", "j", "p", "q", "y"] {
            let art = generated(text, family, 100, 1000.);
            let b = bounds(&art);
            assert!((b[0] - 80.).abs() < 0.01, "{family}/{text}: {b:?}");
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
    for text in ["H", "a", "g", "j", "p", "q", "y", "fi", "가"] {
        let normal = generated(text, "sans", 100, 640.);
        let narrow = generated(text, "sans", 75, 640.);
        let wide = generated(text, "sans", 125, 640.);
        for art in [&normal, &narrow, &wide] {
            let b = bounds(art);
            assert_eq!(art.advance, 640.);
            assert!((b[0] - 80.).abs() < 0.01 && b[2] <= 560.01);
            assert_eq!(b[1], bounds(&normal)[1]);
            assert_eq!(b[3], bounds(&normal)[3]);
        }
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
        assert!((bounds(&art)[0] - 80.).abs() < 0.01);
        assert!(bounds(&art)[3] > layout::BASELINE);
        outputs.push(art.shapes[0].points.clone());
    }
    assert_eq!(outputs[0], outputs[1]);
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
    assert_eq!(
        image.get_pixel((80. * 0.512_f64).round() as u32, 10).0,
        [161, 181, 156, 255]
    );
}
