use crate::domain::entities::{Event, EventKind};
use crate::infrastructure::database::EventRepository;
use crate::infrastructure::crypto::SignatureService;
use crate::infrastructure::p2p::{EventDistributor, DistributionStrategy};
use std::sync::Arc;

pub struct EventService {
    repository: Arc<dyn EventRepository>,
    signature_service: Arc<dyn SignatureService>,
    distributor: Arc<dyn EventDistributor>,
}

impl EventService {
    pub fn new(
        repository: Arc<dyn EventRepository>,
        signature_service: Arc<dyn SignatureService>,
        distributor: Arc<dyn EventDistributor>,
    ) -> Self {
        Self {
            repository,
            signature_service,
            distributor,
        }
    }

    pub async fn create_event(&self, kind: u32, content: String, pubkey: String, private_key: &str) -> Result<Event, Box<dyn std::error::Error>> {
        let mut event = Event::new(kind, content, pubkey);
        
        // Sign the event
        self.signature_service.sign_event(&mut event, private_key).await?;
        
        // Save to database
        self.repository.create_event(&event).await?;
        
        // Distribute
        self.distributor.distribute(&event, DistributionStrategy::Hybrid).await?;
        
        Ok(event)
    }

    pub async fn process_received_event(&self, event: Event) -> Result<(), Box<dyn std::error::Error>> {
        // Verify signature
        if !self.signature_service.verify_event(&event).await? {
            return Err("Invalid event signature".into());
        }
        
        // Save to database
        self.repository.create_event(&event).await?;
        
        // Process based on event kind
        match EventKind::from_u32(event.kind) {
            Some(EventKind::TextNote) => {
                // TODO: Convert to Post and save
            }
            Some(EventKind::Metadata) => {
                // TODO: Update user metadata
            }
            Some(EventKind::Reaction) => {
                // TODO: Process reaction
            }
            Some(EventKind::Repost) => {
                // TODO: Process repost
            }
            _ => {
                // Unknown or unhandled event kind
            }
        }
        
        Ok(())
    }

    pub async fn get_event(&self, id: &str) -> Result<Option<Event>, Box<dyn std::error::Error>> {
        self.repository.get_event(id).await
    }

    pub async fn get_events_by_kind(&self, kind: u32, limit: usize) -> Result<Vec<Event>, Box<dyn std::error::Error>> {
        self.repository.get_events_by_kind(kind, limit).await
    }

    pub async fn get_events_by_author(&self, pubkey: &str, limit: usize) -> Result<Vec<Event>, Box<dyn std::error::Error>> {
        self.repository.get_events_by_author(pubkey, limit).await
    }

    pub async fn delete_event(&self, id: &str, pubkey: String, private_key: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Create deletion event (Kind 5)
        let mut deletion_event = Event::new(EventKind::EventDeletion.as_u32(), "", pubkey);
        deletion_event.add_e_tag(id.to_string());
        
        self.signature_service.sign_event(&mut deletion_event, private_key).await?;
        self.repository.create_event(&deletion_event).await?;
        self.distributor.distribute(&deletion_event, DistributionStrategy::Hybrid).await?;
        
        // Mark original event as deleted in database
        self.repository.delete_event(id).await
    }

    pub async fn sync_pending_events(&self) -> Result<u32, Box<dyn std::error::Error>> {
        let unsync_events = self.repository.get_unsync_events().await?;
        let mut synced_count = 0;
        
        for event in unsync_events {
            self.distributor.distribute(&event, DistributionStrategy::Hybrid).await?;
            self.repository.mark_event_synced(&event.id).await?;
            synced_count += 1;
        }
        
        Ok(synced_count)
    }
}