use std::sync::Arc;

use axum::Router;
use sqlx::PgPool;
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

use crate::{Config, routes};

/// Shared by every request handler.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Arc<Config>,
}

/// The whole HTTP app: the API under `/api` and, when `STATIC_DIR` is set, the
/// built frontend for every other path (unknown paths get `index.html` so the
/// client-side router can handle them).
pub fn router(state: AppState) -> Router {
    let mut app = Router::new().nest("/api", routes::api());

    if let Some(dir) = &state.config.static_dir {
        let index = ServeFile::new(dir.join("index.html"));
        app = app.fallback_service(ServeDir::new(dir).fallback(index));
    }

    app.layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
