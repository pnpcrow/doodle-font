//! Real dropdown input, process restart and unchanged document checks in native egui.
use super::*;

fn language(view: &str) -> Language {
    match view.split('-').nth(1).unwrap() {
        "en" => Language::English,
        "ja" => Language::Japanese,
        "ko" => Language::Korean,
        "system" => Language::System,
        _ => panic!("Unknown language case"),
    }
}
impl Studio {
    pub(super) fn language_qa_setup(&mut self, ctx: &egui::Context, view: &str) {
        if !view.starts_with("language-") {
            return;
        }
        let target = language(view);
        if view.ends_with("restored") {
            assert_eq!(
                self.preferences.language, target,
                "language survives restart"
            );
        } else {
            self.preferences.language = if view.ends_with("write") {
                if target == Language::Korean {
                    Language::English
                } else {
                    Language::Korean
                }
            } else {
                target
            };
        }
        i18n::set_language(self.preferences.language);
        self.preferences.theme = if target == Language::Japanese {
            Theme::Dark
        } else {
            Theme::Light
        };
        ctx.set_theme(self.preferences.theme.preference());
        self.api_provider = "off".into();
        self.ai_dock_collapsed = true;
        self.project.name = "봄날의 글씨 · 春の文字 · {0}".into();
        self.project.family = "My Font Family {1}".into();
        self.qa_settings.language_project = Some(serde_json::to_string(&self.project).unwrap());
        self.tab = if view.contains("editor") || view.contains("export") {
            0
        } else if view.contains("templates") {
            2
        } else {
            3
        };
        self.show_properties = view.contains("editor");
        self.export_open = view.contains("export");
        if view.contains("api") {
            self.select_ai_provider("openai");
        }
        if view.ends_with("small") {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(1120., 760.)));
        }
    }
    pub(super) fn language_qa_input(
        &mut self,
        ctx: &egui::Context,
        input: &mut egui::RawInput,
        view: &str,
    ) -> bool {
        if !view.starts_with("language-") {
            return false;
        }
        input
            .events
            .retain(|e| matches!(e, egui::Event::Screenshot { .. }));
        let frame = ctx.cumulative_frame_nr();
        input.time = Some(frame as f64 / 30.);
        if view.ends_with("write") {
            let rect = match frame {
                3 | 4 => self.qa_settings.language_menu,
                7 | 8 => {
                    self.qa_settings.languages[Language::ALL
                        .iter()
                        .position(|l| *l == language(view))
                        .unwrap()]
                }
                _ => None,
            };
            if let Some(rect) = rect {
                layout_qa::click(input, rect, frame == 3 || frame == 7, false);
            }
        }
        if view.contains("api") && (frame == 3 || frame == 13) {
            let pos = Pos2::new(
                ctx.content_rect().center().x,
                ctx.content_rect().bottom() - 100.,
            );
            input.events.push(egui::Event::PointerMoved(pos));
            input.events.push(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: Vec2::new(0., -650.),
                modifiers: Default::default(),
            });
        }
        if view.contains("api") && (frame == 9 || frame == 10) {
            assert!(dialog_bounds(ctx).contains(
                self.qa_connection_header
                    .expect("connection control")
                    .center()
            ));
            layout_qa::click(
                input,
                self.qa_connection_header.expect("connection control"),
                frame == 9,
                false,
            );
        }
        if view.contains("export") && frame == 13 {
            input.events.push(egui::Event::PointerMoved(
                self.qa_settings.export_window.unwrap().center(),
            ));
            input.events.push(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: Vec2::new(0., -350.),
                modifiers: Default::default(),
            });
        }
        true
    }
    pub(super) fn language_qa_ready(&mut self, ctx: &egui::Context, view: &str) -> bool {
        if !view.starts_with("language-") || ctx.cumulative_frame_nr() != 30 {
            return false;
        }
        let target = language(view);
        assert_eq!(
            self.preferences.language, target,
            "language selection by mouse"
        );
        assert_eq!(i18n::current(), target.resolved());
        if view.ends_with("write") || view.ends_with("restored") {
            assert_eq!(
                Preferences::load(&self.save_path.with_file_name("appearance.json")).language,
                target
            );
        }
        assert_eq!(
            self.qa_settings.language_project.as_deref(),
            Some(serde_json::to_string(&self.project).unwrap().as_str()),
            "changing display language must not mutate project data"
        );
        assert_eq!(
            tr("설정"),
            match target.resolved() {
                Language::English => "Settings",
                Language::Japanese => "設定",
                _ => "설정",
            }
        );
        let heading = self
            .qa_settings
            .project_heading
            .as_ref()
            .expect("project heading");
        assert_eq!(heading.1, self.project.name);
        assert_eq!(heading.2, self.project.family);
        for rect in self.qa_settings.tabs.iter().flatten() {
            assert!(
                ctx.content_rect().contains_rect(*rect),
                "translated tab clipped"
            );
        }
        if view.contains("export") {
            assert!(
                dialog_bounds(ctx)
                    .contains_rect(self.qa_settings.export_window.expect("export window")),
                "export title and content must stay below the native caption: {:?} inside {:?}",
                self.qa_settings.export_window,
                dialog_bounds(ctx)
            );
            assert!(
                dialog_bounds(ctx).contains_rect(self.qa_export_check.expect("export action")),
                "export action must be reachable by scrolling"
            );
        }
        if view.contains("api") {
            assert!(
                dialog_bounds(ctx)
                    .contains_rect(self.qa_api_key.expect("expanded connection fields")),
                "API key must be reachable by scrolling"
            );
        }
        println!(
            "Language UI passed: {view}; project unchanged; {}",
            target.label()
        );
        true
    }
}
