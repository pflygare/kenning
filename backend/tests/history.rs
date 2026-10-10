mod common;

use axum::http::StatusCode;
use common::Browser;
use serde_json::{Value, json};
use sqlx::PgPool;

const MIGRATOR: &sqlx::migrate::Migrator = &kenning_server::db::MIGRATOR;

const PAGES: &str = "/api/orgs/acme/pages";

async fn ada(pool: &PgPool) -> Browser {
    let mut browser = Browser::new(pool);
    browser
        .sign_up_verified(pool, "Ada", "ada@example.com")
        .await;
    let (status, _) = browser
        .post("/api/orgs", json!({"name": "Acme", "slug": "acme"}))
        .await;
    assert_eq!(status, StatusCode::CREATED);
    browser
}

/// Pretend the last save was long ago, so the next one starts a new revision.
async fn age_revisions(pool: &PgPool) {
    sqlx::query("UPDATE page_revisions SET updated_at = updated_at - interval '1 hour'")
        .execute(pool)
        .await
        .unwrap();
}

async fn save(browser: &mut Browser, url: &str, base: &Value, body: &str) -> Value {
    let (status, saved) = browser
        .put(
            &format!("{url}/draft"),
            json!({"base_revision_id": base, "title": "Guide", "body_md": body}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    saved["revision_id"].clone()
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn history_marks_published_and_discarded_revisions(pool: PgPool) {
    let mut ada = ada(&pool).await;
    let (_, page) = ada
        .post(PAGES, json!({"title": "Guide", "body_md": "one"}))
        .await;
    let url = format!("{PAGES}/{}", page["short_id"].as_str().unwrap());
    let first = page["draft"]["revision_id"].clone();
    ada.post(&format!("{url}/publish"), json!({"revision_id": first}))
        .await;

    let second = save(&mut ada, &url, &first, "two").await;
    ada.post(&format!("{url}/publish"), json!({"revision_id": second}))
        .await;
    let third = save(&mut ada, &url, &second, "three").await;
    ada.delete(&format!("{url}/draft")).await;

    let (status, history) = ada.get(&format!("{url}/revisions")).await;
    assert_eq!(status, StatusCode::OK);
    let ids: Vec<&Value> = history
        .as_array()
        .unwrap()
        .iter()
        .map(|r| &r["revision_id"])
        .collect();
    assert_eq!(ids, [&third, &second, &first]);
    assert!(history[0]["discarded_at"].is_string());
    assert_eq!(history[0]["is_current"], false);
    assert_eq!(history[1]["is_live"], true);
    assert_eq!(history[1]["is_current"], true);
    assert!(history[2]["published_at"].is_string(), "{history}");
    assert_eq!(history[2]["is_live"], false);
    assert_eq!(history[2]["published_by_name"], "Ada");

    let (status, detail) = ada
        .get(&format!("{url}/revisions/{}", second.as_str().unwrap()))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(detail["body_md"], "two");
    assert_eq!(detail["previous"]["body_md"], "one");
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn restoring_makes_a_new_draft(pool: PgPool) {
    let mut ada = ada(&pool).await;
    let (_, page) = ada
        .post(PAGES, json!({"title": "Guide", "body_md": "one"}))
        .await;
    let url = format!("{PAGES}/{}", page["short_id"].as_str().unwrap());
    let first = page["draft"]["revision_id"].clone();
    ada.post(&format!("{url}/publish"), json!({"revision_id": first}))
        .await;
    let second = save(&mut ada, &url, &first, "two").await;
    ada.post(&format!("{url}/publish"), json!({"revision_id": second}))
        .await;

    let restore = format!("{url}/revisions/{}/restore", first.as_str().unwrap());
    // Built on an old revision: refused.
    let (status, _) = ada.post(&restore, json!({"base_revision_id": first})).await;
    assert_eq!(status, StatusCode::CONFLICT);

    let (status, page) = ada
        .post(&restore, json!({"base_revision_id": second}))
        .await;
    assert_eq!(status, StatusCode::OK, "{page}");
    assert_eq!(page["published"]["body_md"], "two");
    assert_eq!(page["draft"]["body_md"], "one");
    let restored = page["draft"]["revision_id"].clone();
    assert_ne!(restored, first);

    let (_, history) = ada.get(&format!("{url}/revisions")).await;
    assert_eq!(history[0]["revision_id"], restored);
    assert_eq!(history[0]["restored_from"], first);
    assert_eq!(history[0]["is_current"], true);

    // Saving keeps working on the restored draft.
    age_revisions(&pool).await;
    save(&mut ada, &url, &restored, "one, again").await;
    let (_, page) = ada.get(&url).await;
    assert_eq!(page["draft"]["body_md"], "one, again");

    let (status, _) = ada
        .post(
            &format!("{url}/revisions/{}/restore", uuid::Uuid::now_v7()),
            json!({"base_revision_id": page["draft"]["revision_id"]}),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn archived_pages_can_be_restored(pool: PgPool) {
    let mut ada = ada(&pool).await;
    let (_, page) = ada.post(PAGES, json!({"title": "Old notes"})).await;
    let url = format!("{PAGES}/{}", page["short_id"].as_str().unwrap());
    let (status, _) = ada.post(&format!("{url}/archive"), json!({})).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(ada.get(&url).await.0, StatusCode::NOT_FOUND);

    let (_, archived) = ada.get("/api/orgs/acme/archive").await;
    assert_eq!(archived[0]["title"], "Old notes");
    assert_eq!(archived[0]["archived_by_name"], "Ada");

    let (status, page) = ada.post(&format!("{url}/unarchive"), json!({})).await;
    assert_eq!(status, StatusCode::OK, "{page}");
    assert_eq!(page["draft"]["title"], "Old notes");
    let (_, archived) = ada.get("/api/orgs/acme/archive").await;
    assert_eq!(archived, json!([]));
    let (status, _) = ada.post(&format!("{url}/unarchive"), json!({})).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
