use crate::app::Chaos;

mod app;

fn main() -> eframe::Result<()>{
    env_logger::init();
    let native_options = eframe::NativeOptions{
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };
    eframe::run_native(
        "chaos",
        native_options,
        Box::new(|cc| Box::new(Chaos::new(cc))),
    )
}
