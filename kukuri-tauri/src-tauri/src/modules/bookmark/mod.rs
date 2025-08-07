pub mod manager;
pub mod types;

#[cfg(test)]
mod tests;

pub use manager::BookmarkManager;
// pub use types::{CreateBookmarkRequest}; // Currently unused