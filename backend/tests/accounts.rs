mod common;

use axum::http::StatusCode;
use common::{Browser, emails_to, link_token};
use kenning_server::accounts;
use serde_json::json;
use sqlx::PgPool;

const MIGRATOR: &sqlx::migrate::Migrator = &kenning_server::db::MIGRATOR;

#[sqlx::test(migrator = "MIGRATOR")]
async fn sign_up_signs_in_and_sends_confirmation(pool: PgPool) {
    let mut browser = Browser::new(&pool);
    let (status, me) = browser
        .post(
            "/api/auth/signup",
            json!({"name": "Ada", "email": " Ada@Example.com ", "password": "correct horse"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["user"]["email"], "ada@example.com");
    assert_eq!(me["user"]["email_verified"], false);
    assert_eq!(me["orgs"], json!([]));

    let (status, me) = browser.get("/api/auth/me").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["user"]["name"], "Ada");

    let emails = emails_to(&pool, "ada@example.com").await;
    assert_eq!(emails.len(), 1);
    assert!(emails[0].0.contains("Confirm your email"));
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn confirming_email_marks_it_verified_once(pool: PgPool) {
    let mut browser = Browser::new(&pool);
    browser
        .post(
            "/api/auth/signup",
            json!({"name": "Ada", "email": "ada@example.com", "password": "correct horse"}),
        )
        .await;
    let token = link_token(&pool, "ada@example.com", "token=").await;

    let (status, _) = browser
        .post("/api/auth/verify-email", json!({"token": token}))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, me) = browser.get("/api/auth/me").await;
    assert_eq!(me["user"]["email_verified"], true);

    let (status, body) = browser
        .post("/api/auth/verify-email", json!({"token": token}))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "invalid_token");
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn duplicate_and_invalid_sign_ups_are_rejected(pool: PgPool) {
    let mut browser = Browser::new(&pool);
    browser
        .post(
            "/api/auth/signup",
            json!({"name": "Ada", "email": "ada@example.com", "password": "correct horse"}),
        )
        .await;

    let mut other = Browser::new(&pool);
    let (status, body) = other
        .post(
            "/api/auth/signup",
            json!({"name": "Imposter", "email": "ADA@example.com", "password": "correct horse"}),
        )
        .await;
    assert_eq!(
        (status, body["error"]["code"].as_str()),
        (StatusCode::CONFLICT, Some("email_taken"))
    );

    let (status, body) = other
        .post(
            "/api/auth/signup",
            json!({"name": "Bob", "email": "bob@example.com", "password": "short"}),
        )
        .await;
    assert_eq!(
        (status, body["error"]["code"].as_str()),
        (StatusCode::BAD_REQUEST, Some("weak_password"))
    );

    let (status, body) = other
        .post(
            "/api/auth/signup",
            json!({"name": "Bob", "email": "not-an-email", "password": "correct horse"}),
        )
        .await;
    assert_eq!(
        (status, body["error"]["code"].as_str()),
        (StatusCode::BAD_REQUEST, Some("invalid_email"))
    );
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn log_in_and_out(pool: PgPool) {
    let mut browser = Browser::new(&pool);
    browser
        .post(
            "/api/auth/signup",
            json!({"name": "Ada", "email": "ada@example.com", "password": "correct horse"}),
        )
        .await;
    let (status, _) = browser.post("/api/auth/logout", json!({})).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(
        browser.get("/api/auth/me").await.0,
        StatusCode::UNAUTHORIZED
    );

    let (status, body) = browser
        .post(
            "/api/auth/login",
            json!({"email": "ada@example.com", "password": "wrong password"}),
        )
        .await;
    assert_eq!(
        (status, body["error"]["code"].as_str()),
        (StatusCode::UNAUTHORIZED, Some("invalid_credentials"))
    );
    let (status, body) = browser
        .post(
            "/api/auth/login",
            json!({"email": "nobody@example.com", "password": "correct horse"}),
        )
        .await;
    assert_eq!(
        (status, body["error"]["code"].as_str()),
        (StatusCode::UNAUTHORIZED, Some("invalid_credentials"))
    );

    let (status, me) = browser
        .post(
            "/api/auth/login",
            json!({"email": "ADA@example.com", "password": "correct horse"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["user"]["email"], "ada@example.com");
    assert_eq!(browser.get("/api/auth/me").await.0, StatusCode::OK);
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn logged_out_session_cookie_stops_working(pool: PgPool) {
    let mut browser = Browser::new(&pool);
    browser
        .post(
            "/api/auth/signup",
            json!({"name": "Ada", "email": "ada@example.com", "password": "correct horse"}),
        )
        .await;
    let stolen = browser.session.clone();
    browser.post("/api/auth/logout", json!({})).await;

    let mut thief = Browser::new(&pool);
    thief.session = stolen;
    assert_eq!(thief.get("/api/auth/me").await.0, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn password_reset_replaces_password_and_signs_out_elsewhere(pool: PgPool) {
    let mut laptop = Browser::new(&pool);
    laptop
        .post(
            "/api/auth/signup",
            json!({"name": "Ada", "email": "ada@example.com", "password": "correct horse"}),
        )
        .await;

    let mut phone = Browser::new(&pool);
    // Unknown addresses get the same answer, and no email.
    assert_eq!(
        phone
            .post(
                "/api/auth/forgot-password",
                json!({"email": "nobody@example.com"})
            )
            .await
            .0,
        StatusCode::NO_CONTENT
    );
    assert!(emails_to(&pool, "nobody@example.com").await.is_empty());

    assert_eq!(
        phone
            .post(
                "/api/auth/forgot-password",
                json!({"email": "ada@example.com"})
            )
            .await
            .0,
        StatusCode::NO_CONTENT
    );
    let token = link_token(&pool, "ada@example.com", "token=").await;
    let (status, me) = phone
        .post(
            "/api/auth/reset-password",
            json!({"token": token, "password": "battery staple"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["user"]["email_verified"], true);

    assert_eq!(
        laptop.get("/api/auth/me").await.0,
        StatusCode::UNAUTHORIZED,
        "old sessions end"
    );
    assert_eq!(phone.get("/api/auth/me").await.0, StatusCode::OK);

    let mut again = Browser::new(&pool);
    assert_eq!(
        again
            .post(
                "/api/auth/login",
                json!({"email": "ada@example.com", "password": "correct horse"})
            )
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        again
            .post(
                "/api/auth/login",
                json!({"email": "ada@example.com", "password": "battery staple"})
            )
            .await
            .0,
        StatusCode::OK
    );

    let (status, _) = phone
        .post(
            "/api/auth/reset-password",
            json!({"token": token, "password": "another one"}),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "reset links work once");
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn external_sign_in_links_to_existing_account(pool: PgPool) {
    let mut browser = Browser::new(&pool);
    browser
        .post(
            "/api/auth/signup",
            json!({"name": "Ada", "email": "ada@example.com", "password": "correct horse"}),
        )
        .await;

    let mut conn = pool.acquire().await.unwrap();
    let user = accounts::sign_in_external(
        &mut conn,
        "google",
        "google-123",
        "ada@example.com",
        "Ada L",
        None,
    )
    .await
    .unwrap();
    assert!(user.email_verified, "Google vouches for the address");

    let again = accounts::sign_in_external(
        &mut conn,
        "google",
        "google-123",
        "ada@example.com",
        "Ada L",
        None,
    )
    .await
    .unwrap();
    assert_eq!(again.id, user.id);

    let fresh = accounts::sign_in_external(
        &mut conn,
        "google",
        "google-456",
        "grace@example.com",
        "Grace",
        Some("https://example.com/g.png"),
    )
    .await
    .unwrap();
    assert_ne!(fresh.id, user.id);
    assert_eq!(
        fresh.avatar_url.as_deref(),
        Some("https://example.com/g.png")
    );
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn writes_without_json_are_refused(pool: PgPool) {
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    let app = common::app(pool);
    let response = app
        .oneshot(
            Request::post("/api/auth/logout")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn google_is_hidden_when_not_configured(pool: PgPool) {
    let mut browser = Browser::new(&pool);
    assert_eq!(
        browser.get("/api/auth/config").await.1,
        json!({"google": false, "dev_tools": false})
    );
    assert_eq!(
        browser.get("/api/auth/google/start").await.0,
        StatusCode::NOT_FOUND
    );
}
