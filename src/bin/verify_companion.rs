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
                    ui.horizontal(|ui| {
                        for (index, label) in ["Idle / gaze", "Thinking", "Done", "Needs help"]
                            .into_iter()
                            .enumerate()
                        {
                            ui.vertical(|ui| {
                                ui.set_width(170.);
                                ui.label(
                                    egui::RichText::new(label).color(Color32::from_rgb(31, 41, 55)),
                                );
                                let (rect, _) =
                                    ui.allocate_exact_size(Vec2::splat(170.), egui::Sense::hover());
                                let pet = &mut self.pets[index];
                                let mood =
                                    [Mood::Idle, Mood::Thinking, Mood::Done, Mood::NeedsHelp]
                                        [index];
                                if frame == 1 || (index >= 2 && frame % 180 == 1) {
                                    pet.motion.react(mood);
                                }
                                let pose = pet.motion.step(
                                    time,
                                    Vec2::new((time as f32).sin(), -0.3),
                                    false,
                                );
                                pet.paint(ui.painter(), rect, pose);
                            });
                        }
                    });
                });
            if frame >= 2 && frame.is_multiple_of(2) && self.saved < 180 {
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
                image::save_buffer(
                    self.directory.join(format!("{:04}.png", self.saved)),
                    &bytes,
                    im.width() as u32,
                    im.height() as u32,
                    image::ColorType::Rgba8,
                )
                .unwrap();
                self.saved += 1;
                if self.saved >= 180 {
                    println!("Captured {} native frames; idle, gaze, blink, thinking, done, and error rig states.",self.saved);
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
            viewport: egui::ViewportBuilder::default().with_inner_size([740., 220.]),
            ..Default::default()
        },
        Box::new(move |cc| {
            Ok(Box::new(Preview {
                pets: (0..4).map(|_| Companion::new(&cc.egui_ctx)).collect(),
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
