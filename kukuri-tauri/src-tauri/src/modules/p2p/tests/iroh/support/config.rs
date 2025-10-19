use crate::infrastructure::p2p::utils::parse_peer_hint;

use super::bootstrap::BootstrapContext;
use super::logging::log_step;

pub(crate) fn load_bootstrap_context(test_name: &str) -> Option<BootstrapContext> {
    if std::env::var("ENABLE_P2P_INTEGRATION").unwrap_or_default() != "1" {
        log_step!("skipping {} (ENABLE_P2P_INTEGRATION != 1)", test_name);
        return None;
    }

    let raw = std::env::var("KUKURI_BOOTSTRAP_PEERS").unwrap_or_default();
    if raw.trim().is_empty() {
        log_step!("skipping {} (KUKURI_BOOTSTRAP_PEERS not set)", test_name);
        return None;
    }

    let mut hints = Vec::new();
    let mut addrs = Vec::new();

    for entry in raw.split(',') {
        let trimmed = entry.trim();
        if trimmed.is_empty() {
            continue;
        }
        hints.push(trimmed.to_string());
        match parse_peer_hint(trimmed) {
            Ok(parsed) => {
                if let Some(addr) = parsed.node_addr {
                    addrs.push(addr);
                } else {
                    log_step!("bootstrap peer '{}' missing address; skipping", trimmed);
                }
            }
            Err(err) => {
                log_step!("failed to parse bootstrap peer '{}': {:?}", trimmed, err);
                return None;
            }
        }
    }

    if addrs.is_empty() {
        log_step!(
            "skipping {} (no usable bootstrap node addresses)",
            test_name
        );
        return None;
    }

    log_step!(
        "test {} using bootstrap peers: {}",
        test_name,
        hints.join(", ")
    );

    Some(BootstrapContext {
        hints,
        node_addrs: addrs,
    })
}
