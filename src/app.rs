use base64::{engine::general_purpose::STANDARD, Engine};
use doodle_core::{
    appearance::{self, Palette, Preferences, Theme},
    document::*,
    font,
    model::*,
    storage,
};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};
const DRAW_INK: Color32 = Color32::from_rgb(48, 51, 46);
#[cfg(feature = "ui-qa")]
#[path = "companion_placement_qa.rs"]
mod companion_placement_qa;
#[cfg(feature = "ui-qa")]
#[path = "layout_qa.rs"]
mod layout_qa;
#[cfg(feature = "ui-qa")]
#[path = "settings_qa.rs"]
mod settings_qa;

type JobResult = Result<Job, String>;

#[cfg(feature = "ui-qa")]
#[derive(Default)]
struct DockQa {
    baseline: Option<Rect>,
    dock: Option<Rect>,
    answer: Option<Rect>,
    answer_offset: f32,
    prompt: Option<Rect>,
    prompt_offset: f32,
    collapse: Option<Rect>,
    restore: Option<Rect>,
    expanded: Option<Rect>,
    companion: Option<Rect>,
    companion_baseline: Option<Rect>,
    footer: Option<Rect>,
    footer_spinner: Option<Rect>,
}
enum Job {
    Export(font::ExportResult, Option<PathBuf>),
    AiReply(String, bool, serde_json::Value),
    AiInstalled(doodle_core::ai_install::Package),
    ConnectionTest(Result<(), String>),
    FontPreview(Vec<u8>, String),
    Imported(Project),
}
pub struct Studio {
    #[cfg(windows)]
    window_corners: crate::native_window::Corners,
    companion: doodle_core::companion::Companion,
    companion_comparison: Option<doodle_core::companion_compare::Comparison>,
    preferences: Preferences,
    palette: Palette,
    show_properties: bool,
    project: Project,
    index: HashMap<(String, String), usize>,
    selected: usize,
    group: usize,
    catalog: Vec<String>,
    filtered: Vec<usize>,
    search: String,
    only_done: bool,
    tab: usize,
    tool: usize,
    color: Color32,
    width: f64,
    fill: bool,
    guides: bool,
    ghost: bool,
    zoom: f32,
    layer: Option<usize>,
    draft: Option<Shape>,
    drag_start: Option<[f64; 2]>,
    history: Vec<Glyph>,
    redo: Vec<Glyph>,
    textures: HashMap<String, egui::TextureHandle>,
    system_fonts: HashMap<String, Vec<u8>>,
    revision: u64,
    message: String,
    message_time: Instant,
    dirty: bool,
    last_edit: Instant,
    save_path: PathBuf,
    save_error: bool,
    add_open: bool,
    add_text: String,
    add_locale: String,
    add_custom: bool,
    pua: String,
    export_open: bool,
    export_format: String,
    export_mode: String,
    export_locale: String,
    export_group: bool,
    export_subset: bool,
    export_text: String,
    export_split: bool,
    export_report: String,
    preview_text: String,
    preview_locale: String,
    preview_size: f32,
    preview_controls: bool,
    actual_font: Option<Vec<u8>>,
    api_provider: String,
    api_endpoint: String,
    api_model: String,
    api_key: String,
    connections: doodle_core::ai_settings::Connections,
    connections_path: PathBuf,
    connections_pending: Option<Instant>,
    connections_status: String,
    connections_error: bool,
    connection_test_status: String,
    connection_test_error: bool,
    testing_connection: bool,
    generation_test_open: bool,
    ai_prompt: String,
    send_reference: bool,
    ai_color: bool,
    ai_action: bool,
    ai_answer: String,
    ai_dock_collapsed: bool,
    ai_dock_restore: Option<Rect>,
    candidate_review: Option<(String, String)>,
    ai_status: String,
    ai_started: Instant,
    ai_cancel: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    ai_progress: Option<mpsc::Receiver<doodle_core::ai_progress::Progress>>,
    local_installed: bool,
    decision_mode: doodle_core::needle::Mode,
    needle_installed: bool,
    close_after_ai: bool,
    installing_ai: bool,
    install_error: String,
    install_progress: Option<mpsc::Receiver<doodle_core::ai_install::Progress>>,
    install_fraction: f32,
    last_install_check: Instant,
    job: Option<mpsc::Receiver<JobResult>>,
    component_name: String,
    template_provider: String,
    template_version: String,
    template_source: String,
    template_license: String,
    template_attribution: String,
    template_rights: [bool; 5],
    help: bool,
    variant_open: bool,
    variant_targets: std::collections::HashSet<String>,
    #[cfg(feature = "ui-qa")]
    qa_canvas: Option<Rect>,
    #[cfg(feature = "ui-qa")]
    qa_chrome: Option<doodle_core::window_chrome::ChromeRects>,
    #[cfg(feature = "ui-qa")]
    qa_themes: [Option<Rect>; 3],
    #[cfg(feature = "ui-qa")]
    qa_renderers: [Option<Rect>; 3],
    #[cfg(feature = "ui-qa")]
    qa_install_button: Option<Rect>,
    #[cfg(feature = "ui-qa")]
    qa_candidate_accept: Option<Rect>,
    #[cfg(feature = "ui-qa")]
    qa_ai_send: Option<Rect>,
    #[cfg(feature = "ui-qa")]
    qa_decision_button: Option<Rect>,
    #[cfg(feature = "ui-qa")]
    qa_ai_providers: HashMap<String, Rect>,
    #[cfg(feature = "ui-qa")]
    qa_connection_header: Option<Rect>,
    #[cfg(feature = "ui-qa")]
    qa_test_connection: Option<Rect>,
    #[cfg(feature = "ui-qa")]
    qa_generation_test: Option<Rect>,
    #[cfg(feature = "ui-qa")]
    qa_settings: settings_qa::State,
    #[cfg(feature = "ui-qa")]
    qa_api_key: Option<Rect>,
    #[cfg(feature = "ui-qa")]
    qa_forget_key: Option<Rect>,
    #[cfg(feature = "ui-qa")]
    qa_dock: DockQa,
    #[cfg(feature = "ui-qa")]
    qa_export_formats: HashMap<String, Rect>,
    #[cfg(feature = "ui-qa")]
    qa_export_check: Option<Rect>,
}
impl Studio {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let ctx = &cc.egui_ctx;
        let mut fonts = egui::FontDefinitions::default();
        for (name, path) in [
            ("korean", "C:/Windows/Fonts/malgun.ttf"),
            ("symbols", "C:/Windows/Fonts/seguisym.ttf"),
        ] {
            if let Ok(bytes) = std::fs::read(path) {
                fonts
                    .font_data
                    .insert(name.into(), egui::FontData::from_owned(bytes).into());
                for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                    fonts.families.get_mut(&family).unwrap().push(name.into());
                }
            }
        }
        ctx.set_fonts(fonts);
        let root = std::env::var_os("DOODLE_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_default())
                    .join("DoodleFont")
            });
        let preferences = Preferences::load(&root.join("appearance.json"));
        appearance::install(ctx, preferences.theme);
        let save_path = root.join("project.json");
        let connections_path = root.join("ai-connections.dat");
        let (connections, connections_status, connections_error) =
            match doodle_core::ai_settings::Connections::load(&connections_path) {
                Ok(settings) => (settings, String::new(), false),
                Err(error) => (Default::default(), error, true),
            };
        let profile = connections.profile(&connections.provider);
        let api_key = connections.key(&connections.provider, &profile.endpoint);
        let (project, error) = if save_path.exists() {
            match storage::read(&save_path){Ok(p)=>(p,String::new()),Err(e)=>(new_project(),format!("{e} 원본 자동 저장 파일은 보존됩니다. 프로젝트 열기로 백업을 복구해 주세요."))}
        } else {
            (new_project(), String::new())
        };
        let mut s = Self {
            #[cfg(windows)]
            window_corners: crate::native_window::Corners::new(cc),
            companion: doodle_core::companion::Companion::with_renderer(
                ctx,
                preferences.companion_renderer,
            ),
            companion_comparison: None,
            preferences,
            palette: Palette::of(ctx),
            show_properties: false,
            project,
            index: HashMap::new(),
            selected: 0,
            group: 0,
            catalog: vec![],
            filtered: vec![],
            search: String::new(),
            only_done: false,
            tab: 0,
            tool: 0,
            color: DRAW_INK,
            width: 36.,
            fill: false,
            guides: true,
            ghost: true,
            zoom: 1.,
            layer: None,
            draft: None,
            drag_start: None,
            history: vec![],
            redo: vec![],
            textures: HashMap::new(),
            system_fonts: HashMap::new(),
            revision: 0,
            message: error.clone(),
            message_time: Instant::now(),
            dirty: false,
            last_edit: Instant::now(),
            save_path,
            save_error: !error.is_empty(),
            add_open: false,
            add_text: String::new(),
            add_locale: "default".into(),
            add_custom: false,
            pua: "E000".into(),
            export_open: false,
            export_format: "ttf".into(),
            export_mode: "color".into(),
            export_locale: "combined".into(),
            export_group: false,
            export_subset: false,
            export_text: String::new(),
            export_split: false,
            export_report: String::new(),
            preview_text: "A little doodle, a lot of you.\n나의 글씨로 쓰는 오늘 ❤️ 😊".into(),
            preview_locale: "ko".into(),
            preview_size: 64.,
            preview_controls: true,
            actual_font: None,
            api_provider: connections.provider.clone(),
            api_endpoint: profile.endpoint,
            api_model: profile.model,
            api_key,
            connections,
            connections_path,
            connections_pending: None,
            connections_status,
            connections_error,
            connection_test_status: String::new(),
            connection_test_error: false,
            testing_connection: false,
            generation_test_open: false,
            ai_prompt: String::new(),
            send_reference: false,
            ai_color: true,
            ai_action: false,
            ai_answer: String::new(),
            ai_dock_collapsed: false,
            ai_dock_restore: None,
            candidate_review: None,
            ai_status: String::new(),
            ai_started: Instant::now(),
            ai_cancel: None,
            ai_progress: None,
            local_installed: doodle_core::local_ai::installed(),
            decision_mode: doodle_core::needle::Options::load(&root.join("ai-options.json"))
                .decision,
            needle_installed: doodle_core::needle::installed(),
            close_after_ai: false,
            installing_ai: false,
            install_error: String::new(),
            install_progress: None,
            install_fraction: 0.,
            last_install_check: Instant::now(),
            job: None,
            component_name: "나의 부품".into(),
            template_provider: "사용자 가져오기".into(),
            template_version: "1.0".into(),
            template_source: String::new(),
            template_license: String::new(),
            template_attribution: String::new(),
            template_rights: [true, false, false, false, false],
            help: false,
            variant_open: false,
            variant_targets: Default::default(),
            #[cfg(feature = "ui-qa")]
            qa_canvas: None,
            #[cfg(feature = "ui-qa")]
            qa_chrome: None,
            #[cfg(feature = "ui-qa")]
            qa_themes: [None; 3],
            #[cfg(feature = "ui-qa")]
            qa_renderers: [None; 3],
            #[cfg(feature = "ui-qa")]
            qa_install_button: None,
            #[cfg(feature = "ui-qa")]
            qa_candidate_accept: None,
            #[cfg(feature = "ui-qa")]
            qa_ai_send: None,
            #[cfg(feature = "ui-qa")]
            qa_decision_button: None,
            #[cfg(feature = "ui-qa")]
            qa_ai_providers: HashMap::new(),
            #[cfg(feature = "ui-qa")]
            qa_connection_header: None,
            #[cfg(feature = "ui-qa")]
            qa_test_connection: None,
            #[cfg(feature = "ui-qa")]
            qa_generation_test: None,
            #[cfg(feature = "ui-qa")]
            qa_settings: Default::default(),
            #[cfg(feature = "ui-qa")]
            qa_api_key: None,
            #[cfg(feature = "ui-qa")]
            qa_forget_key: None,
            #[cfg(feature = "ui-qa")]
            qa_dock: Default::default(),
            #[cfg(feature = "ui-qa")]
            qa_export_formats: HashMap::new(),
            #[cfg(feature = "ui-qa")]
            qa_export_check: None,
        };
        s.reindex();
        s.refresh_catalog();
        s
    }
    fn notify(&mut self, text: impl Into<String>) {
        self.message = text.into();
        self.message_time = Instant::now();
    }
    fn reindex(&mut self) {
        self.index = self
            .project
            .glyphs
            .iter()
            .enumerate()
            .map(|(i, g)| ((g.text.clone(), g.locale.clone()), i))
            .collect();
        self.selected = self
            .selected
            .min(self.project.glyphs.len().saturating_sub(1));
    }
    fn refresh_catalog(&mut self) {
        let group = GROUPS[self.group].0;
        self.catalog = catalog(group);
        let mut seen: std::collections::HashSet<String> = self.catalog.iter().cloned().collect();
        for g in &self.project.glyphs {
            if g.group == group && seen.insert(g.text.clone()) {
                self.catalog.push(g.text.clone())
            }
        }
        self.filter();
    }
    fn filter(&mut self) {
        let locale = GROUPS[self.group].2;
        let query = self.search.to_lowercase();
        self.filtered = self
            .catalog
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                let g = self
                    .index
                    .get(&(s.to_string(), locale.into()))
                    .map(|i| &self.project.glyphs[*i]);
                (!self.only_done || g.is_some_and(|g| g.status == "edited" || g.status == "blank"))
                    && (query.is_empty()
                        || s.to_lowercase().contains(&query)
                        || codepoints(s).to_lowercase().contains(&query)
                        || g.is_some_and(|g| g.name.to_lowercase().contains(&query)))
            })
            .map(|(i, _)| i)
            .collect();
    }
    fn changed(&mut self) {
        self.revision += 1;
        self.dirty = true;
        self.last_edit = Instant::now();
        self.project.updated_at = timestamp();
        self.actual_font = None;
        self.textures.clear();
    }
    fn edit(&mut self, f: impl FnOnce(&mut Glyph) -> Result<(), String>) {
        if self.ai_cancel.is_some() {
            return;
        }
        if self.project.glyphs.is_empty() {
            return;
        }
        let before = self.project.glyphs[self.selected].clone();
        match f(&mut self.project.glyphs[self.selected]) {
            Ok(()) => {
                self.history.push(before);
                self.redo.clear();
                while self.history.len() > 30
                    || self.history.iter().map(glyph_bytes).sum::<usize>() > 24_000_000
                {
                    self.history.remove(0);
                }
                self.changed();
                self.filter();
            }
            Err(e) => {
                self.project.glyphs[self.selected] = before;
                self.notify(e);
            }
        }
    }
    fn undo(&mut self, redo: bool) {
        if self.ai_cancel.is_some() {
            return;
        }
        let from = if redo {
            &mut self.redo
        } else {
            &mut self.history
        };
        if let Some(g) = from.pop() {
            if let Some(idx) = self.project.glyphs.iter().position(|v| v.id == g.id) {
                let old = std::mem::replace(&mut self.project.glyphs[idx], g);
                if redo {
                    self.history.push(old)
                } else {
                    self.redo.push(old)
                }
                self.selected = idx;
                self.layer = None;
                self.changed();
            }
        }
    }
    fn select_text(&mut self, text: &str) {
        if self.ai_cancel.is_some() {
            return;
        }
        let (group, _, locale) = GROUPS[self.group];
        let key = (text.to_owned(), locale.to_owned());
        self.selected = if let Some(i) = self.index.get(&key) {
            *i
        } else {
            let i = self.project.glyphs.len();
            self.project
                .glyphs
                .push(new_glyph(text, locale, group, false));
            self.index.insert(key, i);
            i
        };
        self.layer = None;
        self.draft = None;
    }
    fn save(&mut self) {
        if self.save_error {
            return;
        }
        match storage::save(&self.save_path, &self.project) {
            Ok(()) => {
                self.dirty = false;
            }
            Err(e) => {
                self.notify(format!("자동 저장 실패: {e}"));
                self.save_error = true;
            }
        }
    }
    fn spawn(&mut self, ctx: &egui::Context, work: impl FnOnce() -> JobResult + Send + 'static) {
        let (tx, rx) = mpsc::channel();
        self.job = Some(rx);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = work();
            let _ = tx.send(result);
            ctx.request_repaint();
        });
    }
    fn poll(&mut self) {
        if let Some(rx) = &self.install_progress {
            while let Ok(p) = rx.try_recv() {
                self.ai_status = p.message;
                self.install_fraction = p.fraction;
            }
        }
        if let Some(rx) = &self.ai_progress {
            while let Ok(status) = rx.try_recv() {
                match status {
                    doodle_core::ai_progress::Progress::Message(message) => {
                        self.ai_status = message
                    }
                    doodle_core::ai_progress::Progress::Animation(mood) => {
                        self.companion.motion.react(mood)
                    }
                }
            }
        }
        if let Some(rx) = &self.job {
            match rx.try_recv() {
                Ok(result) => {
                    self.job = None;
                    let was_installing = std::mem::take(&mut self.installing_ai);
                    self.install_progress = None;
                    if was_installing {
                        self.local_installed = doodle_core::local_ai::installed();
                        self.needle_installed = doodle_core::needle::installed();
                    }
                    let was_testing = std::mem::take(&mut self.testing_connection);
                    let was_ai = self.ai_cancel.is_some() && !was_installing && !was_testing;
                    let cancelled = self
                        .ai_cancel
                        .take()
                        .is_some_and(|c| c.load(std::sync::atomic::Ordering::Relaxed));
                    self.ai_progress = None;
                    if cancelled {
                        self.companion
                            .motion
                            .react(doodle_core::companion::Mood::Idle);
                        if was_testing {
                            self.connection_test_status = "연결 테스트를 취소했어요.".into();
                            self.connection_test_error = false;
                            return;
                        }
                        if was_installing {
                            self.install_error =
                                "설치를 취소했어요. 다시 설치하면 내려받던 파일을 이어받습니다."
                                    .into();
                            self.notify(self.install_error.clone());
                            return;
                        }
                        self.notify("AI 작업을 취소했어요. 편집을 계속할 수 있어요.");
                        self.ai_answer =
                            "AI 작업을 취소했어요. 새 후보는 저장하지 않았습니다.".into();
                        return;
                    }
                    match result {
                        Ok(Job::ConnectionTest(result)) => {
                            self.connection_test_error = result.is_err();
                            self.connection_test_status = match result {
                                Ok(()) => "연결 성공 · 모델이 텍스트 요청에 응답했어요.".into(),
                                Err(e) => e,
                            };
                            self.companion.motion.react(if self.connection_test_error {
                                doodle_core::companion::Mood::NeedsHelp
                            } else {
                                doodle_core::companion::Mood::Done
                            });
                        }
                        Err(e) => {
                            self.companion
                                .motion
                                .react(doodle_core::companion::Mood::NeedsHelp);
                            if was_installing {
                                self.install_error = e.clone();
                            } else if was_ai {
                                self.ai_answer = format!("AI 작업을 완료하지 못했어요.\n{e}");
                            }
                            self.notify(e)
                        }
                        Ok(Job::AiInstalled(package)) => {
                            self.companion
                                .motion
                                .react(doodle_core::companion::Mood::Done);
                            self.local_installed = doodle_core::local_ai::installed();
                            self.needle_installed = doodle_core::needle::installed();
                            self.install_error.clear();
                            self.notify(if package == doodle_core::ai_install::Package::Needle {
                                "판단 AI 설치를 마쳤어요. 모든 AI 연결의 입력 앞단에서 사용할 수 있어요."
                            } else { "Gemma 4 설치를 마쳤어요. 하단 입력창에서 사용할 수 있어요." });
                        }
                        Ok(Job::AiReply(id, drawing, result)) => {
                            self.companion.motion.react(if drawing {
                                doodle_core::companion::Mood::Done
                            } else {
                                doodle_core::companion::Mood::Explaining
                            });
                            match self.receive_ai_reply(&id, drawing, result) {
                                Ok(()) => self.notify(if drawing {
                                    "그림 후보를 만들었어요. 미리보기를 확인하고 채택해 주세요."
                                } else {
                                    "그림 상담을 마쳤어요."
                                }),
                                Err(e) => {
                                    self.companion
                                        .motion
                                        .react(doodle_core::companion::Mood::NeedsHelp);
                                    self.ai_answer = format!(
                                        "{}\n{e}",
                                        if drawing {
                                            "후보를 만들지 못했어요."
                                        } else {
                                            "그림 상담을 완료하지 못했어요."
                                        }
                                    );
                                    self.notify(e);
                                }
                            }
                        }
                        Ok(Job::Export(r, path)) => {
                            self.export_report = r
                                .reports
                                .iter()
                                .map(|r| {
                                    format!(
                                        "{} · {} · {}개 글리프 · {}\n{}\n{}",
                                        r.family,
                                        r.format.to_uppercase(),
                                        r.glyph_count,
                                        r.mode,
                                        r.tables.join(" / "),
                                        r.warnings.join("\n")
                                    )
                                })
                                .collect::<Vec<_>>()
                                .join("\n\n");
                            if let Some(path) = path {
                                self.notify(format!("폰트를 저장했어요: {}", path.display()))
                            } else {
                                self.notify("출력 폰트의 구조를 검증했어요.");
                            }
                        }
                        Ok(Job::FontPreview(bytes, report)) => {
                            self.actual_font = Some(bytes);
                            self.export_report = report;
                            self.notify("실제 출력 폰트를 생성하고 검사했어요.");
                        }
                        Ok(Job::Imported(p)) => {
                            self.project = p;
                            self.selected = 0;
                            self.history.clear();
                            self.redo.clear();
                            self.save_error = false;
                            self.reindex();
                            self.refresh_catalog();
                            self.changed();
                            self.notify("프로젝트를 열었어요. 템플릿 원본과 후보도 복원했습니다.");
                        }
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    if self.ai_cancel.is_some() {
                        self.companion
                            .motion
                            .react(doodle_core::companion::Mood::NeedsHelp);
                    }
                    if self.ai_cancel.is_some() && !self.installing_ai {
                        self.companion
                            .motion
                            .react(doodle_core::companion::Mood::NeedsHelp);
                        self.ai_answer = "AI 작업이 중단되었어요. 다시 요청해 주세요.".into();
                    }
                    self.job = None;
                    self.ai_cancel = None;
                    self.ai_progress = None;
                    if self.installing_ai {
                        self.installing_ai = false;
                        self.install_progress = None;
                        self.install_error =
                            "설치 작업이 중단되었어요. 다시 설치를 눌러 이어서 진행해 주세요."
                                .into();
                    }
                    self.notify("작업 스레드가 종료되었어요. 프로젝트는 유지됩니다.");
                }
                Err(_) => {}
            }
        }
    }
    fn open_project(&mut self, ctx: &egui::Context) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Doodle Font", &["json"])
            .pick_file()
        {
            self.spawn(ctx, move || storage::read(&path).map(Job::Imported));
        }
    }
    fn fresh_project(&mut self) {
        let archive = self
            .save_path
            .parent()
            .unwrap()
            .join("projects")
            .join(format!("{}.doodle.json", uid()));
        if let Err(e) = storage::save(&archive, &self.project) {
            self.notify(e);
            return;
        }
        if self.save_error {
            self.notify(
                "현재 작업을 사본으로 보관했어요. 기존 자동 저장 파일의 오류를 먼저 해결해 주세요.",
            );
            return;
        }
        self.project = new_project();
        self.selected = 0;
        self.group = 0;
        self.history.clear();
        self.redo.clear();
        self.layer = None;
        self.reindex();
        self.refresh_catalog();
        self.changed();
        self.save();
        self.notify(format!(
            "새 프로젝트를 만들었어요. 이전 작업: {}",
            archive.display()
        ));
    }
    fn save_copy(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name(format!("{}.doodle.json", self.project.name))
            .add_filter("Doodle Font", &["json"])
            .save_file()
        {
            match storage::save(&path, &self.project) {
                Ok(()) => self.notify("원본·승인본·후보를 포함해 프로젝트를 저장했어요."),
                Err(e) => self.notify(e),
            }
        }
    }
    fn import_image(&mut self) {
        if !editable(&self.project.glyphs[self.selected]) {
            self.notify("편집할 수 없는 글리프예요.");
            return;
        }
        let Some(file) = rfd::FileDialog::new()
            .add_filter("이미지", &["png", "jpg", "jpeg"])
            .pick_file()
        else {
            return;
        };
        let result = (|| -> Result<Shape, String> {
            if std::fs::metadata(&file).map_err(|e| e.to_string())?.len() > 8_000_000 {
                return Err("8MB 이하 이미지를 사용해 주세요.".into());
            }
            let mut reader = image::ImageReader::open(file)
                .map_err(|e| e.to_string())?
                .with_guessed_format()
                .map_err(|e| e.to_string())?;
            let mut limits = image::Limits::default();
            limits.max_image_width = Some(4096);
            limits.max_image_height = Some(4096);
            limits.max_alloc = Some(64 * 1024 * 1024);
            reader.limits(limits);
            let im = reader.decode().map_err(|e| e.to_string())?;
            let mut out = std::io::Cursor::new(vec![]);
            im.write_to(&mut out, image::ImageFormat::Png)
                .map_err(|e| e.to_string())?;
            let mut s = new_shape("image");
            s.width = 700.;
            s.height = 700. * im.height() as f64 / im.width() as f64;
            if s.height > 700. {
                s.width *= 700. / s.height;
                s.height = 700.;
            }
            s.src = Some(format!(
                "data:image/png;base64,{}",
                STANDARD.encode(out.into_inner())
            ));
            Ok(s)
        })();
        match result {
            Ok(s) => self.edit(|g| {
                g.artwork.shapes.push(s);
                g.status = "edited".into();
                Ok(())
            }),
            Err(e) => self.notify(e),
        }
    }
    fn appearance_options(&mut self, ui: &mut egui::Ui) {
        let old = self.preferences.clone();
        ui.label("화면 테마");
        ui.horizontal(|ui| {
            for (index, theme) in [Theme::System, Theme::Light, Theme::Dark]
                .into_iter()
                .enumerate()
            {
                let response =
                    ui.selectable_value(&mut self.preferences.theme, theme, theme.label());
                #[cfg(feature = "ui-qa")]
                {
                    self.qa_themes[index] = Some(response.rect);
                }
                let _ = (index, response);
            }
        });
        self.save_preferences(ui.ctx(), old);
    }
    fn companion_options(&mut self, ui: &mut egui::Ui) {
        let old = self.preferences.clone();
        ui.strong("컴패니언");
        let motion = ui
            .checkbox(&mut self.preferences.reduce_motion, "움직임 줄이기")
            .on_hover_text("컴패니언과 패널의 애니메이션을 줄입니다.");
        #[cfg(feature = "ui-qa")]
        {
            self.qa_settings.reduce_motion = Some(motion.rect);
        }
        let _ = motion;
        ui.label("컴패니언 표현");
        ui.horizontal(|ui| {
            use doodle_core::companion::Renderer;
            for (index, renderer) in [Renderer::Image, Renderer::Vector].into_iter().enumerate() {
                let response = ui.selectable_value(
                    &mut self.preferences.companion_renderer,
                    renderer,
                    renderer.label(),
                );
                #[cfg(feature = "ui-qa")]
                {
                    self.qa_renderers[index] = Some(response.rect);
                }
                let _ = (index, response);
            }
            let compare = ui.button("나란히 비교");
            #[cfg(feature = "ui-qa")]
            {
                self.qa_renderers[2] = Some(compare.rect);
            }
            if compare.clicked() {
                self.companion_comparison =
                    Some(doodle_core::companion_compare::Comparison::new(ui.ctx()));
            }
        });
        let reset = ui.add_enabled(
            self.preferences.companion_position.is_some(),
            egui::Button::new("기본 위치로 되돌리기"),
        );
        #[cfg(feature = "ui-qa")]
        {
            self.qa_settings.companion_reset = Some(reset.rect);
        }
        if reset.clicked() {
            self.preferences.companion_position = None;
            ui.close();
        }
        self.save_preferences(ui.ctx(), old);
    }
    fn save_preferences(&mut self, ctx: &egui::Context, old: Preferences) {
        if old != self.preferences {
            self.companion
                .set_renderer(ctx, self.preferences.companion_renderer);
            ctx.set_theme(self.preferences.theme.preference());
            self.palette = Palette::of(ctx);
            self.store_preferences();
        }
    }
    fn store_preferences(&mut self) {
        if let Err(e) = self
            .preferences
            .save(&self.save_path.with_file_name("appearance.json"))
        {
            self.notify(format!("화면 설정을 저장하지 못했어요: {e}"));
        }
    }
    fn top(&mut self, ctx: &egui::Context) {
        #[cfg(feature = "ui-qa")]
        {
            self.qa_settings.project_name = None;
            self.qa_settings.project_family = None;
            self.qa_settings.companion_menu = None;
            self.qa_renderers = [None; 3];
        }
        egui::TopBottomPanel::top("top")
            .exact_height(doodle_core::window_chrome::TITLE_HEIGHT + 44.)
            .frame(egui::Frame::new().fill(self.palette.surface))
            .show(ctx, |ui| {
                // The native caption is painted above modals after the document UI.
                ui.add_space(doodle_core::window_chrome::TITLE_HEIGHT);
                egui::Frame::new()
                    .inner_margin(egui::Margin::symmetric(20, 4))
                    .show(ui, |ui| {
                        if self.ai_cancel.is_some() {
                            ui.disable();
                        }
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 12.;
                            for (i, label) in ["그리기", "문장 미리보기", "템플릿", "설정"]
                                .iter()
                                .enumerate()
                            {
                                let tab = navigation_tab(ui, label, self.tab == i, self.palette);
                                #[cfg(feature = "ui-qa")]
                                {
                                    self.qa_settings.tabs[i] = Some(tab.rect);
                                }
                                if tab.clicked() {
                                    self.tab = i;
                                    if i == 3 {
                                        self.ai_dock_collapsed = true;
                                    }
                                }
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new("폰트 내보내기 ↗")
                                                    .color(Color32::WHITE),
                                            )
                                            .fill(Color32::from_rgb(190, 83, 51)),
                                        )
                                        .clicked()
                                    {
                                        self.export_open = true;
                                    }
                                    let project_menu = ui.button("프로젝트");
                                    egui::Popup::from_toggle_button_response(&project_menu)
                                        .close_behavior(
                                            egui::PopupCloseBehavior::CloseOnClickOutside,
                                        )
                                        .width(340.)
                                        .show(|ui| {
                                            self.project_options_ui(ui);
                                            ui.separator();
                                            if ui.button("프로젝트 사본 저장").clicked() {
                                                ui.close();
                                                self.save_copy();
                                            }
                                            if ui.button("열기…").clicked() {
                                                ui.close();
                                                self.open_project(ctx);
                                            }
                                            if ui.button("새 프로젝트").clicked() {
                                                ui.close();
                                                self.fresh_project();
                                            }
                                        });
                                    #[cfg(feature = "ui-qa")]
                                    {
                                        self.qa_settings.project_menu = Some(project_menu.rect);
                                    }
                                    let _ = project_menu;
                                    ui.menu_button("화면", |ui| self.appearance_options(ui));
                                },
                            );
                        });
                    });
            });
    }
    fn footer(&mut self, ctx: &egui::Context) {
        let footer = egui::TopBottomPanel::bottom("footer")
            .exact_height(32.)
            .frame(
                egui::Frame::new()
                    .fill(self.palette.surface)
                    .inner_margin(egui::Margin::symmetric(20, 6)),
            )
            .show(ctx, |ui| {
                // The footer has 20 points of inner height; the regular controls are 32.
                ui.spacing_mut().interact_size.y = 20.;
                ui.spacing_mut().button_padding.y = 1.;
                #[cfg(feature = "ui-qa")]
                {
                    self.qa_dock.footer_spinner = None;
                }
                if self.ai_cancel.is_some() {
                    ui.disable();
                }
                ui.horizontal(|ui| {
                    ui.label(RichText::new("●").color(if self.save_error {
                        self.palette.accent
                    } else {
                        Color32::from_rgb(109, 140, 105)
                    }));
                    ui.small(if self.save_error {
                        "저장 오류 · 프로젝트 저장으로 백업하세요"
                    } else if self.dirty {
                        "저장 중…"
                    } else {
                        "이 기기에 저장됨"
                    });
                    ui.separator();
                    ui.add_sized(
                        [(ui.available_width() - 450.).max(80.), 20.],
                        egui::Label::new(
                            RichText::new(format!(
                                "{}  /  {}개 작업 글리프",
                                self.project.name,
                                self.project.glyphs.len()
                            ))
                            .size(12.),
                        )
                        .truncate(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(RichText::new("도움말  ?").size(12.))
                                    .frame(false)
                                    .min_size(Vec2::new(64., 20.)),
                            )
                            .clicked()
                        {
                            self.help = true;
                        }
                        ui.small("두들 폰트 0.1.0");
                        if self.api_provider != "off" {
                            let restore = ui.add_sized(
                                [106., 20.],
                                egui::Button::new(if self.ai_dock_collapsed {
                                    "어시스트 열기 ↑"
                                } else {
                                    "어시스트 접기 ↓"
                                })
                                .frame(false),
                            );
                            self.ai_dock_restore = Some(restore.rect);
                            #[cfg(feature = "ui-qa")]
                            {
                                self.qa_dock.restore = Some(restore.rect);
                            }
                            if restore.clicked() {
                                self.ai_dock_collapsed = !self.ai_dock_collapsed;
                            }
                        }
                        if self.job.is_some() || self.ai_cancel.is_some() {
                            let spinner = ui.add(egui::Spinner::new().size(14.));
                            #[cfg(feature = "ui-qa")]
                            {
                                self.qa_dock.footer_spinner = Some(spinner.rect);
                            }
                            let _ = spinner;
                            ui.small("처리 중");
                        }
                    });
                });
            });
        #[cfg(feature = "ui-qa")]
        {
            self.qa_dock.footer = Some(footer.response.rect);
        }
        let _ = footer;
    }
    fn library(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("glyphs")
            .exact_width(252.)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(self.palette.surface)
                    .inner_margin(16),
            )
            .show(ctx, |ui| {
                if self.ai_cancel.is_some() {
                    ui.disable();
                }
                ui.label(RichText::new(&self.project.name).size(15.).strong());
                ui.add_space(6.);
                let old_group = self.group;
                egui::ComboBox::from_id_salt("collection-picker")
                    .width(214.)
                    .selected_text(GROUPS[self.group].1)
                    .show_ui(ui, |ui| {
                        for (i, (_, name, _)) in GROUPS.iter().enumerate() {
                            ui.selectable_value(&mut self.group, i, *name);
                        }
                    });
                if old_group != self.group {
                    self.search.clear();
                    self.refresh_catalog();
                }
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{}개 문자", self.catalog.len()))
                            .size(12.)
                            .color(self.palette.muted),
                    );
                    if text_button(ui, "+ 문자 추가").clicked() {
                        self.add_open = true;
                    }
                });
                ui.add_space(6.);
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut self.search)
                            .hint_text("글자 또는 U+0041 검색")
                            .desired_width(217.),
                    )
                    .changed()
                {
                    self.filter();
                }
                if ui.checkbox(&mut self.only_done, "작성한 글자만").changed() {
                    self.filter();
                }
                ui.add_space(8.);
                let count = self.filtered.len();
                let rows = count.div_ceil(3);
                let mut clicked = None;
                let locale = GROUPS[self.group].2;
                let selected = self
                    .project
                    .glyphs
                    .get(self.selected)
                    .map(|g| g.text.clone())
                    .unwrap_or_default();
                egui::ScrollArea::vertical()
                    .id_salt("glyph-grid")
                    .auto_shrink([false, false])
                    .show_rows(ui, 78., rows, |ui, range| {
                        for row in range {
                            ui.horizontal(|ui| {
                                for col in 0..3 {
                                    let Some(&cat) = self.filtered.get(row * 3 + col) else {
                                        break;
                                    };
                                    let text = &self.catalog[cat];
                                    let g = self
                                        .index
                                        .get(&(text.clone(), locale.into()))
                                        .map(|idx| &self.project.glyphs[*idx]);
                                    let is_selected = *text == selected
                                        && g.is_some_and(|v| {
                                            v.id == self.project.glyphs[self.selected].id
                                        });
                                    let (rect, r) =
                                        ui.allocate_exact_size(Vec2::new(64., 72.), Sense::click());
                                    let fill = if is_selected {
                                        self.palette.selection
                                    } else {
                                        self.palette.surface
                                    };
                                    ui.painter().rect(
                                        rect,
                                        8.,
                                        fill,
                                        Stroke::new(
                                            if is_selected { 1.5_f32 } else { 1_f32 },
                                            if is_selected {
                                                self.palette.accent
                                            } else {
                                                self.palette.line
                                            },
                                        ),
                                        egui::StrokeKind::Inside,
                                    );
                                    if let Some(g) = g.filter(|g| g.status == "edited") {
                                        ui.painter().rect_filled(
                                            rect.shrink(5.),
                                            4.,
                                            Color32::from_rgb(255, 254, 251),
                                        );
                                        paint_art(
                                            ui.ctx(),
                                            ui.painter(),
                                            rect.shrink(8.),
                                            &g.artwork,
                                            &g.id,
                                            self.revision,
                                            &mut self.textures,
                                        );
                                    } else {
                                        ui.painter().text(
                                            rect.center() - Vec2::new(0., 5.),
                                            Align2::CENTER_CENTER,
                                            display(text),
                                            FontId::proportional(if text.chars().count() > 3 {
                                                13.
                                            } else {
                                                27.
                                            }),
                                            if g.is_some_and(|g| g.status == "blank") {
                                                self.palette.muted
                                            } else {
                                                self.palette.text
                                            },
                                        );
                                    }
                                    let dot = if g.is_some_and(|g| g.status == "edited") {
                                        Color32::from_rgb(113, 145, 103)
                                    } else {
                                        self.palette.line
                                    };
                                    ui.painter().circle_filled(
                                        Pos2::new(rect.center().x, rect.bottom() - 8.),
                                        2.5,
                                        dot,
                                    );
                                    r.clone().on_hover_text(format!(
                                        "{}\n{}",
                                        codepoints(text),
                                        g.map_or("미작성", |g| status(&g.status))
                                    ));
                                    if r.clicked() {
                                        clicked = Some(text.clone());
                                    }
                                }
                            });
                        }
                    });
                if let Some(text) = clicked {
                    self.select_text(&text);
                }
            });
    }
}
fn paint_art(
    ctx: &egui::Context,
    painter: &egui::Painter,
    rect: Rect,
    art: &Artwork,
    id: &str,
    revision: u64,
    cache: &mut HashMap<String, egui::TextureHandle>,
) {
    let key = format!("{id}-{revision}");
    if !cache.contains_key(&key) {
        if cache.len() > 80 {
            cache.clear();
        }
        if let Ok(im) = font::raster(art, 512) {
            let image = egui::ColorImage::from_rgba_unmultiplied([512, 512], im.as_raw());
            cache.insert(
                key.clone(),
                ctx.load_texture(key.clone(), image, egui::TextureOptions::LINEAR),
            );
        }
    }
    if let Some(t) = cache.get(&key) {
        painter.image(
            t.id(),
            rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)),
            Color32::WHITE,
        );
    }
}
fn chip(ui: &mut egui::Ui, label: &str) {
    ui.label(
        RichText::new(label)
            .size(11.)
            .color(ui.visuals().weak_text_color()),
    );
}
fn text_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let text = ui.painter().layout_no_wrap(
        label.into(),
        FontId::proportional(12.),
        ui.visuals().text_color(),
    );
    let response = ui.add(
        egui::Button::new(RichText::new(label).size(12.))
            .frame(false)
            .small()
            .min_size(Vec2::new(text.size().x + 16., 28.)),
    );
    if response.hovered() || response.has_focus() {
        ui.painter().hline(
            response.rect.x_range().shrink(8.),
            response.rect.bottom() - 3.,
            Stroke::new(1_f32, Palette::of(ui.ctx()).accent),
        );
    }
    response
}
fn navigation_tab(
    ui: &mut egui::Ui,
    label: &str,
    selected: bool,
    palette: Palette,
) -> egui::Response {
    let response = ui.add_sized(
        [
            if label == "문장 미리보기" {
                128.
            } else {
                80.
            },
            36.,
        ],
        egui::Button::new(RichText::new(label).color(if selected {
            palette.accent
        } else {
            palette.muted
        }))
        .frame(false),
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            label,
        )
    });
    if selected || response.hovered() || response.has_focus() {
        ui.painter().hline(
            response.rect.x_range(),
            response.rect.bottom() - 1.,
            Stroke::new(if selected { 2_f32 } else { 1_f32 }, palette.accent),
        );
    }
    response
}
impl Studio {
    fn properties(&mut self, ctx: &egui::Context) {
        let openness = ctx.animate_bool_with_time_and_easing(
            egui::Id::new("properties-openness"),
            self.show_properties,
            if self.preferences.reduce_motion {
                0.
            } else {
                0.24
            },
            egui::emath::easing::cubic_out,
        );
        let panel = egui::SidePanel::right("properties")
            .exact_width(egui::lerp(48. ..=278., openness))
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(self.palette.surface)
                    .inner_margin(0),
            )
            .show(ctx, |ui| {
                if self.ai_cancel.is_some() {
                    ui.disable();
                }
                let bounds = ui.max_rect();
                let toggle_rect = Rect::from_min_size(
                    Pos2::new(bounds.right() - 40., bounds.top() + 12.),
                    Vec2::splat(32.),
                );
                let toggle = ui
                    .put(
                        toggle_rect,
                        egui::Button::new(
                            RichText::new(if self.show_properties { "›" } else { "‹" }).size(24.),
                        )
                        .frame(false),
                    )
                    .on_hover_text(if self.show_properties {
                        "레이어·속성 접기"
                    } else {
                        "레이어·속성 펼치기"
                    });
                #[cfg(feature = "ui-qa")]
                {
                    self.qa_settings.sidebar_toggle = Some(toggle.rect);
                }
                if toggle.clicked() {
                    self.show_properties = !self.show_properties;
                }
                if openness > 0.6 {
                    ui.painter().with_clip_rect(bounds).text(
                        Pos2::new(bounds.left() + 18., bounds.top() + 28.),
                        Align2::LEFT_CENTER,
                        "레이어 · 속성",
                        FontId::proportional(16.),
                        self.palette.text.gamma_multiply(openness),
                    );
                } else if openness == 0. {
                    ui.painter().text(
                        Pos2::new(bounds.center().x, bounds.top() + 64.),
                        Align2::CENTER_CENTER,
                        "레이어\n속성",
                        FontId::proportional(11.),
                        self.palette.muted,
                    );
                }
                if openness == 0. {
                    return;
                }
                let body_rect = Rect::from_min_size(
                    bounds.min + Vec2::new(18., 56.),
                    Vec2::new(242., (bounds.height() - 68.).max(0.)),
                );
                let mut body = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(body_rect)
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                );
                body.set_clip_rect(bounds.intersect(body_rect));
                body.set_opacity(openness);
                if openness < 1. {
                    body.disable();
                }
                self.properties_contents(&mut body);
            });
        #[cfg(feature = "ui-qa")]
        {
            self.qa_settings.sidebar = Some(panel.response.rect);
        }
        let _ = panel;
    }
    fn properties_contents(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .id_salt("properties-scroll")
            .show(ui, |ui| {
                if self.project.glyphs.is_empty() {
                    return;
                }
                let g = &self.project.glyphs[self.selected];
                let text = g.text.clone();
                let mut locked = g.locked;
                let current_status = g.status.clone();
                let locale = g.locale.clone();
                let mapping = g.mapping.clone();
                let mut advance = g.artwork.advance;
                ui.add_space(9.);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("글리프 속성").size(17.).strong());
                });
                let candidate_count = g.candidates.len();
                ui.add_space(12.);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(display(&text)).size(30.));
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(codepoints(&text))
                                .size(10.)
                                .color(self.palette.muted),
                        );
                        chip(
                            ui,
                            match locale.as_str() {
                                "ko" => "한국어 자형",
                                "ja" => "일본어 자형",
                                _ => "공통 자형",
                            },
                        );
                    });
                });
                chip(
                    ui,
                    match mapping.as_str() {
                        "control" => "제어 문자 · 확인 전용",
                        "custom" => "사용자 지정 문자열 매핑",
                        "sequence" => "유니코드 시퀀스 · 원문 보존",
                        "pua" => "사적 사용 영역 · 사용자 정의",
                        _ => "단일 유니코드 문자",
                    },
                );
                if candidate_count > 0
                    && ui
                        .button(format!("AI 후보 {candidate_count}개 보기 · 채택"))
                        .clicked()
                {
                    self.open_candidate_review();
                }
                ui.add_space(7.);
                if ui.checkbox(&mut locked, "글리프 잠금").changed() {
                    self.edit(|g| {
                        g.locked = locked;
                        Ok(())
                    });
                }
                ui.add_enabled_ui(!locked && mapping != "control", |ui| {
                    let mut state = current_status.clone();
                    egui::ComboBox::from_id_salt("status")
                        .selected_text(status(&state))
                        .show_ui(ui, |ui| {
                            for v in ["missing", "edited", "blank", "system", "unsupported"] {
                                ui.selectable_value(&mut state, v.into(), status(v));
                            }
                        });
                    if state != current_status {
                        self.edit(|g| {
                            g.status = state;
                            Ok(())
                        });
                    }
                    ui.add_space(6.);
                    ui.label(RichText::new("글자 폭").size(12.).color(self.palette.muted));
                    if ui
                        .add(
                            egui::DragValue::new(&mut advance)
                                .range(0.0..=4000.)
                                .speed(5.)
                                .suffix(" units"),
                        )
                        .changed()
                    {
                        self.edit(|g| {
                            check(editable(g), "템플릿 편집 권한이 없어요.")?;
                            g.artwork.advance = advance;
                            if g.status == "missing" {
                                g.status = "blank".into();
                            }
                            Ok(())
                        });
                    }
                });
                ui.add_space(15.);
                ui.separator();
                ui.add_space(10.);
                ui.horizontal(|ui| {
                    ui.strong("레이어");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        chip(
                            ui,
                            &format!(
                                "{}개",
                                self.project.glyphs[self.selected].artwork.shapes.len()
                            ),
                        );
                    });
                });
                ui.add_space(6.);
                let layers: Vec<_> = self.project.glyphs[self.selected]
                    .artwork
                    .shapes
                    .iter()
                    .enumerate()
                    .map(|(i, s)| (i, s.name.clone(), s.visible, s.locked))
                    .collect();
                let enabled = editable(&self.project.glyphs[self.selected]);
                for (i, name, mut visible, mut lock) in layers.into_iter().rev() {
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(enabled, egui::Checkbox::without_text(&mut visible))
                            .changed()
                        {
                            self.edit(|g| {
                                g.artwork.shapes[i].visible = visible;
                                Ok(())
                            });
                        }
                        if ui
                            .selectable_label(self.layer == Some(i), RichText::new(name).size(12.))
                            .clicked()
                        {
                            self.layer = Some(i);
                        }
                        if ui
                            .add_enabled(
                                enabled,
                                egui::Button::new(if lock { "잠김" } else { "잠금" })
                                    .frame(false)
                                    .min_size(Vec2::new(44., 28.)),
                            )
                            .clicked()
                        {
                            lock = !lock;
                            self.edit(|g| {
                                g.artwork.shapes[i].locked = lock;
                                Ok(())
                            });
                        }
                    });
                }
                if let Some(i) = self
                    .layer
                    .filter(|i| *i < self.project.glyphs[self.selected].artwork.shapes.len())
                {
                    let s = &self.project.glyphs[self.selected].artwork.shapes[i];
                    let mut opacity = s.opacity;
                    let mut stroke = s.stroke;
                    let mut fill = s.fill;
                    let mut xy = [s.x, s.y, s.width, s.height];
                    let is_path = s.kind == "path";
                    let is_image = s.kind == "image";
                    let is_outline = s.kind == "outline";
                    let layer_enabled = enabled && !s.locked;
                    let mut rgba = color32(&s.color);
                    let mut name = s.name.clone();
                    ui.add_space(8.);
                    ui.add_enabled_ui(layer_enabled, |ui| {
                        if ui.text_edit_singleline(&mut name).changed() {
                            self.edit(|g| {
                                g.artwork.shapes[i].name = name;
                                Ok(())
                            });
                        }
                        ui.horizontal(|ui| {
                            if ui.color_edit_button_srgba(&mut rgba).changed() {
                                let c =
                                    format!("#{:02x}{:02x}{:02x}", rgba.r(), rgba.g(), rgba.b());
                                self.edit(|g| {
                                    g.artwork.shapes[i].color = c;
                                    Ok(())
                                });
                            }
                            if !is_outline && ui.checkbox(&mut fill, "채우기").changed() {
                                self.edit(|g| {
                                    g.artwork.shapes[i].fill = fill;
                                    Ok(())
                                });
                            }
                        });
                        if ui
                            .add(egui::Slider::new(&mut opacity, 0.0..=1.).text("불투명도"))
                            .changed()
                        {
                            self.edit(|g| {
                                g.artwork.shapes[i].opacity = opacity;
                                Ok(())
                            });
                        }
                        if !is_image
                            && !is_outline
                            && ui
                                .add(egui::Slider::new(&mut stroke, 1.0..=180.).text("획 굵기"))
                                .changed()
                        {
                            self.edit(|g| {
                                g.artwork.shapes[i].stroke = stroke;
                                Ok(())
                            });
                        }
                        if !is_path {
                            ui.horizontal(|ui| {
                                ui.small("X / Y");
                                let changed = ui
                                    .add(egui::DragValue::new(&mut xy[0]).range(-4000.0..=4000.))
                                    .changed()
                                    | ui.add(
                                        egui::DragValue::new(&mut xy[1]).range(-4000.0..=4000.),
                                    )
                                    .changed();
                                if changed {
                                    self.edit(|g| {
                                        let s = &mut g.artwork.shapes[i];
                                        s.x = xy[0];
                                        s.y = xy[1];
                                        Ok(())
                                    });
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.small("W / H");
                                let changed = ui
                                    .add(egui::DragValue::new(&mut xy[2]).range(1.0..=4000.))
                                    .changed()
                                    | ui.add(egui::DragValue::new(&mut xy[3]).range(1.0..=4000.))
                                        .changed();
                                if changed {
                                    self.edit(|g| {
                                        let s = &mut g.artwork.shapes[i];
                                        s.width = xy[2];
                                        s.height = xy[3];
                                        Ok(())
                                    });
                                }
                            });
                        }
                        ui.horizontal(|ui| {
                            if text_button(ui, "복제").clicked() {
                                self.edit(|g| {
                                    let mut s = g.artwork.shapes[i].clone();
                                    s.id = uid();
                                    s.x += 30.;
                                    s.y += 30.;
                                    for pt in &mut s.points {
                                        pt[0] += 30.;
                                        pt[1] += 30.;
                                    }
                                    g.artwork.shapes.push(s);
                                    Ok(())
                                });
                            }
                            if text_button(ui, "위로").clicked()
                                && i + 1 < self.project.glyphs[self.selected].artwork.shapes.len()
                            {
                                self.edit(|g| {
                                    g.artwork.shapes.swap(i, i + 1);
                                    Ok(())
                                });
                                self.layer = Some(i + 1);
                            }
                            if text_button(ui, "아래로").clicked() && i > 0 {
                                self.edit(|g| {
                                    g.artwork.shapes.swap(i, i - 1);
                                    Ok(())
                                });
                                self.layer = Some(i - 1);
                            }
                            if text_button(ui, "삭제").clicked() {
                                self.edit(|g| {
                                    g.artwork.shapes.remove(i);
                                    Ok(())
                                });
                                self.layer = None;
                            }
                        });
                        if is_path && text_button(ui, "선 부드럽게 보정").clicked() {
                            self.edit(|g| {
                                smooth(&mut g.artwork.shapes[i].points);
                                Ok(())
                            });
                        }
                        if is_image && text_button(ui, "이미지 → 단색 벡터 변환").clicked()
                        {
                            let polygons = font::polygons(
                                &self.project.glyphs[self.selected].artwork.shapes[i],
                            );
                            match polygons {
                                Ok(polys) => self.edit(|g| {
                                    if g.baseline.is_none() {
                                        g.baseline = Some(g.artwork.clone());
                                    }
                                    g.artwork.shapes[i].visible = false;
                                    g.artwork.shapes[i].locked = true;
                                    g.artwork.shapes[i].name = "원본 이미지 · 숨김 보관".into();
                                    for pts in polys {
                                        let mut s = path(&pts);
                                        s.fill = true;
                                        g.artwork.shapes.push(s);
                                    }
                                    Ok(())
                                }),
                                Err(e) => self.notify(e),
                            }
                            self.layer = None;
                        }
                    });
                }
                ui.add_space(14.);
                ui.separator();
                ui.add_space(10.);
                ui.strong("원본과 기준 디자인");
                ui.add_space(8.);
                let g = &self.project.glyphs[self.selected];
                if let Some(t) = &g.template {
                    chip(ui, &format!("{} · v{}", t.name, t.version));
                    chip(ui, &format!("{} / {}", t.provider, t.license));
                } else {
                    chip(ui, "템플릿 연결 없음");
                }
                let original = g.template.as_ref().and_then(|t| t.original.as_ref());
                if let Some(art) = original {
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(72., 72.), Sense::hover());
                    paint_art(
                        ui.ctx(),
                        ui.painter(),
                        rect,
                        art,
                        "original",
                        self.revision,
                        &mut self.textures,
                    );
                }
                ui.add_enabled_ui(!locked && mapping != "control", |ui| {
                    if ui.button("템플릿 원본으로 복원").clicked() {
                        self.edit(|g| restore(g, "template"));
                    }
                    ui.horizontal(|ui| {
                        if ui.button("기준 저장").clicked() {
                            self.edit(|g| {
                                g.baseline = Some(g.artwork.clone());
                                Ok(())
                            });
                        }
                        if ui.button("기준 복원").clicked() {
                            self.edit(|g| restore(g, "baseline"));
                        }
                    });
                    if ui.button("사용자 정의 해제 · 시스템 표시").clicked() {
                        self.edit(|g| restore(g, "system"));
                    }
                    if ui.button("선택한 변형에 그림 복사…").clicked() {
                        self.variant_targets.clear();
                        self.variant_open = true;
                    }
                });
                ui.add_space(14.);
                ui.separator();
                ui.add_space(10.);
                egui::CollapsingHeader::new("부품 보관함").show(ui, |ui| {
                    ui.text_edit_singleline(&mut self.component_name);
                    if ui.button("현재 그림을 부품으로 저장").clicked() {
                        let g = &self.project.glyphs[self.selected];
                        self.project.components.push(Component {
                            id: uid(),
                            name: self.component_name.clone(),
                            artwork: g.artwork.clone(),
                            template: g.template.clone(),
                            sources: g.sources.clone(),
                        });
                        self.changed();
                    }
                    let entries: Vec<_> = self
                        .project
                        .components
                        .iter()
                        .enumerate()
                        .map(|(i, c)| (i, c.name.clone()))
                        .collect();
                    for (i, name) in entries {
                        if ui.button(format!("+ {name}")).clicked() {
                            let c = self.project.components[i].clone();
                            self.edit(|g| {
                                check(editable(g), "글리프가 보호되어 있어요.")?;
                                check(
                                    c.template
                                        .iter()
                                        .chain(&c.sources)
                                        .all(|t| t.permissions.edit),
                                    "부품의 템플릿에 편집 권한이 없어요.",
                                )?;
                                for t in c.template.into_iter().chain(c.sources) {
                                    if !g.sources.iter().any(|v| v.id == t.id) {
                                        g.sources.push(t)
                                    }
                                }
                                for mut s in c.artwork.shapes {
                                    s.id = uid();
                                    g.artwork.shapes.push(s);
                                }
                                g.status = "edited".into();
                                Ok(())
                            });
                        }
                    }
                });
                egui::CollapsingHeader::new("AI 후보").show(ui, |ui| {
                    if ui
                        .add_enabled(
                            self.job.is_none(),
                            egui::Button::new("현재 글리프 후보 생성"),
                        )
                        .clicked()
                    {
                        self.generate_ai(ui.ctx());
                    }
                    if ui.button("후보 비교 화면 열기").clicked() {
                        self.open_candidate_review();
                    }
                    let candidates: Vec<_> = self.project.glyphs[self.selected]
                        .candidates
                        .iter()
                        .map(|c| (c.id.clone(), c.name.clone()))
                        .collect();
                    for (id, name) in candidates {
                        ui.horizontal(|ui| {
                            let art = &self.project.glyphs[self.selected]
                                .candidates
                                .iter()
                                .find(|c| c.id == id)
                                .unwrap()
                                .artwork;
                            let (rect, _) =
                                ui.allocate_exact_size(Vec2::splat(56.), Sense::hover());
                            paint_art(
                                ui.ctx(),
                                ui.painter(),
                                rect,
                                art,
                                &id,
                                self.revision,
                                &mut self.textures,
                            );
                            ui.vertical(|ui| {
                                chip(ui, &name);
                                if text_button(ui, "채택").clicked() {
                                    self.edit(|g| accept(g, &id));
                                }
                            });
                        });
                    }
                });
            });
    }
    fn editor(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(self.palette.background)
                    .inner_margin(22),
            )
            .show(ctx, |ui| {
                if self.ai_cancel.is_some() {
                    ui.disable();
                }
                if self.project.glyphs.is_empty() {
                    ui.label("문자를 추가해 작업을 시작하세요.");
                    return;
                }
                let g = &self.project.glyphs[self.selected];
                let text = g.text.clone();
                let locale = g.locale.clone();
                let current_status = g.status.clone();
                let enabled = editable(g) && g.mapping != "control";
                let advance = g.artwork.advance;
                let id = g.id.clone();
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(format!("{}  ·  나의 글자", display(&text)))
                                .size(24.)
                                .strong(),
                        );
                        ui.label(
                            RichText::new(format!(
                                "{}   /   {}",
                                codepoints(&text),
                                status(&current_status)
                            ))
                            .size(11.)
                            .color(self.palette.muted),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add_enabled(!self.redo.is_empty(), egui::Button::new("↪"))
                            .on_hover_text("다시 실행 Ctrl+Y")
                            .clicked()
                        {
                            self.undo(true)
                        }
                        if ui
                            .add_enabled(!self.history.is_empty(), egui::Button::new("↶"))
                            .on_hover_text("실행 취소 Ctrl+Z")
                            .clicked()
                        {
                            self.undo(false)
                        }
                    });
                });
                ui.add_space(17.);
                egui::Frame::new()
                    .fill(self.palette.surface)
                    .corner_radius(9)
                    .inner_margin(8)
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            for (i, label) in ["펜", "직선", "네모", "동그라미", "이동"]
                                .iter()
                                .enumerate()
                            {
                                if ui.selectable_label(self.tool == i, *label).clicked() {
                                    self.tool = i;
                                }
                            }
                            ui.separator();
                            ui.color_edit_button_srgba(&mut self.color);
                            ui.add(
                                egui::DragValue::new(&mut self.width)
                                    .range(1.0..=180.)
                                    .suffix(" px"),
                            );
                            ui.checkbox(&mut self.fill, "채우기");
                            if text_button(ui, "이미지 +").clicked() {
                                self.import_image()
                            }
                        });
                    });
                ui.add_space(10.);
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.guides, "가이드");
                    ui.checkbox(&mut self.ghost, "밑글자");
                    for hex in &self.project.palette {
                        if ui
                            .add(
                                egui::Button::new(" ")
                                    .fill(color32(hex))
                                    .min_size(Vec2::splat(18.)),
                            )
                            .on_hover_text(hex)
                            .clicked()
                        {
                            self.color = color32(hex);
                        }
                    }
                    if ui
                        .add(
                            egui::Button::new("+")
                                .frame(false)
                                .min_size(Vec2::splat(28.)),
                        )
                        .on_hover_text("현재 색을 팔레트에 저장")
                        .clicked()
                    {
                        let hex = format!(
                            "#{:02x}{:02x}{:02x}",
                            self.color.r(),
                            self.color.g(),
                            self.color.b()
                        );
                        if self.project.palette.len() < 64 && !self.project.palette.contains(&hex) {
                            self.project.palette.push(hex);
                            self.changed();
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add(
                            egui::Slider::new(&mut self.zoom, 0.6..=1.6)
                                .show_value(false)
                                .text("확대"),
                        );
                    });
                });
                ui.add_space(6.);
                let avail = ui.available_size() - Vec2::new(0., 68.);
                let side = avail.x.min(avail.y).max(260.) * self.zoom;
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .max_height(avail.y.max(260.))
                    .show(ui, |ui| {
                        let (outer, response) =
                            ui.allocate_exact_size(Vec2::splat(side), Sense::click_and_drag());
                        #[cfg(feature = "ui-qa")]
                        {
                            self.qa_canvas = Some(outer);
                        }
                        let painter = ui.painter_at(outer);
                        painter.rect_filled(outer, 12., Color32::from_rgb(255, 254, 251));
                        let to_screen = |p: [f64; 2]| {
                            Pos2::new(
                                outer.left() + p[0] as f32 / 1000. * side,
                                outer.top() + p[1] as f32 / 1000. * side,
                            )
                        };
                        for i in 1..20 {
                            for j in 1..20 {
                                painter.circle_filled(
                                    to_screen([i as f64 * 50., j as f64 * 50.]),
                                    0.8,
                                    Color32::from_rgb(226, 227, 217),
                                );
                            }
                        }
                        if self.ghost
                            && !control(&text)
                            && !matches!(current_status.as_str(), "system" | "blank")
                        {
                            painter.text(
                                outer.center(),
                                Align2::CENTER_CENTER,
                                &text,
                                FontId::proportional(side * 0.62),
                                Color32::from_rgba_unmultiplied(160, 163, 145, 30),
                            );
                        }
                        if self.guides {
                            for (y, name, [r, g, b]) in doodle_core::glyph_layout::guides() {
                                let c = Color32::from_rgb(r, g, b);
                                painter.line_segment(
                                    [to_screen([0., y]), to_screen([1000., y])],
                                    Stroke::new(1_f32, c),
                                );
                                painter.text(
                                    to_screen([12., y - 10.]),
                                    Align2::LEFT_BOTTOM,
                                    name,
                                    FontId::proportional(10.),
                                    c,
                                );
                            }
                            for x in doodle_core::glyph_layout::vertical_guides(advance) {
                                painter.line_segment(
                                    [to_screen([x, 0.]), to_screen([x, 1000.])],
                                    Stroke::new(1_f32, Color32::from_rgb(161, 181, 156)),
                                );
                            }
                        }
                        let g = &self.project.glyphs[self.selected];
                        if current_status == "system"
                            || current_status == "missing" && control(&text)
                        {
                            if let Some(tex) = system_texture(
                                ctx,
                                &text,
                                &locale,
                                side * 0.6,
                                &mut self.system_fonts,
                                &mut self.textures,
                            ) {
                                let size = tex.size_vec2();
                                let scale =
                                    (side * 0.88 / size.x).min(side * 0.88 / size.y).min(1.);
                                painter.image(
                                    tex.id(),
                                    Rect::from_center_size(outer.center(), size * scale),
                                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)),
                                    Color32::WHITE,
                                );
                            } else {
                                painter.text(
                                    outer.center(),
                                    Align2::CENTER_CENTER,
                                    display(&text),
                                    FontId::proportional(side * 0.45),
                                    self.palette.muted,
                                );
                            }
                        } else if current_status != "blank" && current_status != "unsupported" {
                            paint_art(
                                ctx,
                                &painter,
                                outer,
                                &g.artwork,
                                &id,
                                self.revision,
                                &mut self.textures,
                            );
                        }
                        if let Some(i) = self.layer.filter(|i| *i < g.artwork.shapes.len()) {
                            let s = &g.artwork.shapes[i];
                            let bounds = shape_bounds(s);
                            let rect = Rect::from_min_max(
                                to_screen([bounds[0], bounds[1]]),
                                to_screen([bounds[2], bounds[3]]),
                            );
                            painter.rect_stroke(
                                rect,
                                0.,
                                Stroke::new(1_f32, self.palette.accent),
                                egui::StrokeKind::Outside,
                            );
                            if s.kind == "path" {
                                for pt in s.points.iter().step_by((s.points.len() / 24).max(1)) {
                                    painter.circle_filled(to_screen(*pt), 2.5, self.palette.accent);
                                }
                            }
                        }
                        let point = response.interact_pointer_pos().map(|p| {
                            [
                                ((p.x - outer.left()) / side * 1000.).clamp(0., 1000.) as f64,
                                ((p.y - outer.top()) / side * 1000.).clamp(0., 1000.) as f64,
                            ]
                        });
                        if response.drag_started() && enabled {
                            self.ai_dock_collapsed = true;
                            if let Some(pt) = point {
                                self.drag_start = Some(pt);
                                if self.tool == 4 {
                                    if self.layer.is_none() {
                                        self.layer = g
                                            .artwork
                                            .shapes
                                            .iter()
                                            .enumerate()
                                            .rev()
                                            .find(|(_, s)| {
                                                let b = shape_bounds(s);
                                                s.visible
                                                    && !s.locked
                                                    && pt[0] >= b[0] - 20.
                                                    && pt[0] <= b[2] + 20.
                                                    && pt[1] >= b[1] - 20.
                                                    && pt[1] <= b[3] + 20.
                                            })
                                            .map(|(i, _)| i);
                                    }
                                    self.draft = self
                                        .layer
                                        .and_then(|i| g.artwork.shapes.get(i))
                                        .filter(|s| !s.locked)
                                        .cloned();
                                } else {
                                    let mut s = new_shape(match self.tool {
                                        2 => "rect",
                                        3 => "ellipse",
                                        _ => "path",
                                    });
                                    s.x = pt[0];
                                    s.y = pt[1];
                                    s.width = 0.;
                                    s.height = 0.;
                                    s.points.push(pt);
                                    s.stroke = self.width;
                                    s.fill = self.fill && self.tool != 1;
                                    s.color = format!(
                                        "#{:02x}{:02x}{:02x}",
                                        self.color.r(),
                                        self.color.g(),
                                        self.color.b()
                                    );
                                    self.draft = Some(s);
                                }
                            }
                        }
                        if response.dragged() {
                            if let (Some(pt), Some(start), Some(s)) =
                                (point, self.drag_start, self.draft.as_mut())
                            {
                                match self.tool {
                                    0 => {
                                        if s.points
                                            .last()
                                            .is_none_or(|p| (p[0] - pt[0]).hypot(p[1] - pt[1]) > 4.)
                                        {
                                            s.points.push(pt);
                                        }
                                    }
                                    1 => {
                                        s.points = vec![start, pt];
                                    }
                                    2 | 3 => {
                                        s.x = start[0].min(pt[0]);
                                        s.y = start[1].min(pt[1]);
                                        s.width = (pt[0] - start[0]).abs();
                                        s.height = (pt[1] - start[1]).abs();
                                    }
                                    4 => {
                                        if let Some(i) = self.layer {
                                            let original = &self.project.glyphs[self.selected]
                                                .artwork
                                                .shapes[i];
                                            let dx = pt[0] - start[0];
                                            let dy = pt[1] - start[1];
                                            s.x = original.x + dx;
                                            s.y = original.y + dy;
                                            s.points = original
                                                .points
                                                .iter()
                                                .map(|p| [p[0] + dx, p[1] + dy])
                                                .collect();
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        if let Some(s) = &self.draft {
                            let rgba = color32(&s.color);
                            if s.kind == "path" {
                                let pts: Vec<_> = s.points.iter().map(|p| to_screen(*p)).collect();
                                painter.add(egui::Shape::line(
                                    pts,
                                    Stroke::new(s.stroke as f32 / 1000. * side, rgba),
                                ));
                            } else {
                                let r = Rect::from_min_max(
                                    to_screen([s.x, s.y]),
                                    to_screen([s.x + s.width, s.y + s.height]),
                                );
                                if s.kind == "ellipse" {
                                    painter.add(egui::Shape::ellipse_stroke(
                                        r.center(),
                                        r.size() / 2.,
                                        Stroke::new(s.stroke as f32 / 1000. * side, rgba),
                                    ));
                                } else {
                                    painter.rect_stroke(
                                        r,
                                        0.,
                                        Stroke::new(s.stroke as f32 / 1000. * side, rgba),
                                        egui::StrokeKind::Inside,
                                    );
                                }
                            }
                        }
                        if response.drag_stopped() {
                            if let Some(mut s) = self.draft.take() {
                                if self.tool == 0 {
                                    smooth(&mut s.points);
                                }
                                let tool = self.tool;
                                let layer = self.layer;
                                self.edit(|g| {
                                    check(editable(g), "글리프가 보호되어 있어요.")?;
                                    if tool == 4 {
                                        if let Some(i) = layer {
                                            g.artwork.shapes[i] = s;
                                        }
                                    } else {
                                        g.artwork.shapes.push(s);
                                    }
                                    g.status = "edited".into();
                                    Ok(())
                                });
                                if tool != 4 {
                                    self.layer = Some(
                                        self.project.glyphs[self.selected].artwork.shapes.len() - 1,
                                    );
                                }
                            }
                            self.drag_start = None;
                        }
                        if response.clicked() && self.tool == 0 && enabled {
                            if let Some(pt) = point {
                                let mut s = path(&[pt]);
                                s.stroke = self.width;
                                s.color = format!(
                                    "#{:02x}{:02x}{:02x}",
                                    self.color.r(),
                                    self.color.g(),
                                    self.color.b()
                                );
                                self.edit(|g| {
                                    g.artwork.shapes.push(s);
                                    g.status = "edited".into();
                                    Ok(())
                                });
                            }
                        }
                    });
                ui.add_space(10.);
                ui.horizontal(|ui| {
                    chip(
                        ui,
                        if enabled {
                            "드래그해서 그려 보세요.  Ctrl+Z 실행 취소"
                        } else {
                            "현재 글리프는 읽기 전용이에요."
                        },
                    );
                    if locale == "ko"
                        && text
                            .chars()
                            .next()
                            .is_some_and(|c| ('가'..='힣').contains(&c))
                        && text_button(ui, "자모로 조합").clicked()
                    {
                        match compose_hangul(&self.project, &text) {
                            Ok((a, sources)) => self.edit(|g| {
                                check(editable(g), "글리프가 보호되어 있어요.")?;
                                g.artwork = a;
                                for t in sources {
                                    if !g.sources.iter().any(|v| v.id == t.id) {
                                        g.sources.push(t);
                                    }
                                }
                                g.status = "edited".into();
                                Ok(())
                            }),
                            Err(e) => self.notify(e),
                        }
                    }
                });
            });
    }
}
fn color32(s: &str) -> Color32 {
    Color32::from_rgb(
        u8::from_str_radix(&s[1..3], 16).unwrap_or(0),
        u8::from_str_radix(&s[3..5], 16).unwrap_or(0),
        u8::from_str_radix(&s[5..7], 16).unwrap_or(0),
    )
}
fn font_texture(
    ctx: &egui::Context,
    bytes: &[u8],
    text: &str,
    locale: &str,
    size: f32,
    key: &str,
    cache: &mut HashMap<String, egui::TextureHandle>,
) -> Option<egui::TextureHandle> {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    locale.hash(&mut hasher);
    let key = format!("{key}-{}-{size}", hasher.finish());
    if !cache.contains_key(&key) {
        if cache.len() > 100 {
            cache.clear();
        }
        if let Ok(im) = doodle_core::render::text(bytes, text, locale, size) {
            let texture = ctx.load_texture(
                key.clone(),
                egui::ColorImage::from_rgba_unmultiplied(
                    [im.width() as usize, im.height() as usize],
                    im.as_raw(),
                ),
                egui::TextureOptions::LINEAR,
            );
            cache.insert(key.clone(), texture);
        }
    }
    cache.get(&key).cloned()
}
fn system_texture(
    ctx: &egui::Context,
    text: &str,
    locale: &str,
    size: f32,
    fonts: &mut HashMap<String, Vec<u8>>,
    cache: &mut HashMap<String, egui::TextureHandle>,
) -> Option<egui::TextureHandle> {
    let emoji = text.chars().any(|c| c as u32 >= 0x1f000 || c == '\u{fe0f}');
    let candidates: &[&str] = if emoji {
        &["seguiemj.ttf", "seguisym.ttf", "malgun.ttf"]
    } else if locale == "ja" {
        &["YuGothM.ttc", "meiryo.ttc", "msgothic.ttc", "malgun.ttf"]
    } else {
        &["malgun.ttf", "segoeui.ttf"]
    };
    let name = *candidates
        .iter()
        .find(|name| std::path::Path::new(&format!("C:/Windows/Fonts/{name}")).exists())?;
    if !fonts.contains_key(name) {
        let bytes = std::fs::read(format!("C:/Windows/Fonts/{name}")).ok()?;
        fonts.insert(name.into(), bytes);
    }
    font_texture(ctx, &fonts[name], text, locale, size, name, cache)
}
impl Studio {
    fn preview_ui(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.);
        ui.label(
            RichText::new("글자가 모이면, 나의 이야기가 돼요.")
                .size(28.)
                .strong(),
        );
        ui.label(
            RichText::new("작성한 그림과 시스템 대체 표시를 나란히 확인하세요.")
                .color(self.palette.muted),
        );
        ui.add_space(24.);
        ui.horizontal(|ui| {
            ui.add(egui::Slider::new(&mut self.preview_size, 24.0..=120.).text("크기"));
            egui::ComboBox::from_id_salt("preview-locale")
                .selected_text(match self.preview_locale.as_str() {
                    "ko" => "한국어 자형",
                    "ja" => "일본어 자형",
                    _ => "공통 자형",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.preview_locale, "ko".into(), "한국어 자형");
                    ui.selectable_value(&mut self.preview_locale, "ja".into(), "일본어 자형");
                    ui.selectable_value(&mut self.preview_locale, "default".into(), "공통 자형");
                });
            ui.checkbox(&mut self.preview_controls, "비출력 문자 표시");
        });
        ui.add_space(10.);
        ui.add(
            egui::TextEdit::multiline(&mut self.preview_text)
                .desired_rows(3)
                .desired_width(f32::INFINITY)
                .hint_text("미리 볼 문장을 입력하세요"),
        );
        ui.add_space(20.);
        egui::ScrollArea::vertical().show(ui,|ui|{
            egui::Frame::new().fill(Color32::from_rgb(255,254,251)).stroke(Stroke::new(1_f32,self.palette.line)).corner_radius(16).inner_margin(24).show(ui,|ui|{ui.visuals_mut().override_text_color=Some(DRAW_INK);ui.visuals_mut().weak_text_color=Some(Palette::new(false).muted);
                ui.horizontal(|ui|{chip(ui,"PROJECT PREVIEW");chip(ui,"● 프로젝트 그림");ui.label(RichText::new("○ 시스템 대체").size(11.).color(Palette::new(false).muted));});ui.add_space(20.);
                for line in self.preview_text.split('\n'){ui.horizontal_wrapped(|ui|{ui.spacing_mut().item_spacing.x=0.;for(text,index)in preview(line,&self.project.glyphs,&self.preview_locale){let g=index.map(|i|&self.project.glyphs[i]);let size=self.preview_size;if let Some(g)=g.filter(|g|g.status=="edited"||g.status=="blank"){
                    let width=(size*g.artwork.advance as f32/1000.).max(1.);let(rect,r)=ui.allocate_exact_size(Vec2::new(width,size+14.),Sense::hover());if g.status=="edited"{paint_art(ui.ctx(),ui.painter(),Rect::from_min_size(rect.min,Vec2::splat(size)),&g.artwork,&g.id,self.revision,&mut self.textures);}ui.painter().line_segment([rect.left_bottom(),rect.right_bottom()],Stroke::new(2_f32,Color32::from_rgb(119,150,111)));r.on_hover_text(format!("프로젝트 글리프 · {}",codepoints(text)));
                }else if control(text){if self.preview_controls{ui.label(RichText::new(display(text)).size(size*0.4).color(Palette::new(false).muted));}}
                else if let Some(tex)=system_texture(ui.ctx(),if self.preview_controls&&text==" "{"·"}else{text},&self.preview_locale,size*0.78,&mut self.system_fonts,&mut self.textures){ui.image((tex.id(),tex.size_vec2())).on_hover_text(format!("시스템 폰트 대체 · {}",codepoints(text)));}
                else{ui.label(RichText::new(text).size(size*0.78).color(Palette::new(false).muted));}
                }if self.preview_controls{ui.label(RichText::new("↵").size(16.).color(Palette::new(false).muted));}});ui.add_space(10.);}
            });ui.add_space(20.);
            ui.horizontal(|ui|{ui.label(RichText::new("실제 출력 폰트 확인").size(20.).strong());if ui.add_enabled(self.job.is_none(),egui::Button::new("TTF 생성 · 검사")).clicked(){let p=self.project.clone();let locale=self.preview_locale.clone();self.spawn(ui.ctx(),move||{let r=font::export(&p,&font::ExportOptions{mode:Some("color".into()),locale:Some(locale),..Default::default()})?;let bytes=STANDARD.decode(&r.data).map_err(|e|e.to_string())?;let report=r.reports.iter().map(|r|format!("{}개 글리프 · {}\n{}",r.glyph_count,r.tables.join(" / "),r.warnings.join("\n"))).collect::<Vec<_>>().join("\n");Ok(Job::FontPreview(bytes,report))});}});
            chip(ui,"실제 생성한 TTF를 다시 읽고 Rustybuzz로 문자 치환을 시험합니다. 외부 앱의 컬러·시퀀스 지원은 다를 수 있어요.");
            if let Some(bytes)=&self.actual_font{
                ui.add_space(10.);chip(ui,"이 아래는 실제 TTF만 사용합니다. 폰트에 없는 글자는 빈칸으로 표시됩니다.");
                for line in self.preview_text.lines(){if let Some(tex)=font_texture(ui.ctx(),bytes,line,&self.preview_locale,self.preview_size,&format!("actual-{}",self.revision),&mut self.textures){egui::Frame::new().fill(Color32::from_rgb(255,254,251)).inner_margin(8).show(ui,|ui|{ui.image((tex.id(),tex.size_vec2()));});}}
                egui::CollapsingHeader::new("폰트 구조 검사 결과").show(ui,|ui|{ui.label(&self.export_report);});
            }
        });
    }
    fn templates_ui(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.);
        ui.label(RichText::new("좋은 시작을 모아 두는 곳").size(28.).strong());
        ui.label(
            RichText::new(
                "원본과 이용 조건을 함께 보관해요. 적용해도 문자 매핑은 그대로 유지됩니다.",
            )
            .color(self.palette.muted),
        );
        ui.add_space(24.);
        egui::ScrollArea::vertical().show(ui,|ui|{
            let mut apply = None;
            #[cfg(feature = "ui-qa")]
            { self.qa_settings.template_cards.clear(); }
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::splat(16.);
                for t in &self.project.templates {
                    let card = egui::Frame::new().fill(self.palette.raised)
                        .stroke(Stroke::new(1_f32, self.palette.line)).corner_radius(14).inner_margin(18)
                        .show(ui, |ui| {
                            let (content, _) = ui.allocate_exact_size(Vec2::new(244., 368.), Sense::hover());
                            let mut card_ui = ui.new_child(egui::UiBuilder::new().max_rect(content).layout(egui::Layout::top_down(egui::Align::Min)));
                            let ui = &mut card_ui;
                            // Fixed rows keep every card and action aligned. Long rights text scrolls.
                            let top = content.min;
                            let rect = Rect::from_center_size(Pos2::new(top.x + 122., top.y + 80.), Vec2::splat(160.));
                            ui.painter().rect_filled(rect, 8., Color32::from_rgb(255,254,251));
                            if let Some(art) = &t.original {
                                paint_art(ui.ctx(), ui.painter(), rect, art, &t.id, self.revision, &mut self.textures);
                            } else {
                                ui.painter().text(rect.center(), Align2::CENTER_CENTER, "원본 참조 없음", FontId::proportional(14.), self.palette.muted);
                            }
                            let row = |y, h| Rect::from_min_size(top + Vec2::new(0., y), Vec2::new(244., h));
                            ui.put(row(172., 26.), egui::Label::new(RichText::new(&t.name).size(18.).strong()).truncate()).on_hover_text(&t.name);
                            ui.put(row(204., 18.), egui::Label::new(RichText::new(format!("{} · v{}", t.provider, t.version)).size(11.).color(self.palette.muted)).truncate()).on_hover_text(format!("{} · v{}", t.provider, t.version));
                            ui.scope_builder(egui::UiBuilder::new().max_rect(row(230., 60.)).layout(egui::Layout::top_down(egui::Align::Min)), |ui| {
                                egui::ScrollArea::vertical().id_salt(("template-rights", &t.id)).max_height(60.).auto_shrink([false, false]).show(ui, |ui| {
                                    chip(ui, &t.license);
                                    chip(ui, &t.attribution);
                                });
                            });
                            ui.put(row(300., 26.), egui::Label::new(RichText::new(format!("편집 {} / 폰트 포함 {} / AI {}", if t.permissions.edit {"가능"} else {"제한"}, if t.permissions.embed {"가능"} else {"제한"}, if t.permissions.ai {"가능"} else {"제한"})).size(11.).color(self.palette.muted)));
                            let action = ui.put(row(336., 32.), egui::Button::new("선택한 글리프에 적용"));
                            if action.clicked() { apply = Some(t.clone()); }
                            action.rect
                        });
                    #[cfg(feature = "ui-qa")]
                    { self.qa_settings.template_cards.push((card.response.rect, card.inner)); }
                    let _ = card;
                }
            });
            if let Some(t)=apply{self.edit(|g|apply_template(g,&t));self.tab=0;}
            ui.add_space(24.);ui.separator();ui.add_space(18.);ui.label(RichText::new("내 템플릿 연결").size(22.).strong());ui.label("Apple·Samsung 등 제공자별 자산은 권한을 확인한 원본만 직접 연결하세요. 앱에는 해당 업체 그림이 포함되어 있지 않습니다.");ui.add_space(12.);
            ui.horizontal(|ui|{if ui.button("템플릿 JSON 가져오기").clicked(){if let Some(path)=rfd::FileDialog::new().add_filter("Template JSON",&["json"]).pick_file(){let result=(||->Result<Template,String>{check(std::fs::metadata(&path).map_err(|e|e.to_string())?.len()<16_000_000,"템플릿은 16MB 이하여야 해요.")?;let t:Template=serde_json::from_slice(&std::fs::read(path).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;t.validate()?;Ok(t)})();match result{Ok(t)=>{if self.project.templates.iter().any(|v|v.id==t.id){self.notify("같은 ID의 템플릿이 이미 있어요. 새 버전은 별도 ID로 가져오세요.");}else{self.project.templates.push(t);self.changed();}},Err(e)=>self.notify(e)}}}
            if ui.button("시스템 기본으로 표시").clicked(){self.edit(|g|restore(g,"system"));self.tab=0;}});
            ui.add_space(18.);egui::Grid::new("template-info").num_columns(2).spacing([20.,12.]).show(ui,|ui|{for(label,value)in [("제공자",&mut self.template_provider),("버전",&mut self.template_version),("출처 / 원본 위치",&mut self.template_source),("라이선스 / 이용 조건",&mut self.template_license),("출처 표시 문구",&mut self.template_attribution)]{ui.label(label);ui.add(egui::TextEdit::singleline(value).desired_width(440.));ui.end_row();}});
            ui.horizontal_wrapped(|ui|{for(i,label)in ["표시 허용","편집 허용","폰트 포함 허용","원본 재배포 허용","AI 전송 허용"].iter().enumerate(){ui.checkbox(&mut self.template_rights[i],*label);}});
            if ui.button("현재 그림을 새 템플릿으로 보관").clicked(){if self.template_license.trim().is_empty()||self.template_source.trim().is_empty(){self.notify("출처와 이용 조건을 기록해 주세요.");}else{let g=&self.project.glyphs[self.selected];if g.template.iter().chain(&g.sources).any(|t|!t.permissions.redistribute){self.notify("기존 템플릿은 원본 재배포 권한이 없어요.");}else{let mut r=self.template_rights;
                let mut credits=self.template_attribution.clone();
                for source in g.template.iter().chain(&g.sources){let p=&source.permissions;r[0]&=p.display;r[1]&=p.edit;r[2]&=p.embed;r[3]&=p.redistribute;r[4]&=p.ai;credits.push_str(&format!("\n{} v{} | {} | {} | {}",source.provider,source.version,source.source,source.license,source.attribution));}
                self.project.templates.push(Template{id:uid(),name:format!("{} · {}",self.template_provider,g.name),provider:self.template_provider.clone(),version:self.template_version.clone(),source:self.template_source.clone(),license:self.template_license.clone(),attribution:credits,permissions:Permissions{display:r[0],edit:r[1],embed:r[2],redistribute:r[3],ai:r[4]},original:Some(g.artwork.clone())});self.changed();self.notify("현재 원본을 버전과 함께 보관했어요.");}}}
        });
    }
    fn generate_ai(&mut self, ctx: &egui::Context) {
        self.start_ai(ctx, true);
    }
    fn receive_ai_reply(
        &mut self,
        id: &str,
        drawing: bool,
        result: serde_json::Value,
    ) -> Result<(), String> {
        if drawing {
            let artwork: Artwork =
                serde_json::from_value(result["artwork"].clone()).map_err(|_| {
                    "AI가 그림 후보를 반환하지 않았어요. 설명만 있는 응답은 채택할 수 없습니다."
                })?;
            doodle_core::ai::validate_candidate(&artwork)?;
            let sources: Vec<Template> = if let Some(value) = result.get("sources") {
                serde_json::from_value(value.clone())
                    .map_err(|_| "후보의 출처 정보를 읽지 못했어요.")?
            } else {
                vec![]
            };
            check(sources.len() <= 100, "후보 출처가 너무 많아요.")?;
            for source in &sources {
                source.validate()?;
            }
            let g = self
                .project
                .glyphs
                .iter_mut()
                .find(|g| g.id == id)
                .ok_or("후보를 저장할 글리프를 찾을 수 없어요.")?;
            if !editable(g) || matches!(g.status.as_str(), "blank" | "unsupported") {
                return Err("잠금·빈칸·미지원 항목에는 후보를 저장할 수 없어요.".into());
            }
            if g.candidates.len() >= 50 {
                return Err("이 글리프의 AI 후보가 50개여서 새 후보를 저장할 수 없어요.".into());
            }
            let candidate = Candidate {
                id: uid(),
                name: format!("AI 후보 {}", g.candidates.len() + 1),
                artwork,
                sources,
            };
            self.candidate_review = Some((id.to_owned(), candidate.id.clone()));
            g.candidates.push(candidate);
            self.ai_answer = format!(
                "‘{}’의 그림 후보를 만들었어요. 후보 보기에서 비교하고 채택해 주세요.",
                g.text
            );
            if let Some(note) = result["generationNote"].as_str() {
                self.ai_answer.push_str(&format!("\n{note}"));
            }
            self.changed();
        } else {
            self.ai_answer = result["answer"]
                .as_str()
                .filter(|s| !s.trim().is_empty())
                .ok_or("AI 상담 응답이 비어 있어요.")?
                .to_owned();
        }
        if let Some(note) = result["decisionNote"].as_str().filter(|s| !s.is_empty()) {
            self.ai_answer.push_str(&format!("\n\n{note}"));
        }
        Ok(())
    }
    fn open_candidate_review(&mut self) {
        if let Some(g) = self.project.glyphs.get(self.selected) {
            if let Some(candidate) = g.candidates.last() {
                self.candidate_review = Some((g.id.clone(), candidate.id.clone()));
            } else {
                self.notify("아직 저장된 후보가 없어요. 후보 만들기로 그림을 생성해 주세요.");
            }
        }
    }
    fn candidate_review_ui(&mut self, ctx: &egui::Context) {
        if self.ai_cancel.is_some() {
            return;
        }
        let Some((glyph_id, mut candidate_id)) = self.candidate_review.clone() else {
            return;
        };
        let Some(index) = self.project.glyphs.iter().position(|g| g.id == glyph_id) else {
            self.candidate_review = None;
            return;
        };
        let g = &self.project.glyphs[index];
        let mut apply = false;
        let mut close = false;
        let modal = egui::Modal::new("ai-candidate-review".into()).show(ctx, |ui| {
            ui.set_width(620.);
            ui.horizontal(|ui| {
                ui.heading(format!("‘{}’ AI 후보", display(&g.text)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    close = ui.button("닫기").clicked();
                });
            });
            ui.label("그림을 비교하고 채택해 주세요. 채택 전에는 현재 그림이 바뀌지 않습니다.");
            #[cfg(feature = "ui-qa")]
            match std::env::var("DOODLE_QA_VIEW").unwrap_or_default().as_str() {
                "ai-candidates" | "ai-candidate-accept" | "ai-typography" => {
                    ui.label("화면 동작 검증용 모의 후보 · 실제 AI 생성 결과가 아닙니다.");
                }
                "ai-candidate-live" | "ai-typography-live" => {
                    ui.label("실제 내장 AI 생성 결과 · 글자 정확도는 별도로 확인합니다.");
                }
                "decision-fast-live" | "decision-gemma-live" => {
                    ui.label("실제 Needle 3 판단 경로 · 모의 응답을 사용하지 않습니다.");
                }
                _ => {}
            }
            egui::ScrollArea::horizontal()
                .id_salt("candidate-choices")
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for candidate in &g.candidates {
                            ui.selectable_value(
                                &mut candidate_id,
                                candidate.id.clone(),
                                &candidate.name,
                            );
                        }
                    });
                });
            if let Some(candidate) = g.candidates.iter().find(|c| c.id == candidate_id) {
                if !candidate.sources.is_empty() {
                    ui.label(RichText::new("공개 글꼴 자형을 활용했어요. 출력 폰트에 출처와 OFL 이용 조건이 포함됩니다.").size(12.).color(self.palette.muted));
                }
                ui.columns(2, |columns| {
                    for (column, (label, artwork, key)) in columns.iter_mut().zip([
                        (
                            "현재 그림",
                            &g.artwork,
                            format!("review-current-{glyph_id}"),
                        ),
                        (
                            "선택한 AI 후보",
                            &candidate.artwork,
                            format!("review-{candidate_id}"),
                        ),
                    ]) {
                        column.label(RichText::new(label).strong());
                        let size = column
                            .available_width()
                            .min((ctx.content_rect().height() - 310.).clamp(160., 280.));
                        let (rect, _) =
                            column.allocate_exact_size(Vec2::splat(size), Sense::hover());
                        column.painter().rect_filled(rect, 8., Color32::WHITE);
                        paint_art(
                            ctx,
                            column.painter(),
                            rect.shrink(10.),
                            artwork,
                            &key,
                            self.revision,
                            &mut self.textures,
                        );
                    }
                });
                ui.add_space(8.);
                let can_apply =
                    editable(g) && !matches!(g.status.as_str(), "blank" | "unsupported");
                let response = ui.add_enabled(
                    can_apply,
                    egui::Button::new("이 후보 채택").fill(self.palette.selection),
                );
                #[cfg(feature = "ui-qa")]
                {
                    self.qa_candidate_accept = Some(response.rect);
                }
                apply = response.clicked();
                if !can_apply {
                    ui.label("잠금·빈칸·미지원 상태를 해제한 뒤 채택할 수 있어요.");
                }
            } else {
                ui.label("선택한 후보가 없어요. 위에서 다른 후보를 골라 주세요.");
            }
        });
        self.candidate_review = if close || modal.should_close() {
            None
        } else {
            Some((glyph_id, candidate_id.clone()))
        };
        if apply {
            self.selected = index;
            self.group = GROUPS
                .iter()
                .position(|entry| entry.0 == self.project.glyphs[index].group)
                .unwrap_or(self.group);
            self.refresh_catalog();
            let mut accepted = false;
            self.edit(|g| {
                accept(g, &candidate_id)?;
                accepted = true;
                Ok(())
            });
            if accepted {
                self.tab = 0;
                self.layer = None;
                self.candidate_review = None;
                self.notify("AI 후보를 적용했어요. 실행 취소로 이전 그림으로 돌아갈 수 있어요.");
            }
        }
    }
    fn install_ai(&mut self, ctx: &egui::Context) {
        self.install_package(ctx, doodle_core::ai_install::Package::Gemma);
    }
    fn install_package(&mut self, ctx: &egui::Context, package: doodle_core::ai_install::Package) {
        if self.job.is_some() {
            return;
        }
        #[cfg(feature = "ui-qa")]
        {
            self.qa_settings.installs += 1;
        }
        self.install_error.clear();
        self.installing_ai = true;
        self.install_fraction = 0.;
        self.ai_started = Instant::now();
        self.ai_status = if package == doodle_core::ai_install::Package::Needle {
            "판단 AI 설치 준비 중"
        } else {
            "Gemma 4 설치 준비 중"
        }
        .into();
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        self.ai_cancel = Some(cancel.clone());
        self.draft = None;
        self.drag_start = None;
        ctx.memory_mut(|m| {
            if let Some(id) = m.focused() {
                m.surrender_focus(id);
            }
        });
        let (tx, rx) = mpsc::channel();
        self.install_progress = Some(rx);
        let root = if package == doodle_core::ai_install::Package::Needle {
            doodle_core::needle::root()
        } else {
            doodle_core::local_ai::root()
        };
        self.spawn(ctx, move || {
            doodle_core::ai_install::install_package(root, package, cancel, tx)
                .map(|()| Job::AiInstalled(package))
        });
    }
    fn ai_ready(&self) -> bool {
        match self.api_provider.as_str() {
            "local" => self.local_installed,
            "openai" | "anthropic" => {
                !self.api_model.trim().is_empty()
                    && !self.api_endpoint.trim().is_empty()
                    && !self.api_key.trim().is_empty()
            }
            _ => false,
        }
    }
    fn start_ai(&mut self, ctx: &egui::Context, drawing: bool) {
        if self.job.is_some() {
            return;
        }
        if self.connections_pending.is_some() {
            self.save_connections();
        }
        if !self.ai_ready() {
            self.notify("AI 연결 화면에서 로컬 모델 또는 API를 먼저 준비해 주세요.");
            self.tab = 3;
            return;
        }
        let Some(g) = self.project.glyphs.get(self.selected).cloned() else {
            self.notify("먼저 작업할 글리프를 선택해 주세요.");
            return;
        };
        if drawing && g.candidates.len() >= 50 {
            self.ai_answer = "이 글리프의 AI 후보가 50개여서 새 후보를 만들 수 없어요.".into();
            self.notify(self.ai_answer.clone());
            return;
        }
        let id = g.id.clone();
        let payload = serde_json::json!({"glyph":g,"action":if drawing{"generate"}else{"assist"},"decision":self.decision_mode,"config":{"provider":self.api_provider,"endpoint":self.api_endpoint,"model":self.api_model,"key":self.api_key},"prompt":self.ai_prompt,"sendReference":self.send_reference || self.api_provider=="local","colorMode":if self.ai_color{"color"}else{"mono"}});
        if let Err(e) = doodle_core::ai::validate_target(&payload, drawing) {
            self.notify(e);
            return;
        }
        self.ai_action = drawing;
        self.ai_answer.clear();
        self.candidate_review = None;
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        self.ai_cancel = Some(cancel.clone());
        self.ai_started = Instant::now();
        self.ai_status = "AI 작업 준비 중".into();
        self.companion
            .motion
            .react(doodle_core::companion::Mood::Thinking);
        self.draft = None;
        self.drag_start = None;
        let (tx, rx) = mpsc::channel();
        self.ai_progress = Some(rx);
        ctx.memory_mut(|m| {
            if let Some(id) = m.focused() {
                m.surrender_focus(id);
            }
        });
        self.spawn(ctx, move || {
            doodle_core::ai_pipeline::run(payload, cancel, tx).map(|r| Job::AiReply(id, drawing, r))
        });
    }
    fn ai_dock(&mut self, ctx: &egui::Context) {
        if self.ai_cancel.is_none() && self.companion.motion.mood.working() {
            self.companion
                .motion
                .react(doodle_core::companion::Mood::Idle);
        }
        if (self.export_open || self.api_provider == "off") && self.ai_cancel.is_none() {
            return;
        }

        let frame = egui::Frame::new()
            .fill(self.palette.surface)
            .stroke(Stroke::new(1_f32, self.palette.line))
            .corner_radius(18)
            .inner_margin(18)
            .shadow(egui::epaint::Shadow {
                offset: [0, 5],
                blur: 20,
                spread: 2,
                color: Color32::from_black_alpha(30),
            });
        let view = ctx.content_rect();
        let companion_size = (view.width() * 0.14).clamp(160., 192.);
        let gap = 14.;
        let card_width = (view.width() - companion_size - gap - 80.).min(700.);
        let card_outer_width = card_width + frame.total_margin().sum().x;
        let card_center_x = view
            .center()
            .x
            .min(view.right() - 16. - companion_size - gap - card_outer_width / 2.);
        let companion_offset = Vec2::new(
            card_center_x - view.center().x + card_outer_width / 2. + gap + companion_size / 2.,
            -44.,
        );
        let companion_bounds = doodle_core::companion::placement::bounds(view);
        let companion_rect = doodle_core::companion::placement::resolve(
            companion_bounds,
            companion_size,
            Pos2::new(
                view.center().x + companion_offset.x - companion_size / 2.,
                view.bottom() + companion_offset.y - companion_size,
            ),
            self.preferences.companion_position,
        );
        let area = egui::Area::new("ai-dock".into())
            .anchor(
                Align2::CENTER_BOTTOM,
                [card_center_x - view.center().x, -44.],
            )
            .order(egui::Order::Foreground);
        if let Some(cancel) = self.ai_cancel.clone() {
            let dock = egui::Modal::new("ai-busy".into())
                .area(area)
                .frame(frame)
                .backdrop_color(Color32::from_black_alpha(35))
                .show(ctx, |ui| {
                    ui.set_width(card_width);
                    if !self.companion.motion.mood.working() {
                        self.companion
                            .motion
                            .react(doodle_core::companion::Mood::Thinking);
                    }
                    ui.horizontal(|ui| {
                        ui.add(egui::Spinner::new().size(20.));
                        ui.label(RichText::new(&self.ai_status).strong());
                        ui.label(format!("{}초", self.ai_started.elapsed().as_secs()));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add_enabled(
                                    !cancel.load(std::sync::atomic::Ordering::Relaxed),
                                    egui::Button::new("작업 취소"),
                                )
                                .clicked()
                            {
                                cancel.store(true, std::sync::atomic::Ordering::Relaxed);
                                self.ai_status = "작업을 취소하고 편집을 다시 여는 중".into();
                            }
                        });
                    });
                    if self.installing_ai {
                        ui.add(egui::ProgressBar::new(self.install_fraction).show_percentage());
                    }
                    ui.label(
                        RichText::new(if self.installing_ai {
                            "최초 설치에는 인터넷이 필요합니다. 취소해도 다음에 이어받을 수 있어요."
                        } else {
                            "작업 중에는 화면 입력과 편집이 잠깁니다."
                        })
                        .size(12.)
                        .color(self.palette.muted),
                    );
                });
            // Use the modal's paint layer so its backdrop cannot dim the companion.
            // The character is display-only here; the existing modal keeps editing locked.
            let pointer = ctx.pointer_hover_pos().map_or(Vec2::ZERO, |p| {
                (p - companion_rect.center())
                    / Vec2::new(
                        (view.width() * 0.55).max(180.),
                        (view.height() * 0.75).max(180.),
                    )
            });
            let pose = self.companion.motion.step(
                ctx.input(|i| i.time),
                pointer,
                self.preferences.reduce_motion,
            );
            self.companion.paint(
                &ctx.layer_painter(dock.response.layer_id),
                companion_rect,
                pose,
            );
            #[cfg(feature = "ui-qa")]
            {
                self.qa_dock.dock = Some(dock.response.rect);
                self.qa_dock.companion = Some(companion_rect);
            }
        } else {
            let openness = ctx.animate_bool_with_time_and_easing(
                egui::Id::new("ai-dock-openness"),
                !self.ai_dock_collapsed,
                if self.preferences.reduce_motion {
                    0.
                } else {
                    0.28
                },
                egui::emath::easing::cubic_out,
            );
            #[cfg(feature = "ui-qa")]
            {
                self.qa_dock.dock = None;
            }
            if openness > 0. {
                let dock = area.show(ctx, |ui| {
                if openness < 1. { ui.disable(); }
                ui.multiply_opacity(openness);
                frame.show(ui, |ui| {
                    ui.set_width(card_width);
                    ui.style_mut().spacing.scroll = egui::style::ScrollStyle::solid();
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("두들 어시스트")
                                .strong()
                                .color(self.palette.accent),
                        );
                        chip(
                            ui,
                            if self.api_provider == "local" {
                                "로컬 AI"
                            } else if self.send_reference {
                                "API · 그림 포함"
                            } else {
                                "API · 텍스트만"
                            },
                        );
                        ui.selectable_value(&mut self.ai_action, false, "그림 상담");
                        ui.selectable_value(&mut self.ai_action, true, "후보 만들기");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let collapse = ui.button("접기");
                            #[cfg(feature = "ui-qa")]
                            {
                                self.qa_dock.collapse = Some(collapse.rect);
                            }
                            if collapse.clicked() {
                                self.ai_dock_collapsed = !self.ai_dock_collapsed;
                            }
                        });
                    });
                    if !self.ai_ready() {
                        ui.horizontal(|ui| {
                            ui.label("한 글자부터 가볍게 그려보세요. AI를 연결하면 두들이 도와드려요.");
                            if ui.button("AI 연결하기").clicked() { self.tab=3; self.ai_dock_collapsed=true; }
                        });
                        return;
                    }
                    if self.ai_answer.is_empty() {
                        ui.horizontal_wrapped(|ui| {
                            for (label,prompt,drawing) in [
                                ("획 다듬기", "현재 그림의 형태와 위치는 유지하고 획 굵기를 48로 통일해줘.",true),
                                ("둥근 손글씨", "선택한 글자를 동글동글한 손글씨로 만들어줘.",true),
                                ("균형 봐주기", "현재 글자의 기준선, 획의 균형과 간격을 살펴보고 개선할 점을 알려줘.",false),
                            ] {
                                if text_button(ui, label).clicked() { self.ai_prompt=prompt.into(); self.ai_action=drawing; }
                            }
                        });
                    }
                    let candidate_count = self
                        .project
                        .glyphs
                        .get(self.selected)
                        .map_or(0, |g| g.candidates.len());
                    if candidate_count > 0
                        && ui
                            .button(format!("저장된 후보 {candidate_count}개 보기 · 채택"))
                            .clicked()
                    {
                        self.open_candidate_review();
                    }
                    if !self.ai_answer.is_empty() {
                        let answer = egui::Frame::new()
                            .fill(self.palette.surface)
                            .corner_radius(8)
                            .inner_margin(8)
                            .show(ui, |ui| {
                                egui::ScrollArea::vertical()
                                    .id_salt("ai-answer")
                                    .max_height((ctx.content_rect().height() * 0.18).min(120.))
                                    .auto_shrink([false, true])
                                    .scroll_bar_visibility(
                                        egui::scroll_area::ScrollBarVisibility::AlwaysVisible,
                                    )
                                    .show(ui, |ui| {
                                        ui.add(
                                            egui::Label::new(&self.ai_answer)
                                                .wrap()
                                                .selectable(true),
                                        );
                                    })
                            })
                            .inner;
                        #[cfg(feature = "ui-qa")]
                        {
                            self.qa_dock.answer = Some(answer.inner_rect);
                            self.qa_dock.answer_offset = answer.state.offset.y;
                        }
                        let _ = answer;
                    }
                    ui.horizontal(|ui| {
                        let width = ui.available_width() - 85.;
                        let prompt = egui::Frame::new()
                            .fill(self.palette.background)
                            .stroke(Stroke::new(1_f32, self.palette.line))
                            .corner_radius(6)
                            .inner_margin(6)
                            .show(ui, |ui| {
                                egui::ScrollArea::vertical()
                                    .id_salt("ai-prompt")
                                    .max_width(width - 14.)
                                    .max_height(56.)
                                    .min_scrolled_height(56.)
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        ui.add(
                                    egui::TextEdit::multiline(&mut self.ai_prompt)
                                        .frame(false)
                                        .margin(0)
                                        .desired_width(f32::INFINITY)
                                        .desired_rows(2)
                                        .hint_text(if self.ai_action {
                                            "이 글자를 굵고 둥근 손글씨로 그려줘."
                                        } else {
                                            "이 글자의 균형을 봐줘. 획을 더 부드럽게 만들려면?"
                                        }),
                                );
                                    })
                            })
                            .inner;
                        #[cfg(feature = "ui-qa")]
                        {
                            self.qa_dock.prompt = Some(prompt.inner_rect);
                            self.qa_dock.prompt_offset = prompt.state.offset.y;
                        }
                        let _ = prompt;
                        let send = ui.add_enabled(
                            self.job.is_none() && !self.ai_prompt.trim().is_empty(),
                            egui::Button::new(if self.ai_action {
                                "그림 생성"
                            } else {
                                "상담하기"
                            })
                            .min_size(Vec2::new(70., 46.))
                            .fill(self.palette.selection),
                        );
                        #[cfg(feature = "ui-qa")]
                        {
                            self.qa_ai_send = Some(send.rect);
                        }
                        if send.clicked() {
                            self.start_ai(ctx, self.ai_action);
                        }
                    });
                });
            });
                let target = self.ai_dock_restore.unwrap_or_else(|| {
                    Rect::from_center_size(
                        Pos2::new(view.center().x, view.bottom() - 16.),
                        Vec2::new(106., 20.),
                    )
                });
                let scale = egui::lerp(
                    (target.width() / dock.response.rect.width())
                        .min(target.height() / dock.response.rect.height())
                        ..=1.,
                    openness,
                );
                let center = target.center().lerp(dock.response.rect.center(), openness);
                let transform = egui::emath::TSTransform {
                    scaling: scale,
                    translation: center.to_vec2() - dock.response.rect.center().to_vec2() * scale,
                };
                ctx.transform_layer_shapes(dock.response.layer_id, transform);
                #[cfg(feature = "ui-qa")]
                {
                    self.qa_dock.dock = Some(transform * dock.response.rect);
                }
                let _ = dock;
            }
            let area = egui::Area::new("ai-companion".into())
                .current_pos(companion_rect.min)
                .default_size(companion_rect.size())
                .constrain_to(companion_bounds)
                .order(egui::Order::Foreground)
                .sense(Sense::click_and_drag())
                .movable(!ctx.input(|i| i.pointer.secondary_down()))
                .show(ctx, |ui| {
                    let companion =
                        self.companion
                            .show(ui, companion_size, self.preferences.reduce_motion);
                    #[cfg(feature = "ui-qa")]
                    {
                        self.qa_dock.companion = Some(companion.rect);
                    }
                    let companion = companion
                        .on_hover_cursor(egui::CursorIcon::Grab)
                        .on_hover_text("드래그하여 이동 · 마우스 오른쪽 버튼으로 설정");
                    egui::Popup::context_menu(&companion)
                        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                        .show(|ui| {
                            self.companion_options(ui);
                            #[cfg(feature = "ui-qa")]
                            {
                                self.qa_settings.companion_menu = Some(ui.min_rect());
                            }
                        });
                });
            if area.response.dragged_by(egui::PointerButton::Primary)
                || area.response.drag_stopped_by(egui::PointerButton::Primary)
            {
                self.preferences.companion_position =
                    Some(doodle_core::companion::placement::remember(
                        companion_bounds,
                        area.response.rect,
                    ));
                if area.response.drag_stopped_by(egui::PointerButton::Primary) {
                    self.store_preferences();
                } else {
                    ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
                }
            }
        }
    }
    fn decision_options_ui(&mut self, ui: &mut egui::Ui) {
        use doodle_core::needle::Mode;
        let previous = self.decision_mode;
        let mut enabled = previous == Mode::Needle;
        ui.horizontal_wrapped(|ui| {
            let response = ui
                .add_enabled(
                    self.job.is_none(),
                    egui::Checkbox::new(&mut enabled, "내장 판단 AI 사용"),
                )
                .on_hover_text("처음 켜면 판단 모델 약 37 MB를 자동으로 설치해요.");
            #[cfg(feature = "ui-qa")]
            {
                self.qa_decision_button = Some(response.rect);
            }
            let _ = response;
            if enabled {
                chip(
                    ui,
                    if self.needle_installed {
                        "준비됨"
                    } else {
                        "설치 필요 · 37 MB"
                    },
                );
                if !self.needle_installed
                    && previous == Mode::Needle
                    && ui
                        .add_enabled(self.job.is_none(), egui::Button::new("설치 다시 시도"))
                        .clicked()
                {
                    self.install_package(ui.ctx(), doodle_core::ai_install::Package::Needle);
                }
            } else {
                chip(ui, "입력 보조 · 선택 사항");
            }
        });
        self.decision_mode = if enabled { Mode::Needle } else { Mode::Off };
        if enabled {
            chip(
                ui,
                "간단한 편집을 먼저 판단해요. 한국어 정확도는 아직 제한적이에요.",
            );
            egui::CollapsingHeader::new("판단 AI 자세히")
                .id_salt("decision-details")
                .show(ui, |ui| {
                    ui.label("Needle 3 · 시험 기능");
                    ui.label("그림을 만드는 AI와 별개로, 내장 AI와 API 모두에 적용돼요. PC에서 입력한 텍스트만 확인합니다.");
                    ui.label("설치되지 않았거나 판단이 불확실하면 선택한 AI가 그대로 처리해요. 판단 결과는 두들의 동작 선택에도 쓰입니다.");
                    if self.needle_installed && ui.add_enabled(self.job.is_none(), egui::Button::new("판단 모델 검증 · 복구")).clicked() {
                        self.install_package(ui.ctx(), doodle_core::ai_install::Package::Needle);
                    }
                });
        }
        if previous != self.decision_mode {
            let path = self.save_path.with_file_name("ai-options.json");
            if let Err(e) = (doodle_core::needle::Options {
                decision: self.decision_mode,
            })
            .save(&path)
            {
                self.notify(format!("판단 AI 설정을 저장하지 못했어요: {e}"));
            }
            if enabled && !self.needle_installed {
                self.install_package(ui.ctx(), doodle_core::ai_install::Package::Needle);
            }
        }
    }
    fn connection_changed(&mut self) {
        self.connection_test_status.clear();
        self.connections.remember(
            &self.api_provider,
            &self.api_endpoint,
            &self.api_model,
            &self.api_key,
        );
        self.connections_pending = Some(Instant::now());
        self.connections_status = "연결 설정을 저장하는 중이에요…".into();
        self.connections_error = false;
    }
    fn save_connections(&mut self) {
        self.connections_pending = None;
        match self.connections.save(&self.connections_path) {
            Ok(()) => {
                self.connections_status = "연결 설정 저장 완료 · 다음 실행에도 유지돼요".into();
                self.connections_error = false;
            }
            Err(error) => {
                self.connections_status = error;
                self.connections_error = true;
            }
        }
    }
    fn select_ai_provider(&mut self, provider: &str) {
        if self.api_provider == provider {
            return;
        }
        self.connections.remember(
            &self.api_provider,
            &self.api_endpoint,
            &self.api_model,
            &self.api_key,
        );
        self.api_provider = provider.into();
        let profile = self.connections.profile(provider);
        self.api_key = self.connections.key(provider, &profile.endpoint);
        self.api_endpoint = profile.endpoint;
        self.api_model = profile.model;
        self.connection_changed();
    }
    fn test_connection(&mut self, ctx: &egui::Context) {
        if self.job.is_some()
            || !self.ai_ready()
            || !["openai", "anthropic"].contains(&self.api_provider.as_str())
        {
            return;
        }
        if self.connections_pending.is_some() {
            self.save_connections();
        }
        self.connection_test_status = "연결 테스트 중…".into();
        self.connection_test_error = false;
        self.testing_connection = true;
        self.ai_started = Instant::now();
        self.ai_status = "API 연결을 확인하는 중".into();
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        self.ai_cancel = Some(cancel.clone());
        let config = serde_json::json!({"provider":self.api_provider,"endpoint":self.api_endpoint,"model":self.api_model,"key":self.api_key});
        self.spawn(ctx, move || {
            let result = (|| {
                let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|_| "연결 테스트를 시작하지 못했어요.")?;
                runtime.block_on(async {
                    let request = doodle_core::ai::test_connection(config);
                    tokio::pin!(request);
                    loop {
                        tokio::select! {
                            result = &mut request => return result,
                            _ = tokio::time::sleep(Duration::from_millis(100)) => {
                                if cancel.load(std::sync::atomic::Ordering::Relaxed) { return Err("연결 테스트를 취소했어요.".into()); }
                            }
                        }
                    }
                })
            })();
            Ok(Job::ConnectionTest(result))
        });
    }
    fn settings_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("내 작업실 설정");
        self.appearance_options(ui);
        ui.add_space(12.);
        self.companion_options(ui);
        ui.add_space(16.);
        ui.separator();
        ui.add_space(12.);
        ui.label(RichText::new("AI 어시스트").size(20.).strong());
        egui::Frame::new()
            .fill(self.palette.raised)
            .corner_radius(12)
            .inner_margin(18)
            .show(ui, |ui| {
                ui.set_width(ui.available_width().min(740.));
                ui.label("그림 상담과 후보를 만들어 줄 AI");
                ui.horizontal_wrapped(|ui| {
                    for (provider, label) in [
                        ("off", "사용 안 함"),
                        ("local", "로컬 AI"),
                        ("openai", "OpenAI 호환"),
                        ("anthropic", "Anthropic 호환"),
                    ] {
                        let response = ui.selectable_label(self.api_provider == provider, label);
                        #[cfg(feature="ui-qa")]
                        { self.qa_ai_providers.insert(provider.into(), response.rect); }
                        let response = if provider == "local" { response.on_hover_text("처음 선택하면 로컬 AI 약 3.4 GB를 자동으로 설치해요.") } else { response };
                        if response.clicked() {
                            let changed = self.api_provider != provider;
                            self.select_ai_provider(provider);
                            if changed && provider == "local" && !self.local_installed {
                                self.install_ai(ui.ctx());
                            }
                        }
                    }
                });
                ui.add_space(8.);
                if self.api_provider == "off" {
                    chip(ui, "AI 없이 직접 그리기와 폰트 내보내기를 사용할 수 있어요.");
                } else if self.api_provider == "local" {
                    ui.horizontal_wrapped(|ui| {
                        chip(
                            ui,
                            if self.local_installed {
                                "사용 준비 완료 · 인터넷 없이 사용해요"
                            } else {
                                "설치 필요 · 약 3.4 GB"
                            },
                        );
                        if !self.local_installed {
                            let button = ui.add_enabled(
                                self.job.is_none(),
                                egui::Button::new("설치 다시 시도").fill(self.palette.selection),
                            );
                            #[cfg(feature = "ui-qa")]
                            {
                                self.qa_install_button = Some(button.rect);
                            }
                            if button.clicked() {
                                self.install_ai(ui.ctx());
                            }
                        }
                    });
                    egui::CollapsingHeader::new("모델 정보 · 설치 관리")
                        .id_salt("local-ai-details")
                        .show(ui, |ui| {
                            ui.label("Gemma 4 E2B · 작업할 때만 읽고 끝나면 메모리를 반환해요.");
                            ui.label(
                                RichText::new(format!(
                                    "설치 위치: {}",
                                    doodle_core::local_ai::root().display()
                                ))
                                .size(12.)
                                .color(self.palette.muted),
                            );
                            if !self.local_installed {
                                let missing = doodle_core::ai_install::missing(
                                    &doodle_core::local_ai::root(),
                                );
                                ui.label(format!("확인이 필요한 파일: {}", missing.join(", ")));
                            }
                            ui.horizontal_wrapped(|ui| {
                                if self.local_installed
                                    && ui
                                        .add_enabled(
                                            self.job.is_none(),
                                            egui::Button::new("설치 파일 검증 · 복구"),
                                        )
                                        .clicked()
                                {
                                    self.install_ai(ui.ctx());
                                }
                                if ui.button("설치 상태 다시 확인").clicked() {
                                    self.local_installed = doodle_core::local_ai::installed();
                                    self.last_install_check = Instant::now();
                                }
                            });
                        });
                } else {
                    ui.horizontal_wrapped(|ui| {
                        chip(ui, if self.ai_ready() { "연결 정보 입력됨" } else { "연결 설정 필요" });
                        if !self.api_model.trim().is_empty() { ui.label(&self.api_model); }
                    });
                    let connection_header = egui::CollapsingHeader::new("연결 설정")
                        .id_salt(("api-connection-settings", self.api_provider.clone()))
                        .show(ui, |ui| {
                    let mut changed = false;
                    egui::Grid::new("api-info")
                        .num_columns(2)
                        .spacing([16., 8.])
                        .show(ui, |ui| {
                            ui.label("API 기본 주소");
                            if ui
                                .add(
                                    egui::TextEdit::singleline(&mut self.api_endpoint)
                                        .desired_width(460.),
                                )
                                .changed()
                            {
                                self.api_key =
                                    self.connections.key(&self.api_provider, &self.api_endpoint);
                                changed = true;
                            }
                            ui.end_row();
                            ui.label("모델 이름");
                            changed |= ui
                                .add(
                                    egui::TextEdit::singleline(&mut self.api_model)
                                        .hint_text("연결할 제공자의 정확한 모델 ID"),
                                )
                                .changed();
                            ui.end_row();
                            ui.label("API 키");
                            let key_input = ui.add(
                                egui::TextEdit::singleline(&mut self.api_key)
                                    .password(true)
                                    .hint_text("다음 실행에도 유지할 API 키"),
                            );
                            #[cfg(feature = "ui-qa")]
                            {
                                self.qa_api_key = Some(key_input.rect);
                            }
                            changed |= key_input.changed();
                            ui.end_row();
                        });
                    if changed {
                        self.connection_changed();
                    }
                    ui.add_space(4.);
                    ui.horizontal(|ui| {
                        if ui.button("연결 설정 저장").clicked() {
                            self.connection_changed();
                            self.save_connections();
                        }
                        let forget = ui.add_enabled(
                            !self.api_key.is_empty(),
                            egui::Button::new("저장된 키 삭제"),
                        );
                        #[cfg(feature = "ui-qa")]
                        {
                            self.qa_forget_key = Some(forget.rect);
                        }
                        if forget.clicked() {
                            self.api_key.clear();
                            self.connection_changed();
                            self.save_connections();
                        }
                        let test = ui.add_enabled(self.job.is_none() && self.ai_ready(), egui::Button::new("연결 테스트"))
                            .on_hover_text("그림 없이 짧은 텍스트 요청을 보내요. 제공자의 API 사용량이 발생할 수 있어요.");
                        #[cfg(feature="ui-qa")] { self.qa_test_connection = Some(test.rect); }
                        if test.clicked() { self.test_connection(ui.ctx()); }
                    });
                    chip(ui, "연결 정보는 이 PC에 암호화해 자동 저장해요.");
                    });
                    #[cfg(feature="ui-qa")] { self.qa_connection_header = Some(connection_header.header_response.rect); }
                    let _ = connection_header;
                }
                if !self.connections_status.is_empty() {
                    ui.colored_label(
                        if self.connections_error {
                            self.palette.accent
                        } else {
                            self.palette.muted
                        },
                        &self.connections_status,
                    );
                }
                if !self.connection_test_status.is_empty() {
                    ui.colored_label(if self.connection_test_error { self.palette.accent } else { self.palette.muted }, &self.connection_test_status);
                }
                if self.api_provider != "off" { ui.horizontal_wrapped(|ui| {
                    ui.checkbox(&mut self.ai_color, "컬러 그림 제안");
                    if self.api_provider != "local" {
                        ui.checkbox(&mut self.send_reference, "현재 그림을 API에 함께 전송")
                            .on_hover_text("선택한 글리프의 드로잉 영역 이미지와 벡터를 전송해요.");
                    }
                }); }
                ui.separator();
                self.decision_options_ui(ui);
                if !self.install_error.is_empty() {
                    ui.colored_label(self.palette.accent, &self.install_error);
                }
                if self.api_provider != "off" {
                    ui.separator();
                    let test = ui.add_enabled(self.ai_ready(), egui::Button::new("생성 테스트…"));
                    #[cfg(feature="ui-qa")] { self.qa_generation_test = Some(test.rect); }
                    if test.clicked() { self.generation_test_open = true; self.ai_dock_collapsed = true; }
                }
            });
    }
    fn project_options_ui(&mut self, ui: &mut egui::Ui) {
        ui.set_max_width(340.);
        ui.strong("프로젝트 정보");
        let mut name = self.project.name.clone();
        let mut family = self.project.family.clone();
        ui.label("프로젝트 이름");
        let name_input = ui.text_edit_singleline(&mut name);
        #[cfg(feature = "ui-qa")]
        {
            self.qa_settings.project_name = Some(name_input.rect);
        }
        if name_input.changed() {
            self.project.name = name;
            self.changed();
        }
        ui.label("폰트 패밀리");
        let family_input = ui.text_edit_singleline(&mut family);
        #[cfg(feature = "ui-qa")]
        {
            self.qa_settings.project_family = Some(family_input.rect);
        }
        if family_input.changed() {
            self.project.family = family;
            self.changed();
        }
        ui.add_space(10.);
        chip(ui, "자동 저장 위치");
        let path = self.save_path.display().to_string();
        ui.add(
            egui::Label::new(RichText::new(&path).size(12.).color(self.palette.muted)).truncate(),
        )
        .on_hover_text(path);
        if self.save_error && ui.button("자동 저장 다시 시도").clicked() {
            self.save_error = false;
            self.save();
        }
    }
    fn dialogs(&mut self, ctx: &egui::Context) {
        if self.ai_cancel.is_none() {
            if let Some(mut comparison) = self.companion_comparison.take() {
                use doodle_core::companion_compare::Action;
                match comparison.show(ctx) {
                    Action::KeepOpen => self.companion_comparison = Some(comparison),
                    Action::Close => {}
                    Action::Choose(renderer) => {
                        self.preferences.companion_renderer = renderer;
                        self.companion.set_renderer(ctx, renderer);
                        if let Err(e) = self
                            .preferences
                            .save(&self.save_path.with_file_name("appearance.json"))
                        {
                            self.notify(format!("화면 설정을 저장하지 못했어요: {e}"));
                        }
                    }
                }
            }
        }
        if self.ai_cancel.is_some() {
            return;
        }
        if self.generation_test_open {
            let mut open = true;
            let mut generated = false;
            egui::Window::new("생성 테스트")
                .open(&mut open)
                .default_width(520.)
                .default_pos(ctx.content_rect().center() - Vec2::new(270., 140.))
                .resizable(false)
                .collapsible(false)
                .show(ctx, |ui| {
                    if let Some(glyph) = self.project.glyphs.get(self.selected) {
                        ui.label(format!(
                            "선택한 글자: {} · {}",
                            display(&glyph.text),
                            codepoints(&glyph.text)
                        ));
                    }
                    ui.label("원하는 그림 스타일을 입력해 주세요.");
                    ui.add(
                        egui::TextEdit::multiline(&mut self.ai_prompt)
                            .desired_width(f32::INFINITY)
                            .desired_rows(4),
                    );
                    if ui
                        .add_enabled(
                            self.job.is_none()
                                && self.ai_ready()
                                && !self.ai_prompt.trim().is_empty(),
                            egui::Button::new("후보 생성").fill(self.palette.selection),
                        )
                        .clicked()
                    {
                        self.generate_ai(ctx);
                        generated = self.job.is_some();
                    }
                    chip(
                        ui,
                        "현재 글자에 바로 적용하지 않고, 후보를 만든 뒤 비교해요.",
                    );
                });
            self.generation_test_open = open && !generated;
        }
        if self.variant_open {
            let mut open = true;
            egui::Window::new("선택한 변형에 그림 복사")
                .open(&mut open)
                .default_width(480.)
                .show(ctx, |ui| {
                    if self.ai_cancel.is_some() {
                        ui.disable();
                    }
                    ui.label(
                        "선택한 대상에만 그림을 복사합니다. 원문·지역·원본 템플릿은 유지됩니다.",
                    );
                    let entries: Vec<_> = self
                        .project
                        .glyphs
                        .iter()
                        .enumerate()
                        .filter(|(i, g)| *i != self.selected && editable(g))
                        .map(|(_, g)| {
                            (
                                g.id.clone(),
                                format!(
                                    "{} · {} · {}",
                                    display(&g.text),
                                    g.locale,
                                    codepoints(&g.text)
                                ),
                            )
                        })
                        .collect();
                    egui::ScrollArea::vertical().max_height(330.).show_rows(
                        ui,
                        26.,
                        entries.len(),
                        |ui, rows| {
                            for i in rows {
                                let (id, label) = &entries[i];
                                let mut checked = self.variant_targets.contains(id);
                                if ui.checkbox(&mut checked, label).changed() {
                                    if checked {
                                        self.variant_targets.insert(id.clone());
                                    } else {
                                        self.variant_targets.remove(id);
                                    }
                                }
                            }
                        },
                    );
                    if ui
                        .button(format!("선택한 {}개에 적용", self.variant_targets.len()))
                        .clicked()
                    {
                        let original = self.selected;
                        let source = self.project.glyphs[original].clone();
                        let targets: Vec<usize> = self
                            .project
                            .glyphs
                            .iter()
                            .enumerate()
                            .filter(|(_, g)| self.variant_targets.contains(&g.id))
                            .map(|(i, _)| i)
                            .collect();
                        for index in targets {
                            self.selected = index;
                            self.edit(|g| {
                                check(editable(g), "대상 글리프가 보호되어 있어요.")?;
                                g.artwork = source.artwork.clone();
                                for t in source.template.iter().chain(&source.sources) {
                                    if !g.sources.iter().any(|v| v.id == t.id) {
                                        g.sources.push(t.clone());
                                    }
                                }
                                g.status = "edited".into();
                                Ok(())
                            });
                        }
                        self.selected = original;
                        self.variant_open = false;
                        self.variant_targets.clear();
                    }
                });
            if !open {
                self.variant_open = false;
            }
        }
        if self.add_open {
            let mut open = true;
            egui::Window::new("문자와 문자열 추가")
                .open(&mut open)
                .resizable(false)
                .default_width(500.)
                .show(ctx, |ui| {
                    if self.ai_cancel.is_some() {
                        ui.disable();
                    }
                    ui.label("문장을 붙여 넣으면 사람이 한 글자로 읽는 묶음으로 추출해요.");
                    chip(
                        ui,
                        "ZWJ·표시 선택자·분해된 악센트는 원문 그대로 보존합니다.",
                    );
                    ui.add_space(10.);
                    ui.add(
                        egui::TextEdit::multiline(&mut self.add_text)
                            .desired_width(470.)
                            .desired_rows(4),
                    );
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt("add-locale")
                            .selected_text(&self.add_locale)
                            .show_ui(ui, |ui| {
                                for (l, label) in [
                                    ("default", "공통"),
                                    ("ko", "한국어 자형"),
                                    ("ja", "일본어 자형"),
                                ] {
                                    ui.selectable_value(&mut self.add_locale, l.into(), label);
                                }
                            });
                        ui.checkbox(&mut self.add_custom, "전체 문자열 → 하나의 글리프");
                    });
                    if ui.button("추출해서 추가").clicked() {
                        if self.add_text.len() > 1_000_000 {
                            self.notify("100만 바이트 이하 문장을 사용해 주세요.");
                        } else {
                            let texts = if self.add_custom {
                                vec![self.add_text.clone()]
                            } else {
                                graphemes(&self.add_text)
                            };
                            let n = add_texts(
                                &mut self.project,
                                &texts,
                                &self.add_locale,
                                "custom",
                                self.add_custom,
                            );
                            self.reindex();
                            self.group = 9;
                            self.refresh_catalog();
                            self.changed();
                            self.notify(format!(
                                "{n}개 항목을 추가했어요. 이미 있는 매핑은 유지됩니다."
                            ));
                            self.add_open = false;
                        }
                    }
                    ui.add_space(14.);
                    ui.separator();
                    ui.label("나만의 아이콘 · 사적 사용 영역 (PUA)");
                    ui.horizontal(|ui| {
                        ui.label("U+");
                        ui.text_edit_singleline(&mut self.pua);
                        if ui.button("추가").clicked() {
                            match u32::from_str_radix(self.pua.trim(), 16)
                                .ok()
                                .filter(|cp| (0xe000..=0xf8ff).contains(cp))
                                .and_then(char::from_u32)
                            {
                                Some(c) => {
                                    add_texts(
                                        &mut self.project,
                                        &[c.to_string()],
                                        "default",
                                        "custom",
                                        false,
                                    );
                                    self.reindex();
                                    self.group = 9;
                                    self.refresh_catalog();
                                    self.select_text(&c.to_string());
                                    self.changed();
                                    self.add_open = false;
                                }
                                None => self.notify("PUA는 E000–F8FF 범위로 입력해 주세요."),
                            }
                        }
                    });
                });
            if !open {
                self.add_open = false;
            }
        }
        if self.export_open {
            let mut open = true;
            egui::Window::new("나의 폰트 내보내기")
                .open(&mut open).default_width(570.).default_height(700.).resizable(true)
                .vscroll(true).max_height(ctx.content_rect().height() * 0.85)
                .show(ctx, |ui| {
                    ui.label(RichText::new("그린 글자를, 진짜 폰트로.").size(23.).strong());
                    ui.add_space(12.);
                    ui.label("폰트 패밀리 이름");
                    if ui.text_edit_singleline(&mut self.project.family).changed() { self.changed(); }
                    ui.add_space(8.);
                    ui.label("파일 포맷");
                    ui.horizontal_wrapped(|ui| {
                        for (value, label) in [("ttf", "TTF"), ("otf", "OTF"), ("woff2", "WOFF2"), ("woff", "WOFF")] {
                            let response = ui.selectable_value(&mut self.export_format, value.into(), label);
                            if response.changed() { self.export_report.clear(); }
                            #[cfg(feature = "ui-qa")]
                            { self.qa_export_formats.insert(value.into(), response.rect); }
                        }
                    });
                    chip(ui, match self.export_format.as_str() {
                        "otf" => "컴퓨터에 설치 · 디자인 작업용 OpenType 폰트",
                        "woff2" => "웹사이트용 · 압축된 웹폰트",
                        "woff" => "웹사이트용 · WOFF 지원 환경에 제공하는 웹폰트",
                        _ => "컴퓨터에 설치 · 일반 문서와 편집 작업용 TrueType 폰트",
                    });
                    ui.add_space(8.);
                    ui.label("글자 표현 방식");
                    egui::ComboBox::from_id_salt("export-mode")
                        .selected_text(match self.export_mode.as_str() { "mono" => "단색 · 이미지 실루엣 변환", "bitmap" => "비트맵 · 그림 그대로", _ => "컬러 · 벡터 레이어" })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.export_mode, "color".into(), "컬러 · 벡터 레이어");
                            ui.selectable_value(&mut self.export_mode, "mono".into(), "단색 · 이미지 실루엣 변환");
                            ui.selectable_value(&mut self.export_mode, "bitmap".into(), "비트맵 · 그림 그대로");
                        });
                    ui.add_space(8.);
                    ui.label("출력 범위");
                    egui::ComboBox::from_id_salt("export-locale")
                        .selected_text(match self.export_locale.as_str() { "ko" => "한국어용", "ja" => "일본어용", "default" => "공통 자형만", _ => "통합 · 지역별 자형" })
                        .show_ui(ui, |ui| {
                            for (v, l) in [("combined", "통합 · 지역별 자형"), ("ko", "한국어용"), ("ja", "일본어용"), ("default", "공통 자형만")] {
                                ui.selectable_value(&mut self.export_locale, v.into(), l);
                            }
                        });
                    ui.checkbox(&mut self.export_subset, "이 문장에 필요한 글리프만 출력");
                    if self.export_subset { ui.text_edit_multiline(&mut self.export_text); }
                    ui.checkbox(&mut self.export_group, "현재 문자 모음만 출력");
                    ui.checkbox(&mut self.export_split, "문자 모음별 분할 · ZIP");
                    ui.add_space(12.);
                    let count = self.project.glyphs.iter().filter(|g| g.status == "edited" || g.status == "blank").count();
                    ui.label(format!("프로젝트 작성 항목 {count}개 · 레이어·시퀀스 부품을 포함해 출력 개수를 검증합니다."));
                    chip(ui, "시스템 대체 그림은 포함되지 않습니다. 미작성·미지원·시스템 표시 항목은 제외해요.");
                    chip(ui, "컬러 모드에 이미지가 있으면 비트맵으로 출력합니다. 단색 모드는 이미지를 실루엣으로 변환해요.");
                    chip(ui, "컬러·비트맵 표시 여부는 사용하는 앱이나 브라우저에 따라 달라집니다.");
                    ui.add_space(12.);
                    ui.horizontal(|ui| {
                        for (save, label) in [(false, "출력 전 검사"), (true, "파일로 저장")] {
                            let button = ui.add_enabled(self.job.is_none(), egui::Button::new(label));
                            #[cfg(feature = "ui-qa")]
                            if !save { self.qa_export_check = Some(button.rect); }
                            if button.clicked() {
                                let ext = if self.export_split { "zip" } else { &self.export_format };
                                let path = if save {
                                    rfd::FileDialog::new()
                                        .add_filter(ext.to_uppercase(), &[ext])
                                        .set_file_name(if self.export_split { "Doodle-Font-fonts.zip".into() } else { format!("{}.{ext}", self.project.family) })
                                        .save_file()
                                } else { None };
                                if save && path.is_none() { continue; }
                                if path.as_ref().is_some_and(|p| !p.extension().and_then(|s| s.to_str()).is_some_and(|s| s.eq_ignore_ascii_case(ext))) {
                                    self.notify(format!("선택한 포맷에 맞게 파일 이름을 .{ext}로 끝내 주세요."));
                                    continue;
                                }
                                let p = self.project.clone();
                                let options = font::ExportOptions {
                                    format: Some(self.export_format.clone()),
                                    mode: Some(self.export_mode.clone()),
                                    locale: Some(self.export_locale.clone()),
                                    group: Some(if self.export_group { GROUPS[self.group].0.into() } else { "all".into() }),
                                    ids: if self.export_subset { Some(self.project.glyphs.iter().filter(|g| self.export_text.contains(&g.text)).map(|g| g.id.clone()).collect()) } else { None },
                                    split: Some(self.export_split),
                                };
                                self.spawn(ctx, move || {
                                    let result = font::export(&p, &options)?;
                                    if let Some(path) = &path {
                                        std::fs::write(path, STANDARD.decode(&result.data).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
                                        std::fs::write(path.with_extension("report.json"), serde_json::to_vec_pretty(&result.reports).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
                                    }
                                    Ok(Job::Export(result, path))
                                });
                            }
                        }
                    });
                    if !self.export_report.is_empty() {
                        ui.add_space(10.);
                        egui::ScrollArea::vertical().max_height(180.).show(ui, |ui| { ui.label(&self.export_report); });
                    }
                });
            if !open {
                self.export_open = false;
            }
        }
        if self.help {
            egui::Window::new("두들 폰트 사용하기").open(&mut self.help).default_width(580.).show(ctx,|ui|{ui.label(RichText::new("한 글자부터 시작하세요.").size(24.));for t in ["1. 왼쪽 문자 모음에서 글자를 고릅니다. 전체 한글·한자는 선택할 때만 작업 데이터가 만들어집니다.","2. 펜·직선·도형으로 그립니다. 레이어를 고르면 이동·크기·색·순서·잠금을 바꿀 수 있습니다.","3. 기준 저장을 눌러 승인본을 남겨 두세요. 템플릿 원본 복원과 실행 취소는 별개입니다.","4. 문자 추가에서 문장을 추출하거나 전체 문자열을 하나의 그림에 매핑할 수 있습니다.","5. 문장 미리보기에서 프로젝트 그림과 시스템 대체를 구분합니다. 출력한 TTF로 치환 결과도 검사하세요.","6. 폰트 내보내기에서 단색·컬러·비트맵 및 한국어·일본어·분할 출력을 선택합니다.","단축키: Ctrl+Z 실행 취소 · Ctrl+Y 다시 실행 · Ctrl+S 프로젝트 사본 저장","이미지 원본은 프로젝트에 PNG로 보존합니다. 단색 벡터 변환 전 그림은 기준 버전으로 남깁니다.","자모 조합은 U+1100 초성·U+1161 중성·U+11A8 종성의 작성 글리프를 이용합니다. 조합 후 각 음절을 개별 수정할 수 있습니다.","이 앱은 다른 기기나 모든 앱의 이모지를 일괄 변경하지 않습니다. 각 앱에서 출력 폰트를 선택해야 합니다."]{ui.add_space(9.);ui.label(t);}});
        }
    }
}
fn glyph_bytes(g: &Glyph) -> usize {
    fn art(a: &Artwork) -> usize {
        a.shapes
            .iter()
            .map(|s| {
                s.src.as_ref().map_or(0, String::len)
                    + (s.points.len() + s.contours.iter().map(Vec::len).sum::<usize>()) * 16
                    + 256
            })
            .sum()
    }
    art(&g.artwork)
        + g.baseline.as_ref().map_or(0, art)
        + g.template
            .iter()
            .chain(&g.sources)
            .map(|t| t.original.as_ref().map_or(0, art))
            .sum::<usize>()
        + g.candidates.iter().map(|c| art(&c.artwork)).sum::<usize>()
}
fn shape_bounds(s: &Shape) -> [f64; 4] {
    if s.kind == "path" && !s.points.is_empty() {
        [
            s.points.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min),
            s.points.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min),
            s.points
                .iter()
                .map(|p| p[0])
                .fold(f64::NEG_INFINITY, f64::max),
            s.points
                .iter()
                .map(|p| p[1])
                .fold(f64::NEG_INFINITY, f64::max),
        ]
    } else {
        [s.x, s.y, s.x + s.width, s.y + s.height]
    }
}
impl eframe::App for Studio {
    #[cfg(feature = "ui-qa")]
    fn raw_input_hook(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
        let qa_view = std::env::var("DOODLE_QA_VIEW").unwrap_or_default();
        if self.layout_qa_input(ctx, input, &qa_view) {
            return;
        }
        if self.settings_qa_input(ctx, input, &qa_view) {
            return;
        }
        if qa_view.starts_with("renderer-") {
            input
                .events
                .retain(|event| matches!(event, egui::Event::Screenshot { .. }));
            let frame = ctx.cumulative_frame_nr();
            let rect = match (qa_view.as_str(), frame) {
                ("renderer-write" | "renderer-compare-small", 3 | 4) => self.qa_renderers[2],
                ("renderer-image", 3 | 4) => self.qa_renderers[0],
                ("renderer-write", 7 | 8) => self
                    .companion_comparison
                    .as_ref()
                    .and_then(|c| c.qa_choices[1]),
                _ => None,
            };
            if let Some(rect) = rect {
                input.events.push(egui::Event::PointerMoved(rect.center()));
                input.events.push(egui::Event::PointerButton {
                    pos: rect.center(),
                    button: egui::PointerButton::Primary,
                    pressed: frame == 3 || frame == 7,
                    modifiers: Default::default(),
                });
            }
            return;
        }
        if qa_view == "appearance-write" {
            input
                .events
                .retain(|event| matches!(event, egui::Event::Screenshot { .. }));
            let frame = ctx.cumulative_frame_nr();
            if let Some(rect) = self.qa_themes[2].filter(|_| frame == 3 || frame == 4) {
                let pos = rect.center();
                input.events.push(egui::Event::PointerMoved(pos));
                input.events.push(egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: frame == 3,
                    modifiers: Default::default(),
                });
            }
            return;
        }
        if let Some(format) = qa_view.strip_prefix("export-") {
            let format = format.trim_end_matches("-small");
            input
                .events
                .retain(|event| matches!(event, egui::Event::Screenshot { .. }));
            let frame = ctx.cumulative_frame_nr();
            let rect = match frame {
                3 | 4 => self.qa_export_formats.get(format).copied(),
                6 | 7 => self.qa_export_check,
                _ => None,
            };
            if let Some(rect) = rect {
                input.events.push(egui::Event::PointerMoved(rect.center()));
                input.events.push(egui::Event::PointerButton {
                    pos: rect.center(),
                    button: egui::PointerButton::Primary,
                    pressed: frame == 3 || frame == 6,
                    modifiers: Default::default(),
                });
            }
        }
        if qa_view.starts_with("ai-floating") {
            input
                .events
                .retain(|event| matches!(event, egui::Event::Screenshot { .. }));
            let frame = ctx.cumulative_frame_nr();
            input.time = Some(frame as f64 / 30.);
            if (7..=9).contains(&frame) {
                let rect = self.qa_dock.companion.expect("separate companion");
                let pos = rect.center() + Vec2::new(if frame == 7 { -12. } else { 12. }, 0.);
                input.events.push(egui::Event::PointerMoved(pos));
                if frame != 8 {
                    input.events.push(egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: frame == 7,
                        modifiers: Default::default(),
                    });
                }
            }
            let scroll = match frame {
                16 => self.qa_dock.answer,
                22 => self.qa_dock.prompt,
                _ => None,
            };
            if let Some(rect) = scroll {
                input.events.push(egui::Event::PointerMoved(rect.center()));
                input.events.push(egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: Vec2::new(0., -160.),
                    modifiers: Default::default(),
                });
            }
            if (18..=20).contains(&frame) {
                let rect = self
                    .qa_dock
                    .answer
                    .unwrap()
                    .intersect(self.qa_canvas.unwrap());
                let pos = rect.center() + Vec2::new(if frame == 18 { -10. } else { 10. }, 0.);
                assert!(
                    self.qa_canvas.unwrap().contains(pos),
                    "Test drag must overlap the canvas below the dock"
                );
                input.events.push(egui::Event::PointerMoved(pos));
                if frame != 19 {
                    input.events.push(egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: frame == 18,
                        modifiers: Default::default(),
                    });
                }
            }
            if let Some(rect) = (if frame >= 48 {
                self.qa_dock.restore
            } else {
                self.qa_dock.collapse
            })
            .filter(|_| [26, 27, 48, 49].contains(&frame))
            {
                input.events.push(egui::Event::PointerMoved(rect.center()));
                input.events.push(egui::Event::PointerButton {
                    pos: rect.center(),
                    button: egui::PointerButton::Primary,
                    pressed: frame == 26 || frame == 48,
                    modifiers: Default::default(),
                });
            }
            if (40..=42).contains(&frame) {
                let rect = self
                    .qa_dock
                    .expanded
                    .unwrap()
                    .intersect(self.qa_canvas.unwrap());
                let pos = rect.center() + Vec2::new(if frame == 40 { -12. } else { 12. }, 0.);
                input.events.push(egui::Event::PointerMoved(pos));
                if frame != 41 {
                    input.events.push(egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: frame == 40,
                        modifiers: Default::default(),
                    });
                }
            }
            return;
        }
        if qa_view.starts_with("connections-") {
            input
                .events
                .retain(|event| matches!(event, egui::Event::Screenshot { .. }));
            let frame = ctx.cumulative_frame_nr();
            input.time = Some(frame as f64 / 30.);
            let rect = if frame == 3 || frame == 4 {
                self.qa_connection_header
            } else {
                match qa_view.as_str() {
                    "connections-write" => self.qa_api_key,
                    "connections-delete" => self.qa_forget_key,
                    _ => None,
                }
            };
            if let Some(rect) = rect.filter(|_| [3, 4, 12, 13].contains(&frame)) {
                let pos = rect.center();
                input.events.push(egui::Event::PointerMoved(pos));
                input.events.push(egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: frame == 3 || frame == 12,
                    modifiers: Default::default(),
                });
            }
            if qa_view == "connections-write" && frame == 14 {
                input
                    .events
                    .push(egui::Event::Text("qa-persistence-key-only".into()));
            }
            return;
        }
        if [
            "ai-candidate-live",
            "ai-typography-live",
            "ai-candidate-accept",
            "ai-typography",
            "decision-settings",
            "decision-fast-live",
            "decision-gemma-live",
        ]
        .contains(&qa_view.as_str())
        {
            input
                .events
                .retain(|e| matches!(e, egui::Event::Screenshot { .. }));
            let (rect, press_frame) = if qa_view == "decision-settings" {
                (self.qa_decision_button, 3)
            } else if qa_view.ends_with("-live") {
                (self.qa_ai_send, 3)
            } else {
                (self.qa_candidate_accept, 5)
            };
            let frame = ctx.cumulative_frame_nr();
            if let Some(rect) = rect.filter(|_| frame == press_frame || frame == press_frame + 1) {
                let pos = rect.center();
                input.events.push(egui::Event::PointerMoved(pos));
                input.events.push(egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: frame == press_frame,
                    modifiers: Default::default(),
                });
            }
            return;
        }
        if std::env::var("DOODLE_QA_VIEW").as_deref() == Ok("install-flow") {
            input
                .events
                .retain(|e| matches!(e, egui::Event::Screenshot { .. }));
            if let Some(rect) = self.qa_install_button {
                let frame = ctx.cumulative_frame_nr();
                if frame == 3 || frame == 4 {
                    let pos = rect.center();
                    input.events.push(egui::Event::PointerMoved(pos));
                    input.events.push(egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: frame == 3,
                        modifiers: Default::default(),
                    });
                }
            }
            return;
        }
        if ![
            "interaction",
            "ai-busy",
            "ai-busy-small",
            "ai-busy-light",
            "ai-recover",
        ]
        .contains(&std::env::var("DOODLE_QA_VIEW").unwrap_or_default().as_str())
        {
            return;
        }
        input
            .events
            .retain(|e| matches!(e, egui::Event::Screenshot { .. }));
        if let Some(rect) = self.qa_canvas {
            let frame = ctx.cumulative_frame_nr();
            let p = rect.min
                + rect.size()
                    * match frame {
                        3 => Vec2::new(0.22, 0.22),
                        4 => Vec2::new(0.35, 0.38),
                        _ => Vec2::new(0.48, 0.28),
                    };
            if (3..=6).contains(&frame) {
                input.events.push(egui::Event::PointerMoved(p));
            }
            if frame == 3 || frame == 6 {
                input.events.push(egui::Event::PointerButton {
                    pos: p,
                    button: egui::PointerButton::Primary,
                    pressed: frame == 3,
                    modifiers: Default::default(),
                });
            }
        }
    }
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(windows)]
        self.window_corners.sync(ctx, _frame);
        #[cfg(all(windows, feature = "ui-qa"))]
        self.window_corners_qa(ctx, _frame);
        if ctx.input(|i| i.viewport().close_requested())
            && self.ai_cancel.is_some()
            && self.job.is_some()
        {
            self.close_after_ai = true;
            if let Some(cancel) = &self.ai_cancel {
                cancel.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        #[cfg(feature = "ui-qa")]
        if let Ok(output) = std::env::var("DOODLE_QA_OUTPUT") {
            if ctx.cumulative_frame_nr() == 0 {
                let qa_view = std::env::var("DOODLE_QA_VIEW").unwrap_or_default();
                if !qa_view.starts_with("settings") && self.api_provider == "off" {
                    self.api_provider = "local".into();
                }
                self.settings_qa_setup(ctx, &qa_view);
                self.layout_qa_setup(ctx, &qa_view);
                match std::env::var("DOODLE_QA_VIEW").unwrap_or_default().as_str() {
                    "renderer-write"
                    | "renderer-restored"
                    | "renderer-image"
                    | "renderer-compare-small" => {
                        use doodle_core::companion::Renderer;
                        let view = std::env::var("DOODLE_QA_VIEW").unwrap();
                        self.tab = 3;
                        self.ai_dock_collapsed = true;
                        if view == "renderer-write" {
                            self.preferences.companion_renderer = Renderer::Image;
                            self.companion.set_renderer(ctx, Renderer::Image);
                        }
                        if view == "renderer-restored" || view == "renderer-image" {
                            assert_eq!(self.preferences.companion_renderer, Renderer::Vector);
                            assert_eq!(self.companion.renderer(), Renderer::Vector);
                        }
                        if view.ends_with("small") {
                            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(
                                1120., 760.,
                            )));
                        }
                    }
                    "appearance-write" => {
                        self.tab = 3;
                        self.preferences.theme = Theme::Light;
                        ctx.set_theme(egui::Theme::Light);
                        self.ai_dock_collapsed = true;
                    }
                    "appearance-restored" => {
                        self.tab = 3;
                        assert_eq!(self.preferences.theme, Theme::Dark);
                        assert!(ctx.style().visuals.dark_mode);
                        self.ai_dock_collapsed = true;
                    }
                    "theme-light" | "theme-light-small" | "theme-dark-small" => {
                        let view = std::env::var("DOODLE_QA_VIEW").unwrap();
                        self.preferences.theme = if view.contains("dark") {
                            Theme::Dark
                        } else {
                            Theme::Light
                        };
                        ctx.set_theme(self.preferences.theme.preference());
                        if view.ends_with("small") {
                            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(
                                1120., 760.,
                            )));
                        }
                    }
                    "preview" => self.tab = 1,
                    "templates" => self.tab = 2,
                    "settings" => {
                        self.tab = 3;
                        self.ai_dock_collapsed = true;
                    }
                    "install-flow" | "install-missing" => self.tab = 3,
                    "ai-ready" => {
                        self.local_installed = true;
                    }
                    "ai-candidates" | "ai-candidate-accept" | "ai-candidate-invalid" => {
                        self.tab = 3;
                        self.local_installed = true;
                        self.ai_action = true;
                        self.ai_prompt =
                            "현재 A의 모양을 유지하고 획 굵기를 70으로 통일해 줘.".into();
                        let (tx, rx) = mpsc::channel();
                        self.job = Some(rx);
                        self.ai_cancel = Some(std::sync::Arc::new(
                            std::sync::atomic::AtomicBool::new(false),
                        ));
                        let invalid = std::env::var("DOODLE_QA_VIEW").as_deref()
                            == Ok("ai-candidate-invalid");
                        let result = if invalid {
                            serde_json::json!({"answer":"설명만 반환된 응답"})
                        } else {
                            // This fixture tests delivery and review, not AI accuracy.
                            // Keep the target letter instead of borrowing the smiley glyph.
                            let mut artwork = self.project.glyphs[0].artwork.clone();
                            for shape in &mut artwork.shapes {
                                shape.stroke = 70.;
                            }
                            serde_json::json!({"artwork":artwork})
                        };
                        tx.send(Ok(Job::AiReply(
                            self.project.glyphs[0].id.clone(),
                            true,
                            result,
                        )))
                        .unwrap();
                        self.poll();
                        assert!(self.ai_cancel.is_none());
                        assert_eq!(
                            self.project.glyphs[0].candidates.len(),
                            usize::from(!invalid)
                        );
                        assert_eq!(self.candidate_review.is_some(), !invalid);
                        assert_eq!(
                            self.project.glyphs[0].artwork.shapes.len(),
                            2,
                            "generation must not apply automatically"
                        );
                        if invalid {
                            assert!(self.ai_answer.contains("후보를 만들지 못했어요"));
                        }
                    }
                    "ai-candidate-existing" => self.open_candidate_review(),
                    "decision-settings" => {
                        self.tab = 3;
                        self.api_provider = "anthropic".into();
                        self.api_endpoint = "https://api.anthropic.com/v1".into();
                        self.decision_mode = doodle_core::needle::Mode::Off;
                    }
                    "ai-floating" | "ai-floating-small" | "ai-floating-reduced" => {
                        self.tab = 0;
                        self.local_installed = false;
                        self.preferences.reduce_motion = qa_view.ends_with("reduced");
                        if std::env::var("DOODLE_QA_VIEW").as_deref() == Ok("ai-floating-small") {
                            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(
                                1120., 760.,
                            )));
                        }
                    }
                    "connections-write" => {
                        self.tab = 3;
                        self.select_ai_provider("openai");
                        self.api_endpoint = "https://example.invalid/v1".into();
                        self.api_model = "qa-model".into();
                        self.api_key.clear();
                        self.connection_changed();
                    }
                    "connections-restored" | "connections-delete" | "connections-empty" => {
                        self.tab = 3;
                        assert_eq!(self.api_provider, "openai");
                        assert_eq!(self.api_endpoint, "https://example.invalid/v1");
                        assert_eq!(self.api_model, "qa-model");
                        let deleted =
                            std::env::var("DOODLE_QA_VIEW").as_deref() == Ok("connections-empty");
                        assert_eq!(
                            self.api_key,
                            if deleted {
                                ""
                            } else {
                                "qa-persistence-key-only"
                            }
                        );
                        assert_eq!(self.ai_ready(), !deleted);
                    }
                    "decision-fast-live" | "decision-gemma-live" => {
                        self.tab = 3;
                        self.decision_mode = doodle_core::needle::Mode::Needle;
                        self.ai_action = true;
                        assert!(self.needle_installed);
                        if std::env::var("DOODLE_QA_VIEW").as_deref() == Ok("decision-fast-live") {
                            self.api_provider = "openai".into();
                            self.api_endpoint = "http://127.0.0.1:1/v1".into();
                            self.api_model = "unreachable".into();
                            self.api_key = "qa-only".into();
                            self.ai_prompt = "Set the stroke width to 70.".into();
                        } else {
                            assert!(self.local_installed);
                            self.ai_prompt = "A를 굵고 둥근 고딕체로 그려줘.".into();
                        }
                    }
                    "ai-layout" => {
                        self.group = 0;
                        self.select_text("g");
                        let generated = doodle_core::typography::generate(
                            &self.project.glyphs[self.selected],
                            &Default::default(),
                            true,
                        )
                        .unwrap();
                        let glyph = &mut self.project.glyphs[self.selected];
                        glyph.artwork = generated.artwork;
                        glyph.sources = generated.sources;
                        glyph.status = "edited".into();
                        self.refresh_catalog();
                        self.ai_dock_collapsed = true;
                    }
                    "ai-typography" => {
                        self.local_installed = true;
                        let g = &self.project.glyphs[0];
                        let generated = doodle_core::typography::generate(
                            g,
                            &doodle_core::typography::Style {
                                weight: 650,
                                roundness: 80,
                                ..Default::default()
                            },
                            true,
                        )
                        .unwrap();
                        let (tx, rx) = mpsc::channel();
                        self.job = Some(rx);
                        tx.send(Ok(Job::AiReply(g.id.clone(),true,serde_json::json!({"artwork":generated.artwork,"sources":generated.sources,"generationNote":generated.note})))).unwrap();
                        self.poll();
                        assert!(self.candidate_review.is_some());
                    }
                    "ai-candidate-live" | "ai-typography-live" => {
                        self.tab = 3;
                        let typography =
                            std::env::var("DOODLE_QA_VIEW").as_deref() == Ok("ai-typography-live");
                        self.selected = if typography { 0 } else { 1 };
                        self.ai_action = true;
                        self.ai_prompt = if typography { "A를 굵고 둥근 고딕체로 정확하게 그려줘." } else { "참고 그림처럼 노란 원 안에 검은 눈 두 개와 웃는 입이 있는 얼굴을 만들어 줘." }.into();
                        assert!(
                            self.local_installed,
                            "live test needs installed local models"
                        );
                    }
                    "companion-inspecting" | "companion-drawing" | "companion-coloring" => {
                        use doodle_core::{ai_progress::Progress, companion::Mood};
                        self.local_installed = true;
                        self.ai_cancel = Some(std::sync::Arc::new(
                            std::sync::atomic::AtomicBool::new(false),
                        ));
                        let mood = match std::env::var("DOODLE_QA_VIEW").unwrap().as_str() {
                            "companion-inspecting" => Mood::Inspecting,
                            "companion-coloring" => Mood::Coloring,
                            _ => Mood::Drawing,
                        };
                        let (tx, rx) = mpsc::channel();
                        self.ai_progress = Some(rx);
                        tx.send(Progress::Message(mood.label().into())).unwrap();
                        tx.send(Progress::Animation(mood)).unwrap();
                        self.poll();
                    }
                    "companion-explaining" => {
                        let (tx, rx) = mpsc::channel();
                        self.job = Some(rx);
                        tx.send(Ok(Job::AiReply(
                            self.project.glyphs[self.selected].id.clone(),
                            false,
                            serde_json::json!({"answer":"기준선과 획의 균형을 확인했어요."}),
                        )))
                        .unwrap();
                        self.poll();
                    }
                    "ai-busy" | "ai-busy-small" | "ai-busy-light" | "ai-recover" => {
                        self.local_installed = true;
                        self.ai_cancel = Some(std::sync::Arc::new(
                            std::sync::atomic::AtomicBool::new(false),
                        ));
                        self.ai_status = "프롬프트와 그림을 함께 분석 중".into();
                        let view = std::env::var("DOODLE_QA_VIEW").unwrap();
                        if view.ends_with("-small") {
                            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(
                                1120., 760.,
                            )));
                        }
                        if view.ends_with("-light") {
                            self.preferences.theme = Theme::Light;
                            ctx.set_theme(egui::Theme::Light);
                        }
                    }
                    "export" | "export-ttf" | "export-otf" | "export-woff" | "export-woff2"
                    | "export-woff2-small" => {
                        self.export_open = true;
                        if std::env::var("DOODLE_QA_VIEW").unwrap().ends_with("-small") {
                            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(
                                1120., 760.,
                            )));
                        }
                    }
                    "hangul" => {
                        self.group = 2;
                        self.refresh_catalog();
                    }
                    _ => {}
                }
                if qa_view.ends_with("-live") {
                    self.tab = 0;
                }
            }
            ctx.request_repaint();
            if std::env::var("DOODLE_QA_VIEW")
                .unwrap_or_default()
                .starts_with("ai-floating")
            {
                let frame = ctx.cumulative_frame_nr();
                if frame == 4 {
                    self.qa_dock.baseline = self.qa_canvas;
                    self.qa_dock.companion_baseline = self.qa_dock.companion;
                }
                if frame == 6 {
                    self.local_installed = true;
                }
                if frame == 10 {
                    self.qa_dock.companion_baseline = self.qa_dock.companion;
                    assert_eq!(
                        self.qa_canvas, self.qa_dock.baseline,
                        "AI availability must not resize the canvas"
                    );
                    self.ai_answer = (1..=80).map(|n| format!("{n}. A의 획 균형과 자간을 살펴보세요. 이 설명은 스크롤로 확인하는 UI 시험용 문장입니다.\n")).collect();
                    self.ai_prompt =
                        "현재 A의 모양을 유지하고 획을 부드럽게 다듬어 줘.\n".repeat(60);
                }
                if frame == 24 {
                    assert_eq!(
                        self.qa_canvas, self.qa_dock.baseline,
                        "Long messages must not resize the canvas"
                    );
                    assert!(self.qa_dock.answer.unwrap().height() <= 121.);
                    assert!(self.qa_dock.prompt.unwrap().height() <= 57.);
                    assert!(
                        self.qa_dock.answer_offset > 0.,
                        "Answer must respond to mouse-wheel scrolling"
                    );
                    assert!(
                        self.qa_dock.prompt_offset > 0.,
                        "Prompt must scroll independently"
                    );
                    assert!(self.qa_dock.dock.unwrap().height() < 340.);
                }
                if frame == 24 {
                    self.qa_dock.expanded = self.qa_dock.dock;
                }
                if frame == 32 && !self.preferences.reduce_motion {
                    let dock = self.qa_dock.dock.unwrap();
                    assert!(
                        dock.height() > 1.
                            && dock.height() < self.qa_dock.expanded.unwrap().height()
                    );
                }
                if frame == 44 {
                    assert!(self.ai_dock_collapsed);
                    assert!(
                        self.qa_dock.dock.is_none(),
                        "collapsed dock must leave no overlay"
                    );
                    assert!(self
                        .qa_dock
                        .footer
                        .unwrap()
                        .contains_rect(self.qa_dock.restore.unwrap()));
                    assert_eq!(
                        self.project.glyphs[0].artwork.shapes.len(),
                        3,
                        "collapsed area must allow drawing underneath"
                    );
                    self.undo(false);
                    assert_eq!(self.qa_canvas, self.qa_dock.baseline);
                    assert_eq!(
                        self.qa_dock.companion, self.qa_dock.companion_baseline,
                        "Collapsing the prompt must not shrink or move the companion"
                    );
                }
                if frame == 62 {
                    assert!(!self.ai_dock_collapsed);
                    assert!(self.qa_dock.dock.unwrap().height() < 340.);
                    assert_eq!(self.qa_canvas, self.qa_dock.baseline);
                    assert_eq!(
                        self.project.glyphs[0].artwork.shapes.len(),
                        2,
                        "Dock input must not draw through to the canvas"
                    );
                    let dock = self.qa_dock.dock.unwrap();
                    assert!(ctx.content_rect().contains_rect(dock));
                    self.notify("플로팅 UI 검증: 작업 영역 유지 · 응답/입력 스크롤 · 접기/펼치기");
                }
            }
            if std::env::var("DOODLE_QA_VIEW").as_deref() == Ok("decision-settings")
                && ctx.cumulative_frame_nr() == 10
            {
                assert_eq!(self.decision_mode, doodle_core::needle::Mode::Needle);
                assert_eq!(
                    doodle_core::needle::Options::load(
                        &self.save_path.with_file_name("ai-options.json")
                    )
                    .decision,
                    self.decision_mode
                );
                assert_eq!(self.api_provider, "anthropic");
            }
            if std::env::var("DOODLE_QA_VIEW").as_deref() == Ok("ai-typography")
                && ctx.cumulative_frame_nr() == 10
            {
                assert!(self.candidate_review.is_none());
                assert_eq!(self.project.glyphs[0].artwork.shapes[0].kind, "outline");
                assert_eq!(self.project.glyphs[0].sources.len(), 1);
                self.undo(false);
                assert_eq!(self.project.glyphs[0].artwork.shapes.len(), 2);
                assert!(self.project.glyphs[0].sources.is_empty());
                self.undo(true);
                assert_eq!(self.project.glyphs[0].sources.len(), 1);
                self.save();
                let saved = storage::read(&self.save_path).unwrap();
                assert_eq!(saved.glyphs[0].sources.len(), 1);
                self.open_candidate_review();
                self.notify("글자 형태 · 채택 · 출처 · 실행 취소 · 저장 검증 통과");
            }
            if std::env::var("DOODLE_QA_VIEW").as_deref() == Ok("ai-candidate-accept")
                && ctx.cumulative_frame_nr() == 10
            {
                assert!(
                    self.candidate_review.is_none(),
                    "click should apply and close review"
                );
                assert_eq!(self.tab, 0, "accept should return to editor from settings");
                let glyph = &self.project.glyphs[0];
                assert_eq!(
                    serde_json::to_value(&glyph.artwork).unwrap(),
                    serde_json::to_value(&glyph.candidates[0].artwork).unwrap()
                );
                self.undo(false);
                assert_eq!(self.project.glyphs[0].artwork.shapes.len(), 2);
                assert!(self.project.glyphs[0]
                    .artwork
                    .shapes
                    .iter()
                    .all(|shape| shape.stroke == 36.));
                self.undo(true);
                assert_eq!(
                    serde_json::to_value(&self.project.glyphs[0].artwork).unwrap(),
                    serde_json::to_value(&self.project.glyphs[0].candidates[0].artwork).unwrap()
                );
                assert!(self.project.glyphs[0]
                    .artwork
                    .shapes
                    .iter()
                    .all(|shape| shape.stroke == 70.));
                self.save();
                assert_eq!(
                    storage::read(&self.save_path).unwrap().glyphs[0]
                        .candidates
                        .len(),
                    1
                );
                self.open_candidate_review();
                assert!(
                    self.candidate_review.is_some(),
                    "saved candidates must reopen"
                );
                self.notify("후보 UI QA: 채택 · 실행 취소 · 다시 실행 · 저장 · 다시 열기 통과");
            }
            if ["ai-busy", "ai-busy-small", "ai-busy-light", "ai-recover"]
                .contains(&std::env::var("DOODLE_QA_VIEW").unwrap_or_default().as_str())
                && ctx.cumulative_frame_nr() == 8
            {
                assert_eq!(
                    self.project.glyphs[self.selected].artwork.shapes.len(),
                    2,
                    "AI lock must reject pointer drawing"
                );
                let original = serde_json::to_string(&self.project).unwrap();
                self.edit(|g| {
                    g.artwork.shapes.clear();
                    Ok(())
                });
                self.select_text("B");
                self.undo(false);
                assert_eq!(
                    serde_json::to_string(&self.project).unwrap(),
                    original,
                    "AI lock must block mutation commands"
                );
                if std::env::var("DOODLE_QA_VIEW").as_deref() == Ok("ai-recover") {
                    let (tx, rx) = mpsc::channel();
                    self.job = Some(rx);
                    tx.send(Err("QA: simulated connection error".into()))
                        .unwrap();
                    self.poll();
                    assert!(self.ai_cancel.is_none());
                    self.edit(|g| {
                        g.artwork.advance = 1234.;
                        Ok(())
                    });
                    assert_eq!(self.project.glyphs[self.selected].artwork.advance, 1234.);
                    self.notify("AI 오류 후 입력 잠금 해제 검증 통과");
                }
            }
            if std::env::var("DOODLE_QA_VIEW").as_deref() == Ok("interaction")
                && ctx.cumulative_frame_nr() == 8
            {
                assert_eq!(
                    self.project.glyphs[self.selected].artwork.shapes.len(),
                    3,
                    "pointer drawing should add a layer"
                );
                self.undo(false);
                assert_eq!(self.project.glyphs[self.selected].artwork.shapes.len(), 2);
                self.undo(true);
                assert_eq!(self.project.glyphs[self.selected].artwork.shapes.len(), 3);
                assert_eq!(
                    self.project.glyphs[self.selected]
                        .template
                        .as_ref()
                        .unwrap()
                        .original
                        .as_ref()
                        .unwrap()
                        .shapes
                        .len(),
                    2
                );
                self.save();
                let saved = storage::read(&self.save_path).expect("native autosave");
                assert_eq!(saved.glyphs[self.selected].artwork.shapes.len(), 3);
                self.notify("UI 자동 검증 통과: 그리기 · 실행 취소 · 다시 실행 · 원본 보존 · 저장");
            }
            let install_flow = std::env::var("DOODLE_QA_VIEW").as_deref() == Ok("install-flow");
            let candidate_live = [
                "ai-candidate-live",
                "ai-typography-live",
                "decision-fast-live",
                "decision-gemma-live",
            ]
            .contains(&std::env::var("DOODLE_QA_VIEW").unwrap_or_default().as_str());
            if candidate_live && self.ai_started.elapsed() > Duration::from_secs(420) {
                panic!("Local candidate UI timed out: {}", self.ai_status);
            }
            if candidate_live && ctx.cumulative_frame_nr() > 12 && self.job.is_none() {
                assert!(
                    self.candidate_review.is_some(),
                    "local drawing must open candidate review: {}",
                    self.ai_answer
                );
                assert_eq!(self.project.glyphs[self.selected].candidates.len(), 1);
                if std::env::var("DOODLE_QA_VIEW").as_deref() == Ok("decision-fast-live") {
                    assert!(self.ai_answer.contains("호출하지"));
                    assert_eq!(
                        self.project.glyphs[0].candidates[0].artwork.shapes[0].stroke,
                        70.
                    );
                }
                if std::env::var("DOODLE_QA_VIEW").as_deref() == Ok("decision-gemma-live") {
                    assert!(self.ai_answer.contains("넘겼어요"));
                    assert_eq!(
                        self.project.glyphs[0].candidates[0].artwork.shapes[0].kind,
                        "outline"
                    );
                }
                if std::env::var("DOODLE_QA_VIEW").as_deref() == Ok("ai-typography-live") {
                    let candidate = &self.project.glyphs[self.selected].candidates[0];
                    assert_eq!(candidate.artwork.shapes[0].kind, "outline");
                    assert!(!candidate.sources.is_empty());
                }
                self.save();
            }
            if install_flow && self.ai_started.elapsed() > Duration::from_secs(90) {
                panic!("Install UI flow timed out: {}", self.install_error);
            }
            if std::env::var("DOODLE_QA_VIEW").as_deref() == Ok("appearance-write")
                && ctx.cumulative_frame_nr() == 10
            {
                assert_eq!(self.preferences.theme, Theme::Dark);
                assert!(ctx.style().visuals.dark_mode);
                assert_eq!(
                    Preferences::load(&self.save_path.with_file_name("appearance.json")).theme,
                    Theme::Dark
                );
            }
            let connection_view = std::env::var("DOODLE_QA_VIEW").unwrap_or_default();
            if connection_view.starts_with("renderer-") && ctx.cumulative_frame_nr() == 10 {
                use doodle_core::companion::Renderer;
                if connection_view == "renderer-compare-small" {
                    let comparison = self
                        .companion_comparison
                        .as_ref()
                        .expect("comparison opened by pointer");
                    let rect = comparison.qa_rect.expect("comparison rendered");
                    assert!(
                        ctx.content_rect().contains_rect(rect),
                        "comparison exceeds small window: {rect:?}"
                    );
                    assert!(comparison.qa_choices.iter().all(Option::is_some));
                } else {
                    let expected = if connection_view == "renderer-image" {
                        Renderer::Image
                    } else {
                        Renderer::Vector
                    };
                    assert_eq!(self.preferences.companion_renderer, expected);
                    assert_eq!(self.companion.renderer(), expected);
                    assert!(
                        self.companion_comparison.is_none(),
                        "choosing renderer closes comparison"
                    );
                    assert_eq!(
                        Preferences::load(&self.save_path.with_file_name("appearance.json"))
                            .companion_renderer,
                        expected
                    );
                }
            }
            let connection_flow = connection_view.starts_with("connections-");
            let floating_flow = connection_view.starts_with("ai-floating");
            let export_flow = connection_view.starts_with("export-");
            let export_ready = export_flow
                && ctx.cumulative_frame_nr() >= 12
                && self.job.is_none()
                && !self.export_report.is_empty();
            if export_flow && self.ai_started.elapsed() > Duration::from_secs(15) {
                panic!("Export UI timed out: {}", self.message);
            }
            if export_ready {
                let format = connection_view
                    .strip_prefix("export-")
                    .unwrap()
                    .trim_end_matches("-small");
                assert_eq!(self.export_format, format);
                assert!(self.export_report.contains(&format.to_uppercase()));
                assert!(self
                    .export_report
                    .contains(if format == "otf" { "CFF " } else { "glyf" }));
            }
            let connection_ready = connection_flow
                && ctx.cumulative_frame_nr() >= 24
                && self.connections_pending.is_none();
            if connection_flow && self.ai_started.elapsed() > Duration::from_secs(15) {
                panic!("Connection persistence UI timed out");
            }
            if connection_ready {
                assert!(!self.connections_error);
                let deleted =
                    ["connections-delete", "connections-empty"].contains(&connection_view.as_str());
                let key = if deleted {
                    ""
                } else {
                    "qa-persistence-key-only"
                };
                assert_eq!(self.api_key, key);
                let restored =
                    doodle_core::ai_settings::Connections::load(&self.connections_path).unwrap();
                assert_eq!(restored.key("openai", "https://example.invalid/v1"), key);
                assert!(!serde_json::to_string(&self.project)
                    .unwrap()
                    .contains("qa-persistence-key-only"));
            }
            if (ctx.cumulative_frame_nr() == 12
                && !connection_view.starts_with("settings-")
                && !connection_view.starts_with("layout-")
                && !install_flow
                && !candidate_live
                && !connection_flow
                && !export_flow
                && !floating_flow)
                || self.settings_qa_ready(ctx, &connection_view)
                || self.layout_qa_ready(ctx, &connection_view)
                || export_ready
                || (floating_flow && ctx.cumulative_frame_nr() == 62)
                || connection_ready
                || (candidate_live
                    && ctx.cumulative_frame_nr() > 12
                    && self.job.is_none()
                    && self.candidate_review.is_some())
                || (install_flow
                    && ctx.cumulative_frame_nr() >= 12
                    && self.local_installed
                    && !self.installing_ai)
            {
                if install_flow {
                    assert!(self.install_error.is_empty());
                    assert!(self.ai_cancel.is_none());
                    assert!(doodle_core::local_ai::installed());
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            }
            let screenshot = ctx.input(|i| {
                i.events.iter().find_map(|e| {
                    if let egui::Event::Screenshot { image, .. } = e {
                        Some(image.clone())
                    } else {
                        None
                    }
                })
            });
            if let Some(im) = screenshot {
                let bytes: Vec<u8> = im.pixels.iter().flat_map(|p| p.to_array()).collect();
                image::save_buffer(
                    &output,
                    &bytes,
                    im.width() as u32,
                    im.height() as u32,
                    image::ColorType::Rgba8,
                )
                .expect("save QA frame");
                if std::env::var("DOODLE_QA_VIEW").as_deref() != Ok("layout-window") {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
        self.poll();
        if self.job.is_none() && self.last_install_check.elapsed() > Duration::from_secs(2) {
            self.local_installed = doodle_core::local_ai::installed();
            self.needle_installed = doodle_core::needle::installed();
            self.last_install_check = Instant::now();
        }
        if self.close_after_ai && self.job.is_none() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if self.dirty && self.last_edit.elapsed() > Duration::from_millis(900) {
            self.save();
        }
        if let Some(changed) = self.connections_pending {
            if changed.elapsed() >= Duration::from_millis(600) {
                self.save_connections();
            } else {
                ctx.request_repaint_after(Duration::from_millis(100));
            }
        }
        if self.dirty || self.job.is_some() {
            ctx.request_repaint_after(Duration::from_millis(300));
        }
        if self.tab == 3 && self.job.is_none() {
            ctx.request_repaint_after(Duration::from_secs(2));
        }
        self.palette = Palette::of(ctx);
        self.top(ctx);
        self.footer(ctx);
        if self.tab == 0 {
            self.library(ctx);
            self.properties(ctx);
            self.editor(ctx);
        } else {
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::new()
                        .fill(self.palette.surface)
                        .inner_margin(32),
                )
                .show(ctx, |ui| {
                    if self.ai_cancel.is_some() {
                        ui.disable();
                    }
                    match self.tab {
                        1 => self.preview_ui(ui),
                        2 => self.templates_ui(ui),
                        _ => {
                            egui::ScrollArea::vertical().show(ui, |ui| self.settings_ui(ui));
                        }
                    }
                });
        }
        self.dialogs(ctx);
        self.ai_dock(ctx);
        let chrome = doodle_core::window_chrome::show(
            ctx,
            &self.project.name,
            &self.project.family,
            self.palette,
        );
        #[cfg(feature = "ui-qa")]
        {
            self.qa_settings.project_heading = Some((
                chrome.project,
                self.project.name.clone(),
                self.project.family.clone(),
            ));
            self.qa_chrome = Some(chrome);
        }
        #[cfg(not(feature = "ui-qa"))]
        let _ = chrome;
        #[cfg(feature = "ui-qa")]
        if ctx.cumulative_frame_nr() == 10 {
            use doodle_core::companion::Mood;
            let expected = match std::env::var("DOODLE_QA_VIEW").unwrap_or_default().as_str() {
                "ai-busy" | "ai-busy-small" | "ai-busy-light" => Some(Mood::Thinking),
                "companion-inspecting" => Some(Mood::Inspecting),
                "companion-drawing" => Some(Mood::Drawing),
                "companion-coloring" => Some(Mood::Coloring),
                "companion-explaining" => Some(Mood::Explaining),
                "ai-recover" | "ai-candidate-invalid" => Some(Mood::NeedsHelp),
                "ai-candidates" | "ai-candidate-accept" | "ai-typography" => Some(Mood::Done),
                _ => None,
            };
            if let Some(expected) = expected {
                assert_eq!(self.companion.motion.mood, expected);
            }
        }
        #[cfg(feature = "ui-qa")]
        if ctx.cumulative_frame_nr() == 12 {
            if let (Some(companion), Some(dock)) = (self.qa_dock.companion, self.qa_dock.dock) {
                assert!(companion.width() >= 159. && companion.height() >= 159.);
                if !self.ai_dock_collapsed && self.preferences.companion_position.is_none() {
                    assert!(
                        dock.right() + 8. <= companion.left(),
                        "companion must be beside the expanded prompt"
                    );
                }
                assert!(ctx.content_rect().contains_rect(companion));
                assert!(
                    ctx.content_rect().contains_rect(dock),
                    "dock {dock:?} exceeds {:?}",
                    ctx.content_rect()
                );
            }
            if let Some(spinner) = self.qa_dock.footer_spinner {
                let footer = self.qa_dock.footer.unwrap();
                assert!(
                    footer.height() <= 32.1,
                    "spinner must not grow the footer: {footer:?}"
                );
                assert!(
                    footer.shrink(2.).contains_rect(spinner),
                    "spinner escaped footer: {spinner:?} / {footer:?}"
                );
                assert!(spinner.width() <= 14.1 && spinner.height() <= 14.1);
            }
        }
        if !self.message.is_empty()
            && (self.message_time.elapsed() < Duration::from_secs(9) || self.save_error)
        {
            egui::Area::new("toast".into())
                .anchor(Align2::CENTER_TOP, [0., 100.])
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    if self.ai_cancel.is_some() {
                        ui.disable();
                    }
                    egui::Frame::new()
                        .fill(Color32::from_rgb(45, 52, 63))
                        .corner_radius(10)
                        .inner_margin(16)
                        .show(ui, |ui| {
                            ui.set_max_width(780.);
                            ui.label(RichText::new(&self.message).color(Color32::WHITE));
                        });
                });
        }
        self.candidate_review_ui(ctx);
        if self.ai_cancel.is_none()
            && self.candidate_review.is_none()
            && !ctx.wants_keyboard_input()
        {
            if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Z)) {
                self.undo(false)
            }
            if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Y)) {
                self.undo(true)
            }
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.draft = None;
            }
        }
        if self.ai_cancel.is_none()
            && ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::S))
        {
            self.save_copy();
        }
    }
    fn on_exit(&mut self, _: Option<&eframe::glow::Context>) {
        if self.connections_pending.is_some() {
            self.save_connections();
        }
        if let Some(cancel) = &self.ai_cancel {
            cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        if self.dirty {
            self.save();
        }
    }
}
