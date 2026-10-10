use doodle_core::{
    appearance::{self, Preferences, Theme},
    companion::{Mood, Motion, Renderer},
};
use eframe::egui::{self, Pos2, Vec2};

#[test]
fn theme_preferences_survive_restart_and_system_mode_tracks_os_changes() {
    let root = std::env::temp_dir().join(format!("doodle-appearance-{}", uuid::Uuid::new_v4()));
    let path = root.join("appearance.json");
    for theme in [Theme::System, Theme::Light, Theme::Dark] {
        let prefs = Preferences {
            theme,
            reduce_motion: true,
            companion_renderer: Renderer::Vector,
        };
        prefs.save(&path).unwrap();
        assert_eq!(Preferences::load(&path), prefs);
        let ctx = egui::Context::default();
        appearance::install(&ctx, theme);
        for os_theme in [egui::Theme::Light, egui::Theme::Dark, egui::Theme::Light] {
            let _ = ctx.run(
                egui::RawInput {
                    system_theme: Some(os_theme),
                    ..Default::default()
                },
                |_| {},
            );
            assert_eq!(
                ctx.theme(),
                match theme {
                    Theme::System => os_theme,
                    Theme::Light => egui::Theme::Light,
                    Theme::Dark => egui::Theme::Dark,
                }
            );
        }
    }
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(root).unwrap();
}

#[test]
fn older_preferences_adopt_vector_without_losing_theme_or_explicit_renderer_choice() {
    let old: Preferences =
        serde_json::from_str(r#"{"theme":"dark","reduce_motion":true}"#).unwrap();
    assert_eq!(old.theme, Theme::Dark);
    assert!(old.reduce_motion);
    assert_eq!(old.companion_renderer, Renderer::Vector);
    let explicit: Preferences = serde_json::from_str(
        r#"{"theme":"dark","reduce_motion":true,"companion_renderer":"image"}"#,
    )
    .unwrap();
    assert_eq!(explicit.companion_renderer, Renderer::Image);
}

#[test]
fn button_painted_bounds_and_hit_area_are_stable_on_hover_focus_press_and_selection() {
    for theme in [Theme::Light, Theme::Dark] {
        let ctx = egui::Context::default();
        appearance::install(&ctx, theme);
        let mut button = None;
        let mut previous = None;
        for state in 0..5 {
            let pos = previous.map_or(Pos2::new(-100., -100.), |r: egui::Rect| r.center());
            let mut events = vec![egui::Event::PointerMoved(if state == 0 {
                Pos2::new(-100., -100.)
            } else {
                pos
            })];
            if state == 3 {
                events.push(egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                });
            }
            if state == 4 {
                events.push(egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: Default::default(),
                });
            }
            if state == 2 {
                ctx.memory_mut(|m| m.request_focus(button.unwrap()));
            }
            let output = ctx.run(
                egui::RawInput {
                    events,
                    screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, Vec2::new(400., 200.))),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let r = ui.add(egui::Button::new("Stable button").selected(state == 4));
                        button = Some(r.id);
                        if let Some(previous) = previous {
                            assert_eq!(r.rect, previous, "hit target moved in state {state}");
                        }
                        previous = Some(r.rect);
                    });
                },
            );
            let hit = previous.unwrap();
            let painted = output
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::Shape::Rect(r) if r.rect.center() == hit.center() => Some(r.rect),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert!(!painted.is_empty());
            assert!(
                painted.iter().all(|r| *r == hit),
                "button visually expands in state {state}"
            );
        }
    }
}

#[test]
fn companion_motion_is_bounded_frame_independent_and_reduced_motion_is_still() {
    let run = |fps: u32| {
        let mut m = Motion::default();
        m.step(0., Vec2::ZERO, false);
        m.react(Mood::Thinking);
        for frame in 1..=fps * 2 {
            m.step(frame as f64 / fps as f64, Vec2::new(1., -1.), false);
        }
        m.pose
    };
    let a = run(30);
    let b = run(60);
    let c = run(120);
    assert!((a.look - b.look).length() < 0.03 && (c.look - b.look).length() < 0.03);
    assert!((a.scale - b.scale).length() < 0.003);
    let mut m = Motion::default();
    let mut min_eye = 1_f32;
    for i in 0..600 {
        if i == 60 {
            m.react(Mood::Done);
        }
        let p = m.step(i as f64 / 60., Vec2::splat(50.), false);
        assert!(p.look.x <= 1. && p.look.y <= 1. && p.lift.abs() < 0.15);
        min_eye = min_eye.min(p.eyes);
    }
    assert!(min_eye < 0.1, "blink never closes");
    assert_eq!(m.mood, Mood::Idle);
    for t in [10., 11., 100.] {
        let p = m.step(t, Vec2::splat(1.), true);
        assert_eq!(p.look, Vec2::ZERO);
        assert_eq!(p.lift, 0.);
        assert_eq!(p.scale, Vec2::splat(1.));
        assert_eq!(p.eyes, 1.);
    }
}
