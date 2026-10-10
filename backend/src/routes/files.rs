use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Path, Query, State, rejection::BytesRejection},
    http::{StatusCode, header},
    response::IntoResponse,
    routing::{get, post},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    AppResult, AppState,
    auth::CurrentUser,
    db,
    files::{self, UploadedFile},
    orgs::OrgContext,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/orgs/{org}/files",
            // A little over the limit, so files::upload can say how big is too big.
            post(upload).layer(DefaultBodyLimit::max(files::MAX_SIZE + 1)),
        )
        .route("/files/{id}", get(download))
}

#[derive(Deserialize)]
struct UploadQuery {
    name: Option<String>,
}

/// The image is the raw request body; its name comes in `?name=`.
async fn upload(
    State(state): State<AppState>,
    ctx: OrgContext,
    Query(query): Query<UploadQuery>,
    body: Result<Bytes, BytesRejection>,
) -> AppResult<(StatusCode, Json<UploadedFile>)> {
    let body = body.map_err(|_| files::too_large())?;
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    let file = files::upload(
        &mut tx,
        ctx.org.id,
        ctx.user.id,
        query.name.as_deref(),
        &body,
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(file)))
}

#[derive(Deserialize)]
struct FilePath {
    id: Uuid,
}

async fn download(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(path): Path<FilePath>,
) -> AppResult<impl IntoResponse> {
    let mut conn = state.pool.acquire().await?;
    let file = files::get_for_member(&mut conn, path.id, user.id).await?;
    Ok((
        [
            (header::CONTENT_TYPE, file.content_type),
            (
                header::CONTENT_DISPOSITION,
                format!(
                    "inline; filename=\"{}\"",
                    file.name.replace(|c: char| !c.is_ascii(), "_")
                ),
            ),
            // Files never change; only members may see them.
            (
                header::CACHE_CONTROL,
                "private, max-age=31536000, immutable".to_string(),
            ),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff".to_string()),
            (
                header::CONTENT_SECURITY_POLICY,
                "default-src 'none'; sandbox".to_string(),
            ),
        ],
        file.data,
    ))
}
