use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::Deserialize;

use crate::{
    AppResult, AppState, db,
    orgs::OrgContext,
    search::{self, QuickResults, SearchQuery, SearchResults},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/orgs/{org}/search", get(search))
        .route("/orgs/{org}/search/quick", get(quick))
}

async fn search(
    State(state): State<AppState>,
    ctx: OrgContext,
    Query(query): Query<SearchQuery>,
) -> AppResult<Json<SearchResults>> {
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    let results = search::search(&mut tx, &query).await?;
    tx.commit().await?;
    Ok(Json(results))
}

#[derive(Deserialize)]
struct QuickQuery {
    #[serde(default)]
    q: String,
}

async fn quick(
    State(state): State<AppState>,
    ctx: OrgContext,
    Query(query): Query<QuickQuery>,
) -> AppResult<Json<QuickResults>> {
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    let results = search::quick(&mut tx, &query.q).await?;
    tx.commit().await?;
    Ok(Json(results))
}
