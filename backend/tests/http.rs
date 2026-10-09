mod common;

use axum::http::StatusCode;
use sqlx::PgPool;

#[sqlx::test(migrator = "kenning_server::db::MIGRATOR")]
async fn health_reports_database(pool: PgPool) {
    let app = common::app(pool);
    let (status, body) = common::get_json(&app, "/api/health").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
    assert_eq!(body["database"], "ok");
}

#[sqlx::test(migrator = "kenning_server::db::MIGRATOR")]
async fn unknown_api_path_is_json_404(pool: PgPool) {
    let app = common::app(pool);
    let (status, body) = common::get_json(&app, "/api/nope").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "not_found");
}

#[sqlx::test(migrator = "kenning_server::db::MIGRATOR")]
async fn serves_spa_with_index_fallback(pool: PgPool) {
    let dir = std::env::temp_dir().join(format!("kenning-static-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir_all(dir.join("assets")).unwrap();
    std::fs::write(dir.join("index.html"), "<div id=root></div>").unwrap();
    std::fs::write(dir.join("assets/app.js"), "console.log(1)").unwrap();
    let app = common::app_with_static(pool, Some(dir.clone()));

    let (status, body) = common::get(&app, "/assets/app.js").await;
    assert_eq!((status, body.as_str()), (StatusCode::OK, "console.log(1)"));

    // Client-side routes get index.html.
    let (status, body) = common::get(&app, "/acme/p/some-page").await;
    assert_eq!(
        (status, body.as_str()),
        (StatusCode::OK, "<div id=root></div>")
    );

    // The API still answers JSON, never index.html.
    let (status, _) = common::get_json(&app, "/api/nope").await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    std::fs::remove_dir_all(dir).unwrap();
}
