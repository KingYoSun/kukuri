use super::core::{P2PService, P2PServiceTrait};
use crate::domain::p2p::events::P2PEvent;
use crate::infrastructure::p2p::{
    DiscoveryOptions, GossipService, NetworkService, iroh_gossip_service::IrohGossipService,
    iroh_network_service::IrohNetworkService,
};
use crate::shared::config::NetworkConfig as AppNetworkConfig;
use crate::shared::error::AppError;
use iroh::SecretKey;
use std::sync::Arc;
use tokio::sync::broadcast;

pub struct P2PStack {
    pub network_service: Arc<dyn NetworkService>,
    pub gossip_service: Arc<dyn GossipService>,
    pub p2p_service: Arc<dyn P2PServiceTrait>,
}

pub struct P2PServiceBuilder {
    secret_key: SecretKey,
    network_config: AppNetworkConfig,
    discovery_options: DiscoveryOptions,
    event_sender: Option<broadcast::Sender<P2PEvent>>,
}

impl P2PServiceBuilder {
    pub(crate) fn new(
        secret_key: SecretKey,
        network_config: AppNetworkConfig,
        discovery_options: DiscoveryOptions,
    ) -> Self {
        Self {
            secret_key,
            network_config,
            discovery_options,
            event_sender: None,
        }
    }

    pub fn with_discovery_options(mut self, options: DiscoveryOptions) -> Self {
        self.discovery_options = options;
        self
    }

    pub fn enable_mainline(mut self, enabled: bool) -> Self {
        self.discovery_options = self.discovery_options.with_mainline(enabled);
        self
    }

    pub fn with_event_sender(mut self, sender: broadcast::Sender<P2PEvent>) -> Self {
        self.event_sender = Some(sender);
        self
    }

    pub fn discovery_options(&self) -> DiscoveryOptions {
        self.discovery_options
    }

    pub async fn build(self) -> Result<P2PStack, AppError> {
        let P2PServiceBuilder {
            secret_key,
            network_config,
            discovery_options,
            event_sender,
        } = self;

        let (network_event_sender, gossip_event_sender) = match event_sender {
            Some(sender) => (Some(sender.clone()), Some(sender)),
            None => (None, None),
        };

        let iroh_network = Arc::new(
            IrohNetworkService::new(
                secret_key,
                network_config,
                discovery_options,
                network_event_sender,
            )
            .await?,
        );
        let endpoint_arc = iroh_network.endpoint().clone();
        let mut gossip_inner = IrohGossipService::new(endpoint_arc)?;
        if let Some(tx) = gossip_event_sender {
            gossip_inner.set_event_sender(tx);
        }
        let iroh_gossip = Arc::new(gossip_inner);

        let network_service_dyn: Arc<dyn NetworkService> = iroh_network.clone();
        let gossip_service_dyn: Arc<dyn GossipService> = iroh_gossip.clone();
        let p2p_service: Arc<dyn P2PServiceTrait> = Arc::new(P2PService::with_discovery(
            Arc::clone(&network_service_dyn),
            Arc::clone(&gossip_service_dyn),
            discovery_options,
        ));

        Ok(P2PStack {
            network_service: network_service_dyn,
            gossip_service: gossip_service_dyn,
            p2p_service,
        })
    }
}
