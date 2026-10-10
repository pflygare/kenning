use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post, put},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    AppResult, AppState, db,
    orgs::OrgContext,
    tags::SetPageTagsRequest,
    templates::{
        self, CreateTemplateRequest, TemplateDetail, TemplateSummary, UpdateTemplateRequest,
    },
    topics::SetPageTopicsRequest,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/orgs/{org}/templates", get(list).post(create))
        .route("/orgs/{org}/templates/{template}", get(show).put(update))
        .route("/orgs/{org}/templates/{template}/topics", put(set_topics))
        .route("/orgs/{org}/templates/{template}/tags", put(set_tags))
        .route("/orgs/{org}/templates/{template}/archive", post(archive))
}

#[derive(Deserialize)]
struct TemplatePath {
    template: Uuid,
}

async fn list(
    State(state): State<AppState>,
    ctx: OrgContext,
) -> AppResult<Json<Vec<TemplateSummary>>> {
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    let list = templates::list(&mut tx).await?;
    tx.commit().await?;
    Ok(Json(list))
}

async fn create(
    State(state): State<AppState>,
    ctx: OrgContext,
    Json(req): Json<CreateTemplateRequest>,
) -> AppResult<(StatusCode, Json<TemplateDetail>)> {
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    let id = templates::create(&mut tx, ctx.org.id, ctx.user.id, &req).await?;
    let template = templates::detail(&mut tx, id).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(template)))
}

async fn show(
    State(state): State<AppState>,
    ctx: OrgContext,
    Path(path): Path<TemplatePath>,
) -> AppResult<Json<TemplateDetail>> {
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    let template = templates::detail(&mut tx, path.template).await?;
    tx.commit().await?;
    Ok(Json(template))
}

async fn update(
    State(state): State<AppState>,
    ctx: OrgContext,
    Path(path): Path<TemplatePath>,
    Json(req): Json<UpdateTemplateRequest>,
) -> AppResult<Json<TemplateDetail>> {
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    templates::update(&mut tx, ctx.org.id, ctx.user.id, path.template, &req).await?;
    let template = templates::detail(&mut tx, path.template).await?;
    tx.commit().await?;
    Ok(Json(template))
}

async fn set_topics(
    State(state): State<AppState>,
    ctx: OrgContext,
    Path(path): Path<TemplatePath>,
    Json(req): Json<SetPageTopicsRequest>,
) -> AppResult<Json<TemplateDetail>> {
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    templates::set_topics(&mut tx, ctx.org.id, path.template, &req.topic_ids).await?;
    let template = templates::detail(&mut tx, path.template).await?;
    tx.commit().await?;
    Ok(Json(template))
}

async fn set_tags(
    State(state): State<AppState>,
    ctx: OrgContext,
    Path(path): Path<TemplatePath>,
    Json(req): Json<SetPageTagsRequest>,
) -> AppResult<Json<TemplateDetail>> {
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    templates::set_tags(&mut tx, ctx.org.id, ctx.user.id, path.template, &req.names).await?;
    let template = templates::detail(&mut tx, path.template).await?;
    tx.commit().await?;
    Ok(Json(template))
}

async fn archive(
    State(state): State<AppState>,
    ctx: OrgContext,
    Path(path): Path<TemplatePath>,
) -> AppResult<StatusCode> {
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    templates::archive(&mut tx, ctx.org.id, ctx.user.id, path.template).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
