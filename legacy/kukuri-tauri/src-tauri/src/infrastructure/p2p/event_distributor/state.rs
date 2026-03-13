use std::collections::VecDeque;

use crate::domain::entities::Event;
use crate::domain::p2p::distribution::DistributionStrategy;

pub(crate) struct DistributorState {
    strategy: DistributionStrategy,
    pending_events: VecDeque<Event>,
    failed_events: Vec<(Event, DistributionStrategy)>,
}

impl DistributorState {
    pub(crate) fn new(strategy: DistributionStrategy) -> Self {
        Self {
            strategy,
            pending_events: VecDeque::new(),
            failed_events: Vec::new(),
        }
    }

    #[cfg(test)]
    pub(crate) fn strategy(&self) -> DistributionStrategy {
        self.strategy.clone()
    }

    pub(crate) fn set_strategy(&mut self, strategy: DistributionStrategy) {
        self.strategy = strategy;
    }

    pub(crate) fn pop_pending(&mut self) -> Option<Event> {
        self.pending_events.pop_front()
    }

    pub(crate) fn pending_events_snapshot(&self) -> Vec<Event> {
        self.pending_events.iter().cloned().collect()
    }

    pub(crate) fn record_failure(&mut self, event: Event, strategy: DistributionStrategy) {
        self.failed_events.push((event, strategy));
    }

    pub(crate) fn drain_failures(&mut self) -> Vec<(Event, DistributionStrategy)> {
        std::mem::take(&mut self.failed_events)
    }

    pub(crate) fn restore_failures(&mut self, still_failed: Vec<(Event, DistributionStrategy)>) {
        self.failed_events = still_failed;
    }
}
