//! Persist placement relative to the workspace, independent of window size and DPI.
use eframe::egui::{Pos2, Rect, Vec2};

pub fn bounds(view: Rect) -> Rect {
    Rect::from_min_max(
        view.min + Vec2::new(12., crate::window_chrome::TITLE_HEIGHT + 44. + 8.),
        view.max - Vec2::new(12., 44.),
    )
}

pub fn resolve(bounds: Rect, size: f32, default: Pos2, saved: Option<[f32; 2]>) -> Rect {
    let travel = (bounds.size() - Vec2::splat(size)).max(Vec2::ZERO);
    let position = saved
        .filter(|p| p.iter().all(|v| v.is_finite()))
        .map_or(default, |[x, y]| {
            bounds.min + travel * Vec2::new(x.clamp(0., 1.), y.clamp(0., 1.))
        });
    Rect::from_min_size(
        position.clamp(bounds.min, bounds.min + travel),
        Vec2::splat(size),
    )
}

pub fn remember(bounds: Rect, rect: Rect) -> [f32; 2] {
    let travel = (bounds.size() - rect.size()).max(Vec2::splat(1.));
    let fraction = ((rect.min - bounds.min) / travel).clamp(Vec2::ZERO, Vec2::splat(1.));
    [fraction.x, fraction.y]
}
