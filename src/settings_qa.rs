//! Native settings flows. All API traffic uses a local fixture, never user credentials.
use super::*;
use std::io::{Read, Write};

#[derive(Default)]
pub(super) struct State {
    pub installs: usize,
    pub project_menu: Option<Rect>,
    pub project_name: Option<Rect>,
    completed: bool,
    request: Option<mpsc::Receiver<serde_json::Value>>,
}

impl Studio {
    pub(super) fn settings_qa_setup(&mut self, ctx: &egui::Context, view: &str) {
        if !view.starts_with("settings-") {
            return;
        }
        self.tab = 3;
        self.ai_dock_collapsed = true;
        assert_eq!(
            self.api_provider, "off",
            "fresh profiles start with AI disabled"
        );
        if view.starts_with("settings-dock-") {
            self.api_provider = "local".into();
            self.tab = 0;
            self.ai_dock_collapsed = false;
            self.ai_answer =
                "획의 굵기와 기준선을 확인했어요. 아래 입력창에서 계속 요청할 수 있어요.".into();
        }
        if view.contains("small") {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(1120., 760.)));
            self.preferences.theme = Theme::Light;
            ctx.set_theme(egui::Theme::Light);
        }
        if view.contains("api") || view.contains("needle") || view == "settings-disable" {
            self.select_ai_provider("openai");
            self.api_endpoint = "https://example.invalid/v1".into();
            self.api_model = "test-model".into();
            self.api_key = "qa-key-only".into();
            self.connection_changed();
        }
        if view == "settings-generation" {
            self.select_ai_provider("local");
            assert!(self.local_installed);
        }
        if view == "settings-needle-auto" {
            assert!(!self.needle_installed);
        }
        if view == "settings-local-auto" {
            assert!(!self.local_installed);
        }
        if view == "settings-disable" {
            self.decision_mode = doodle_core::needle::Mode::Needle;
        }
        if view == "settings-api-test" || view == "settings-api-error" {
            let (endpoint, receiver) = probe_server(view.ends_with("error"));
            self.api_endpoint = endpoint;
            self.qa_settings.request = Some(receiver);
            self.connection_changed();
        }
    }
    pub(super) fn settings_qa_input(
        &mut self,
        ctx: &egui::Context,
        input: &mut egui::RawInput,
        view: &str,
    ) -> bool {
        if !view.starts_with("settings-") {
            return false;
        }
        input
            .events
            .retain(|e| matches!(e, egui::Event::Screenshot { .. }));
        let frame = ctx.cumulative_frame_nr();
        input.time = Some(frame as f64 / 30.);
        let rect = if frame == 3 || frame == 4 {
            match view {
                "settings-local" | "settings-local-auto" => {
                    self.qa_ai_providers.get("local").copied()
                }
                "settings-disable" => self.qa_ai_providers.get("off").copied(),
                "settings-needle-auto" => self.qa_decision_button,
                "settings-generation" => self.qa_generation_test,
                "settings-project" => self.qa_settings.project_menu,
                "settings-dock-collapsed" | "settings-dock-transition" => self.qa_dock.collapse,
                "settings-api-expanded-small" | "settings-api-test" | "settings-api-error" => {
                    self.qa_connection_header
                }
                _ => None,
            }
        } else if frame == 14 || frame == 15 {
            if view == "settings-api-test" || view == "settings-api-error" {
                self.qa_test_connection
            } else {
                None
            }
        } else {
            None
        };
        if let Some(rect) = rect {
            assert!(
                ctx.content_rect().contains(rect.center()),
                "control must be on screen: {view}"
            );
            input.events.push(egui::Event::PointerMoved(rect.center()));
            input.events.push(egui::Event::PointerButton {
                pos: rect.center(),
                button: egui::PointerButton::Primary,
                pressed: frame == 3 || frame == 14,
                modifiers: Default::default(),
            });
        }
        true
    }
    pub(super) fn settings_qa_ready(&mut self, ctx: &egui::Context, view: &str) -> bool {
        if !view.starts_with("settings-") || self.qa_settings.completed {
            return false;
        }
        assert!(
            self.ai_started.elapsed() < Duration::from_secs(180),
            "settings QA timed out: {view}"
        );
        let target_frame = if view == "settings-dock-transition" {
            8
        } else {
            32
        };
        if ctx.cumulative_frame_nr() < target_frame
            || self.job.is_some()
            || self.connections_pending.is_some()
        {
            return false;
        }
        if view != "settings-dock-transition" {
            assert!(
                self.qa_dock.dock.is_none(),
                "settings must remain unobscured"
            );
        }
        match view {
            "settings-dock-transition" => {
                assert!(self.ai_dock_collapsed);
                let dock = self
                    .qa_dock
                    .dock
                    .expect("transition should still be visible");
                assert!(dock.height() > 5. && dock.height() < 180.);
                assert!(self.qa_dock.companion.unwrap().height() >= 160.);
            }
            "settings-dock-collapsed" => {
                assert!(self.ai_dock_collapsed);
                assert!(self.qa_dock.companion.is_some());
            }
            "settings-project" => {
                assert!(ctx.content_rect().contains_rect(
                    self.qa_settings
                        .project_name
                        .expect("project fields visible in project menu")
                ));
            }
            "settings-off" | "settings-disable" => {
                assert_eq!(self.api_provider, "off");
                assert!(!self.ai_ready());
                assert_eq!(self.qa_settings.installs, 0);
                if view == "settings-disable" {
                    let saved = doodle_core::ai_settings::Connections::load(&self.connections_path)
                        .unwrap();
                    assert_eq!(saved.provider, "off");
                    assert_eq!(
                        saved.key("openai", "https://example.invalid/v1"),
                        "qa-key-only"
                    );
                    self.start_ai(ctx, true);
                    assert!(self.job.is_none());
                }
            }
            "settings-local-auto" | "settings-needle-auto" => {
                assert_eq!(
                    self.qa_settings.installs, 1,
                    "selection starts installation exactly once"
                );
                assert!(self.install_error.is_empty(), "{}", self.install_error);
                assert!(self.ai_cancel.is_none());
                if view == "settings-local-auto" {
                    assert!(self.local_installed);
                    assert_eq!(self.api_provider, "local");
                } else {
                    assert!(self.needle_installed);
                    assert_eq!(self.decision_mode, doodle_core::needle::Mode::Needle);
                    assert_eq!(self.api_provider, "openai");
                    assert_eq!(
                        doodle_core::needle::Options::load(
                            &self.save_path.with_file_name("ai-options.json")
                        )
                        .decision,
                        self.decision_mode
                    );
                }
            }
            "settings-local" => {
                assert_eq!(self.api_provider, "local");
                assert_eq!(self.qa_settings.installs, 0);
            }
            "settings-generation" => {
                assert!(self.generation_test_open);
                assert!(self.project.glyphs[0].candidates.is_empty());
            }
            "settings-api-test" | "settings-api-error" => {
                assert!(!self.connection_test_status.is_empty());
                assert_ne!(self.connection_test_status, "연결 테스트 중…");
                assert_eq!(self.connection_test_error, view.ends_with("error"));
                assert!(!self.connection_test_status.contains("qa-key-only"));
                let body = self
                    .qa_settings
                    .request
                    .take()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(1))
                    .unwrap();
                assert_eq!(
                    body["messages"],
                    serde_json::json!([{"role":"user","content":"Reply with OK."}])
                );
                assert!(self.project.glyphs[0].candidates.is_empty());
            }
            "settings-api" | "settings-api-small" => {
                assert!(self.qa_api_key.is_none());
            }
            "settings-api-expanded-small" => {
                assert!(self.qa_api_key.is_some());
            }
            _ => panic!("Unknown settings QA view {view}"),
        }
        self.qa_settings.completed = true;
        true
    }
}

fn probe_server(error: bool) -> (String, mpsc::Receiver<serde_json::Value>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}/v1", listener.local_addr().unwrap());
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut bytes = Vec::new();
        loop {
            let mut buffer = [0; 2048];
            let n = stream.read(&mut buffer).unwrap();
            assert!(n > 0);
            bytes.extend_from_slice(&buffer[..n]);
            let text = String::from_utf8_lossy(&bytes);
            if let Some(end) = text.find("\r\n\r\n") {
                let length = text[..end]
                    .lines()
                    .find_map(|line| {
                        line.to_lowercase()
                            .strip_prefix("content-length:")
                            .and_then(|v| v.trim().parse::<usize>().ok())
                    })
                    .unwrap();
                if bytes.len() >= end + 4 + length {
                    tx.send(serde_json::from_slice(&bytes[end + 4..]).unwrap())
                        .unwrap();
                    break;
                }
            }
        }
        let (status, body) = if error {
            ("401 Unauthorized", "qa-key-only must not appear in the UI")
        } else {
            ("200 OK", r#"{"choices":[{"message":{"content":"OK"}}]}"#)
        };
        let _ = write!(stream, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len());
    });
    (endpoint, rx)
}
