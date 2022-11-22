fn init_native_log() {
    use tracing::metadata::LevelFilter;
    use tracing_subscriber::{filter, prelude::*};

    let stdout_log = tracing_subscriber::fmt::layer()
        .with_ansi(true)
        .pretty()
        .with_filter(LevelFilter::WARN);

    let mut buffer = String::new();
    let file_log = create_log_file().map(|file| {
        tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_filter(LevelFilter::INFO)
            // TODO: remove this
            .with_filter(filter::filter_fn(|metadata| {
                !metadata.target().ends_with("wayland::seat::pointer")
            }))
    });

    tracing_subscriber::registry()
        .with(stdout_log)
        .with(file_log)
        .init();
}

fn create_log_file() -> Option<std::fs::File> {
    let path = cache_dir.join("logs");
    std::fs::create_dir_all(path.clone()).ok()?;
    let file_name = format!("{}.log", chrono::Local::now().format("%Y%m%dT%H%M%S"));
    std::fs::File::create(path.join(file_name)).ok()
}
