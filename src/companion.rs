//! A generated body sprite with a small, time-based native face rig.
//! Motion is independently implemented; Coucou's easing/state approach is a reference.
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Vec2};
use std::f32::consts::{PI, TAU};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mood {
    #[default]
    Idle,
    Thinking,
    Done,
    NeedsHelp,
}
#[derive(Clone, Copy, Debug)]
pub struct Pose {
    pub look: Vec2,
    pub tilt: f32,
    pub lift: f32,
    pub scale: Vec2,
    pub eyes: f32,
}
impl Default for Pose {
    fn default() -> Self {
        Self {
            look: Vec2::ZERO,
            tilt: 0.,
            lift: 0.,
            scale: Vec2::splat(1.),
            eyes: 1.,
        }
    }
}
pub struct Motion {
    pub pose: Pose,
    pub mood: Mood,
    time: f64,
    changed: f64,
    last: Option<f64>,
}
impl Default for Motion {
    fn default() -> Self {
        Self {
            pose: Pose::default(),
            mood: Mood::Idle,
            time: 0.,
            changed: 0.,
            last: None,
        }
    }
}
impl Motion {
    pub fn react(&mut self, mood: Mood) {
        if self.mood != mood {
            self.changed = self.time;
            self.mood = mood;
        }
    }
    pub fn step(&mut self, now: f64, pointer: Vec2, reduce: bool) -> Pose {
        let dt = self.last.map_or(0., |last| (now - last).clamp(0., 0.05)) as f32;
        self.last = Some(now);
        self.time = now;
        let age = (now - self.changed) as f32;
        if matches!(self.mood, Mood::Done | Mood::NeedsHelp) && age > 3. {
            self.react(Mood::Idle);
        }
        if reduce {
            self.pose = Pose::default();
            return self.pose;
        }
        let t = now as f32;
        let look = if self.mood == Mood::Thinking {
            Vec2::new((t * 0.9).sin() * 0.5, -0.35)
        } else {
            pointer.clamp(Vec2::splat(-1.), Vec2::splat(1.))
        };
        let settle = 1. - (-10. * dt).exp();
        self.pose.look += (look - self.pose.look) * settle;
        let tilt = if self.mood == Mood::Thinking {
            (t * 1.4).sin() * 0.055
        } else if self.mood == Mood::NeedsHelp {
            -0.07
        } else {
            self.pose.look.x * 0.035
        };
        self.pose.tilt += (tilt - self.pose.tilt) * settle;
        let breath = (t * 1.9).sin() * 0.014;
        let bounce = if self.mood == Mood::Done {
            (age * 8.).sin().abs() * (-age * 2.5).exp() * 0.13
        } else {
            0.
        };
        let target_lift = if self.mood == Mood::Done {
            bounce
        } else {
            (t * 1.9).sin() * 0.006
        };
        self.pose.lift += (target_lift - self.pose.lift) * settle;
        let target_scale = Vec2::new(1. - breath * 0.5, 1. + breath);
        self.pose.scale += (target_scale - self.pose.scale) * settle;
        // Smooth close/open, with occasional double blinks; no frame-count timers.
        let phase = (now % 5.7) as f32;
        let blink = |p: f32| {
            if (0.0..0.20).contains(&p) {
                (PI * p / 0.20).sin().powi(2)
            } else {
                0.
            }
        };
        self.pose.eyes = 1. - 0.94 * blink(phase - 3.7).max(blink(phase - 4.04));
        self.pose
    }
}
pub struct Companion {
    texture: egui::TextureHandle,
    pub motion: Motion,
}
impl Companion {
    pub fn new(ctx: &egui::Context) -> Self {
        let image = image::load_from_memory(include_bytes!("../assets/companion/body.png"))
            .expect("companion sprite")
            .resize(384, 384, image::imageops::FilterType::Lanczos3)
            .to_rgba8();
        let texture = ctx.load_texture(
            "doodle-companion",
            egui::ColorImage::from_rgba_unmultiplied(
                [image.width() as usize, image.height() as usize],
                image.as_raw(),
            ),
            egui::TextureOptions::LINEAR,
        );
        Self {
            texture,
            motion: Motion::default(),
        }
    }
    pub fn show(&mut self, ui: &mut egui::Ui, size: f32, reduce: bool) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
        let pointer = ui
            .ctx()
            .pointer_hover_pos()
            .map_or(Vec2::ZERO, |p| (p - rect.center()) / 180.);
        let now = ui.input(|i| i.time);
        if response.clicked() && self.motion.mood != Mood::Thinking {
            self.motion.react(Mood::Done);
        }
        let pose = self.motion.step(now, pointer, reduce);
        self.paint(ui.painter(), rect, pose);
        if !reduce && ui.is_rect_visible(rect) && ui.input(|i| i.focused) {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(16));
        }
        response.on_hover_text("두들 · 한 획부터 함께 시작해요")
    }
    pub fn paint(&self, painter: &egui::Painter, rect: Rect, pose: Pose) {
        let rotation = egui::emath::Rot2::from_angle(pose.tilt);
        let transform = |p: Vec2| {
            rect.center() + rotation * ((p - Vec2::splat(0.5)) * pose.scale * rect.width())
                - Vec2::new(0., pose.lift * rect.height())
        };
        let mut mesh = egui::Mesh::with_texture(self.texture.id());
        for uv in [
            Vec2::new(0., 0.),
            Vec2::new(1., 0.),
            Vec2::new(1., 1.),
            Vec2::new(0., 1.),
        ] {
            mesh.vertices.push(egui::epaint::Vertex {
                pos: transform(uv),
                uv: Pos2::new(uv.x, uv.y),
                color: Color32::WHITE,
            });
        }
        mesh.indices.extend([0, 1, 2, 0, 2, 3]);
        painter.add(egui::Shape::mesh(mesh));
        // Same 18-degree clockwise tilt as the generated body and blush.
        let eye_rotation = egui::emath::Rot2::from_angle(18_f32.to_radians());
        for center in [Vec2::new(0.364, 0.524), Vec2::new(0.631, 0.617)] {
            let center = center + pose.look * 0.012;
            let width = 0.044_f32;
            let height = 0.119 * pose.eyes;
            let radius = width.min(height) / 2.;
            let mut points = vec![];
            for corner in 0..4 {
                let cx = if corner == 0 || corner == 3 {
                    width / 2. - radius
                } else {
                    -width / 2. + radius
                };
                let cy = if corner < 2 {
                    height / 2. - radius
                } else {
                    -height / 2. + radius
                };
                for i in 0..=6 {
                    let a = (corner as f32 * 0.25 + i as f32 / 24.) * TAU;
                    let p = Vec2::new(cx + radius * a.cos(), cy + radius * a.sin());
                    points.push(transform(center + eye_rotation * p));
                }
            }
            painter.add(egui::Shape::convex_polygon(
                points,
                Color32::from_rgb(17, 17, 17),
                egui::Stroke::NONE,
            ));
        }
    }
}
