//! Native pointer/keyboard regressions for project editing and the studio layout.
use super::*;

fn click(input: &mut egui::RawInput, rect: Rect, pressed: bool, secondary: bool) {
    input.events.push(egui::Event::PointerMoved(rect.center()));
    input.events.push(egui::Event::PointerButton {
        pos: rect.center(),
        button: if secondary {
            egui::PointerButton::Secondary
        } else {
            egui::PointerButton::Primary
        },
        pressed,
        modifiers: Default::default(),
    });
}
fn key(input: &mut egui::RawInput, key: egui::Key, command: bool) {
    input.events.push(egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers {
            ctrl: command,
            command,
            ..Default::default()
        },
    });
}
fn project_names(view: &str) -> (&'static str, &'static str) {
    if view.ends_with("small") {
        (
            "매일 쓰는 나만의 글씨와 그림을 모아 만드는 아주 긴 이름의 두들 폰트 프로젝트",
            "Doodle Spring Handwriting Family With A Long Descriptive Name",
        )
    } else {
        ("봄날의 낙서", "Doodle Spring")
    }
}
impl Studio {
    #[cfg(windows)]
    pub(super) fn window_corners_qa(&mut self, ctx: &egui::Context, window: &eframe::Frame) {
        let view = std::env::var("DOODLE_QA_VIEW").unwrap_or_default();
        if !view.starts_with("layout-corners") {
            return;
        }
        let frame = ctx.cumulative_frame_nr();
        let command = match frame {
            3 => Some(egui::ViewportCommand::Maximized(true)),
            11 => Some(egui::ViewportCommand::Maximized(false)),
            19 => Some(egui::ViewportCommand::Fullscreen(true)),
            27 => Some(egui::ViewportCommand::Fullscreen(false)),
            35 => Some(egui::ViewportCommand::InnerSize(Vec2::new(1180., 800.))),
            _ => None,
        };
        if let Some(command) = command {
            ctx.send_viewport_cmd(command);
        }
        if [2, 10, 18, 26, 34, 42].contains(&frame) {
            let (maximized, fullscreen) = ctx.input(|i| {
                (
                    i.viewport().maximized.unwrap_or(false),
                    i.viewport().fullscreen.unwrap_or(false),
                )
            });
            assert_eq!(maximized, frame == 10, "native maximize/restore at {frame}");
            assert_eq!(
                fullscreen,
                frame == 26,
                "native fullscreen/restore at {frame}"
            );
            self.window_corners
                .assert_applied(window, !maximized && !fullscreen);
            if frame == 42 {
                let size = ctx.content_rect().size();
                assert!(
                    (size - Vec2::new(1180., 800.)).length() < 2.,
                    "native resize: {size:?}"
                );
            }
            println!("Window corners verified: {view}, frame {frame}, maximized={maximized}, fullscreen={fullscreen}");
        }
    }
    pub(super) fn layout_qa_setup(&mut self, ctx: &egui::Context, view: &str) {
        if !view.starts_with("layout-") {
            return;
        }
        self.api_provider = "off".into();
        self.ai_dock_collapsed = true;
        self.tab = 0;
        self.preferences.theme = if view.contains("dark") {
            Theme::Dark
        } else {
            Theme::Light
        };
        ctx.set_theme(self.preferences.theme.preference());
        if view.ends_with("small") {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(1120., 760.)));
        }
        if view.starts_with("layout-templates") {
            self.tab = 2;
            self.project.templates[1].attribution =
                "긴 출처 표시 문구도 카드 안에서 끝까지 스크롤하여 확인할 수 있습니다. ".repeat(20);
            self.project.templates[2].name =
                "이름이 아주 길어도 다른 템플릿의 높이와 버튼 위치가 달라지지 않는 템플릿".into();
        }
        if view == "layout-sidebar-reduced" {
            self.preferences.reduce_motion = true;
        }
        if view == "layout-companion-small" {
            self.api_provider = "local".into();
        }
        if view == "layout-window" || view.starts_with("layout-corners") {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(1120., 760.)));
        }
    }
    pub(super) fn layout_qa_input(
        &mut self,
        ctx: &egui::Context,
        input: &mut egui::RawInput,
        view: &str,
    ) -> bool {
        if !view.starts_with("layout-") {
            return false;
        }
        input
            .events
            .retain(|e| matches!(e, egui::Event::Screenshot { .. }));
        let frame = ctx.cumulative_frame_nr();
        input.time = Some(frame as f64 / 30.);
        let mut target = None;
        let mut pressed = false;
        let mut secondary = false;
        if view == "layout-window" {
            if let Some(chrome) = &self.qa_chrome {
                target = match frame {
                    3 | 4 | 15 | 16 | 41 | 42 => Some(chrome.controls[1]),
                    // Separate this pair from the earlier restore click's triple-click window.
                    29 | 30 | 34 | 35 => Some(chrome.project),
                    52 | 53 => Some(chrome.controls[2]),
                    _ => None,
                };
                pressed = [3, 15, 29, 34, 41, 52].contains(&frame);
            }
            if frame == 12 || frame == 38 {
                assert_eq!(
                    input.viewport().maximized,
                    Some(true),
                    "native window actually maximizes at frame {frame}"
                );
            }
            if frame == 24 {
                assert_eq!(
                    input.viewport().maximized,
                    Some(false),
                    "native window restores"
                );
                let v = input.viewport();
                let inner = v.inner_rect.unwrap();
                let outer = v.outer_rect.unwrap();
                assert!(
                    (inner.top() - outer.top()).abs() < 2.,
                    "no OS caption remains: {inner:?} / {outer:?}"
                );
            }
            if frame == 52 {
                self.project.family = "Window Close Save Check".into();
                self.changed();
            }
        } else if view.starts_with("layout-project") {
            target = match frame {
                3 | 4 | 25 | 26 | 34 | 35 => self.qa_settings.project_menu,
                7 | 8 => Some(
                    self.qa_settings
                        .project_name
                        .expect("name input must stay visible"),
                ),
                13 | 14 => Some(
                    self.qa_settings
                        .project_family
                        .expect("family input must stay visible after name focus"),
                ),
                30 | 31 => self.qa_settings.tabs[0],
                _ => None,
            };
            pressed = [3, 7, 13, 25, 30, 34].contains(&frame);
            if frame == 9 || frame == 15 {
                key(input, egui::Key::A, true);
            }
            if frame == 10 || frame == 16 {
                let (name, family) = project_names(view);
                input.events.push(egui::Event::Text(
                    if frame == 10 { name } else { family }.into(),
                ));
            }
            if frame == 20 {
                let (name, family) = project_names(view);
                assert_eq!(self.project.name, name);
                assert_eq!(self.project.family, family);
                assert!(
                    self.qa_settings.project_family.is_some(),
                    "editing cannot dismiss project popup"
                );
                self.save();
                let saved = storage::read(&self.save_path).expect("edited project saved");
                assert_eq!(saved.name, name);
                assert_eq!(saved.family, family);
            }
            if frame == 22 {
                key(input, egui::Key::Escape, false);
            }
            if frame == 24 || frame == 33 {
                assert!(
                    self.qa_settings.project_name.is_none(),
                    "Escape and outside click dismiss popup"
                );
            }
        } else if view == "layout-tabs" {
            let index = match frame {
                3 | 4 => Some(1),
                9 | 10 => Some(2),
                15 | 16 => Some(3),
                21 | 22 => Some(0),
                _ => None,
            };
            target = index.and_then(|i| self.qa_settings.tabs[i]);
            pressed = [3, 9, 15, 21].contains(&frame);
            match frame {
                8 => assert_eq!(self.tab, 1),
                14 => assert_eq!(self.tab, 2),
                20 => assert_eq!(self.tab, 3),
                26 => assert_eq!(self.tab, 0),
                _ => {}
            }
        } else if view.starts_with("layout-sidebar") {
            if [3, 4, 15, 16, 27, 28].contains(&frame) {
                target = self.qa_settings.sidebar_toggle;
                pressed = [3, 15, 27].contains(&frame);
            }
            let width = self.qa_settings.sidebar.map(|r| r.width());
            match frame {
                6 | 18 if view != "layout-sidebar-reduced" => assert!(
                    width.is_some_and(|w| w > 48. && w < 278.),
                    "sidebar must animate through intermediate widths: {width:?}"
                ),
                6 | 14 | 40 => assert_eq!(width, Some(278.)),
                18 | 26 => assert_eq!(width, Some(48.)),
                _ => {}
            }
        } else if view == "layout-companion-small" {
            target = match frame {
                3 | 4 | 31 | 32 => self.qa_dock.companion,
                9 | 10 => self.qa_renderers[0],
                15 | 16 => self.qa_renderers[1],
                21 | 22 => self.qa_settings.reduce_motion,
                _ => None,
            };
            pressed = [3, 9, 15, 21, 31].contains(&frame);
            secondary = [3, 4, 31, 32].contains(&frame);
            if frame == 8 || frame == 14 || frame == 26 {
                assert!(ctx.content_rect().contains_rect(
                    self.qa_settings
                        .companion_menu
                        .expect("right click opens companion controls")
                ));
            }
            if frame == 14 {
                assert_eq!(
                    self.companion.renderer(),
                    doodle_core::companion::Renderer::Image
                );
            }
            if frame == 26 {
                assert_eq!(
                    self.companion.renderer(),
                    doodle_core::companion::Renderer::Vector
                );
                assert!(self.preferences.reduce_motion);
                let saved = Preferences::load(&self.save_path.with_file_name("appearance.json"));
                assert!(saved.reduce_motion);
                assert_eq!(
                    saved.companion_renderer,
                    doodle_core::companion::Renderer::Vector
                );
            }
            if frame == 28 {
                key(input, egui::Key::Escape, false);
            }
            if frame == 30 {
                assert!(self.qa_settings.companion_menu.is_none());
            }
        }
        if let Some(rect) = target {
            assert!(
                ctx.content_rect().contains_rect(rect),
                "clickable control is off screen: {view}: {rect:?}"
            );
            click(input, rect, pressed, secondary);
        }
        true
    }
    pub(super) fn layout_qa_ready(&mut self, ctx: &egui::Context, view: &str) -> bool {
        if !view.starts_with("layout-") || ctx.cumulative_frame_nr() != 48 {
            return false;
        }
        let screen = ctx.content_rect();
        let (heading, name, family) = self.qa_settings.project_heading.as_ref().unwrap();
        assert!(screen.contains_rect(*heading));
        assert_eq!(name, &self.project.name);
        assert_eq!(family, &self.project.family);
        let tabs = self.qa_settings.tabs.map(Option::unwrap);
        assert!(tabs
            .iter()
            .all(|r| screen.contains_rect(*r) && r.height() >= 36.));
        assert!(tabs.windows(2).all(|pair| pair[0].right() < pair[1].left()));
        if view.starts_with("layout-project") {
            assert!(screen.contains_rect(
                self.qa_settings
                    .project_name
                    .expect("project popup reopens")
            ));
            assert!(screen.contains_rect(self.qa_settings.project_family.unwrap()));
        }
        if view.starts_with("layout-templates") {
            let cards = &self.qa_settings.template_cards;
            assert_eq!(cards.len(), 3);
            let (first, action) = cards[0];
            assert!(
                first.height() < 410.,
                "cards must not fill the whole scroll area"
            );
            for (card, button) in cards {
                assert_eq!(
                    card.size(),
                    first.size(),
                    "long titles or credits must not change card sizes"
                );
                assert_eq!(button.top(), action.top(), "actions align on the same row");
                assert!(screen.contains_rect(*card));
                assert!(card.contains_rect(*button));
            }
        }
        if view.starts_with("layout-sidebar") {
            assert!(self.show_properties);
            assert_eq!(self.qa_settings.sidebar.unwrap().width(), 278.);
            assert!(screen.contains_rect(self.qa_settings.sidebar.unwrap()));
            assert!(self.qa_canvas.unwrap().right() <= self.qa_settings.sidebar.unwrap().left());
        }
        if view == "layout-companion-small" {
            assert!(screen.contains_rect(self.qa_settings.companion_menu.unwrap()));
        }
        true
    }
}
