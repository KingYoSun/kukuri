use iroh::SecretKey;
use kukuri_lib::application::services::p2p_service::P2PService;
use kukuri_lib::infrastructure::p2p::DiscoveryOptions;
use kukuri_lib::shared::config::NetworkConfig as AppNetworkConfig;

fn mainline_ready_config() -> AppNetworkConfig {
    AppNetworkConfig {
        bootstrap_peers: vec!["peer1.example:1337".into()],
        max_peers: 24,
        connection_timeout: 30,
        retry_interval: 10,
        enable_dht: true,
        enable_dns: false,
        enable_local: true,
    }
}

#[test]
fn builder_enables_mainline_when_configured() {
    let config = mainline_ready_config();
    let secret = SecretKey::from_bytes(&[1u8; 32]);

    let builder = P2PService::builder(secret, config.clone());
    let options = builder.discovery_options();

    assert!(options.enable_mainline(), "mainline should be enabled");
    assert!(!options.enable_dns, "DNS discovery stays disabled");
    assert!(options.enable_local, "local discovery remains enabled");
}

#[test]
fn builder_can_disable_mainline_via_toggle() {
    let mut config = mainline_ready_config();
    config.enable_dht = false;
    let secret = SecretKey::from_bytes(&[2u8; 32]);

    let builder = P2PService::builder(secret, config).enable_mainline(false);
    let options = builder.discovery_options();

    assert_eq!(
        options,
        DiscoveryOptions::new(false, false, true),
        "explicit toggle should override configuration flags"
    );
}
