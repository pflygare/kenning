use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, patch, post, put},
};
use serde::Deserialize;
use uuid::Uuid;

use super::pages::PagePath;
use crate::{
    AppResult, AppState, db,
    orgs::OrgContext,
    pages::{self, PageDetail},
    tags::{
        self, CreateTagRequest, MergeTagRequest, SetPageTagsRequest, Tag, TagRef, UpdateTagRequest,
    },
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/orgs/{org}/tags", get(list).post(create))
        .route("/orgs/{org}/tags/{tag}", patch(update).delete(delete))
        .route("/orgs/{org}/tags/{tag}/merge", post(merge))
        .route("/orgs/{org}/pages/{page}/tags", put(set_for_page))
}

#[derive(Deserialize)]
struct TagPath {
    tag: Uuid,
}

async fn list(State(state): State<AppState>, ctx: OrgContext) -> AppResult<Json<Vec<Tag>>> {
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    let tags = tags::list(&mut tx).await?;
    tx.commit().await?;
    Ok(Json(tags))
}

async fn create(
    State(state): State<AppState>,
    ctx: OrgContext,
    Json(req): Json<CreateTagRequest>,
) -> AppResult<(StatusCode, Json<TagRef>)> {
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    let tag = tags::create(&mut tx, ctx.org.id, ctx.user.id, &req).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(tag)))
}

async fn update(
    State(state): State<AppState>,
    ctx: OrgContext,
    Path(path): Path<TagPath>,
    Json(req): Json<UpdateTagRequest>,
) -> AppResult<Json<TagRef>> {
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    let tag = tags::update(&mut tx, ctx.org.id, ctx.user.id, path.tag, &req).await?;
    tx.commit().await?;
    Ok(Json(tag))
}

async fn delete(
    State(state): State<AppState>,
    ctx: OrgContext,
    Path(path): Path<TagPath>,
) -> AppResult<StatusCode> {
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    tags::delete(&mut tx, ctx.org.id, ctx.user.id, path.tag).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn merge(
    State(state): State<AppState>,
    ctx: OrgContext,
    Path(path): Path<TagPath>,
    Json(req): Json<MergeTagRequest>,
) -> AppResult<StatusCode> {
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    tags::merge(&mut tx, ctx.org.id, ctx.user.id, path.tag, req.into_id).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn set_for_page(
    State(state): State<AppState>,
    ctx: OrgContext,
    Path(path): Path<PagePath>,
    Json(req): Json<SetPageTagsRequest>,
) -> AppResult<Json<PageDetail>> {
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    let page_id = pages::id_of(&mut tx, &path.page).await?;
    tags::set_for_page(&mut tx, ctx.org.id, ctx.user.id, page_id, &req.names).await?;
    let page = pages::detail(&mut tx, &path.page).await?;
    tx.commit().await?;
    Ok(Json(page))
}
