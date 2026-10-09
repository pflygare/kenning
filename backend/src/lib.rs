//! Kenning server: the HTTP API and, in production, the built frontend.

pub mod accounts;
pub mod app;
pub mod audit;
pub mod auth;
pub mod config;
pub mod db;
pub mod error;
pub mod invites;
pub mod jobs;
pub mod mail;
pub mod orgs;
pub mod pages;
pub mod routes;
pub mod tags;
pub mod tokens;
pub mod topics;

pub use app::{AppState, router};
pub use config::Config;
pub use error::{AppError, AppResult};
