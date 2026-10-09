use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};

use crate::{
    AppResult, AppState,
    auth::CurrentUser,
    invites::{self, InvitePreview},
    orgs::Membership,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/invites/{token}", get(preview))
        .route("/invites/{token}/accept", post(accept))
}

async fn preview(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> AppResult<Json<InvitePreview>> {
    let mut conn = state.pool.acquire().await?;
    Ok(Json(invites::preview(&mut conn, &token).await?))
}

async fn accept(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(token): Path<String>,
) -> AppResult<Json<Membership>> {
    let mut tx = state.pool.begin().await?;
    let membership = invites::accept(&mut tx, &token, &user).await?;
    tx.commit().await?;
    Ok(Json(membership))
}
