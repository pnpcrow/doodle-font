//! Native-rendered motion evidence using the exact production sprite and face rig.
#[cfg(feature = "ui-qa")]
fn main() -> eframe::Result {
    use doodle_core::companion::{Companion, Mood};
    use eframe::egui::{self, Color32, Vec2};
    struct Preview {
        pets: Vec<Companion>,
        directory: std::path::PathBuf,
        saved: usize,
    }
    impl eframe::App for Preview {
        fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
            let frame = ctx.cumulative_frame_nr();
            // Fixed 60Hz animation clock makes captured phases reproducible.
            let time = frame as f64 / 60.;
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::new()
                        .fill(Color32::from_rgb(245, 246, 248))
                        .inner_margin(12),
                )
                .show(ctx, |ui| {
                    ui.heading("Doodle · motion sets");
                    for row in 0..2 {
                        ui.horizontal(|ui| {
                            for index in row * 4..row * 4 + 4 {
                                let label = [
                                    "Idle / stretch / roll",
                                    "Think / tilt",
                                    "Inspect / scan",
                                    "Draw / squircle",
                                    "Color / circle",
                                    "Explain / nod",
                                    "Done / spring",
                                    "Help / lean",
                                ][index];
                                ui.vertical(|ui| {
                                    ui.set_width(155.);
                                    ui.label(
                                        egui::RichText::new(label)
                                            .color(Color32::from_rgb(31, 41, 55)),
                                    );
                                    let (rect, _) = ui.allocate_exact_size(
                                        Vec2::splat(155.),
                                        egui::Sense::hover(),
                                    );
                                    let pet = &mut self.pets[index];
                                    let mood = [
                                        Mood::Idle,
                                        Mood::Thinking,
                                        Mood::Inspecting,
                                        Mood::Drawing,
                                        Mood::Coloring,
                                        Mood::Explaining,
                                        Mood::Done,
                                        Mood::NeedsHelp,
                                    ][index];
                                    if frame == 1 || (index >= 5 && frame % 300 == 1) {
                                        pet.motion.react(mood);
                                    }
                                    let pose = pet.motion.step(time, Vec2::ZERO, false);
                                    pet.paint(ui.painter(), rect, pose);
                                });
                            }
                        });
                    }
                    ui.separator();
                    ui.horizontal(|ui| {
                        for index in 8..11 {
                            ui.vertical(|ui| {
                                ui.set_width(210.);
                                let mood = [
                                    Mood::Idle,
                                    Mood::Thinking,
                                    Mood::Drawing,
                                    Mood::Coloring,
                                    Mood::Done,
                                    Mood::NeedsHelp,
                                ][((time / 2.) as usize) % 6];
                                ui.label(if index == 8 {
                                    "Continuous transitions"
                                } else if index == 9 {
                                    "Cursor · eyes lead head"
                                } else {
                                    "Reduced motion · same state"
                                });
                                let (rect, _) =
                                    ui.allocate_exact_size(Vec2::splat(145.), egui::Sense::hover());
                                let pet = &mut self.pets[index];
                                if index != 9 {
                                    pet.motion.react(mood);
                                }
                                let pointer = if index == 9 {
                                    Vec2::new((time as f32 * 1.2).sin(), (time as f32 * 0.9).cos())
                                } else {
                                    Vec2::ZERO
                                };
                                let pose = pet.motion.step(time, pointer, index == 10);
                                pet.paint(ui.painter(), rect, pose);
                                if index == 9 {
                                    ui.painter().circle_filled(
                                        rect.center() + pointer * 65.,
                                        3.,
                                        Color32::from_rgb(90, 110, 190),
                                    );
                                }
                            });
                        }
                    });
                });
            if frame >= 2 && frame.is_multiple_of(2) && self.saved < 360 {
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
                let bytes: Vec<_> = im.pixels.iter().flat_map(|p| p.to_array()).collect();
                use image::ImageEncoder;
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
                if self.saved >= 360 {
                    println!("Captured {} native frames: eight clips, interrupted transitions, pointer head/eye tracking and reduced motion.",self.saved);
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
    }
    let directory = std::path::PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or("artifacts/companion-motion".into()),
    );
    std::fs::create_dir_all(&directory).unwrap();
    eframe::run_native(
        "Doodle companion verification",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([710., 630.]),
            ..Default::default()
        },
        Box::new(move |cc| {
            Ok(Box::new(Preview {
                pets: (0..11).map(|_| Companion::new(&cc.egui_ctx)).collect(),
                directory,
                saved: 0,
            }))
        }),
    )
}
#[cfg(not(feature = "ui-qa"))]
fn main() {
    eprintln!("Run with --features ui-qa to capture native companion motion.");
}
