fn main() {
    tracing_subscriber::fmt::init();

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Speckmeter",
        native_options,
        Box::new(|cc| Box::new(speckmeter::SpeckApp::new(cc))),
    );
}