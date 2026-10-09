pub struct Studio;
impl Studio {pub fn new(_: &eframe::CreationContext<'_>)->Self{Self}}
impl eframe::App for Studio {fn update(&mut self,ctx:&eframe::egui::Context,_:&mut eframe::Frame){eframe::egui::CentralPanel::default().show(ctx,|ui|{ui.heading("Doodle Font");});}}
