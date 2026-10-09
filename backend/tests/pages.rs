mod common;

use axum::http::StatusCode;
use common::Browser;
use serde_json::{Value, json};
use sqlx::PgPool;

const MIGRATOR: &sqlx::migrate::Migrator = &kenning_server::db::MIGRATOR;

const PAGES: &str = "/api/orgs/acme/pages";

async fn member(pool: &PgPool, name: &str, email: &str, create_org: bool) -> Browser {
    let mut browser = Browser::new(pool);
    browser.sign_up_verified(pool, name, email).await;
    if create_org {
        let (status, _) = browser
            .post("/api/orgs", json!({"name": "Acme", "slug": "acme"}))
            .await;
        assert_eq!(status, StatusCode::CREATED);
    } else {
        sqlx::query(
            "INSERT INTO memberships (org_id, user_id, role)
             SELECT o.id, u.id, 'member' FROM organizations o, users u
             WHERE o.slug = 'acme' AND u.email = $1",
        )
        .bind(email)
        .execute(pool)
        .await
        .unwrap();
    }
    browser
}

async fn create_page(browser: &mut Browser, title: &str, body: &str) -> Value {
    let (status, page) = browser
        .post(PAGES, json!({"title": title, "body_md": body}))
        .await;
    assert_eq!(status, StatusCode::CREATED, "{page}");
    page
}

fn page_url(page: &Value) -> String {
    format!("{PAGES}/{}", page["short_id"].as_str().unwrap())
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn a_new_page_is_a_draft_until_published(pool: PgPool) {
    let mut ada = member(&pool, "Ada", "ada@example.com", true).await;
    let page = create_page(&mut ada, "  Onboarding Guide ", "# Welcome").await;
    assert_eq!(page["slug"], "onboarding-guide");
    assert_eq!(page["published"], Value::Null);
    assert_eq!(page["draft"]["title"], "Onboarding Guide");
    assert_eq!(page["draft"]["author_name"], "Ada");

    let (_, list) = ada.get(PAGES).await;
    assert_eq!(list[0]["title"], "Onboarding Guide");
    assert_eq!(list[0]["published"], false);
    assert_eq!(list[0]["has_draft"], true);

    let revision = page["draft"]["revision_id"].clone();
    let (status, page) = ada
        .post(
            &format!("{}/publish", page_url(&page)),
            json!({"revision_id": revision}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page["published"]["body_md"], "# Welcome");
    assert_eq!(page["published_by_name"], "Ada");
    assert_eq!(page["draft"], Value::Null);

    let actions: Vec<String> = sqlx::query_scalar(
        "SELECT action FROM audit_events WHERE object_kind = 'page' ORDER BY seq",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(actions, ["page.created", "page.published"]);
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn readers_keep_seeing_the_published_version_while_a_draft_exists(pool: PgPool) {
    let mut ada = member(&pool, "Ada", "ada@example.com", true).await;
    let page = create_page(&mut ada, "Guide", "v1").await;
    let url = page_url(&page);
    ada.post(
        &format!("{url}/publish"),
        json!({"revision_id": page["draft"]["revision_id"]}),
    )
    .await;
    let (_, page) = ada.get(&url).await;
    let published_id = page["published"]["revision_id"].clone();

    let (status, saved) = ada
        .put(
            &format!("{url}/draft"),
            json!({"base_revision_id": published_id, "title": "Guide v2", "body_md": "v2"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_ne!(saved["revision_id"], published_id);

    let (_, page) = ada.get(&url).await;
    assert_eq!(page["published"]["body_md"], "v1");
    assert_eq!(page["draft"]["body_md"], "v2");
    let (_, list) = ada.get(PAGES).await;
    assert_eq!(list[0]["title"], "Guide", "lists show the published title");
    assert_eq!(list[0]["has_draft"], true);

    // Discarding goes back to what readers see.
    let (status, page) = ada.delete(&format!("{url}/draft")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page["draft"], Value::Null);
    assert_eq!(page["published"]["body_md"], "v1");
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn quick_saves_by_one_author_collapse_into_one_revision(pool: PgPool) {
    let mut ada = member(&pool, "Ada", "ada@example.com", true).await;
    let page = create_page(&mut ada, "Guide", "").await;
    let url = format!("{}/draft", page_url(&page));
    let first = page["draft"]["revision_id"].clone();

    let (_, saved) = ada
        .put(
            &url,
            json!({"base_revision_id": first, "title": "Guide", "body_md": "a"}),
        )
        .await;
    assert_eq!(saved["revision_id"], first);
    let (_, saved) = ada
        .put(
            &url,
            json!({"base_revision_id": first, "title": "Guide", "body_md": "ab"}),
        )
        .await;
    assert_eq!(saved["revision_id"], first);

    // After a pause, the next save starts a new revision.
    sqlx::query("UPDATE page_revisions SET updated_at = now() - interval '11 minutes'")
        .execute(&pool)
        .await
        .unwrap();
    let (_, saved) = ada
        .put(
            &url,
            json!({"base_revision_id": first, "title": "Guide", "body_md": "abc"}),
        )
        .await;
    assert_ne!(saved["revision_id"], first);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM page_revisions")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 2);
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn a_stale_save_or_publish_is_refused(pool: PgPool) {
    let mut ada = member(&pool, "Ada", "ada@example.com", true).await;
    let mut bob = member(&pool, "Bob", "bob@example.com", false).await;
    let page = create_page(&mut ada, "Guide", "").await;
    let url = page_url(&page);
    let base = page["draft"]["revision_id"].clone();

    // Bob saves first: a new revision, since the draft is Ada's.
    let (status, saved) = bob
        .put(
            &format!("{url}/draft"),
            json!({"base_revision_id": base, "title": "Guide", "body_md": "bob"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_ne!(saved["revision_id"], base);

    let (status, body) = ada
        .put(
            &format!("{url}/draft"),
            json!({"base_revision_id": base, "title": "Guide", "body_md": "ada"}),
        )
        .await;
    assert_eq!(
        (status, body["error"]["code"].as_str()),
        (StatusCode::CONFLICT, Some("stale_draft"))
    );
    let (status, _) = ada
        .post(&format!("{url}/publish"), json!({"revision_id": base}))
        .await;
    assert_eq!(status, StatusCode::CONFLICT);

    let (_, page) = ada.get(&url).await;
    assert_eq!(page["draft"]["body_md"], "bob");
    assert_eq!(page["draft"]["author_name"], "Bob");
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn discarding_a_never_published_page_is_refused(pool: PgPool) {
    let mut ada = member(&pool, "Ada", "ada@example.com", true).await;
    let page = create_page(&mut ada, "Guide", "").await;
    let (status, body) = ada.delete(&format!("{}/draft", page_url(&page))).await;
    assert_eq!(
        (status, body["error"]["code"].as_str()),
        (StatusCode::CONFLICT, Some("never_published"))
    );
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn archived_pages_disappear(pool: PgPool) {
    let mut ada = member(&pool, "Ada", "ada@example.com", true).await;
    let page = create_page(&mut ada, "Old", "").await;
    let url = page_url(&page);
    let (status, _) = ada.post(&format!("{url}/archive"), json!({})).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(ada.get(&url).await.0, StatusCode::NOT_FOUND);
    assert_eq!(ada.get(PAGES).await.1, json!([]));
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn pages_stay_inside_their_organization(pool: PgPool) {
    let mut ada = member(&pool, "Ada", "ada@example.com", true).await;
    let page = create_page(&mut ada, "Secret", "").await;

    let mut eve = Browser::new(&pool);
    eve.sign_up_verified(&pool, "Eve", "eve@example.com").await;
    eve.post("/api/orgs", json!({"name": "Evil", "slug": "evil"}))
        .await;
    let short_id = page["short_id"].as_str().unwrap();
    assert_eq!(eve.get(&page_url(&page)).await.0, StatusCode::NOT_FOUND);
    assert_eq!(
        eve.get(&format!("/api/orgs/evil/pages/{short_id}")).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(eve.get("/api/orgs/evil/pages").await.1, json!([]));
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn titles_are_required(pool: PgPool) {
    let mut ada = member(&pool, "Ada", "ada@example.com", true).await;
    let (status, body) = ada.post(PAGES, json!({"title": "  "})).await;
    assert_eq!(
        (status, body["error"]["code"].as_str()),
        (StatusCode::BAD_REQUEST, Some("invalid_title"))
    );
}
