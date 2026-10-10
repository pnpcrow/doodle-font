//! Local display preferences and the two native egui palettes.
use crate::i18n::text as tr;
use eframe::egui::{self, Color32, FontId, Stroke, Vec2};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}
impl Theme {
    pub fn label(self) -> &'static str {
        match self {
            Self::System => tr("시스템"),
            Self::Light => tr("라이트"),
            Self::Dark => tr("다크"),
        }
    }
    pub fn preference(self) -> egui::ThemePreference {
        match self {
            Self::System => egui::ThemePreference::System,
            Self::Light => egui::ThemePreference::Light,
            Self::Dark => egui::ThemePreference::Dark,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub language: crate::i18n::Language,
    pub theme: Theme,
    pub reduce_motion: bool,
    pub companion_renderer: crate::companion::Renderer,
    /// Relative position within the available travel area; None follows the prompt.
    pub companion_position: Option<[f32; 2]>,
}
impl Preferences {
    pub fn load(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let pending = path.with_extension("pending");
        std::fs::write(
            &pending,
            serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        std::fs::rename(pending, path).map_err(|e| e.to_string())
    }
}

#[derive(Clone, Copy)]
pub struct Palette {
    pub background: Color32,
    pub surface: Color32,
    pub raised: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub line: Color32,
    pub selection: Color32,
}
impl Palette {
    pub fn new(dark: bool) -> Self {
        let c = |[r, g, b]: [u8; 3]| Color32::from_rgb(r, g, b);
        if dark {
            Self {
                background: c([21, 25, 31]),
                surface: c([29, 35, 43]),
                raised: c([37, 45, 55]),
                text: c([230, 235, 242]),
                muted: c([163, 175, 189]),
                accent: c([237, 156, 125]),
                line: c([56, 66, 80]),
                selection: c([73, 53, 46]),
            }
        } else {
            Self {
                background: c([245, 246, 248]),
                surface: Color32::WHITE,
                raised: c([239, 241, 244]),
                text: c([31, 41, 55]),
                muted: c([102, 112, 133]),
                accent: c([190, 83, 51]),
                line: c([224, 228, 234]),
                selection: c([252, 230, 216]),
            }
        }
    }
    pub fn of(ctx: &egui::Context) -> Self {
        Self::new(ctx.style().visuals.dark_mode)
    }
}
pub fn install(ctx: &egui::Context, preference: Theme) {
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        ctx.style_mut_of(theme, |s| {
            let p = Palette::new(theme == egui::Theme::Dark);
            s.visuals = if theme == egui::Theme::Dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            };
            let v = &mut s.visuals;
            v.panel_fill = p.surface;
            v.window_fill = p.surface;
            v.extreme_bg_color = p.background;
            v.faint_bg_color = p.raised;
            v.override_text_color = Some(p.text);
            v.weak_text_color = Some(p.muted);
            v.selection.bg_fill = p.selection;
            v.selection.stroke = Stroke::new(1_f32, p.accent);
            v.window_corner_radius = 14.into();
            v.window_stroke = Stroke::new(1_f32, p.line);
            for w in [
                &mut v.widgets.noninteractive,
                &mut v.widgets.inactive,
                &mut v.widgets.hovered,
                &mut v.widgets.active,
                &mut v.widgets.open,
            ] {
                // egui's default hover/focus expansion changes the painted button bounds.
                w.expansion = 0.;
                w.corner_radius = 7.into();
                w.bg_stroke = Stroke::new(1_f32, p.line);
                w.fg_stroke = Stroke::new(1_f32, p.text);
                w.bg_fill = p.raised;
                w.weak_bg_fill = Color32::TRANSPARENT;
            }
            v.widgets.hovered.weak_bg_fill = p.raised;
            v.widgets.hovered.bg_stroke.color = p.accent;
            v.widgets.active.bg_fill = p.selection;
            v.widgets.active.weak_bg_fill = p.selection;
            v.widgets.active.bg_stroke.color = p.accent;
            v.widgets.open.bg_fill = p.selection;
            v.widgets.open.weak_bg_fill = p.selection;
            s.spacing.item_spacing = Vec2::new(8., 8.);
            s.spacing.button_padding = Vec2::new(12., 7.);
            s.spacing.interact_size.y = 32.;
            s.text_styles
                .insert(egui::TextStyle::Body, FontId::proportional(14.));
            s.text_styles
                .insert(egui::TextStyle::Button, FontId::proportional(14.));
            s.text_styles
                .insert(egui::TextStyle::Small, FontId::proportional(12.));
        });
    }
    ctx.set_theme(preference.preference());
}
