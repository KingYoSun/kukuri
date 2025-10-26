use crate::domain::entities::Event;
use crate::domain::p2p::DistributionStrategy;
use crate::infrastructure::p2p::EventDistributor;
use crate::shared::error::AppError;
use std::sync::Arc;

pub(crate) async fn distribute_hybrid(
    distributor: &Arc<dyn EventDistributor>,
    event: &Event,
) -> Result<(), AppError> {
    distributor
        .distribute(event, DistributionStrategy::Hybrid)
        .await?;
    Ok(())
}
