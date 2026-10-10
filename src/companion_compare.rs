//! Optional side-by-side preview. Both renderers receive exactly the same pose.
use crate::companion::{Companion, Mood, Motion, Renderer};
use crate::i18n::{format as tf, text as tr};
use eframe::egui::{self, Vec2};

pub struct Comparison {
    pub image: Companion,
    pub vector: Companion,
    motion: Motion,
    mood: Mood,
    size: f32,
    reduce: bool,
    #[cfg(feature = "ui-qa")]
    pub qa_choices: [Option<egui::Rect>; 2],
    #[cfg(feature = "ui-qa")]
    pub qa_rect: Option<egui::Rect>,
}
pub enum Action {
    KeepOpen,
    Close,
    Choose(Renderer),
}
impl Comparison {
    pub fn new(ctx: &egui::Context) -> Self {
        Self {
            image: Companion::with_renderer(ctx, Renderer::Image),
            vector: Companion::with_renderer(ctx, Renderer::Vector),
            motion: Motion::default(),
            mood: Mood::Thinking,
            size: 192.,
            reduce: false,
            #[cfg(feature = "ui-qa")]
            qa_choices: [None; 2],
            #[cfg(feature = "ui-qa")]
            qa_rect: None,
        }
    }
    pub fn show(&mut self, ctx: &egui::Context) -> Action {
        let mut open = true;
        let mut action = Action::KeepOpen;
        let window = egui::Window::new(tr("두들 표현 방식 비교"))
            .id("companion-compare".into())
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(tr(
                    "같은 동작으로 움직입니다. 질감과 윤곽선을 비교해 보세요.",
                ));
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("compare-mood")
                        .selected_text(self.mood.label())
                        .show_ui(ui, |ui| {
                            for mood in [
                                Mood::Idle,
                                Mood::Thinking,
                                Mood::Inspecting,
                                Mood::Drawing,
                                Mood::Coloring,
                                Mood::Explaining,
                                Mood::Done,
                                Mood::NeedsHelp,
                            ] {
                                ui.selectable_value(&mut self.mood, mood, mood.label());
                            }
                        });
                    ui.checkbox(&mut self.reduce, tr("움직임 줄이기"));
                });
                let max_size = ((ctx.content_rect().width() - 100.) / 2.)
                    .min(ctx.content_rect().height() - 240.)
                    .clamp(64., 384.);
                self.size = self.size.min(max_size);
                ui.add(egui::Slider::new(&mut self.size, 64.0..=max_size).text(tr("확대")));
                self.motion.react(self.mood);
                let pointer = ctx
                    .pointer_hover_pos()
                    .map_or(Vec2::ZERO, |p| (p - ctx.content_rect().center()) / 250.);
                let pose = self.motion.step(ui.input(|i| i.time), pointer, self.reduce);
                ui.horizontal(|ui| {
                    for (index, (pet, renderer, description)) in [
                        (&self.image, Renderer::Image, tr("원본의 부드러운 질감")),
                        (
                            &self.vector,
                            Renderer::Vector,
                            tr("곡선으로 그린 또렷한 윤곽"),
                        ),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        ui.vertical(|ui| {
                            ui.set_width(self.size);
                            ui.label(egui::RichText::new(renderer.label()).strong());
                            let (rect, _) = ui
                                .allocate_exact_size(Vec2::splat(self.size), egui::Sense::hover());
                            pet.paint(ui.painter(), rect, pose);
                            ui.label(description);
                            let button = ui.button(tf("{0} 사용", &[renderer.label().to_string()]));
                            #[cfg(feature = "ui-qa")]
                            {
                                self.qa_choices[index] = Some(button.rect);
                            }
                            let _ = index;
                            if button.clicked() {
                                action = Action::Choose(renderer);
                            }
                        });
                    }
                });
                if !self.reduce && ui.input(|i| i.focused) {
                    ctx.request_repaint_after(std::time::Duration::from_millis(16));
                }
            });
        #[cfg(feature = "ui-qa")]
        {
            self.qa_rect = window.map(|w| w.response.rect);
        }
        #[cfg(not(feature = "ui-qa"))]
        let _ = window;
        if !open {
            Action::Close
        } else {
            action
        }
    }
}
