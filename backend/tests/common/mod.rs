#![allow(dead_code)]

use std::path::PathBuf;

use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode, header},
};
use http_body_util::BodyExt;
use kenning_server::{AppState, Config, router};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

/// The app on a test database from `#[sqlx::test]`.
pub fn app(pool: PgPool) -> Router {
    app_with_static(pool, None)
}

pub fn app_with_static(pool: PgPool, static_dir: Option<PathBuf>) -> Router {
    let mut config = Config::for_tests();
    config.static_dir = static_dir;
    router(AppState::new(pool, config).unwrap())
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

pub async fn get_json(app: &Router, uri: &str) -> (StatusCode, Value) {
    let (status, body) = get(app, uri).await;
    (status, serde_json::from_str(&body).unwrap())
}

/// A browser: remembers its session cookie between requests.
pub struct Browser {
    pub app: Router,
    pub session: Option<String>,
}

impl Browser {
    pub fn new(pool: &PgPool) -> Self {
        Self {
            app: app(pool.clone()),
            session: None,
        }
    }

    pub async fn request(
        &mut self,
        method: Method,
        uri: &str,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(session) = &self.session {
            builder = builder.header(header::COOKIE, format!("kenning_session={session}"));
        }
        let body = match body {
            Some(json) => {
                builder = builder.header(header::CONTENT_TYPE, "application/json");
                Body::from(json.to_string())
            }
            None => Body::empty(),
        };
        let response = self
            .app
            .clone()
            .oneshot(builder.body(body).unwrap())
            .await
            .unwrap();
        for value in response.headers().get_all(header::SET_COOKIE) {
            let cookie = value.to_str().unwrap();
            if let Some(rest) = cookie.strip_prefix("kenning_session=") {
                let token = rest.split(';').next().unwrap();
                self.session = (!token.is_empty()).then(|| token.to_string());
            }
        }
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let json = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        };
        (status, json)
    }

    pub async fn get(&mut self, uri: &str) -> (StatusCode, Value) {
        self.request(Method::GET, uri, None).await
    }

    pub async fn post(&mut self, uri: &str, body: Value) -> (StatusCode, Value) {
        self.request(Method::POST, uri, Some(body)).await
    }

    pub async fn patch(&mut self, uri: &str, body: Value) -> (StatusCode, Value) {
        self.request(Method::PATCH, uri, Some(body)).await
    }

    pub async fn delete(&mut self, uri: &str) -> (StatusCode, Value) {
        self.request(Method::DELETE, uri, Some(Value::Object(Default::default())))
            .await
    }

    /// Sign up with a password and confirm the email, like a real person would.
    pub async fn sign_up_verified(&mut self, pool: &PgPool, name: &str, email: &str) {
        let (status, _) = self
            .post(
                "/api/auth/signup",
                serde_json::json!({"name": name, "email": email, "password": "correct horse"}),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "sign up {email}");
        let token = link_token(pool, email, "token=").await;
        let (status, _) = self
            .post(
                "/api/auth/verify-email",
                serde_json::json!({"token": token}),
            )
            .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
    }
}

/// Emails queued for `to`, oldest first, as (subject, body).
pub async fn emails_to(pool: &PgPool, to: &str) -> Vec<(String, String)> {
    sqlx::query_as(
        "SELECT payload->>'subject', payload->>'body' FROM jobs
         WHERE kind = 'send_email' AND payload->>'to' = $1 ORDER BY created_at, id",
    )
    .bind(to)
    .fetch_all(pool)
    .await
    .unwrap()
}

/// The token after `marker` (such as `token=` or `/invite/`) in the latest email to `to`.
pub async fn link_token(pool: &PgPool, to: &str, marker: &str) -> String {
    let emails = emails_to(pool, to).await;
    let (_, body) = emails.last().unwrap_or_else(|| panic!("no email to {to}"));
    let start = body
        .find(marker)
        .unwrap_or_else(|| panic!("no {marker} in: {body}"))
        + marker.len();
    body[start..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect()
}
