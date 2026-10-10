//! Independent vector artwork. Cubic paths are flattened at scale-appropriate
//! detail once; the animation uses geometry, never rasterized SVGs or sprite frames.
use super::{Pose, CENTER};
use eframe::egui::{self, Color32, Pos2, Vec2};
use std::f32::consts::TAU;

// Each row is [control 1, control 2, endpoint], in the original character space.
const BODY_START: Vec2 = Vec2::new(0.522, 0.252);
const BODY: [[[f32; 2]; 3]; 7] = [
    [[0.603, 0.237], [0.660, 0.257], [0.691, 0.311]],
    [[0.720, 0.371], [0.781, 0.422], [0.813, 0.535]],
    [[0.845, 0.634], [0.854, 0.721], [0.792, 0.789]],
    [[0.732, 0.856], [0.655, 0.870], [0.549, 0.870]],
    [[0.396, 0.885], [0.193, 0.821], [0.151, 0.725]],
    [[0.092, 0.593], [0.199, 0.435], [0.291, 0.363]],
    [[0.363, 0.307], [0.449, 0.267], [0.522, 0.252]],
];
const CURL_START: Vec2 = Vec2::new(0.493, 0.186);
const CURL: [[[f32; 2]; 3]; 7] = [
    [[0.467, 0.161], [0.501, 0.121], [0.543, 0.116]],
    [[0.632, 0.094], [0.696, 0.159], [0.701, 0.220]],
    [[0.710, 0.290], [0.658, 0.345], [0.574, 0.343]],
    [[0.499, 0.345], [0.445, 0.300], [0.485, 0.264]],
    [[0.510, 0.244], [0.553, 0.249], [0.562, 0.219]],
    [[0.571, 0.192], [0.538, 0.181], [0.517, 0.191]],
    [[0.504, 0.197], [0.497, 0.195], [0.493, 0.186]],
];
fn flatten(start: Vec2, curves: &[[[f32; 2]; 3]], detail: usize) -> Vec<Vec2> {
    let mut points = Vec::with_capacity(curves.len() * detail);
    let mut from = start;
    for [a, b, c] in curves {
        let a = Vec2::from(*a);
        let b = Vec2::from(*b);
        let to = Vec2::from(*c);
        for i in 0..detail {
            let t = i as f32 / detail as f32;
            let s = 1. - t;
            points
                .push(from * s * s * s + a * 3. * s * s * t + b * 3. * s * t * t + to * t * t * t);
        }
        from = to;
    }
    points
}
fn cross(a: Vec2, b: Vec2) -> f32 {
    a.x * b.y - a.y * b.x
}
// Triangulate our fixed, simple concave curl once. Filled egui PathShape only
// supports convex polygons, so passing the curl directly would fill its opening.
fn triangulate(points: &[Vec2]) -> Vec<u32> {
    let mut remaining: Vec<usize> = (0..points.len()).collect();
    let mut triangles = Vec::with_capacity((points.len() - 2) * 3);
    while remaining.len() > 3 {
        let ear = (0..remaining.len())
            .find(|&i| {
                let a = remaining[(i + remaining.len() - 1) % remaining.len()];
                let b = remaining[i];
                let c = remaining[(i + 1) % remaining.len()];
                let (pa, pb, pc) = (points[a], points[b], points[c]);
                cross(pb - pa, pc - pb) > 1e-9
                    && !remaining.iter().any(|&p| {
                        p != a
                            && p != b
                            && p != c
                            && cross(pb - pa, points[p] - pa) >= -1e-9
                            && cross(pc - pb, points[p] - pb) >= -1e-9
                            && cross(pa - pc, points[p] - pc) >= -1e-9
                    })
            })
            .expect("simple clockwise companion contour");
        triangles.extend([
            remaining[(ear + remaining.len() - 1) % remaining.len()] as u32,
            remaining[ear] as u32,
            remaining[(ear + 1) % remaining.len()] as u32,
        ]);
        remaining.remove(ear);
    }
    triangles.extend(remaining.into_iter().map(|i| i as u32));
    triangles
}
struct Contour {
    points: Vec<Vec2>,
    indices: Vec<u32>,
}
impl Contour {
    fn new(points: Vec<Vec2>) -> Self {
        let indices = triangulate(&points);
        Self { points, indices }
    }
    fn bytes(&self) -> usize {
        self.points.capacity() * std::mem::size_of::<Vec2>() + self.indices.capacity() * 4
    }
    fn paint(&self, painter: &egui::Painter, transform: &impl Fn(Vec2) -> Pos2, color: Color32) {
        let points: Vec<_> = self.points.iter().map(|p| transform(*p)).collect();
        let mut mesh = egui::Mesh::default();
        mesh.vertices.reserve(points.len() * 2);
        mesh.indices.reserve(self.indices.len() + points.len() * 6);
        // One physical-pixel AA fringe. Geometry and silhouette stay sharp at any DPI.
        let feather = 0.5 / painter.ctx().pixels_per_point();
        for i in 0..points.len() {
            let prev = points[(i + points.len() - 1) % points.len()];
            let next = points[(i + 1) % points.len()];
            let edge_a = (points[i] - prev).normalized();
            let edge_b = (next - points[i]).normalized();
            let normal = Vec2::new(edge_a.y + edge_b.y, -edge_a.x - edge_b.x).normalized();
            for (offset, color) in [(-feather, color), (feather, Color32::TRANSPARENT)] {
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: points[i] + normal * offset,
                    uv: egui::epaint::WHITE_UV,
                    color,
                });
            }
            let a = (i * 2) as u32;
            let b = (((i + 1) % points.len()) * 2) as u32;
            mesh.indices.extend([a, b, b + 1, a, b + 1, a + 1]);
        }
        mesh.indices.extend(self.indices.iter().map(|i| i * 2));
        painter.add(egui::Shape::mesh(mesh));
    }
}
struct Detail {
    body: Contour,
    curl: Contour,
}
pub(super) struct VectorArt {
    levels: Vec<Detail>,
    pub radius: [f32; 128],
}
impl VectorArt {
    pub fn new() -> Self {
        let levels: Vec<_> = [6, 12, 24, 48]
            .into_iter()
            .map(|n| Detail {
                body: Contour::new(flatten(BODY_START, &BODY, n)),
                curl: Contour::new(flatten(CURL_START, &CURL, n)),
            })
            .collect();
        let points = &levels[3].body.points;
        let mut radius = [0.; 128];
        for (i, r) in radius.iter_mut().enumerate() {
            let dir = Vec2::angled(i as f32 / 128. * TAU);
            for (a, b) in points
                .iter()
                .zip(points.iter().cycle().skip(1))
                .take(points.len())
            {
                let edge = *b - *a;
                let denominator = cross(dir, edge);
                if denominator.abs() < 1e-8 {
                    continue;
                }
                let distance = cross(*a - CENTER, edge) / denominator;
                let segment = cross(*a - CENTER, dir) / denominator;
                if distance > 0. && (0.0..=1.0).contains(&segment) {
                    *r = distance;
                    break;
                }
            }
            assert!(*r > 0.1, "body is star-shaped around its center");
        }
        Self { levels, radius }
    }
    pub fn bytes(&self) -> usize {
        self.levels
            .iter()
            .map(|d| d.body.bytes() + d.curl.bytes())
            .sum::<usize>()
            + std::mem::size_of_val(&self.radius)
    }
    pub fn paint(
        &self,
        painter: &egui::Painter,
        physical_size: f32,
        pose: Pose,
        transform: &impl Fn(Vec2) -> Pos2,
    ) {
        let level = if physical_size <= 96. {
            0
        } else if physical_size <= 240. {
            1
        } else if physical_size <= 640. {
            2
        } else {
            3
        };
        let detail = &self.levels[level];
        let tint = |rgb: [u8; 3]| {
            let light = *rgb.iter().max().unwrap() as f32;
            let mix = pose.tint[3].clamp(0., 1.);
            let c = std::array::from_fn::<_, 3, _>(|i| {
                (rgb[i] as f32 * (1. - mix) + light * pose.tint[i] * mix).clamp(0., 255.) as u8
            });
            Color32::from_rgb(c[0], c[1], c[2])
        };
        detail.body.paint(painter, transform, tint([246, 203, 157]));
        detail.curl.paint(painter, transform, tint([204, 120, 82]));
        let rotation = egui::emath::Rot2::from_angle(18_f32.to_radians());
        for center in [Vec2::new(0.294, 0.605), Vec2::new(0.635, 0.720)] {
            let points = (0..32)
                .map(|i| {
                    transform(
                        center
                            + rotation
                                * (Vec2::angled(i as f32 / 32. * TAU) * Vec2::new(0.035, 0.02)),
                    )
                })
                .collect();
            painter.add(egui::Shape::convex_polygon(
                points,
                tint([237, 153, 115]),
                egui::Stroke::NONE,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vector_contours_preserve_area_and_curl_opening() {
        let art = VectorArt::new();
        for level in &art.levels {
            for contour in [&level.body, &level.curl] {
                let signed: f32 = contour
                    .points
                    .iter()
                    .zip(contour.points.iter().cycle().skip(1))
                    .take(contour.points.len())
                    .map(|(a, b)| cross(*a, *b) * 0.5)
                    .sum();
                let triangles: f32 = contour
                    .indices
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .map(|t| {
                        let a = contour.points[t[0] as usize];
                        let b = contour.points[t[1] as usize];
                        let c = contour.points[t[2] as usize];
                        let area = cross(b - a, c - a) * 0.5;
                        assert!(area > 0.);
                        area
                    })
                    .sum();
                assert!((signed - triangles).abs() < 1e-6);
            }
        }
    }
}
