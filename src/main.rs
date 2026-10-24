use ::eframe::egui;

fn main() -> eframe::Result {
    eframe::run_native(
        "WaveR",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(MyApp::default()))),
    )
}
