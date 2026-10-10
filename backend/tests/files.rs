mod common;

use axum::http::{Method, StatusCode};
use common::Browser;
use serde_json::{Value, json};
use sqlx::PgPool;

const MIGRATOR: &sqlx::migrate::Migrator = &kenning_server::db::MIGRATOR;

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR not really the rest of a png";

async fn owner(pool: &PgPool, name: &str, email: &str, org: &str) -> Browser {
    let mut browser = Browser::new(pool);
    browser.sign_up_verified(pool, name, email).await;
    let (status, _) = browser
        .post("/api/orgs", json!({"name": org, "slug": org}))
        .await;
    assert_eq!(status, StatusCode::CREATED);
    browser
}

async fn upload(browser: &mut Browser, org: &str, name: &str, data: &[u8]) -> (StatusCode, Value) {
    let (status, _, body) = browser
        .raw(
            Method::POST,
            &format!("/api/orgs/{org}/files?name={name}"),
            &[
                ("content-type", "image/png"),
                ("x-requested-with", "kenning"),
            ],
            data.to_vec(),
        )
        .await;
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn members_upload_and_see_images(pool: PgPool) {
    let mut alice = owner(&pool, "Alice", "alice@example.com", "acme").await;

    let (status, file) = upload(&mut alice, "acme", "diagram.png", PNG).await;
    assert_eq!(status, StatusCode::CREATED, "{file}");
    assert_eq!(file["name"], "diagram.png");
    assert_eq!(file["content_type"], "image/png");
    let url = file["url"].as_str().unwrap().to_string();
    assert!(url.starts_with("/api/files/"));

    let (status, headers, body) = alice.raw(Method::GET, &url, &[], vec![]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, PNG);
    assert_eq!(headers["content-type"], "image/png");
    assert_eq!(headers["x-content-type-options"], "nosniff");
    assert!(
        headers["content-security-policy"]
            .to_str()
            .unwrap()
            .contains("sandbox")
    );

    // Someone outside the organization, and someone signed out, get nothing.
    let mut mallory = owner(&pool, "Mallory", "mallory@example.com", "evil").await;
    let (status, _, _) = mallory.raw(Method::GET, &url, &[], vec![]).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let mut anonymous = Browser::new(&pool);
    let (status, _, _) = anonymous.raw(Method::GET, &url, &[], vec![]).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Nor can they upload into it.
    let (status, _) = upload(&mut mallory, "acme", "x.png", PNG).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (count,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM audit_events WHERE action = 'file.uploaded'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 1);
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn only_images_within_the_size_limit(pool: PgPool) {
    let mut alice = owner(&pool, "Alice", "alice@example.com", "acme").await;

    // The type comes from the bytes, not the header: an SVG or HTML page is refused.
    let (status, error) = upload(&mut alice, "acme", "x.png", b"<svg onload=alert(1)>").await;
    assert_eq!(status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert_eq!(error["error"]["code"], "unsupported_file");

    let mut big = PNG.to_vec();
    big.resize(kenning_server::files::MAX_SIZE + 1, 0);
    let (status, error) = upload(&mut alice, "acme", "big.png", &big).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(error["error"]["code"], "file_too_large");

    // Raw uploads still need the header that proves they come from our pages.
    let (status, _, _) = alice
        .raw(
            Method::POST,
            "/api/orgs/acme/files?name=x.png",
            &[("content-type", "image/png")],
            PNG.to_vec(),
        )
        .await;
    assert_eq!(status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
}
