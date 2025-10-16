#![allow(unused_imports)]
#![allow(dead_code)]

pub mod config;
pub mod error;

pub use config::AppConfig;
pub use error::{AppError, Result};
