//! The testing page's API, for trying Kenning without a mail server. Every
//! route answers 404 unless `DEV_TOOLS` is on.

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use ts_rs::TS;

use crate::{
    AppError, AppResult, AppState, accounts,
    audit::{self, Event},
    auth::CurrentUser,
    mail,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/dev/emails", get(emails))
        .route("/dev/confirm-email", post(confirm_email))
}

fn require_dev_tools(state: &AppState) -> AppResult<()> {
    if state.config.dev_tools {
        Ok(())
    } else {
        Err(AppError::NotFound)
    }
}

/// An email the app sent (or would have sent), as the outbox shows it.
#[derive(Serialize, sqlx::FromRow, TS)]
#[ts(export)]
pub struct OutboxEmail {
    pub to: String,
    pub subject: String,
    pub body: String,
    pub sent_at: DateTime<Utc>,
}

/// The latest 50 emails, newest first.
async fn emails(State(state): State<AppState>) -> AppResult<Json<Vec<OutboxEmail>>> {
    require_dev_tools(&state)?;
    let emails = sqlx::query_as(
        "SELECT payload->>'to' AS to, payload->>'subject' AS subject,
                payload->>'body' AS body, created_at AS sent_at
         FROM jobs WHERE kind = $1
         ORDER BY created_at DESC, id DESC LIMIT 50",
    )
    .bind(mail::JOB_KIND)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(emails))
}

/// Confirm the signed-in user's email as if they had opened the link.
async fn confirm_email(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<StatusCode> {
    require_dev_tools(&state)?;
    let mut tx = state.pool.begin().await?;
    accounts::mark_verified(&mut tx, user.id).await?;
    audit::record(
        &mut tx,
        Event::new("user.email_verified")
            .actor(user.id)
            .object("user", user.id),
    )
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
