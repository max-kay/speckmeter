fn main() {
    tracing_subscriber::fmt::init();

    let native_options = eframe::NativeOptions {
        icon_data: Some(load_icon("./icon.png")), // an example
        ..Default::default()
    };
    eframe::run_native(
        "Speckmeter",
        native_options,
        Box::new(|cc| Box::new(speckmeter::SpeckApp::new(cc))),
    );
}

fn load_icon(path: &str) -> eframe::IconData {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::open(path)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };

    eframe::IconData {
        rgba: icon_rgba,
        width: icon_width,
        height: icon_height,
    }
}
