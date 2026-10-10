//! Native motion director: semantic cues select clips; a continuous spring rig renders them.
//! No inference runs on the render thread, at rest, or for individual frames.
use crate::i18n::text as tr;
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Vec2};
use std::f32::consts::{PI, TAU};
#[path = "companion_placement.rs"]
pub mod placement;
#[path = "companion_vector.rs"]
mod vector;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Renderer {
    Image,
    #[default]
    Vector,
}
impl Renderer {
    pub fn label(self) -> &'static str {
        match self {
            Self::Image => tr("원본 그림"),
            Self::Vector => tr("벡터"),
        }
    }
}
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct Footprint {
    pub custom_texture_bytes: usize,
    pub geometry_bytes: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mood {
    #[default]
    Idle,
    Thinking,
    Inspecting,
    Drawing,
    Coloring,
    Explaining,
    Done,
    NeedsHelp,
}
impl Mood {
    pub fn working(self) -> bool {
        matches!(
            self,
            Self::Thinking | Self::Inspecting | Self::Drawing | Self::Coloring
        )
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Idle => tr("두들 · 함께 그려요"),
            Self::Thinking => tr("생각하고 있어요"),
            Self::Inspecting => tr("살펴보고 있어요"),
            Self::Drawing => tr("그림을 만들고 있어요"),
            Self::Coloring => tr("색을 고르고 있어요"),
            Self::Explaining => tr("이야기를 들려드려요"),
            Self::Done => tr("그림이 준비됐어요"),
            Self::NeedsHelp => tr("확인이 필요해요"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub look: Vec2,
    pub head: Vec2,
    pub tilt: f32,
    pub lift: f32,
    pub scale: Vec2,
    pub eyes: f32,
    pub round: f32,
    pub square: f32,
    pub tint: [f32; 4],
    pub sparkle: f32,
}
impl Pose {
    fn zero() -> Self {
        Self {
            look: Vec2::ZERO,
            head: Vec2::ZERO,
            tilt: 0.,
            lift: 0.,
            scale: Vec2::ZERO,
            eyes: 0.,
            round: 0.,
            square: 0.,
            tint: [0.; 4],
            sparkle: 0.,
        }
    }
}
impl Default for Pose {
    fn default() -> Self {
        Self {
            scale: Vec2::splat(1.),
            eyes: 1.,
            tint: [1., 1., 1., 0.],
            ..Self::zero()
        }
    }
}

pub struct Motion {
    pub pose: Pose,
    pub mood: Mood,
    velocity: Pose,
    time: f64,
    changed: f64,
    last: Option<f64>,
    last_pointer: Vec2,
    pointer_seen: f64,
}
impl Default for Motion {
    fn default() -> Self {
        Self {
            pose: Pose::default(),
            mood: Mood::Idle,
            velocity: Pose::zero(),
            time: 0.,
            changed: 0.,
            last: None,
            last_pointer: Vec2::ZERO,
            pointer_seen: -10.,
        }
    }
}

// Exact critically damped spring step. Both position AND velocity survive clip changes.
fn spring(value: &mut f32, velocity: &mut f32, target: f32, omega: f32, dt: f32) {
    if dt <= 0. {
        return;
    }
    let offset = *value - target;
    let impulse = *velocity + omega * offset;
    let decay = (-omega * dt).exp();
    *value = target + (offset + impulse * dt) * decay;
    *velocity = (*velocity - omega * impulse * dt) * decay;
}
fn spring_vec(value: &mut Vec2, velocity: &mut Vec2, target: Vec2, omega: f32, dt: f32) {
    spring(&mut value.x, &mut velocity.x, target.x, omega, dt);
    spring(&mut value.y, &mut velocity.y, target.y, omega, dt);
}
impl Motion {
    pub fn react(&mut self, mood: Mood) {
        if self.mood != mood {
            self.changed = self.time;
            self.mood = mood;
        }
    }
    pub fn step(&mut self, now: f64, pointer: Vec2, reduce: bool) -> Pose {
        // Pause the animation clock while hidden; never catch up with a giant jump.
        let elapsed = self.last.map_or(0., |last| (now - last).clamp(0., 0.1));
        self.last = Some(now);
        let pointer = if pointer.is_finite() {
            pointer
        } else {
            Vec2::ZERO
        };
        if (pointer - self.last_pointer).length() > 0.005 {
            self.pointer_seen = self.time;
            self.last_pointer = pointer;
        }
        let pointer = pointer.clamp(Vec2::splat(-1.), Vec2::splat(1.));
        let steps = (elapsed * 120.).ceil().max(1.) as usize;
        let dt = (elapsed / steps as f64) as f32;
        for _ in 0..steps {
            self.time += dt as f64;
            let age = (self.time - self.changed) as f32;
            if matches!(self.mood, Mood::Done | Mood::NeedsHelp | Mood::Explaining) && age > 4.5 {
                self.react(Mood::Idle);
            }
            let target = self.target(pointer, reduce);
            if reduce {
                // Keep state shape/color readable without perpetual movement or blinking.
                self.pose = target;
                self.velocity = Pose::zero();
                continue;
            }
            let p = &mut self.pose;
            let v = &mut self.velocity;
            spring_vec(&mut p.look, &mut v.look, target.look, 19., dt);
            spring_vec(&mut p.head, &mut v.head, target.head, 8., dt);
            spring(&mut p.tilt, &mut v.tilt, target.tilt, 11., dt);
            spring(&mut p.lift, &mut v.lift, target.lift, 15., dt);
            spring_vec(&mut p.scale, &mut v.scale, target.scale, 13., dt);
            spring(&mut p.eyes, &mut v.eyes, target.eyes, 65., dt);
            spring(&mut p.round, &mut v.round, target.round, 9., dt);
            spring(&mut p.square, &mut v.square, target.square, 9., dt);
            for i in 0..4 {
                spring(&mut p.tint[i], &mut v.tint[i], target.tint[i], 8., dt);
            }
            spring(&mut p.sparkle, &mut v.sparkle, target.sparkle, 10., dt);
        }
        self.pose
    }
    fn target(&self, pointer: Vec2, reduce: bool) -> Pose {
        let t = self.time as f32;
        let age = (self.time - self.changed) as f32;
        let mut p = Pose::default();
        // Cursor attention overrides autonomous looking, but preserves the work silhouette.
        let attentive = self.time - self.pointer_seen < 3.;
        let scan = Vec2::new((t * 0.75).sin() * 0.72, (t * 0.47).sin() * 0.35);
        p.look = if attentive { pointer } else { scan };
        p.head = p.look * 0.8;
        p.tilt = p.look.x * 0.14;
        let breath = (t * 1.8).sin();
        p.scale = Vec2::new(1. - breath * 0.018, 1. + breath * 0.027);
        p.lift = breath * 0.014;
        match self.mood {
            Mood::Idle => {
                // Four autonomous clips, with gentle resting gaps; no model call needed.
                let phase = age % 22.;
                if phase < 5. {
                    p.tilt += (phase * 1.4).sin() * 0.12;
                    p.round = 0.2 + 0.16 * (phase * 1.2).sin();
                } else if phase < 9. {
                    let stretch = ((phase - 5.) / 4. * PI).sin().powi(2);
                    p.scale += Vec2::new(-0.13, 0.14) * stretch;
                    p.lift += stretch * 0.028;
                    p.eyes = 1. - stretch * 0.65;
                } else if phase < 15. {
                    p.round = 0.8;
                    p.tilt += ((phase - 9.) * 1.65).sin() * 0.17;
                    p.lift += ((phase - 9.) * 1.65).sin().abs() * 0.025;
                } else if phase < 19. {
                    p.square = 0.65;
                    p.scale += Vec2::new(0.035, -0.035);
                    p.head.y += 0.3;
                }
            }
            Mood::Thinking => {
                p.round = 0.8;
                p.square = 0.16 * (age * 0.7).sin().abs();
                p.tilt += (age * 1.35).sin() * 0.27;
                p.head.y -= 0.5;
                if !attentive {
                    p.look.y = -0.65;
                }
                p.tint = [0.76, 0.66, 1., 0.66];
            }
            Mood::Inspecting => {
                p.square = 0.9;
                p.head.x += (age * 1.5).sin() * 0.35;
                p.scale.x -= 0.06;
                p.tilt *= 0.45;
                p.tint = [0.52, 0.83, 1., 0.68];
            }
            Mood::Drawing => {
                p.square = 0.95;
                p.tilt += (age * 2.8).sin() * 0.20;
                p.scale += Vec2::new((age * 2.8).sin() * 0.07, -(age * 2.8).sin() * 0.06);
                p.head.y += 0.35;
                p.lift += (age * 2.8).sin().abs() * 0.025;
                p.tint = [0.52, 0.90, 0.85, 0.66];
            }
            Mood::Coloring => {
                p.round = 0.92;
                p.tilt += (age * 1.8).sin() * 0.24;
                p.lift += (age * 1.8).sin().abs() * 0.04;
                p.tint = [
                    0.78 + 0.2 * (age * 0.65).sin(),
                    0.76 + 0.2 * (age * 0.65 + 2.).sin(),
                    0.80 + 0.18 * (age * 0.65 + 4.).sin(),
                    0.78,
                ];
            }
            Mood::Explaining => {
                p.round = 0.8;
                p.scale += Vec2::new(0.08, -0.06);
                p.head.y += (age * 3.).sin() * 0.55;
                p.tilt += (age * 1.5).sin() * 0.12;
                p.tint = [0.86, 0.79, 1., 0.5];
            }
            Mood::Done => {
                p.round = 1.;
                let hop = (age * 5.).sin();
                let energy = (-age * 0.7).exp();
                p.lift += hop.abs() * energy * 0.13;
                p.scale += Vec2::new(-hop * 0.10, hop * 0.1) * energy;
                p.tilt += (age * 4.).sin() * 0.12 * energy;
                p.sparkle = (1. - age / 3.5).max(0.);
                p.tint = [0.58, 1., 0.75, 0.7];
            }
            Mood::NeedsHelp => {
                p.square = 0.8;
                p.tilt = -0.27 + p.look.x * 0.1;
                p.scale += Vec2::new(0.07, -0.1);
                p.eyes = 0.72;
                p.tint = [1., 0.66, 0.54, 0.7];
            }
        }
        if reduce {
            p.look = Vec2::ZERO;
            p.head = Vec2::ZERO;
            p.scale = Vec2::splat(1.);
            p.lift = 0.;
            p.tilt = 0.;
            p.eyes = 1.;
            p.sparkle = 0.;
            p.round = if matches!(
                self.mood,
                Mood::Thinking | Mood::Coloring | Mood::Explaining | Mood::Done
            ) {
                0.9
            } else {
                0.
            };
            p.square = if matches!(
                self.mood,
                Mood::Inspecting | Mood::Drawing | Mood::NeedsHelp
            ) {
                0.9
            } else {
                0.
            };
            if self.mood == Mood::Coloring {
                p.tint = [0.95, 0.72, 0.88, 0.7];
            }
            return p;
        }
        let phase = (self.time % 5.7) as f32;
        let blink = |x: f32| {
            if (0.0..0.24).contains(&x) {
                (PI * x / 0.24).sin().powi(2)
            } else {
                0.
            }
        };
        p.eyes *= 1. - 0.99 * blink(phase - 3.7).max(blink(phase - 4.04));
        p
    }
}

const CENTER: Vec2 = Vec2::new(0.49, 0.565);
const GRID: usize = 28;
pub struct Companion {
    data: RenderData,
    pub motion: Motion,
}
enum RenderData {
    Image(Bitmap),
    Vector(vector::VectorArt),
}
struct Bitmap {
    texture: egui::TextureHandle,
    tint_texture: egui::TextureHandle,
    body_radius: [f32; 128],
}
impl Bitmap {
    fn new(ctx: &egui::Context) -> Self {
        let image = image::load_from_memory(include_bytes!("../assets/companion/body.png"))
            .expect("companion sprite")
            .resize(384, 384, image::imageops::FilterType::Lanczos3)
            .to_rgba8();
        let size = [image.width() as usize, image.height() as usize];
        let texture = ctx.load_texture(
            "doodle-companion",
            egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw()),
            egui::TextureOptions::LINEAR,
        );
        // Cache a luminance layer once, preserving shading/curl/blush under animated color.
        let mut tint = image.clone();
        for pixel in tint.pixels_mut() {
            let light = pixel[0].max(pixel[1]).max(pixel[2]);
            pixel.0 = [light, light, light, pixel[3]];
        }
        let tint_texture = ctx.load_texture(
            "doodle-companion-tint",
            egui::ColorImage::from_rgba_unmultiplied(size, tint.as_raw()),
            egui::TextureOptions::LINEAR,
        );
        let mut body_radius = [0.3; 128];
        for (i, radius) in body_radius.iter_mut().enumerate() {
            let angle = i as f32 / 128. * TAU;
            let dir = Vec2::angled(angle);
            // Sample the pale body, not the terracotta curl above it.
            for step in 1..192 {
                let r = step as f32 / 384.;
                let uv = CENTER + dir * r;
                if uv.x < 0. || uv.y < 0. || uv.x >= 1. || uv.y >= 1. {
                    break;
                }
                let p = image.get_pixel(
                    (uv.x * size[0] as f32) as u32,
                    (uv.y * size[1] as f32) as u32,
                );
                if p[3] > 180 && p[0] > 180 && p[1] > 155 && p[2] > 110 {
                    *radius = r;
                }
            }
        }
        Self {
            texture,
            tint_texture,
            body_radius,
        }
    }
}
impl Companion {
    pub fn new(ctx: &egui::Context) -> Self {
        Self::with_renderer(ctx, Renderer::default())
    }
    pub fn with_renderer(ctx: &egui::Context, renderer: Renderer) -> Self {
        Self {
            data: Self::load_renderer(ctx, renderer),
            motion: Motion::default(),
        }
    }
    fn load_renderer(ctx: &egui::Context, renderer: Renderer) -> RenderData {
        match renderer {
            Renderer::Image => RenderData::Image(Bitmap::new(ctx)),
            Renderer::Vector => RenderData::Vector(vector::VectorArt::new()),
        }
    }
    pub fn renderer(&self) -> Renderer {
        match &self.data {
            RenderData::Image(_) => Renderer::Image,
            RenderData::Vector(_) => Renderer::Vector,
        }
    }
    pub fn set_renderer(&mut self, ctx: &egui::Context, renderer: Renderer) {
        if self.renderer() != renderer {
            self.data = Self::load_renderer(ctx, renderer);
        }
    }
    /// Character-owned geometry and uncompressed custom RGBA textures, not process RSS.
    pub fn footprint(&self) -> Footprint {
        match &self.data {
            RenderData::Image(bitmap) => Footprint {
                custom_texture_bytes: 2 * 384 * 384 * 4,
                geometry_bytes: std::mem::size_of_val(&bitmap.body_radius),
            },
            RenderData::Vector(vector) => Footprint {
                custom_texture_bytes: 0,
                geometry_bytes: vector.bytes(),
            },
        }
    }
    pub fn show(&mut self, ui: &mut egui::Ui, size: f32, reduce: bool) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
        let view = ui.ctx().content_rect();
        let travel = Vec2::new(
            (view.width() * 0.55).max(180.),
            (view.height() * 0.75).max(180.),
        );
        let pointer = ui
            .ctx()
            .pointer_hover_pos()
            .map_or(Vec2::ZERO, |p| (p - rect.center()) / travel);
        let now = ui.input(|i| i.time);
        if response.clicked() && !self.motion.mood.working() {
            self.motion.react(Mood::Done);
        }
        let pose = self.motion.step(now, pointer, reduce);
        self.paint(ui.painter(), rect, pose);
        if !reduce && ui.is_rect_visible(rect) && ui.input(|i| i.focused) {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(16));
        }
        response.on_hover_text(self.motion.mood.label())
    }
    fn morph(&self, uv: Vec2, pose: Pose) -> Vec2 {
        let body_radius = match &self.data {
            RenderData::Image(bitmap) => &bitmap.body_radius,
            RenderData::Vector(vector) => &vector.radius,
        };
        let delta = uv - CENTER;
        if delta.length_sq() < 0.000001 {
            return uv;
        }
        let angle = delta.angle().rem_euclid(TAU) / TAU * 128.;
        let index = angle as usize % 128;
        let original = egui::lerp(
            body_radius[index]..=body_radius[(index + 1) % 128],
            angle.fract(),
        );
        let dir = delta.normalized();
        let square = 0.285 / (dir.x.abs().powf(5.) + dir.y.abs().powf(5.)).powf(0.2);
        let radius = original + (0.32 - original) * pose.round + (square - original) * pose.square;
        // The curl sits outside the pale body. Translate that extension with the
        // contour instead of stretching it into a tall spike during a round morph.
        let length = delta.length();
        CENTER + dir * (length + (radius - original) * (length / original).min(1.))
    }
    pub fn paint(&self, painter: &egui::Painter, rect: Rect, pose: Pose) {
        let rotation = egui::emath::Rot2::from_angle(pose.tilt);
        let transform = |uv: Vec2| {
            let mut p = self.morph(uv, pose) - CENTER;
            p.x *= 1. - pose.head.x.abs() * 0.07;
            p.x += pose.head.x * (0.024 - p.y * 0.045);
            p.y += pose.head.y * 0.015;
            rect.center()
                + Vec2::new(0., rect.height() * 0.065)
                + rotation * (p * pose.scale * rect.width() * 0.85)
                - Vec2::new(0., pose.lift * rect.height())
        };
        if let RenderData::Image(bitmap) = &self.data {
            let mut mesh = egui::Mesh::with_texture(bitmap.texture.id());
            for y in 0..=GRID {
                for x in 0..=GRID {
                    let uv = Vec2::new(x as f32 / GRID as f32, y as f32 / GRID as f32);
                    mesh.vertices.push(egui::epaint::Vertex {
                        pos: transform(uv),
                        uv: Pos2::new(uv.x, uv.y),
                        color: Color32::WHITE,
                    });
                }
            }
            for y in 0..GRID {
                for x in 0..GRID {
                    let a = (y * (GRID + 1) + x) as u32;
                    let b = a + GRID as u32 + 1;
                    mesh.indices.extend([a, a + 1, b + 1, a, b + 1, b]);
                }
            }
            if pose.tint[3] > 0.005 {
                let mut tint = mesh.clone();
                tint.texture_id = bitmap.tint_texture.id();
                let color = Color32::from_rgba_unmultiplied(
                    (pose.tint[0].clamp(0., 1.) * 255.) as u8,
                    (pose.tint[1].clamp(0., 1.) * 255.) as u8,
                    (pose.tint[2].clamp(0., 1.) * 255.) as u8,
                    (pose.tint[3].clamp(0., 1.) * 255.) as u8,
                );
                for vertex in &mut tint.vertices {
                    vertex.color = color;
                }
                painter.add(egui::Shape::mesh(mesh));
                painter.add(egui::Shape::mesh(tint));
            } else {
                painter.add(egui::Shape::mesh(mesh));
            }
        } else if let RenderData::Vector(vector) = &self.data {
            vector.paint(
                painter,
                rect.width() * painter.ctx().pixels_per_point(),
                pose,
                &transform,
            );
        }
        let eye_rotation = egui::emath::Rot2::from_angle(18_f32.to_radians());
        for center in [Vec2::new(0.364, 0.524), Vec2::new(0.631, 0.617)] {
            let center = center + pose.look * 0.034 + pose.head * 0.015;
            let width = 0.044_f32;
            let height = 0.119 * pose.eyes.max(0.045);
            let radius = width.min(height) / 2.;
            let mut points = Vec::with_capacity(28);
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
                    points.push(transform(
                        center
                            + eye_rotation
                                * Vec2::new(cx + radius * a.cos(), cy + radius * a.sin()),
                    ));
                }
            }
            painter.add(egui::Shape::convex_polygon(
                points,
                Color32::from_rgb(17, 17, 17),
                egui::Stroke::NONE,
            ));
        }
        if pose.sparkle > 0.01 {
            for (x, y) in [(0.10, 0.28), (0.87, 0.25), (0.80, 0.78)] {
                let center = rect.min + Vec2::new(x, y) * rect.size();
                let r = rect.width() * 0.04 * pose.sparkle;
                let color = Color32::from_rgba_unmultiplied(
                    99,
                    174,
                    121,
                    (pose.sparkle.clamp(0., 1.) * 255.) as u8,
                );
                painter.line_segment(
                    [center - Vec2::new(r, 0.), center + Vec2::new(r, 0.)],
                    egui::Stroke::new(1.5_f32, color),
                );
                painter.line_segment(
                    [center - Vec2::new(0., r), center + Vec2::new(0., r)],
                    egui::Stroke::new(1.5_f32, color),
                );
            }
        }
    }
}
