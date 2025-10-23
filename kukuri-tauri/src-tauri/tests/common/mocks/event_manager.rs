use std::sync::Arc;

use kukuri_lib::modules::event::manager::EventManager;

/// Provides a lightweight EventManager instance for unit tests.
pub fn event_manager_stub() -> Arc<EventManager> {
    Arc::new(EventManager::default())
}
