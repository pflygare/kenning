#![allow(dead_code)]

use std::{path::PathBuf, sync::Arc};

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use kenning_server::{AppState, Config, router};
use sqlx::PgPool;
use tower::ServiceExt;

/// The app on a test database from `#[sqlx::test]`.
pub fn app(pool: PgPool) -> Router {
    app_with_static(pool, None)
}

pub fn app_with_static(pool: PgPool, static_dir: Option<PathBuf>) -> Router {
    let config = Config {
        database_url: String::new(),
        database_max_connections: 5,
        bind_addr: [127, 0, 0, 1].into(),
        port: 0,
        static_dir,
    };
    router(AppState {
        pool,
        config: Arc::new(config),
    })
}

/// Send a GET and return the status and body text.
pub async fn get(app: &Router, uri: &str) -> (StatusCode, String) {
    let response = app
        .clone()
        .oneshot(Request::get(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

pub async fn get_json(app: &Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let (status, body) = get(app, uri).await;
    (status, serde_json::from_str(&body).unwrap())
}
