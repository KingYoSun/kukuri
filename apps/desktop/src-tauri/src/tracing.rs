use tracing_subscriber::EnvFilter;

pub(crate) const DEFAULT_TRACING_DIRECTIVES: &str =
    "warn,kukuri_desktop_tauri_lib=info,kukuri_app_api=info";
pub(crate) const DEFAULT_SUPPRESS_DIRECTIVES: &[&str] = &[
    "mainline::rpc::socket=error",
    "noq_proto::connection=error",
    "iroh::socket::remote_map::remote_state=error",
    "iroh_docs::engine::live=error",
    "iroh_gossip::net=error",
];

pub(crate) fn resolve_tracing_directives(rust_log: Option<&str>) -> String {
    let directives = rust_log
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_TRACING_DIRECTIVES)
        .to_owned();

    let mut resolved = directives.clone();
    for suppress_directive in DEFAULT_SUPPRESS_DIRECTIVES {
        let target = suppress_directive
            .split('=')
            .next()
            .expect("suppress directives must have a target");
        if directives_contains_target(&directives, target) {
            continue;
        }
        resolved.push(',');
        resolved.push_str(suppress_directive);
    }

    resolved
}

fn directives_contains_target(directives: &str, target: &str) -> bool {
    directives
        .split(',')
        .map(str::trim)
        .filter(|directive| !directive.is_empty())
        .any(|directive| directive == target || directive.starts_with(&format!("{target}=")))
}

pub(crate) fn init_tracing() {
    let env_filter = EnvFilter::new(resolve_tracing_directives(
        std::env::var("RUST_LOG").ok().as_deref(),
    ));

    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .try_init();
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_SUPPRESS_DIRECTIVES, DEFAULT_TRACING_DIRECTIVES, resolve_tracing_directives,
    };

    #[test]
    fn default_tracing_directives_add_noise_suppression() {
        let directives = resolve_tracing_directives(None);
        assert!(directives.contains(DEFAULT_TRACING_DIRECTIVES));
        for suppress_directive in DEFAULT_SUPPRESS_DIRECTIVES {
            assert!(directives.contains(suppress_directive));
        }
    }

    #[test]
    fn explicit_rust_log_keeps_target_specific_override() {
        let directives = resolve_tracing_directives(Some(
            "info,iroh_docs::engine::live=warn,kukuri_desktop_tauri_lib=debug",
        ));
        assert!(
            directives.contains("iroh_docs::engine::live=warn"),
            "expected explicit target override to be preserved"
        );
        assert!(!directives.contains("iroh_docs::engine::live=error"));
        assert!(directives.contains("noq_proto::connection=error"));
        assert!(directives.contains("mainline::rpc::socket=error"));
    }
}
