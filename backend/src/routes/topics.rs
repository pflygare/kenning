use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post, put},
};
use serde::Deserialize;

use super::pages::PagePath;
use crate::{
    AppResult, AppState, db,
    orgs::OrgContext,
    pages::{self, PageDetail},
    topics::{
        self, CreateTopicRequest, MoveTopicRequest, SetPageTopicsRequest, Topic, TopicDetail,
        UpdateTopicRequest,
    },
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/orgs/{org}/topics", get(list).post(create))
        .route("/orgs/{org}/topics/{topic}", get(show).patch(update))
        .route("/orgs/{org}/topics/{topic}/move", post(move_to))
        .route("/orgs/{org}/topics/{topic}/archive", post(archive))
        .route("/orgs/{org}/pages/{page}/topics", put(set_for_page))
}

#[derive(Deserialize)]
struct TopicPath {
    topic: String,
}

async fn list(State(state): State<AppState>, ctx: OrgContext) -> AppResult<Json<Vec<Topic>>> {
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    let topics = topics::list(&mut tx).await?;
    tx.commit().await?;
    Ok(Json(topics))
}

async fn create(
    State(state): State<AppState>,
    ctx: OrgContext,
    Json(req): Json<CreateTopicRequest>,
) -> AppResult<(StatusCode, Json<TopicDetail>)> {
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    let short_id = topics::create(&mut tx, ctx.org.id, ctx.user.id, &req).await?;
    let topic = topics::detail(&mut tx, &short_id).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(topic)))
}

async fn show(
    State(state): State<AppState>,
    ctx: OrgContext,
    Path(path): Path<TopicPath>,
) -> AppResult<Json<TopicDetail>> {
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    let topic = topics::detail(&mut tx, &path.topic).await?;
    tx.commit().await?;
    Ok(Json(topic))
}

async fn update(
    State(state): State<AppState>,
    ctx: OrgContext,
    Path(path): Path<TopicPath>,
    Json(req): Json<UpdateTopicRequest>,
) -> AppResult<Json<TopicDetail>> {
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    topics::update(&mut tx, ctx.org.id, ctx.user.id, &path.topic, &req).await?;
    let topic = topics::detail(&mut tx, &path.topic).await?;
    tx.commit().await?;
    Ok(Json(topic))
}

async fn move_to(
    State(state): State<AppState>,
    ctx: OrgContext,
    Path(path): Path<TopicPath>,
    Json(req): Json<MoveTopicRequest>,
) -> AppResult<Json<Vec<Topic>>> {
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    topics::move_to(&mut tx, ctx.org.id, ctx.user.id, &path.topic, &req).await?;
    let topics = topics::list(&mut tx).await?;
    tx.commit().await?;
    Ok(Json(topics))
}

async fn archive(
    State(state): State<AppState>,
    ctx: OrgContext,
    Path(path): Path<TopicPath>,
) -> AppResult<StatusCode> {
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    topics::archive(&mut tx, ctx.org.id, ctx.user.id, &path.topic).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn set_for_page(
    State(state): State<AppState>,
    ctx: OrgContext,
    Path(path): Path<PagePath>,
    Json(req): Json<SetPageTopicsRequest>,
) -> AppResult<Json<PageDetail>> {
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    let page_id = pages::id_of(&mut tx, &path.page).await?;
    topics::set_for_page(&mut tx, ctx.org.id, ctx.user.id, page_id, &req.topic_ids).await?;
    let page = pages::detail(&mut tx, &path.page).await?;
    tx.commit().await?;
    Ok(Json(page))
}
