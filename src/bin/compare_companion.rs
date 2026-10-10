//! Reproducible renderer comparison; CPU measurements exclude GPU and screenshot I/O.
use doodle_core::companion::{Companion, Mood, Motion, Pose, Renderer};
use eframe::egui::{self, Pos2, Rect, Vec2};
use std::{path::Path, time::Instant};

const MOODS: [Mood; 8] = [
    Mood::Idle,
    Mood::Thinking,
    Mood::Inspecting,
    Mood::Drawing,
    Mood::Coloring,
    Mood::Explaining,
    Mood::Done,
    Mood::NeedsHelp,
];

fn frame(ctx: &egui::Context, pet: &Companion, pose: Pose, size: f32) -> (usize, usize) {
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::splat(1200.))),
            ..Default::default()
        },
        |ctx| {
            pet.paint(
                &ctx.debug_painter(),
                Rect::from_min_size(Pos2::new(50., 50.), Vec2::splat(size)),
                pose,
            )
        },
    );
    ctx.tessellate(output.shapes, output.pixels_per_point)
        .iter()
        .fold((0, 0), |(v, t), p| {
            if let egui::epaint::Primitive::Mesh(m) = &p.primitive {
                (v + m.vertices.len(), t + m.indices.len() / 3)
            } else {
                (v, t)
            }
        })
}

fn benchmark(directory: &Path) {
    let contexts = [egui::Context::default(), egui::Context::default()];
    let mut startup = [Vec::new(), Vec::new()];
    for round in 0..12 {
        for index in [round % 2, 1 - round % 2] {
            let start = Instant::now();
            let pet = Companion::with_renderer(
                &contexts[index],
                [Renderer::Image, Renderer::Vector][index],
            );
            startup[index].push(start.elapsed().as_secs_f64() * 1000.);
            std::hint::black_box(pet);
        }
    }
    let pets = [
        Companion::with_renderer(&contexts[0], Renderer::Image),
        Companion::with_renderer(&contexts[1], Renderer::Vector),
    ];
    let mut motion = Motion::default();
    let poses: Vec<_> = (0..960)
        .map(|i| {
            motion.react(MOODS[i / 120]);
            motion.step(
                i as f64 / 60.,
                Vec2::new((i as f32 * 0.03).sin(), (i as f32 * 0.02).cos()),
                false,
            )
        })
        .collect();
    let mut results = Vec::new();
    for dpi in [1., 2.] {
        for ctx in &contexts {
            ctx.set_pixels_per_point(dpi);
        }
        for size in [64., 192., 512.] {
            for (ctx, pet) in contexts.iter().zip(&pets) {
                for pose in poses.iter().take(120) {
                    std::hint::black_box(frame(ctx, pet, *pose, size));
                }
            }
            let mut samples = [Vec::new(), Vec::new()];
            let mut geometry = [(0, 0); 2];
            // Alternate order across five rounds to reduce clock/temperature bias.
            for round in 0..5 {
                for index in [round % 2, 1 - round % 2] {
                    for pose in &poses {
                        let start = Instant::now();
                        let counts = frame(&contexts[index], &pets[index], *pose, size);
                        samples[index].push(start.elapsed().as_secs_f64() * 1_000_000.);
                        geometry[index].0 = geometry[index].0.max(counts.0);
                        geometry[index].1 = geometry[index].1.max(counts.1);
                        std::hint::black_box(counts);
                    }
                }
            }
            for index in 0..2 {
                samples[index].sort_by(f64::total_cmp);
                let s = &samples[index];
                results.push(serde_json::json!({
                    "renderer": pets[index].renderer(), "logical_size": size, "pixels_per_point": dpi,
                    "samples": s.len(), "median_cpu_us": s[s.len()/2], "p95_cpu_us": s[s.len()*95/100],
                    "max_vertices": geometry[index].0, "max_triangles": geometry[index].1,
                }));
            }
        }
    }
    for times in &mut startup {
        times.sort_by(f64::total_cmp);
    }
    let result = serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"), "optimized": !cfg!(debug_assertions),
        "method": "Single companion, identical poses and egui context setup, 120 warmup frames, 5 alternating rounds of 960 frames across 8 moods. CPU includes paint/context/tessellation and excludes GPU submission, frame pacing, startup and screenshot I/O. Texture bytes are custom RGBA allocations, not total process RAM. Startup excludes actual GPU upload.",
        "image": { "footprint": pets[0].footprint(), "median_setup_ms": startup[0][6] },
        "vector": { "footprint": pets[1].footprint(), "median_setup_ms": startup[1][6] },
        "results": results,
    });
    let path = directory.join("benchmark.json");
    std::fs::write(&path, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    println!("{}", result);
    println!("Saved {}", path.display());
}

#[cfg(feature = "ui-qa")]
fn capture(directory: std::path::PathBuf) -> eframe::Result {
    use egui::Color32;
    struct Preview {
        pets: [Companion; 2],
        motion: Motion,
        directory: std::path::PathBuf,
        saved: usize,
    }
    impl eframe::App for Preview {
        fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
            let frame = ctx.cumulative_frame_nr();
            let time = frame as f64 / 60.;
            let mood = MOODS[((time / 2.) as usize) % 8];
            self.motion.react(mood);
            let pointer = Vec2::new((time as f32 * 1.2).sin(), (time as f32 * 0.9).cos());
            let pose = self.motion.step(time, pointer, false);
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::new()
                        .fill(Color32::from_rgb(245, 246, 248))
                        .inner_margin(20),
                )
                .show(ctx, |ui| {
                    ui.visuals_mut().override_text_color = Some(Color32::from_rgb(31, 41, 55));
                    ui.heading("Doodle Font | same motion, two renderers");
                    ui.label(format!("{:?} | cursor follows an identical path", mood));
                    ui.horizontal(|ui| {
                        for (index, label) in ["Original image + vector eyes", "Pure vector curves"]
                            .into_iter()
                            .enumerate()
                        {
                            ui.vertical(|ui| {
                                ui.set_width(400.);
                                ui.heading(label);
                                let (rect, _) =
                                    ui.allocate_exact_size(Vec2::splat(384.), egui::Sense::hover());
                                self.pets[index].paint(ui.painter(), rect, pose);
                                ui.horizontal(|ui| {
                                    for size in [64., 128., 192.] {
                                        ui.vertical(|ui| {
                                            ui.label(format!("{size:.0} pt"));
                                            let (rect, _) = ui.allocate_exact_size(
                                                Vec2::splat(size),
                                                egui::Sense::hover(),
                                            );
                                            self.pets[index].paint(ui.painter(), rect, pose);
                                        });
                                    }
                                });
                            });
                        }
                    });
                });
            if frame >= 2 && frame.is_multiple_of(3) && self.saved < 320 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            }
            if let Some(im) = ctx.input(|i| {
                i.events.iter().find_map(|e| {
                    if let egui::Event::Screenshot { image, .. } = e {
                        Some(image.clone())
                    } else {
                        None
                    }
                })
            }) {
                use image::ImageEncoder;
                let bytes: Vec<_> = im.pixels.iter().flat_map(|p| p.to_array()).collect();
                let file =
                    std::fs::File::create(self.directory.join(format!("{:04}.png", self.saved)))
                        .unwrap();
                image::codecs::png::PngEncoder::new_with_quality(
                    file,
                    image::codecs::png::CompressionType::Fast,
                    image::codecs::png::FilterType::NoFilter,
                )
                .write_image(
                    &bytes,
                    im.width() as u32,
                    im.height() as u32,
                    image::ExtendedColorType::Rgba8,
                )
                .unwrap();
                self.saved += 1;
                if self.saved >= 320 {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
    }
    eframe::run_native(
        "Doodle renderer comparison",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([880., 710.]),
            ..Default::default()
        },
        Box::new(move |cc| {
            Ok(Box::new(Preview {
                pets: [
                    Companion::with_renderer(&cc.egui_ctx, Renderer::Image),
                    Companion::with_renderer(&cc.egui_ctx, Renderer::Vector),
                ],
                motion: Motion::default(),
                directory,
                saved: 0,
            }))
        }),
    )
}

fn main() {
    let directory = std::path::PathBuf::from(
        std::env::args()
            .nth(2)
            .unwrap_or("artifacts/companion-renderers".into()),
    );
    std::fs::create_dir_all(&directory).unwrap();
    if std::env::args().nth(1).as_deref() == Some("--capture") {
        #[cfg(feature = "ui-qa")]
        capture(directory).unwrap();
        #[cfg(not(feature = "ui-qa"))]
        panic!("Capture requires --features ui-qa");
    } else {
        benchmark(&directory);
    }
}
