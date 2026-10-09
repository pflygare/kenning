mod common;

use axum::http::StatusCode;
use common::Browser;
use serde_json::json;
use sqlx::PgPool;

const MIGRATOR: &sqlx::migrate::Migrator = &kenning_server::db::MIGRATOR;

async fn sign_up(browser: &mut Browser) {
    let (status, _) = browser
        .post(
            "/api/auth/signup",
            json!({"name": "Ada", "email": "ada@example.com", "password": "correct horse"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn testing_page_is_off_by_default(pool: PgPool) {
    let mut browser = Browser::new(&pool);
    sign_up(&mut browser).await;
    let (_, config) = browser.get("/api/auth/config").await;
    assert_eq!(config["dev_tools"], false);
    assert_eq!(
        browser.get("/api/dev/emails").await.0,
        StatusCode::NOT_FOUND
    );
    let (status, _) = browser.post("/api/dev/confirm-email", json!({})).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn outbox_lists_sent_emails_newest_first(pool: PgPool) {
    let mut browser = Browser::with_dev_tools(&pool);
    let (_, config) = browser.get("/api/auth/config").await;
    assert_eq!(config["dev_tools"], true);
    sign_up(&mut browser).await;
    browser
        .post(
            "/api/auth/forgot-password",
            json!({"email": "ada@example.com"}),
        )
        .await;

    // Readable without signing in, so an invitee can find their link.
    let (status, emails) = Browser::with_dev_tools(&pool).get("/api/dev/emails").await;
    assert_eq!(status, StatusCode::OK);
    let emails = emails.as_array().unwrap();
    assert_eq!(emails.len(), 2);
    assert_eq!(emails[0]["to"], "ada@example.com");
    assert!(
        emails[0]["body"]
            .as_str()
            .unwrap()
            .contains("/reset-password?token=")
    );
    assert!(
        emails[1]["body"]
            .as_str()
            .unwrap()
            .contains("/verify-email?token=")
    );
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn confirm_email_shortcut_verifies_the_signed_in_user(pool: PgPool) {
    let mut browser = Browser::with_dev_tools(&pool);
    let (status, _) = browser.post("/api/dev/confirm-email", json!({})).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    sign_up(&mut browser).await;
    let (status, _) = browser.post("/api/dev/confirm-email", json!({})).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, me) = browser.get("/api/auth/me").await;
    assert_eq!(me["user"]["email_verified"], true);
    let (status, _) = browser.post("/api/orgs", json!({"name": "Acme"})).await;
    assert_eq!(status, StatusCode::CREATED);
}
