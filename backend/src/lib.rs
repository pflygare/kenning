//! Kenning server: the HTTP API and, in production, the built frontend.

pub mod app;
pub mod audit;
pub mod config;
pub mod db;
pub mod error;
pub mod jobs;
pub mod routes;

pub use app::{AppState, router};
pub use config::Config;
pub use error::{AppError, AppResult};
