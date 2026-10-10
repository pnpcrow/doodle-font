//! Native window controls for the undecorated desktop viewport.
//! Kept above modal backdrops, like the OS caption controls they replace.
use crate::appearance::Palette;
use eframe::egui::{
    self, Align, Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2,
};

pub const TITLE_HEIGHT: f32 = 68.;
pub const PROJECT_SIZE: f32 = 22.;
pub const FAMILY_SIZE: f32 = 15.;

pub struct ChromeRects {
    pub title: Rect,
    pub project: Rect,
    /// Minimize, maximize/restore, close, in screen order.
    pub controls: [Rect; 3],
    pub resize: Vec<(Rect, egui::ResizeDirection)>,
}

pub fn show(ctx: &egui::Context, project: &str, family: &str, palette: Palette) -> ChromeRects {
    let screen = ctx.content_rect();
    let title = Rect::from_min_size(screen.min, Vec2::new(screen.width(), TITLE_HEIGHT));
    let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
    let fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
    let mut rects = ChromeRects {
        title,
        project: Rect::NOTHING,
        controls: [Rect::NOTHING; 3],
        resize: vec![],
    };
    let area = egui::Area::new("native-window-caption".into())
        .order(egui::Order::Foreground)
        .fixed_pos(title.min)
        .constrain(false)
        .sense(Sense::hover())
        .show(ctx, |ui| {
            ui.style_mut().interaction.selectable_labels = false;
            ui.set_min_size(title.size());
            ui.painter().rect_filled(title, 0., palette.surface);
            let control_left = title.right() - 146.;
            let drag_rect = Rect::from_min_max(
                title.min + Vec2::new(5., 5.),
                Pos2::new(control_left - 8., title.bottom()),
            );
            // Register before the read-only labels so they don't consume dragging.
            let drag = ui.interact(drag_rect, ui.id().with("drag"), Sense::click_and_drag());
            if drag.double_clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
            } else if drag.drag_started_by(egui::PointerButton::Primary) {
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            let content = Rect::from_min_max(
                title.min + Vec2::new(20., 8.),
                Pos2::new(control_left - 20., title.bottom() - 8.),
            );
            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(content)
                    .layout(egui::Layout::left_to_right(Align::Center)),
                |ui| {
                    let (logo, _) = ui.allocate_exact_size(Vec2::splat(36.), Sense::hover());
                    ui.painter()
                        .rect_filled(logo, 11., Color32::from_rgb(190, 83, 51));
                    ui.painter().text(
                        logo.center(),
                        Align2::CENTER_CENTER,
                        "d",
                        FontId::proportional(30.),
                        Color32::WHITE,
                    );
                    ui.label(RichText::new("Doodle Font").size(19.).strong());
                    ui.add_space(12.);
                    ui.separator();
                    let heading = ui.allocate_ui_with_layout(
                        Vec2::new(ui.available_width(), 52.),
                        egui::Layout::top_down(Align::Min),
                        |ui| {
                            ui.spacing_mut().item_spacing.y = 2.;
                            ui.add(
                                egui::Label::new(
                                    RichText::new(project).size(PROJECT_SIZE).strong(),
                                )
                                .truncate(),
                            )
                            .on_hover_text(project);
                            ui.add(
                                egui::Label::new(
                                    RichText::new(format!("폰트 패밀리 · {family}"))
                                        .size(FAMILY_SIZE)
                                        .color(palette.muted),
                                )
                                .truncate(),
                            )
                            .on_hover_text(family);
                        },
                    );
                    rects.project = heading.response.rect;
                },
            );
            for (index, label) in [
                "최소화",
                if maximized {
                    "이전 크기로 복원"
                } else {
                    "최대화"
                },
                "닫기",
            ]
            .into_iter()
            .enumerate()
            {
                let rect = Rect::from_min_size(
                    Pos2::new(control_left + index as f32 * 46., title.top() + 14.),
                    Vec2::new(46., 40.),
                );
                let response = ui
                    .interact(rect, ui.id().with(index), Sense::click())
                    .on_hover_text(label);
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label)
                });
                let hover = response.hovered() || response.has_focus();
                let color = if hover && index == 2 {
                    Color32::WHITE
                } else {
                    palette.text
                };
                if hover {
                    ui.painter().rect_filled(
                        rect,
                        6.,
                        if index == 2 {
                            Color32::from_rgb(196, 43, 28)
                        } else {
                            palette.raised
                        },
                    );
                }
                let icon = Rect::from_center_size(rect.center(), Vec2::splat(11.));
                let stroke = Stroke::new(1.2_f32, color);
                match index {
                    0 => {
                        ui.painter().hline(icon.x_range(), icon.center().y, stroke);
                    }
                    1 => {
                        if maximized {
                            ui.painter().rect_stroke(
                                icon.translate(Vec2::new(2., -2.)),
                                0.,
                                stroke,
                                egui::StrokeKind::Inside,
                            );
                            ui.painter().rect_filled(
                                icon.translate(Vec2::new(-1., 1.)),
                                0.,
                                if hover {
                                    palette.raised
                                } else {
                                    palette.surface
                                },
                            );
                        }
                        ui.painter().rect_stroke(
                            icon.translate(if maximized {
                                Vec2::new(-1., 1.)
                            } else {
                                Vec2::ZERO
                            }),
                            0.,
                            stroke,
                            egui::StrokeKind::Inside,
                        );
                    }
                    _ => {
                        ui.painter()
                            .line_segment([icon.left_top(), icon.right_bottom()], stroke);
                        ui.painter()
                            .line_segment([icon.right_top(), icon.left_bottom()], stroke);
                    }
                }
                rects.controls[index] = rect;
                if response.clicked() {
                    ctx.send_viewport_cmd(match index {
                        0 => egui::ViewportCommand::Minimized(true),
                        1 => egui::ViewportCommand::Maximized(!maximized),
                        _ => egui::ViewportCommand::Close,
                    });
                }
            }
        });
    ctx.move_to_top(area.response.layer_id);
    if !maximized && !fullscreen {
        for (index, (rect, direction)) in resize_regions(screen).into_iter().enumerate() {
            let area = egui::Area::new(egui::Id::new(("native-window-resize", index)))
                .fixed_pos(rect.min)
                .order(egui::Order::Foreground)
                .constrain(false)
                .sense(Sense::hover())
                .show(ctx, |ui| {
                    let (_, response) = ui.allocate_exact_size(rect.size(), Sense::drag());
                    response.clone().on_hover_cursor(match direction {
                        egui::ResizeDirection::North | egui::ResizeDirection::South => {
                            egui::CursorIcon::ResizeVertical
                        }
                        egui::ResizeDirection::East | egui::ResizeDirection::West => {
                            egui::CursorIcon::ResizeHorizontal
                        }
                        egui::ResizeDirection::NorthEast | egui::ResizeDirection::SouthWest => {
                            egui::CursorIcon::ResizeNeSw
                        }
                        _ => egui::CursorIcon::ResizeNwSe,
                    });
                    if response.drag_started_by(egui::PointerButton::Primary) {
                        ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(direction));
                    }
                });
            ctx.move_to_top(area.response.layer_id);
            rects.resize.push((rect, direction));
        }
    }
    rects
}

fn resize_regions(r: Rect) -> [(Rect, egui::ResizeDirection); 8] {
    use egui::ResizeDirection::*;
    let rect = |x, y, w, h| Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h));
    let corner = 12.;
    let edge = 5.;
    [
        (rect(r.left(), r.top(), corner, corner), NorthWest),
        (rect(r.right() - corner, r.top(), corner, corner), NorthEast),
        (
            rect(r.left(), r.bottom() - corner, corner, corner),
            SouthWest,
        ),
        (
            rect(r.right() - corner, r.bottom() - corner, corner, corner),
            SouthEast,
        ),
        (
            rect(r.left() + corner, r.top(), r.width() - 2. * corner, edge),
            North,
        ),
        (
            rect(
                r.left() + corner,
                r.bottom() - edge,
                r.width() - 2. * corner,
                edge,
            ),
            South,
        ),
        (
            rect(r.left(), r.top() + corner, edge, r.height() - 2. * corner),
            West,
        ),
        (
            rect(
                r.right() - edge,
                r.top() + corner,
                edge,
                r.height() - 2. * corner,
            ),
            East,
        ),
    ]
}
