//! Real egui pointer input: drag, persistence, context menu, resize and modal locking.
use super::layout_qa::{click, key};
use super::*;
use doodle_core::companion::placement;

impl Studio {
    pub(super) fn companion_placement_qa_input(
        &mut self,
        ctx: &egui::Context,
        input: &mut egui::RawInput,
        view: &str,
    ) {
        let frame = ctx.cumulative_frame_nr();
        let reset = view.ends_with("reset");
        let restored = view.ends_with("restored-small");
        if frame == 2 {
            self.qa_dock.companion_baseline = self.qa_dock.companion;
            self.qa_dock.baseline = self.qa_canvas;
            self.qa_settings.companion_project =
                Some(serde_json::to_string(&self.project).unwrap());
            assert_eq!(
                self.preferences.companion_position.is_some(),
                reset || restored
            );
            if restored {
                let rect = self.qa_dock.companion.unwrap();
                let expected = placement::resolve(
                    placement::bounds(ctx.content_rect()),
                    rect.width(),
                    Pos2::ZERO,
                    self.preferences.companion_position,
                );
                assert!(
                    (rect.min - expected.min).length() < 2.,
                    "restore relative position in a smaller window"
                );
            }
        }
        if reset {
            if frame == 3 || frame == 4 {
                click(input, self.qa_dock.companion.unwrap(), frame == 3, true);
            }
            if frame == 7 || frame == 8 {
                click(
                    input,
                    self.qa_settings
                        .companion_reset
                        .expect("reset in context menu"),
                    frame == 7,
                    false,
                );
            }
            if frame == 12 {
                assert_eq!(self.preferences.companion_position, None);
                assert_eq!(
                    Preferences::load(&self.save_path.with_file_name("appearance.json"))
                        .companion_position,
                    None
                );
                assert!(
                    self.qa_dock
                        .companion
                        .unwrap()
                        .center()
                        .distance(self.qa_dock.companion_baseline.unwrap().center())
                        > 200.
                );
                assert!(
                    self.qa_settings.companion_menu.is_none(),
                    "reset closes the popup"
                );
            }
            if frame == 16 || frame == 17 {
                click(input, self.qa_dock.companion.unwrap(), frame == 16, false);
            }
            if frame == 22 {
                assert_eq!(
                    self.preferences.companion_position, None,
                    "a click is not a move"
                );
                assert_eq!(
                    self.companion.motion.mood,
                    doodle_core::companion::Mood::Done
                );
            }
        } else {
            if (3..=6).contains(&frame) {
                let start = self.qa_dock.companion_baseline.unwrap().center();
                let pos = if frame == 3 {
                    start
                } else if restored {
                    Pos2::new(-100., -100.)
                } else {
                    start + Vec2::new(if frame == 4 { -240. } else { -320. }, -220.)
                };
                input.events.push(egui::Event::PointerMoved(pos));
                if frame == 3 || frame == 6 {
                    click(
                        input,
                        Rect::from_center_size(pos, Vec2::ZERO),
                        frame == 3,
                        false,
                    );
                }
            }
            if frame == 10 {
                let rect = self.qa_dock.companion.unwrap();
                let expected = if restored {
                    placement::bounds(ctx.content_rect()).min
                } else {
                    self.qa_dock.companion_baseline.unwrap().min + Vec2::new(-320., -220.)
                };
                assert!(
                    (rect.min - expected).length() < 2.,
                    "drag follows pointer without jumping: {rect:?} / {expected:?}"
                );
                assert!(
                    self.preferences.companion_position.is_some(),
                    "drag records placement"
                );
                assert_eq!(
                    Preferences::load(&self.save_path.with_file_name("appearance.json"))
                        .companion_position,
                    self.preferences.companion_position
                );
                self.qa_dock.companion_baseline = Some(rect);
            }
            if frame == 11 || frame == 12 {
                click(input, self.qa_dock.companion.unwrap(), frame == 11, true);
            }
            if frame == 14 {
                assert!(
                    self.qa_settings.companion_menu.is_some(),
                    "right click still opens options"
                );
                assert_eq!(self.qa_dock.companion, self.qa_dock.companion_baseline);
            }
            if frame == 15 {
                key(input, egui::Key::Escape, false);
            }
            if frame == 18 || frame == 19 {
                click(
                    input,
                    self.qa_dock.restore.expect("collapsed prompt can reopen"),
                    frame == 18,
                    false,
                );
            }
            if frame == 24 {
                assert!(!self.ai_dock_collapsed);
                assert_eq!(
                    self.qa_dock.companion, self.qa_dock.companion_baseline,
                    "prompt expansion preserves placement"
                );
                assert_eq!(self.qa_canvas, self.qa_dock.baseline);
                self.ai_cancel = Some(std::sync::Arc::new(std::sync::atomic::AtomicBool::new(
                    false,
                )));
                self.ai_status = "이동 잠금 검증 중".into();
            }
            if (26..=28).contains(&frame) {
                let pos = self.qa_dock.companion_baseline.unwrap().center()
                    + if frame == 26 {
                        Vec2::ZERO
                    } else {
                        Vec2::splat(70.)
                    };
                input.events.push(egui::Event::PointerMoved(pos));
                if frame != 27 {
                    click(
                        input,
                        Rect::from_center_size(pos, Vec2::ZERO),
                        frame == 26,
                        false,
                    );
                }
            }
            if frame == 31 {
                let actual = self.qa_dock.companion.unwrap();
                let expected = self.qa_dock.companion_baseline.unwrap();
                assert!(
                    (actual.min - expected.min).length() < 2.,
                    "busy modal locks companion movement"
                );
                self.ai_cancel = None;
            }
            if frame == 34 {
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(1120., 760.)));
            }
        }
        if frame == 42 {
            assert!(placement::bounds(ctx.content_rect())
                .expand(1.)
                .contains_rect(self.qa_dock.companion.unwrap()));
            assert_eq!(
                serde_json::to_string(&self.project).unwrap(),
                *self.qa_settings.companion_project.as_ref().unwrap(),
                "drag must not edit the canvas underneath"
            );
        }
    }
}
