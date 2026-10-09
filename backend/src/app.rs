use std::sync::Arc;

use axum::{
    Router,
    extract::Request,
    http::{Method, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
};
use sqlx::PgPool;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

use crate::{AppError, Config, auth::google::Google, routes};

/// Shared by every request handler.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Arc<Config>,
    /// Present when Google sign-in is configured.
    pub google: Option<Arc<Google>>,
}

impl AppState {
    pub fn new(pool: PgPool, config: Config) -> anyhow::Result<Self> {
        let google = match &config.google {
            Some(google) => Some(Arc::new(Google::new(
                google.clone(),
                config.link("/api/auth/google/callback"),
            )?)),
            None => None,
        };
        Ok(Self {
            pool,
            config: Arc::new(config),
            google,
        })
    }
}

/// The whole HTTP app: the API under `/api` and, when `STATIC_DIR` is set, the
/// built frontend for every other path (unknown paths get `index.html` so the
/// client-side router can handle them).
pub fn router(state: AppState) -> Router {
    let api = routes::api().layer(middleware::from_fn(require_json_for_writes));
    let mut app = Router::new().nest("/api", api);

    if let Some(dir) = &state.config.static_dir {
        let index = ServeFile::new(dir.join("index.html"));
        app = app.fallback_service(ServeDir::new(dir).fallback(index));
    }

    app.layer(TraceLayer::new_for_http()).with_state(state)
}

/// Cross-site request forgery guard. Browsers only send a JSON content type
/// cross-site after a CORS preflight, which this API never approves, so
/// requiring it on every write means a write can only come from our own pages.
/// The session cookie is also SameSite=Lax.
async fn require_json_for_writes(request: Request, next: Next) -> Response {
    let writes = !matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS
    );
    let is_json = request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("application/json"));
    if writes && !is_json {
        return AppError::coded(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "json_required",
            "Requests that change data must send Content-Type: application/json.",
        )
        .into_response();
    }
    next.run(request).await
}
