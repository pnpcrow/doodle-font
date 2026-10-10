use doodle_core::{
    appearance::Preferences,
    document,
    i18n::{self, Language},
};
use std::collections::BTreeSet;

fn placeholders(text: &str) -> BTreeSet<usize> {
    text.split('{')
        .skip(1)
        .filter_map(|s| s.split('}').next()?.parse().ok())
        .collect()
}

#[test]
fn translations_cover_source_keys_and_keep_all_placeholders() {
    let catalog = i18n::catalog();
    assert!(catalog.len() >= 550);
    for (key, translations) in catalog {
        for translation in translations {
            assert!(!translation.trim().is_empty(), "empty translation: {key}");
            assert_eq!(
                placeholders(key),
                placeholders(translation),
                "placeholder mismatch: {key}"
            );
        }
    }
    fn check_sources(path: &std::path::Path) {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                check_sources(&path);
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let source = std::fs::read_to_string(&path).unwrap();
            for call in ["tr(", "tf("] {
                for (_, suffix) in source
                    .match_indices(call)
                    .map(|(i, _)| (i, &source[i + call.len()..]))
                {
                    let suffix = suffix.trim_start();
                    if !suffix.starts_with('"') {
                        continue;
                    }
                    let mut escaped = false;
                    let end = suffix
                        .char_indices()
                        .skip(1)
                        .find_map(|(i, c)| {
                            if escaped {
                                escaped = false;
                                return None;
                            }
                            if c == '\\' {
                                escaped = true;
                                return None;
                            }
                            (c == '"').then_some(i)
                        })
                        .unwrap();
                    let key: String = serde_json::from_str(&suffix[..=end]).unwrap();
                    assert!(
                        i18n::catalog().contains_key(&key),
                        "missing translation in {}: {key}",
                        path.display()
                    );
                }
            }
        }
    }
    check_sources(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"));
    for (_, label, _) in document::GROUPS {
        assert!(catalog.contains_key(label));
    }
}

#[test]
fn formatting_preserves_user_text_even_when_it_looks_like_a_template() {
    let user_text = "설정 · 春 {0} {{1}}\n나의 글씨".to_string();
    for language in [Language::Korean, Language::English, Language::Japanese] {
        i18n::set_language(language);
        let result = i18n::format("폰트 패밀리 · {0}", std::slice::from_ref(&user_text));
        assert!(result.ends_with(&user_text));
        assert_eq!(i18n::format("{{{0}}}", &["{1}".into()]), "{{1}}");
        assert_eq!(
            i18n::text("user-created unknown key"),
            "user-created unknown key"
        );
    }
}

#[test]
fn all_language_preferences_round_trip_and_old_settings_follow_system() {
    let old: Preferences =
        serde_json::from_str(r#"{"theme":"dark","reduce_motion":true}"#).unwrap();
    assert_eq!(old.language, Language::System);
    for language in Language::ALL {
        let preferences = Preferences {
            language,
            ..old.clone()
        };
        let saved = serde_json::to_string(&preferences).unwrap();
        assert_eq!(
            serde_json::from_str::<Preferences>(&saved).unwrap(),
            preferences
        );
    }
    assert_ne!(Language::System.resolved(), Language::System);
}

#[test]
fn display_language_preserves_document_identity_and_korean_input_decisions() {
    use serde_json::json;
    let project = document::new_project();
    let original = serde_json::to_string(&project).unwrap();
    let glyph = &project.glyphs[0];
    let payload = json!({"glyph":glyph, "action":"generate", "prompt":"현재 그림의 획 굵기를 70으로 통일해줘", "colorMode":"color"});
    let decision = json!({"success":true,"confidence":0.95,"function_calls":[{"name":"set_stroke","arguments":{"width":70}}],"suppressed_calls":[],"validation":{"ungrounded":[],"negation":false}});
    let mut expected = None;
    for language in [Language::Korean, Language::English, Language::Japanese] {
        i18n::set_language(language);
        let result = doodle_core::ai_pipeline::plan(&payload, glyph, Some(&decision));
        assert!(
            result.candidate.is_some(),
            "Korean input must work under every display language"
        );
        if let Some(expected) = &expected {
            assert_eq!(expected, result.candidate.as_ref().unwrap());
        }
        expected = result.candidate;
        document::status(&glyph.status);
        assert_eq!(serde_json::to_string(&project).unwrap(), original);
    }
}

#[test]
fn localized_refine_shortcuts_preserve_drawing_position() {
    for language in [Language::Korean, Language::English, Language::Japanese] {
        i18n::set_language(language);
        let glyph = document::new_project().glyphs.remove(0);
        let mut artwork = glyph.artwork.clone();
        let before = serde_json::to_string(&artwork).unwrap();
        let prompt = i18n::text("현재 그림의 형태와 위치는 유지하고 획 굵기를 48로 통일해줘.");
        doodle_core::glyph_layout::place_api_candidate(&glyph, prompt, &mut artwork).unwrap();
        assert_eq!(serde_json::to_string(&artwork).unwrap(), before);
    }
}
