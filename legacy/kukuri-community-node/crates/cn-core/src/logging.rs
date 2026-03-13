use tracing_subscriber::EnvFilter;

pub fn init(service_name: &str) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .json()
        .with_target(false)
        .init();

    tracing::info!(service = service_name, "logging initialized");
}
