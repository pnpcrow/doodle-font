use doodle_core::companion::{Companion, Mood, Motion, Pose, Renderer};
use eframe::egui::{self, Pos2, Rect, Vec2};

const MOODS: [Mood; 8] = [
    Mood::Idle,
    Mood::Thinking,
    Mood::Inspecting,
    Mood::Drawing,
    Mood::Coloring,
    Mood::Explaining,
    Mood::Done,
    Mood::NeedsHelp,
];
fn distance(a: Pose, b: Pose) -> f32 {
    (a.look - b.look).length()
        + (a.head - b.head).length()
        + (a.scale - b.scale).length()
        + (a.tilt - b.tilt).abs()
        + (a.lift - b.lift).abs()
        + (a.round - b.round).abs()
        + (a.square - b.square).abs()
        + a.tint
            .into_iter()
            .zip(b.tint)
            .map(|(a, b)| (a - b).abs())
            .sum::<f32>()
}

#[test]
fn every_clip_transition_preserves_pose_and_velocity_including_interruptions() {
    for from in MOODS {
        for to in MOODS {
            let mut m = Motion::default();
            m.react(from);
            m.step(0., Vec2::ZERO, false);
            for i in 1..=90 {
                m.step(i as f64 / 60., Vec2::new(0.5, -0.3), false);
            }
            let before = m.pose;
            m.react(to);
            assert_eq!(
                m.step(1.5, Vec2::new(0.5, -0.3), false),
                before,
                "jump {from:?}->{to:?}"
            );
            let a = m.step(1.5001, Vec2::new(0.5, -0.3), false);
            let b = m.step(1.5002, Vec2::new(0.5, -0.3), false);
            assert!(
                distance(before, a) < 0.001,
                "discontinuous pose {from:?}->{to:?}"
            );
            assert!(
                ((a.tilt - before.tilt) - (b.tilt - a.tilt)).abs() < 0.00001,
                "velocity reset"
            );
            assert!(distance(a, b) < 0.001);
            for i in 1..=90 {
                if i % 9 == 0 {
                    m.react(MOODS[i / 9 % 8]);
                }
                let p = m.step(1.5002 + i as f64 / 60., Vec2::splat(-1.), false);
                assert!(p.look.is_finite() && p.scale.is_finite());
                assert!(p.scale.x > 0.7 && p.scale.y > 0.7 && p.scale.x < 1.3 && p.scale.y < 1.3);
                assert!(p.round >= -0.01 && p.square >= -0.01 && p.round + p.square < 1.15);
                assert!(p.lift.abs() < 0.15);
            }
        }
    }
}

#[test]
fn all_clips_agree_at_30_60_120_hz_and_idle_uses_multiple_shapes() {
    for mood in MOODS {
        let run = |fps| {
            let mut m = Motion::default();
            m.react(mood);
            m.step(0., Vec2::ZERO, false);
            for frame in 1..=fps * 3 {
                m.step(frame as f64 / fps as f64, Vec2::new(0.7, -0.4), false);
            }
            m.pose
        };
        let a = run(30);
        let b = run(60);
        let c = run(120);
        assert!(
            distance(a, b) < 0.02 && distance(b, c) < 0.02,
            "frame dependence {mood:?}"
        );
    }
    let mut m = Motion::default();
    m.step(0., Vec2::ZERO, false);
    let mut max_round = 0_f32;
    let mut max_square = 0_f32;
    let mut max_height = 0_f32;
    for i in 1..=60 * 24 {
        let p = m.step(i as f64 / 60., Vec2::ZERO, false);
        max_round = max_round.max(p.round);
        max_square = max_square.max(p.square);
        max_height = max_height.max(p.scale.y);
    }
    assert!(max_round > 0.7 && max_square > 0.6 && max_height > 1.1);
}

#[test]
fn eyes_lead_head_and_pointer_reversal_is_continuous_in_both_axes() {
    let mut m = Motion::default();
    m.step(0., Vec2::ZERO, false);
    for i in 1..=12 {
        m.step(i as f64 / 60., Vec2::splat(1.), false);
    }
    assert!(m.pose.look.x > m.pose.head.x && m.pose.look.y > m.pose.head.y);
    for i in 13..=90 {
        m.step(i as f64 / 60., Vec2::splat(1.), false);
    }
    assert!(m.pose.head.x > 0.7 && m.pose.head.y > 0.7);
    let before = m.pose;
    let after = m.step(1.5, Vec2::splat(-1.), false);
    assert_eq!(before, after);
    for i in 91..=180 {
        m.step(i as f64 / 60., Vec2::splat(-1.), false);
    }
    assert!(m.pose.look.x < -0.9 && m.pose.look.y < -0.9);
    assert!(m.pose.head.x < -0.7 && m.pose.head.y < -0.7);
}

#[test]
fn reduced_motion_keeps_state_identity_without_time_varying_animation() {
    for mood in MOODS {
        let mut m = Motion::default();
        m.react(mood);
        let a = m.step(0., Vec2::splat(1.), true);
        let b = m.step(0.5, Vec2::splat(-1.), true);
        assert_eq!(a, b, "reduced motion animates {mood:?}");
        assert_eq!(a.look, Vec2::ZERO);
        assert_eq!(a.head, Vec2::ZERO);
    }
    let mut m = Motion::default();
    m.step(0., Vec2::ZERO, false);
    let p = m.step(3600., Vec2::ZERO, false);
    assert!(
        distance(Pose::default(), p) < 0.3,
        "catch-up jump after hidden window"
    );
}

#[test]
fn morphed_mesh_and_face_stay_inside_the_companion_slot() {
    let ctx = egui::Context::default();
    let mut pet = Companion::with_renderer(&ctx, Renderer::Image);
    let sprite = image::load_from_memory(include_bytes!("../assets/companion/body.png"))
        .unwrap()
        .to_rgba8();
    let rect = Rect::from_min_size(Pos2::new(100., 100.), Vec2::splat(64.));
    for mood in MOODS {
        pet.motion = Motion::default();
        pet.motion.react(mood);
        pet.motion.step(0., Vec2::ZERO, false);
        for frame in 1..=240 {
            let pose = pet.motion.step(
                frame as f64 / 60.,
                Vec2::new((frame as f32 * 0.1).sin(), -1.),
                false,
            );
            let out = ctx.run(egui::RawInput::default(), |ctx| {
                pet.paint(&ctx.debug_painter(), rect, pose)
            });
            for shape in out.shapes {
                if let egui::Shape::Path(path) = &shape.shape {
                    assert!(
                        path.points.iter().all(|p| rect.contains(*p)),
                        "eye escaped {mood:?}"
                    );
                }
                if let egui::Shape::Mesh(mesh) = shape.shape {
                    assert!(mesh.vertices.iter().all(|v| v.pos.is_finite()));
                    // Include the curl, not just the pale body; only transparent margins may escape.
                    for v in &mesh.vertices {
                        let x = (v.uv.x * (sprite.width() - 1) as f32) as u32;
                        let y = (v.uv.y * (sprite.height() - 1) as f32) as u32;
                        if sprite.get_pixel(x, y)[3] > 100 {
                            assert!(
                                rect.contains(v.pos),
                                "visible body escaped {mood:?}: {:?}",
                                v.pos
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn renderer_switch_preserves_motion_and_vector_has_no_custom_texture() {
    let ctx = egui::Context::default();
    let mut pet = Companion::with_renderer(&ctx, Renderer::Image);
    let mut reference = Motion::default();
    pet.motion.react(Mood::Drawing);
    reference.react(Mood::Drawing);
    for i in 0..90 {
        let time = i as f64 / 60.;
        assert_eq!(
            pet.motion.step(time, Vec2::splat(0.5), false),
            reference.step(time, Vec2::splat(0.5), false)
        );
    }
    assert_eq!(pet.footprint().custom_texture_bytes, 2 * 384 * 384 * 4);
    for renderer in [Renderer::Vector, Renderer::Image, Renderer::Vector] {
        let before = pet.motion.pose;
        pet.set_renderer(&ctx, renderer);
        assert_eq!(pet.motion.pose, before);
        assert_eq!(pet.renderer(), renderer);
    }
    assert_eq!(pet.footprint().custom_texture_bytes, 0);
    for i in 90..180 {
        let time = i as f64 / 60.;
        assert_eq!(
            pet.motion.step(time, Vec2::splat(-0.5), false),
            reference.step(time, Vec2::splat(-0.5), false)
        );
    }
    let vector_ctx = egui::Context::default();
    let vector = Companion::with_renderer(&vector_ctx, Renderer::Vector);
    let output = vector_ctx.run(egui::RawInput::default(), |ctx| {
        vector.paint(
            &ctx.debug_painter(),
            Rect::from_min_size(Pos2::ZERO, Vec2::splat(192.)),
            Pose::default(),
        )
    });
    // Managed(0) is egui's shared white/font atlas, not a character texture.
    assert!(output
        .textures_delta
        .set
        .iter()
        .all(|(id, _)| *id == egui::TextureId::Managed(0)));
}

#[test]
fn vector_rendering_is_finite_bounded_and_valid_at_each_detail_level() {
    let ctx = egui::Context::default();
    let mut pet = Companion::with_renderer(&ctx, Renderer::Vector);
    for dpi in [1., 2.] {
        ctx.set_pixels_per_point(dpi);
        for size in [64., 192., 512.] {
            let rect = Rect::from_min_size(Pos2::new(50., 50.), Vec2::splat(size));
            pet.motion = Motion::default();
            for frame in 0..960 {
                pet.motion.react(MOODS[frame / 120]);
                let pose = pet.motion.step(
                    frame as f64 / 60.,
                    Vec2::new((frame as f32 * 0.1).sin(), -1.),
                    false,
                );
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(rect.expand(100.)),
                        ..Default::default()
                    },
                    |ctx| pet.paint(&ctx.debug_painter(), rect, pose),
                );
                for p in ctx.tessellate(output.shapes, output.pixels_per_point) {
                    if let egui::epaint::Primitive::Mesh(mesh) = p.primitive {
                        assert!(mesh.is_valid());
                        for v in mesh.vertices {
                            assert!(
                                v.pos.is_finite() && rect.contains(v.pos),
                                "escaped {size}/{dpi}: {:?}",
                                v.pos
                            );
                        }
                    }
                }
            }
        }
    }
}
