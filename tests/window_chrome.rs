use doodle_core::{
    appearance::{self, Palette, Theme},
    window_chrome::{self, ChromeRects},
};
use eframe::egui::{self, Pos2, Rect, Vec2, ViewportCommand};

struct WindowTest {
    ctx: egui::Context,
    time: f64,
    maximized: bool,
    modal: bool,
}
impl WindowTest {
    fn new(maximized: bool, modal: bool) -> Self {
        let ctx = egui::Context::default();
        appearance::install(&ctx, Theme::Light);
        let mut s = Self {
            ctx,
            time: 0.,
            maximized,
            modal,
        };
        for _ in 0..3 {
            s.frame(vec![]);
        }
        s
    }
    fn frame(&mut self, events: Vec<egui::Event>) -> (ChromeRects, Vec<ViewportCommand>) {
        self.time += 0.05;
        let mut input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1120., 760.))),
            time: Some(self.time),
            events,
            ..Default::default()
        };
        input
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .maximized = Some(self.maximized);
        let mut chrome = None;
        let output = self.ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.label("Editor");
            });
            if self.modal {
                egui::Modal::new("busy-test".into()).show(ctx, |ui| {
                    ui.label("Working; document input is locked");
                });
            }
            chrome = Some(window_chrome::show(
                ctx,
                &"Long project title ".repeat(30),
                &"Font family ".repeat(30),
                Palette::new(false),
            ));
        });
        (
            chrome.unwrap(),
            output.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .clone(),
        )
    }
    fn pointer(&mut self, pos: Pos2, pressed: bool) -> Vec<ViewportCommand> {
        self.frame(vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            },
        ])
        .1
    }
    fn click(&mut self, pos: Pos2) -> Vec<ViewportCommand> {
        let mut result = self.pointer(pos, true);
        result.extend(self.pointer(pos, false));
        result
    }
}

#[test]
fn caption_controls_work_above_busy_modal_and_do_not_drag_or_overlap_project() {
    for modal in [false, true] {
        for maximized in [false, true] {
            for (index, expected) in [
                ViewportCommand::Minimized(true),
                ViewportCommand::Maximized(!maximized),
                ViewportCommand::Close,
            ]
            .into_iter()
            .enumerate()
            {
                let mut window = WindowTest::new(maximized, modal);
                let rects = window.frame(vec![]).0;
                assert!(rects.project.right() < rects.controls[0].left());
                assert!(rects.title.contains_rect(rects.project));
                assert!(rects
                    .controls
                    .iter()
                    .all(|r| rects.title.contains_rect(*r) && r.height() >= 40.));
                assert_eq!(window.click(rects.controls[index].center()), [expected]);
            }
        }
    }
}

#[test]
fn caption_drags_over_project_text_and_double_clicks_toggle_maximization() {
    for maximized in [false, true] {
        let mut window = WindowTest::new(maximized, false);
        let pos = window.frame(vec![]).0.project.center();
        assert!(window.click(pos).is_empty());
        assert_eq!(window.click(pos), [ViewportCommand::Maximized(!maximized)]);
        let mut window = WindowTest::new(maximized, true);
        let pos = window.frame(vec![]).0.project.center();
        assert!(window.pointer(pos, true).is_empty());
        let commands = window
            .frame(vec![egui::Event::PointerMoved(pos + Vec2::new(20., 5.))])
            .1;
        assert_eq!(commands, [ViewportCommand::StartDrag]);
    }
}

#[test]
fn all_resize_edges_request_native_resize_and_disappear_when_maximized() {
    let mut window = WindowTest::new(false, false);
    let rects = window.frame(vec![]).0;
    assert_eq!(rects.resize.len(), 8);
    for (rect, direction) in rects.resize {
        let mut window = WindowTest::new(false, true);
        assert_eq!(
            window.pointer(rect.center(), true),
            [ViewportCommand::BeginResize(direction)]
        );
    }
    assert!(WindowTest::new(true, false)
        .frame(vec![])
        .0
        .resize
        .is_empty());
}
