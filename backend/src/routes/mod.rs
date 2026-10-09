use axum::{Json, Router, extract::State, routing::get};
use serde::Serialize;
use ts_rs::TS;

use crate::{AppError, AppResult, AppState};

mod auth;
mod categories;
mod dev;
mod invites;
mod orgs;
mod pages;
mod tags;
mod topics;

/// Routes mounted under `/api`.
pub fn api() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .merge(auth::routes())
        .merge(orgs::routes())
        .merge(invites::routes())
        .merge(dev::routes())
        .merge(pages::routes())
        .merge(topics::routes())
        .merge(tags::routes())
        .merge(categories::routes())
        .fallback(|| async { AppError::NotFound })
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct Health {
    pub status: String,
    pub version: String,
    pub database: String,
}

async fn health(State(state): State<AppState>) -> AppResult<Json<Health>> {
    sqlx::query("SELECT 1").execute(&state.pool).await?;
    Ok(Json(Health {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        database: "ok".to_string(),
    }))
}
