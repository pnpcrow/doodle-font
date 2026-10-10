use base64::{engine::general_purpose::STANDARD, Engine};
use doodle_core::{document::*, font, model::*, storage};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};
const INK: Color32 = Color32::from_rgb(48, 51, 46);
const MUTED: Color32 = Color32::from_rgb(133, 135, 124);
const ACCENT: Color32 = Color32::from_rgb(211, 102, 65);
const PAPER: Color32 = Color32::from_rgb(249, 248, 243);
const LINE: Color32 = Color32::from_rgb(226, 227, 217);
type JobResult = Result<Job, String>;
enum Job {
    Export(font::ExportResult, Option<PathBuf>),
    AiReply(String, bool, serde_json::Value),
    AiInstalled(doodle_core::ai_install::Package),
    FontPreview(Vec<u8>, String),
    Imported(Project),
}
pub struct Studio {
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
    ai_prompt: String,
    send_reference: bool,
    ai_color: bool,
    ai_action: bool,
    ai_answer: String,
    candidate_review: Option<(String, String)>,
    ai_status: String,
    ai_started: Instant,
    ai_cancel: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    ai_progress: Option<mpsc::Receiver<String>>,
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
    qa_install_button: Option<Rect>,
    #[cfg(feature = "ui-qa")]
    qa_candidate_accept: Option<Rect>,
    #[cfg(feature = "ui-qa")]
    qa_ai_send: Option<Rect>,
    #[cfg(feature = "ui-qa")]
    qa_decision_button: Option<Rect>,
    #[cfg(feature = "ui-qa")]
    qa_api_key: Option<Rect>,
    #[cfg(feature = "ui-qa")]
    qa_forget_key: Option<Rect>,
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
        let mut style = (*ctx.style()).clone();
        style.visuals = egui::Visuals::light();
        style.visuals.panel_fill = PAPER;
        style.visuals.window_fill = PAPER;
        style.visuals.override_text_color = Some(INK);
        style.visuals.selection.bg_fill = Color32::from_rgb(248, 220, 202);
        style.visuals.selection.stroke = Stroke::new(1_f32, ACCENT);
        style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(238, 239, 232);
        style.visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(240, 240, 234);
        style.visuals.widgets.inactive.bg_stroke = Stroke::new(1_f32, LINE);
        style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(242, 223, 208);
        style.visuals.widgets.active.bg_fill = Color32::from_rgb(239, 200, 174);
        style.spacing.item_spacing = Vec2::new(9., 9.);
        style.spacing.button_padding = Vec2::new(12., 7.);
        style
            .text_styles
            .insert(egui::TextStyle::Body, FontId::proportional(15.));
        style
            .text_styles
            .insert(egui::TextStyle::Button, FontId::proportional(14.));
        style
            .text_styles
            .insert(egui::TextStyle::Small, FontId::proportional(12.));
        ctx.set_style(style);
        let root = std::env::var_os("DOODLE_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_default())
                    .join("DoodleFont")
            });
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
            color: INK,
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
            ai_prompt: "동글동글하고 따뜻한 손글씨. 부드러운 획.".into(),
            send_reference: false,
            ai_color: true,
            ai_action: false,
            ai_answer: String::new(),
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
            qa_install_button: None,
            #[cfg(feature = "ui-qa")]
            qa_candidate_accept: None,
            #[cfg(feature = "ui-qa")]
            qa_ai_send: None,
            #[cfg(feature = "ui-qa")]
            qa_decision_button: None,
            #[cfg(feature = "ui-qa")]
            qa_api_key: None,
            #[cfg(feature = "ui-qa")]
            qa_forget_key: None,
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
                self.ai_status = status;
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
                    let was_ai = self.ai_cancel.is_some() && !was_installing;
                    let cancelled = self
                        .ai_cancel
                        .take()
                        .is_some_and(|c| c.load(std::sync::atomic::Ordering::Relaxed));
                    self.ai_progress = None;
                    if cancelled {
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
                        Err(e) => {
                            if was_installing {
                                self.install_error = e.clone();
                            } else if was_ai {
                                self.ai_answer = format!("AI 작업을 완료하지 못했어요.\n{e}");
                            }
                            self.notify(e)
                        }
                        Ok(Job::AiInstalled(package)) => {
                            self.local_installed = doodle_core::local_ai::installed();
                            self.needle_installed = doodle_core::needle::installed();
                            self.install_error.clear();
                            self.notify(if package == doodle_core::ai_install::Package::Needle {
                                "판단 AI 설치를 마쳤어요. 모든 AI 연결의 입력 앞단에서 사용할 수 있어요."
                            } else { "Gemma 4 설치를 마쳤어요. 하단 입력창에서 사용할 수 있어요." });
                        }
                        Ok(Job::AiReply(id, drawing, result)) => {
                            match self.receive_ai_reply(&id, drawing, result) {
                                Ok(()) => self.notify(if drawing {
                                    "그림 후보를 만들었어요. 미리보기를 확인하고 채택해 주세요."
                                } else {
                                    "그림 상담을 마쳤어요."
                                }),
                                Err(e) => {
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
                                        "{} · {}개 글리프 · {}\n{}\n{}",
                                        r.family,
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
                    if self.ai_cancel.is_some() && !self.installing_ai {
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
    fn top(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("header")
            .exact_height(86.)
            .frame(
                egui::Frame::new()
                    .fill(PAPER)
                    .inner_margin(egui::Margin::symmetric(24, 14)),
            )
            .show(ctx, |ui| {
                if self.ai_cancel.is_some() {
                    ui.disable();
                }
                ui.horizontal(|ui| {
                    let (logo, _) = ui.allocate_exact_size(Vec2::splat(46.), Sense::hover());
                    ui.painter().rect_filled(logo, 14., ACCENT);
                    ui.painter().text(
                        logo.center(),
                        Align2::CENTER_CENTER,
                        "d",
                        FontId::proportional(37.),
                        Color32::WHITE,
                    );
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Doodle Font").size(23.).strong());
                        ui.label(
                            RichText::new("나만의 글자를 그리는 작업실")
                                .size(11.)
                                .color(MUTED),
                        );
                    });
                    ui.add_space(28.);
                    for (i, label) in ["글리프 작업실", "문장 미리보기", "템플릿 보관함", "AI 연결"]
                        .iter()
                        .enumerate()
                    {
                        if ui.selectable_label(self.tab == i, *label).clicked() {
                            self.tab = i;
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("폰트 내보내기  ↗").color(Color32::WHITE),
                                )
                                .fill(ACCENT),
                            )
                            .clicked()
                        {
                            self.export_open = true;
                        }
                        if ui.button("프로젝트 저장").clicked() {
                            self.save_copy()
                        }
                        if ui.button("열기").clicked() {
                            self.open_project(ctx)
                        }
                        if ui.button("새로").clicked() {
                            self.fresh_project();
                        }
                    });
                });
            });
    }
    fn footer(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("footer")
            .exact_height(32.)
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(239, 240, 232))
                    .inner_margin(egui::Margin::symmetric(20, 6)),
            )
            .show(ctx, |ui| {
                if self.ai_cancel.is_some() {
                    ui.disable();
                }
                ui.horizontal(|ui| {
                    ui.label(RichText::new("●").color(if self.save_error {
                        ACCENT
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
                    ui.small(format!(
                        "{}  /  {}개 작업 글리프",
                        self.project.name,
                        self.project.glyphs.len()
                    ));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("도움말  ?").clicked() {
                            self.help = true;
                        }
                        ui.small("RUST NATIVE   ·   1,000 UPM");
                        if self.job.is_some() {
                            ui.spinner();
                            ui.small("처리 중");
                        }
                    });
                });
            });
    }
    fn library(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("collections")
            .exact_width(208.)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(241, 242, 235))
                    .inner_margin(18),
            )
            .show(ctx, |ui| {
                if self.ai_cancel.is_some() {
                    ui.disable();
                }
                ui.add_space(14.);
                ui.label(RichText::new("MY FONT").size(11.).color(MUTED));
                ui.add_space(5.);
                ui.label(RichText::new(&self.project.name).size(19.).strong());
                ui.add_space(13.);
                let authored = self
                    .project
                    .glyphs
                    .iter()
                    .filter(|g| g.status == "edited" || g.status == "blank")
                    .count();
                ui.label(
                    RichText::new(format!("{authored}개의 작은 시작"))
                        .size(12.)
                        .color(MUTED),
                );
                ui.add_space(22.);
                ui.label(RichText::new("문자 모음").size(12.).color(MUTED));
                ui.add_space(5.);
                for (i, (id, label, _)) in GROUPS.iter().enumerate() {
                    let count = self
                        .project
                        .glyphs
                        .iter()
                        .filter(|g| g.group == *id && g.status == "edited")
                        .count();
                    let text = format!(
                        "{}  {}",
                        label,
                        if count > 0 {
                            count.to_string()
                        } else {
                            String::new()
                        }
                    );
                    if ui
                        .add_sized(
                            [172., 35.],
                            egui::Button::new(RichText::new(text).size(13.))
                                .selected(self.group == i),
                        )
                        .clicked()
                    {
                        self.group = i;
                        self.search.clear();
                        self.refresh_catalog();
                    }
                }
                ui.add_space(16.);
                if ui
                    .add_sized([172., 34.], egui::Button::new("+ 문자 · 문자열 추가"))
                    .clicked()
                {
                    self.add_open = true;
                }
                ui.add_space(18.);
                ui.separator();
                ui.add_space(14.);
                ui.label(RichText::new("한 획씩, 나답게.").size(16.).strong());
                ui.add_space(5.);
                ui.label(
                    RichText::new("완벽하지 않아도 좋아요.\n내 손끝에서 시작되는 폰트.")
                        .size(12.)
                        .color(MUTED),
                );
            });
        egui::SidePanel::left("glyphs")
            .exact_width(252.)
            .resizable(false)
            .frame(egui::Frame::new().fill(PAPER).inner_margin(16))
            .show(ctx, |ui| {
                if self.ai_cancel.is_some() {
                    ui.disable();
                }
                ui.add_space(10.);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(GROUPS[self.group].1).size(16.).strong());
                });
                ui.label(
                    RichText::new(format!(
                        "{}개 문자 · 필요한 글자만 작업",
                        self.catalog.len()
                    ))
                    .size(11.)
                    .color(MUTED),
                );
                ui.add_space(13.);
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
                                        Color32::from_rgb(251, 230, 212)
                                    } else {
                                        Color32::from_rgb(255, 254, 250)
                                    };
                                    ui.painter().rect(
                                        rect,
                                        8.,
                                        fill,
                                        Stroke::new(
                                            if is_selected { 1.5_f32 } else { 1_f32 },
                                            if is_selected { ACCENT } else { LINE },
                                        ),
                                        egui::StrokeKind::Inside,
                                    );
                                    if let Some(g) = g.filter(|g| g.status == "edited") {
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
                                                MUTED
                                            } else {
                                                INK
                                            },
                                        );
                                    }
                                    let dot = if g.is_some_and(|g| g.status == "edited") {
                                        Color32::from_rgb(113, 145, 103)
                                    } else {
                                        LINE
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
    ui.label(RichText::new(label).size(11.).color(MUTED));
}
impl Studio {
    fn properties(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("properties")
            .exact_width(278.)
            .resizable(false)
            .frame(egui::Frame::new().fill(PAPER).inner_margin(18))
            .show(ctx, |ui| {
                if self.ai_cancel.is_some() {
                    ui.disable();
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
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
                            ui.label(RichText::new(codepoints(&text)).size(10.).color(MUTED));
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
                        ui.label(RichText::new("글자 폭").size(12.).color(MUTED));
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
                                .selectable_label(
                                    self.layer == Some(i),
                                    RichText::new(name).size(12.),
                                )
                                .clicked()
                            {
                                self.layer = Some(i);
                            }
                            if ui
                                .add_enabled(
                                    enabled,
                                    egui::Button::new(if lock { "잠김" } else { "잠금" }).small(),
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
                                    let c = format!(
                                        "#{:02x}{:02x}{:02x}",
                                        rgba.r(),
                                        rgba.g(),
                                        rgba.b()
                                    );
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
                                        .add(
                                            egui::DragValue::new(&mut xy[0]).range(-4000.0..=4000.),
                                        )
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
                                        | ui.add(
                                            egui::DragValue::new(&mut xy[3]).range(1.0..=4000.),
                                        )
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
                                if ui.small_button("복제").clicked() {
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
                                if ui.small_button("위로").clicked()
                                    && i + 1
                                        < self.project.glyphs[self.selected].artwork.shapes.len()
                                {
                                    self.edit(|g| {
                                        g.artwork.shapes.swap(i, i + 1);
                                        Ok(())
                                    });
                                    self.layer = Some(i + 1);
                                }
                                if ui.small_button("아래로").clicked() && i > 0 {
                                    self.edit(|g| {
                                        g.artwork.shapes.swap(i, i - 1);
                                        Ok(())
                                    });
                                    self.layer = Some(i - 1);
                                }
                                if ui.small_button("삭제").clicked() {
                                    self.edit(|g| {
                                        g.artwork.shapes.remove(i);
                                        Ok(())
                                    });
                                    self.layer = None;
                                }
                            });
                            if is_path && ui.small_button("선 부드럽게 보정").clicked() {
                                self.edit(|g| {
                                    smooth(&mut g.artwork.shapes[i].points);
                                    Ok(())
                                });
                            }
                            if is_image && ui.small_button("이미지 → 단색 벡터 변환").clicked()
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
                                    if ui.small_button("채택").clicked() {
                                        self.edit(|g| accept(g, &id));
                                    }
                                });
                            });
                        }
                    });
                });
            });
    }
    fn editor(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(234, 236, 228))
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
                            .color(MUTED),
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
                    .fill(PAPER)
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
                            if ui.small_button("이미지 +").clicked() {
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
                        .small_button("+")
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
                        painter.rect_filled(outer, 4., Color32::from_rgb(255, 254, 250));
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
                                    LINE,
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
                            for (y, name) in
                                [(200., "대문자 높이"), (500., "중심선"), (800., "기준선")]
                            {
                                let c = if y == 800. {
                                    Color32::from_rgb(219, 151, 127)
                                } else {
                                    Color32::from_rgb(194, 202, 185)
                                };
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
                            let x = advance.min(1000.);
                            painter.line_segment(
                                [to_screen([x, 0.]), to_screen([x, 1000.])],
                                Stroke::new(1_f32, Color32::from_rgb(161, 181, 156)),
                            );
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
                                    MUTED,
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
                                Stroke::new(1_f32, ACCENT),
                                egui::StrokeKind::Outside,
                            );
                            if s.kind == "path" {
                                for pt in s.points.iter().step_by((s.points.len() / 24).max(1)) {
                                    painter.circle_filled(to_screen(*pt), 2.5, ACCENT);
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
                        && ui.small_button("자모로 조합").clicked()
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
        ui.label(RichText::new("작성한 그림과 시스템 대체 표시를 나란히 확인하세요.").color(MUTED));
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
            egui::Frame::new().fill(Color32::from_rgb(255,254,250)).stroke(Stroke::new(1_f32,LINE)).corner_radius(16).inner_margin(24).show(ui,|ui|{
                ui.horizontal(|ui|{chip(ui,"PROJECT PREVIEW");chip(ui,"● 프로젝트 그림");ui.label(RichText::new("○ 시스템 대체").size(11.).color(MUTED));});ui.add_space(20.);
                for line in self.preview_text.split('\n'){ui.horizontal_wrapped(|ui|{ui.spacing_mut().item_spacing.x=0.;for(text,index)in preview(line,&self.project.glyphs,&self.preview_locale){let g=index.map(|i|&self.project.glyphs[i]);let size=self.preview_size;if let Some(g)=g.filter(|g|g.status=="edited"||g.status=="blank"){
                    let width=(size*g.artwork.advance as f32/1000.).max(1.);let(rect,r)=ui.allocate_exact_size(Vec2::new(width,size+14.),Sense::hover());if g.status=="edited"{paint_art(ui.ctx(),ui.painter(),Rect::from_min_size(rect.min,Vec2::splat(size)),&g.artwork,&g.id,self.revision,&mut self.textures);}ui.painter().line_segment([rect.left_bottom(),rect.right_bottom()],Stroke::new(2_f32,Color32::from_rgb(119,150,111)));r.on_hover_text(format!("프로젝트 글리프 · {}",codepoints(text)));
                }else if control(text){if self.preview_controls{ui.label(RichText::new(display(text)).size(size*0.4).color(MUTED));}}
                else if let Some(tex)=system_texture(ui.ctx(),if self.preview_controls&&text==" "{"·"}else{text},&self.preview_locale,size*0.78,&mut self.system_fonts,&mut self.textures){ui.image((tex.id(),tex.size_vec2())).on_hover_text(format!("시스템 폰트 대체 · {}",codepoints(text)));}
                else{ui.label(RichText::new(text).size(size*0.78).color(MUTED));}
                }if self.preview_controls{ui.label(RichText::new("↵").size(16.).color(MUTED));}});ui.add_space(10.);}
            });ui.add_space(20.);
            ui.horizontal(|ui|{ui.label(RichText::new("실제 출력 폰트 확인").size(20.).strong());if ui.add_enabled(self.job.is_none(),egui::Button::new("TTF 생성 · 검사")).clicked(){let p=self.project.clone();let locale=self.preview_locale.clone();self.spawn(ui.ctx(),move||{let r=font::export(&p,&font::ExportOptions{mode:Some("color".into()),locale:Some(locale),..Default::default()})?;let bytes=STANDARD.decode(&r.data).map_err(|e|e.to_string())?;let report=r.reports.iter().map(|r|format!("{}개 글리프 · {}\n{}",r.glyph_count,r.tables.join(" / "),r.warnings.join("\n"))).collect::<Vec<_>>().join("\n");Ok(Job::FontPreview(bytes,report))});}});
            chip(ui,"실제 생성한 TTF를 다시 읽고 Rustybuzz로 문자 치환을 시험합니다. 외부 앱의 컬러·시퀀스 지원은 다를 수 있어요.");
            if let Some(bytes)=&self.actual_font{
                ui.add_space(10.);chip(ui,"이 아래는 실제 TTF만 사용합니다. 폰트에 없는 글자는 빈칸으로 표시됩니다.");
                for line in self.preview_text.lines(){if let Some(tex)=font_texture(ui.ctx(),bytes,line,&self.preview_locale,self.preview_size,&format!("actual-{}",self.revision),&mut self.textures){ui.image((tex.id(),tex.size_vec2()));}}
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
            .color(MUTED),
        );
        ui.add_space(24.);
        egui::ScrollArea::vertical().show(ui,|ui|{
            let mut apply=None;ui.horizontal_wrapped(|ui|{for t in &self.project.templates{egui::Frame::new().fill(Color32::from_rgb(255,254,250)).stroke(Stroke::new(1_f32,LINE)).corner_radius(14).inner_margin(20).show(ui,|ui|{ui.set_width(215.);let(rect,_)=ui.allocate_exact_size(Vec2::splat(180.),Sense::hover());if let Some(art)=&t.original{paint_art(ui.ctx(),ui.painter(),rect,art,&t.id,self.revision,&mut self.textures);}else{ui.painter().text(rect.center(),Align2::CENTER_CENTER,"원본 참조 없음",FontId::proportional(16.),MUTED);}ui.label(RichText::new(&t.name).size(18.).strong());chip(ui,&format!("{} · v{}",t.provider,t.version));chip(ui,&t.license);ui.label(RichText::new(&t.attribution).size(10.).color(MUTED));chip(ui,&format!("편집 {} / 폰트 포함 {} / AI {}",if t.permissions.edit{"가능"}else{"제한"},if t.permissions.embed{"가능"}else{"제한"},if t.permissions.ai{"가능"}else{"제한"}));if ui.button("선택한 글리프에 적용").clicked(){apply=Some(t.clone());}});}});
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
                    ui.label(RichText::new("공개 글꼴 자형을 활용했어요. 출력 폰트에 출처와 OFL 이용 조건이 포함됩니다.").size(12.).color(MUTED));
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
                    egui::Button::new("이 후보 채택").fill(Color32::from_rgb(245, 220, 200)),
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
        if self.api_provider == "local" {
            self.local_installed
        } else {
            !self.api_model.trim().is_empty()
                && !self.api_endpoint.trim().is_empty()
                && !self.api_key.trim().is_empty()
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
        if !self.ai_ready() && self.ai_cancel.is_none() {
            return;
        }
        let frame = egui::Frame::new()
            .fill(Color32::from_rgb(255, 254, 250))
            .stroke(Stroke::new(1_f32, LINE))
            .corner_radius(18)
            .inner_margin(18)
            .shadow(egui::epaint::Shadow {
                offset: [0, 5],
                blur: 20,
                spread: 2,
                color: Color32::from_black_alpha(30),
            });
        let area = egui::Area::new("ai-dock".into())
            .anchor(Align2::CENTER_BOTTOM, [0., -44.])
            .order(egui::Order::Foreground);
        if let Some(cancel) = self.ai_cancel.clone() {
            egui::Modal::new("ai-busy".into())
                .area(area)
                .frame(frame)
                .backdrop_color(Color32::from_black_alpha(35))
                .show(ctx, |ui| {
                    ui.set_width((ctx.content_rect().width() - 100.).min(700.));
                    ui.horizontal(|ui| {
                        ui.spinner();
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
                        .color(MUTED),
                    );
                });
        } else {
            area.show(ctx, |ui| {
                frame.show(ui, |ui| {
                    ui.set_width((ctx.content_rect().width() - 100.).min(700.));
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("DOODLE ASSIST").strong().color(ACCENT));
                        chip(
                            ui,
                            if self.api_provider == "local" {
                                "이 PC에서 실행 · 텍스트 + 그림"
                            } else if self.send_reference {
                                "API · 텍스트 + 그림"
                            } else {
                                "API · 텍스트만"
                            },
                        );
                        ui.selectable_value(&mut self.ai_action, false, "그림 상담");
                        ui.selectable_value(&mut self.ai_action, true, "후보 만들기");
                    });
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
                        egui::ScrollArea::vertical()
                            .id_salt("ai-answer")
                            .max_height(130.)
                            .show(ui, |ui| {
                                ui.label(&self.ai_answer);
                            });
                    }
                    ui.horizontal(|ui| {
                        ui.add_sized(
                            [ui.available_width() - 85., 46.],
                            egui::TextEdit::multiline(&mut self.ai_prompt).hint_text(
                                if self.ai_action {
                                    "이 글자를 굵고 둥근 손글씨로 그려줘."
                                } else {
                                    "이 글자의 균형을 봐줘. 획을 더 부드럽게 만들려면?"
                                },
                            ),
                        );
                        let send = ui.add_enabled(
                            self.job.is_none() && !self.ai_prompt.trim().is_empty(),
                            egui::Button::new(if self.ai_action {
                                "그림 생성"
                            } else {
                                "상담하기"
                            })
                            .min_size(Vec2::new(70., 46.))
                            .fill(Color32::from_rgb(245, 220, 200)),
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
        }
    }
    fn decision_options_ui(&mut self, ui: &mut egui::Ui) {
        use doodle_core::needle::Mode;
        let previous = self.decision_mode;
        egui::Frame::new().fill(Color32::from_rgb(238,239,232)).corner_radius(12).inner_margin(18).show(ui, |ui| {
            ui.set_max_width(760.);
            ui.label(RichText::new("입력 판단 AI · 공통 옵션").size(19.).strong());
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.decision_mode, Mode::Off, "사용 안 함");
                let response = ui.selectable_value(&mut self.decision_mode, Mode::Needle, "Needle 3 · 시험 기능");
                #[cfg(feature = "ui-qa")] { self.qa_decision_button = Some(response.rect); }
                let _ = response;
            });
            ui.label("입력 → 판단 → 실행 전 확인 → 아래에서 선택한 내장 AI 또는 API");
            if self.decision_mode == Mode::Needle {
                ui.label("간단한 편집은 후보로 만들고, 불확실한 요청은 선택한 AI로 넘겨요. 한국어 판단 정확도는 제한적입니다.");
                ui.horizontal(|ui| {
                    chip(ui, if self.needle_installed { "판단 모델 설치 완료" } else { "판단 모델 설치 필요 · 약 37 MB" });
                    if ui.add_enabled(self.job.is_none(), egui::Button::new(if self.needle_installed { "판단 모델 검증 · 복구" } else { "판단 모델 설치" })).clicked() {
                        self.install_package(ui.ctx(), doodle_core::ai_install::Package::Needle);
                    }
                });
                ui.label(RichText::new("판단은 PC에서 텍스트만 읽어요. 설치되지 않았거나 실패하면 선택한 AI로 바로 넘깁니다.").size(12.).color(MUTED));
                if !self.install_error.is_empty() { ui.colored_label(ACCENT, &self.install_error); }
            }
        });
        if previous != self.decision_mode {
            let path = self.save_path.with_file_name("ai-options.json");
            if let Err(e) = (doodle_core::needle::Options {
                decision: self.decision_mode,
            })
            .save(&path)
            {
                self.notify(format!("판단 AI 설정을 저장하지 못했어요: {e}"));
            }
        }
    }
    fn connection_changed(&mut self) {
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
    fn settings_ui(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.);
        ui.label(
            RichText::new("필요할 때만, AI의 도움을.")
                .size(28.)
                .strong(),
        );
        ui.label(
            RichText::new("직접 그리기·저장·복원·폰트 출력은 API 없이도 사용할 수 있어요.")
                .color(MUTED),
        );
        ui.add_space(24.);
        self.decision_options_ui(ui);
        ui.add_space(16.);
        ui.label(RichText::new("그림 분석·생성 AI").size(20.).strong());
        egui::Frame::new().fill(Color32::from_rgb(255,254,250)).corner_radius(16).inner_margin(28).show(ui,|ui|{ui.set_max_width(740.);ui.horizontal(|ui|{
            for (provider, label) in [("local", "내장 로컬 AI"), ("openai", "OpenAI 호환"), ("anthropic", "Anthropic 호환")] {
                if ui.selectable_label(self.api_provider == provider, label).clicked() { self.select_ai_provider(provider); }
            }
        });ui.add_space(16.);
            if self.api_provider=="local" {
                ui.label("Gemma 4 E2B · 그림 상담과 후보 생성");
                chip(ui,if self.local_installed {"로컬 모델 설치 완료 · 인터넷 없이 사용 가능"} else {"로컬 AI 설치가 필요해요"});
                ui.label("최초 다운로드 약 3.4 GB · 설치 중에는 편집이 잠깁니다.");
                ui.label(RichText::new(format!("설치 위치: {}",doodle_core::local_ai::root().display())).size(12.).color(MUTED));
                if !self.local_installed {let missing=doodle_core::ai_install::missing(&doodle_core::local_ai::root());ui.label(RichText::new(format!("확인이 필요한 파일: {}",missing.join(", "))).size(12.).color(MUTED));}
                ui.horizontal(|ui|{
                    let button=ui.add_enabled(self.job.is_none(),egui::Button::new(if self.local_installed{"설치 파일 검증 · 복구"}else if self.install_error.is_empty(){"로컬 AI 설치"}else{"설치 다시 시도"}).fill(Color32::from_rgb(245,220,200)));
                    #[cfg(feature="ui-qa")] {self.qa_install_button=Some(button.rect);}
                    if button.clicked(){self.install_ai(ui.ctx());}
                    if ui.button("설치 상태 다시 확인").clicked(){self.local_installed=doodle_core::local_ai::installed();self.last_install_check=Instant::now();}
                });
                if !self.install_error.is_empty(){ui.colored_label(ACCENT,&self.install_error);}
                chip(ui,"필요할 때 Gemma 4를 읽고 작업이 끝나면 메모리를 반환합니다.");
            } else {
            let mut changed = false;
            egui::Grid::new("api-info").num_columns(2).spacing([20.,16.]).show(ui,|ui|{
                ui.label("API 기본 주소");
                if ui.add(egui::TextEdit::singleline(&mut self.api_endpoint).desired_width(460.)).changed() {
                    self.api_key = self.connections.key(&self.api_provider, &self.api_endpoint);
                    changed = true;
                }
                ui.end_row();ui.label("모델 이름");
                changed |= ui.add(egui::TextEdit::singleline(&mut self.api_model).hint_text("연결할 제공자의 정확한 모델 ID")).changed();
                ui.end_row();ui.label("API 키");
                let key_input = ui.add(egui::TextEdit::singleline(&mut self.api_key).password(true).hint_text("다음 실행에도 유지할 API 키"));
                #[cfg(feature="ui-qa")] { self.qa_api_key = Some(key_input.rect); }
                changed |= key_input.changed();
                ui.end_row();
            });
            if changed { self.connection_changed(); }
            ui.add_space(12.);
            ui.horizontal(|ui| {
                if ui.button("연결 설정 저장").clicked() { self.connection_changed(); self.save_connections(); }
                let forget = ui.add_enabled(!self.api_key.is_empty(), egui::Button::new("이 주소의 키 삭제"));
                #[cfg(feature="ui-qa")] { self.qa_forget_key = Some(forget.rect); }
                if forget.clicked() { self.api_key.clear(); self.connection_changed(); self.save_connections(); }
            });
            chip(ui,"자동 저장 · 비밀번호·지문 입력 없이 다음 실행에 복원합니다.");
            ui.label(RichText::new("키는 Windows 계정으로 암호화하며 제공자·주소별로 보관해요. 프로젝트에는 포함하지 않습니다.").size(12.).color(MUTED));
            }
            if !self.connections_status.is_empty() {
                ui.colored_label(if self.connections_error { ACCENT } else { MUTED }, &self.connections_status);
            }
            ui.add_space(20.);ui.label("원하는 그림 스타일");ui.add(egui::TextEdit::multiline(&mut self.ai_prompt).desired_width(680.).desired_rows(3));ui.checkbox(&mut self.ai_color,"컬러 그림 제안");ui.checkbox(&mut self.send_reference,"현재 드로잉 영역의 PNG 이미지와 벡터를 API에 전송");chip(ui,"로컬 AI는 그림을 PC에서 읽습니다. API에서는 체크한 경우에만 그림을 전송합니다.");ui.add_space(18.);if ui.add_enabled(self.job.is_none(),egui::Button::new("선택한 글리프의 후보 생성").fill(Color32::from_rgb(245,220,200))).clicked(){self.generate_ai(ui.ctx());}ui.add_space(8.);ui.label(RichText::new("후보는 검토 후 직접 채택합니다. 잠금·의도적 빈칸·제어 문자는 보호됩니다.").size(12.).color(MUTED));
        });
        ui.add_space(22.);
        ui.label(RichText::new("프로젝트").size(20.).strong());
        let mut name = self.project.name.clone();
        let mut family = self.project.family.clone();
        ui.horizontal(|ui| {
            ui.label("이름");
            if ui.text_edit_singleline(&mut name).changed() {
                self.project.name = name;
                self.changed();
            }
            ui.label("폰트 패밀리");
            if ui.text_edit_singleline(&mut family).changed() {
                self.project.family = family;
                self.changed();
            }
        });
        ui.add_space(10.);
        chip(ui, &format!("자동 저장 위치: {}", self.save_path.display()));
        if self.save_error && ui.button("자동 저장 다시 시도").clicked() {
            self.save_error = false;
            self.save();
        }
    }
    fn dialogs(&mut self, ctx: &egui::Context) {
        if self.ai_cancel.is_some() {
            return;
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
            egui::Window::new("나의 폰트 내보내기").open(&mut open).default_width(570.).resizable(true).show(ctx,|ui|{
            ui.label(RichText::new("그린 글자를, 진짜 폰트로.").size(23.).strong());ui.add_space(12.);ui.label("폰트 패밀리 이름");if ui.text_edit_singleline(&mut self.project.family).changed(){self.changed();}
            egui::ComboBox::from_id_salt("export-mode").selected_text(match self.export_mode.as_str(){"mono"=>"단색 TTF · 이미지 실루엣 변환","bitmap"=>"비트맵 TTF · sbix",_=>"컬러 TTF · COLR/CPAL"}).show_ui(ui,|ui|{ui.selectable_value(&mut self.export_mode,"color".into(),"컬러 TTF · COLR/CPAL");ui.selectable_value(&mut self.export_mode,"mono".into(),"단색 TTF · 이미지 실루엣 변환");ui.selectable_value(&mut self.export_mode,"bitmap".into(),"비트맵 TTF · sbix");});
            egui::ComboBox::from_id_salt("export-locale").selected_text(match self.export_locale.as_str(){"ko"=>"한국어용","ja"=>"일본어용","default"=>"공통 자형만",_=>"통합 · 지역별 locl"}).show_ui(ui,|ui|{for(v,l)in [("combined","통합 · 지역별 locl"),("ko","한국어용"),("ja","일본어용"),("default","공통 자형만")]{ui.selectable_value(&mut self.export_locale,v.into(),l);}});
            ui.checkbox(&mut self.export_subset,"이 문장에 필요한 글리프만 출력");if self.export_subset{ui.text_edit_multiline(&mut self.export_text);}ui.checkbox(&mut self.export_group,"현재 문자 모음만 출력");ui.checkbox(&mut self.export_split,"문자 모음별 분할 · ZIP");ui.add_space(12.);
            let count=self.project.glyphs.iter().filter(|g|g.status=="edited"||g.status=="blank").count();ui.label(format!("프로젝트 작성 항목 {count}개 · 레이어·시퀀스 부품을 포함해 출력 개수를 검증합니다."));chip(ui,"시스템 대체 그림은 포함되지 않습니다. 미작성·미지원·시스템 표시 항목은 제외해요.");chip(ui,"컬러 모드에 이미지가 있으면 전체를 sbix로 출력합니다. 단색 모드는 이미지를 64px 실루엣으로 변환해요.");chip(ui,"공백의 줄바꿈 성격과 제어 문자 동작은 바뀌지 않습니다.");ui.add_space(12.);
            ui.horizontal(|ui|{for(save,label)in [(false,"출력 전 검사"),(true,"파일로 저장")]{if ui.add_enabled(self.job.is_none(),egui::Button::new(label)).clicked(){let path=if save{rfd::FileDialog::new().set_file_name(if self.export_split{"Doodle-Font-fonts.zip".into()}else{format!("{}.ttf",self.project.family)}).save_file()}else{None};if save&&path.is_none(){continue}let p=self.project.clone();let options=font::ExportOptions{mode:Some(self.export_mode.clone()),locale:Some(self.export_locale.clone()),group:Some(if self.export_group{GROUPS[self.group].0.into()}else{"all".into()}),ids:if self.export_subset{Some(self.project.glyphs.iter().filter(|g|self.export_text.contains(&g.text)).map(|g|g.id.clone()).collect())}else{None},split:Some(self.export_split)};self.spawn(ctx,move||{let result=font::export(&p,&options)?;if let Some(path)=&path{std::fs::write(path,STANDARD.decode(&result.data).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;let report=path.with_extension("report.json");std::fs::write(report,serde_json::to_vec_pretty(&result.reports).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;}Ok(Job::Export(result,path))});}}});
            if !self.export_report.is_empty(){ui.add_space(10.);egui::ScrollArea::vertical().max_height(230.).show(ui,|ui|{ui.label(&self.export_report);});}
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
        if qa_view.starts_with("connections-") {
            input
                .events
                .retain(|event| matches!(event, egui::Event::Screenshot { .. }));
            let frame = ctx.cumulative_frame_nr();
            let rect = match qa_view.as_str() {
                "connections-write" => self.qa_api_key,
                "connections-delete" => self.qa_forget_key,
                _ => None,
            };
            if let Some(rect) = rect.filter(|_| frame == 3 || frame == 4) {
                let pos = rect.center();
                input.events.push(egui::Event::PointerMoved(pos));
                input.events.push(egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: frame == 3,
                    modifiers: Default::default(),
                });
            }
            if qa_view == "connections-write" && frame == 5 {
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
        if !["interaction", "ai-busy", "ai-recover"]
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
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
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
                match std::env::var("DOODLE_QA_VIEW").unwrap_or_default().as_str() {
                    "preview" => self.tab = 1,
                    "templates" => self.tab = 2,
                    "settings" => self.tab = 3,
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
                    "ai-busy" | "ai-recover" => {
                        self.local_installed = true;
                        self.ai_cancel = Some(std::sync::Arc::new(
                            std::sync::atomic::AtomicBool::new(false),
                        ));
                        self.ai_status = "프롬프트와 그림을 함께 분석 중".into();
                    }
                    "export" => self.export_open = true,
                    "hangul" => {
                        self.group = 2;
                        self.refresh_catalog();
                    }
                    _ => {}
                }
            }
            ctx.request_repaint();
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
            if ["ai-busy", "ai-recover"]
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
            let connection_view = std::env::var("DOODLE_QA_VIEW").unwrap_or_default();
            let connection_flow = connection_view.starts_with("connections-");
            let connection_ready = connection_flow
                && ctx.cumulative_frame_nr() >= 12
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
                && !install_flow
                && !candidate_live
                && !connection_flow)
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
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
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
        self.top(ctx);
        self.footer(ctx);
        if self.ai_ready() {
            egui::TopBottomPanel::bottom("ai-space")
                .exact_height(
                    (if self.ai_answer.is_empty() {
                        112.
                    } else {
                        252.
                    }) + if self
                        .project
                        .glyphs
                        .get(self.selected)
                        .is_some_and(|g| !g.candidates.is_empty())
                    {
                        40.
                    } else {
                        0.
                    },
                )
                .frame(egui::Frame::new().fill(PAPER))
                .show(ctx, |_| {});
        }
        if self.tab == 0 {
            self.library(ctx);
            self.properties(ctx);
            self.editor(ctx);
        } else {
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(PAPER).inner_margin(32))
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
                        .fill(INK)
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
