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
    AppResult, AppState,
    categories::{
        self, Category, CreateCategoryRequest, CreateValueRequest, SetPageCategoryRequest,
        UpdateCategoryRequest, UpdateValueRequest,
    },
    db,
    orgs::OrgContext,
    pages::{self, PageDetail},
};

const MANAGE: &str = "manage categories";

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/orgs/{org}/categories", get(list).post(create))
        .route(
            "/orgs/{org}/categories/{category}",
            patch(update).delete(delete),
        )
        .route("/orgs/{org}/categories/{category}/values", post(add_value))
        .route(
            "/orgs/{org}/categories/{category}/values/{value}",
            patch(update_value).delete(delete_value),
        )
        .route("/orgs/{org}/pages/{page}/categories", put(set_for_page))
}

#[derive(Deserialize)]
struct CategoryPath {
    category: Uuid,
}

#[derive(Deserialize)]
struct ValuePath {
    category: Uuid,
    value: Uuid,
}

async fn list(State(state): State<AppState>, ctx: OrgContext) -> AppResult<Json<Vec<Category>>> {
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    let list = categories::list(&mut tx).await?;
    tx.commit().await?;
    Ok(Json(list))
}

async fn create(
    State(state): State<AppState>,
    ctx: OrgContext,
    Json(req): Json<CreateCategoryRequest>,
) -> AppResult<(StatusCode, Json<Vec<Category>>)> {
    ctx.require_admin(MANAGE)?;
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    categories::create(&mut tx, ctx.org.id, ctx.user.id, &req).await?;
    let list = categories::list(&mut tx).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(list)))
}

async fn update(
    State(state): State<AppState>,
    ctx: OrgContext,
    Path(path): Path<CategoryPath>,
    Json(req): Json<UpdateCategoryRequest>,
) -> AppResult<Json<Vec<Category>>> {
    ctx.require_admin(MANAGE)?;
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    categories::update(&mut tx, ctx.org.id, ctx.user.id, path.category, &req).await?;
    let list = categories::list(&mut tx).await?;
    tx.commit().await?;
    Ok(Json(list))
}

async fn delete(
    State(state): State<AppState>,
    ctx: OrgContext,
    Path(path): Path<CategoryPath>,
) -> AppResult<Json<Vec<Category>>> {
    ctx.require_admin(MANAGE)?;
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    categories::delete(&mut tx, ctx.org.id, ctx.user.id, path.category).await?;
    let list = categories::list(&mut tx).await?;
    tx.commit().await?;
    Ok(Json(list))
}

async fn add_value(
    State(state): State<AppState>,
    ctx: OrgContext,
    Path(path): Path<CategoryPath>,
    Json(req): Json<CreateValueRequest>,
) -> AppResult<(StatusCode, Json<Vec<Category>>)> {
    ctx.require_admin(MANAGE)?;
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    categories::add_value(&mut tx, ctx.org.id, ctx.user.id, path.category, &req).await?;
    let list = categories::list(&mut tx).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(list)))
}

async fn update_value(
    State(state): State<AppState>,
    ctx: OrgContext,
    Path(path): Path<ValuePath>,
    Json(req): Json<UpdateValueRequest>,
) -> AppResult<Json<Vec<Category>>> {
    ctx.require_admin(MANAGE)?;
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    categories::update_value(
        &mut tx,
        ctx.org.id,
        ctx.user.id,
        path.category,
        path.value,
        &req,
    )
    .await?;
    let list = categories::list(&mut tx).await?;
    tx.commit().await?;
    Ok(Json(list))
}

async fn delete_value(
    State(state): State<AppState>,
    ctx: OrgContext,
    Path(path): Path<ValuePath>,
) -> AppResult<Json<Vec<Category>>> {
    ctx.require_admin(MANAGE)?;
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    categories::delete_value(&mut tx, ctx.org.id, ctx.user.id, path.category, path.value).await?;
    let list = categories::list(&mut tx).await?;
    tx.commit().await?;
    Ok(Json(list))
}

async fn set_for_page(
    State(state): State<AppState>,
    ctx: OrgContext,
    Path(path): Path<PagePath>,
    Json(req): Json<SetPageCategoryRequest>,
) -> AppResult<Json<PageDetail>> {
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    let page_id = pages::id_of(&mut tx, &path.page).await?;
    categories::set_for_page(&mut tx, ctx.org.id, ctx.user.id, page_id, &req).await?;
    let page = pages::detail(&mut tx, &path.page).await?;
    tx.commit().await?;
    Ok(Json(page))
}
