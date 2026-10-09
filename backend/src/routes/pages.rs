use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post, put},
};
use serde::Deserialize;

use crate::{
    AppResult, AppState, db,
    orgs::OrgContext,
    pages::{
        self, CreatePageRequest, ListFilter, PageDetail, PageSummary, PublishRequest,
        SaveDraftRequest, SavedDraft,
    },
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/orgs/{org}/pages", get(list).post(create))
        .route("/orgs/{org}/pages/{page}", get(show))
        .route(
            "/orgs/{org}/pages/{page}/draft",
            put(save_draft).delete(discard_draft),
        )
        .route("/orgs/{org}/pages/{page}/publish", post(publish))
        .route("/orgs/{org}/pages/{page}/archive", post(archive))
}

#[derive(Deserialize)]
pub(super) struct PagePath {
    pub page: String,
}

async fn list(
    State(state): State<AppState>,
    ctx: OrgContext,
    Query(filter): Query<ListFilter>,
) -> AppResult<Json<Vec<PageSummary>>> {
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    let pages = pages::list(&mut tx, &filter).await?;
    tx.commit().await?;
    Ok(Json(pages))
}

async fn create(
    State(state): State<AppState>,
    ctx: OrgContext,
    Json(req): Json<CreatePageRequest>,
) -> AppResult<(StatusCode, Json<PageDetail>)> {
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    let short_id = pages::create(&mut tx, ctx.org.id, ctx.user.id, &req).await?;
    let page = pages::detail(&mut tx, &short_id).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(page)))
}

async fn show(
    State(state): State<AppState>,
    ctx: OrgContext,
    Path(path): Path<PagePath>,
) -> AppResult<Json<PageDetail>> {
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    let page = pages::detail(&mut tx, &path.page).await?;
    tx.commit().await?;
    Ok(Json(page))
}

async fn save_draft(
    State(state): State<AppState>,
    ctx: OrgContext,
    Path(path): Path<PagePath>,
    Json(req): Json<SaveDraftRequest>,
) -> AppResult<Json<SavedDraft>> {
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    let saved = pages::save_draft(&mut tx, ctx.org.id, ctx.user.id, &path.page, &req).await?;
    tx.commit().await?;
    Ok(Json(saved))
}

async fn discard_draft(
    State(state): State<AppState>,
    ctx: OrgContext,
    Path(path): Path<PagePath>,
) -> AppResult<Json<PageDetail>> {
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    pages::discard_draft(&mut tx, ctx.org.id, ctx.user.id, &path.page).await?;
    let page = pages::detail(&mut tx, &path.page).await?;
    tx.commit().await?;
    Ok(Json(page))
}

async fn publish(
    State(state): State<AppState>,
    ctx: OrgContext,
    Path(path): Path<PagePath>,
    Json(req): Json<PublishRequest>,
) -> AppResult<Json<PageDetail>> {
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    pages::publish(
        &mut tx,
        ctx.org.id,
        ctx.user.id,
        &path.page,
        req.revision_id,
    )
    .await?;
    let page = pages::detail(&mut tx, &path.page).await?;
    tx.commit().await?;
    Ok(Json(page))
}

async fn archive(
    State(state): State<AppState>,
    ctx: OrgContext,
    Path(path): Path<PagePath>,
) -> AppResult<StatusCode> {
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    pages::archive(&mut tx, ctx.org.id, ctx.user.id, &path.page).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
