use std::sync::Once;
use tracing_subscriber::EnvFilter;

static INIT_TRACING: Once = Once::new();

pub fn init_tracing() {
    INIT_TRACING.call_once(|| {
        let env_filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info,iroh_tests=info"));
        let subscriber = tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .with_target(true)
            .compact()
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);
    });
}

macro_rules! log_step {
    ($($arg:tt)*) => {{
        tracing::info!(target: "iroh_tests", $($arg)*);
    }};
}

pub(crate) use log_step;
