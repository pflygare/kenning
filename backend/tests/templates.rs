mod common;

use axum::http::StatusCode;
use common::Browser;
use serde_json::{Value, json};
use sqlx::PgPool;

const MIGRATOR: &sqlx::migrate::Migrator = &kenning_server::db::MIGRATOR;

const TEMPLATES: &str = "/api/orgs/acme/templates";

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

async fn topic(browser: &mut Browser, name: &str) -> Value {
    let (status, topic) = browser
        .post("/api/orgs/acme/topics", json!({"name": name}))
        .await;
    assert_eq!(status, StatusCode::CREATED, "{topic}");
    topic["topic"]["id"].clone()
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn a_page_from_a_template_gets_its_content_topics_and_tags(pool: PgPool) {
    let mut ada = ada(&pool).await;
    let meetings = topic(&mut ada, "Meetings").await;
    let team = topic(&mut ada, "Team").await;

    let (status, template) = ada
        .post(
            TEMPLATES,
            json!({
                "name": "Meeting notes",
                "description": "Agenda, notes and actions",
                "title": "Standup {{date}}",
                "body_md": "# {{title}}\n\nTaken by {{author}} on {{date}}.\n\n## Actions",
                "topic_ids": [meetings],
                "tag_names": ["Notes"],
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{template}");
    assert_eq!(template["topics"][0]["name"], "Meetings");
    assert_eq!(template["tags"][0]["name"], "Notes");
    let id = template["id"].as_str().unwrap();

    let (_, list) = ada.get(TEMPLATES).await;
    assert_eq!(list[0]["name"], "Meeting notes");
    assert_eq!(list[0]["uses_title"], true);

    let (status, page) = ada
        .post(
            "/api/orgs/acme/pages",
            json!({"title": "Untitled", "template_id": id, "topic_ids": [team],
                   "local_date": "2026-10-09"}),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{page}");
    assert_eq!(page["draft"]["title"], "Standup 2026-10-09");
    assert_eq!(
        page["draft"]["body_md"],
        "# Standup 2026-10-09\n\nTaken by Ada on 2026-10-09.\n\n## Actions"
    );
    let topics: Vec<&str> = page["topics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(topics, ["Meetings", "Team"]);
    assert_eq!(page["tags"][0]["name"], "Notes");

    // Changing the template later leaves the page alone.
    let (status, _) = ada
        .put(
            &format!("{TEMPLATES}/{id}"),
            json!({"name": "Meeting notes", "description": "", "title": "", "body_md": "New"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (_, again) = ada
        .get(&format!(
            "/api/orgs/acme/pages/{}",
            page["short_id"].as_str().unwrap()
        ))
        .await;
    assert_eq!(again["draft"]["body_md"], page["draft"]["body_md"]);

    // Without a preset title, the writer's title is used.
    let (_, page) = ada
        .post(
            "/api/orgs/acme/pages",
            json!({"title": "Retro", "template_id": id}),
        )
        .await;
    assert_eq!(page["draft"]["title"], "Retro");
    assert_eq!(page["draft"]["body_md"], "New");
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn templates_can_be_edited_tagged_and_archived(pool: PgPool) {
    let mut ada = ada(&pool).await;
    let (status, _) = ada.post(TEMPLATES, json!({"name": "  "})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (_, template) = ada.post(TEMPLATES, json!({"name": "How-to"})).await;
    let url = format!("{TEMPLATES}/{}", template["id"].as_str().unwrap());
    for body in ["One", "Two"] {
        let (status, saved) = ada
            .put(
                &url,
                json!({"name": "How-to", "description": "Steps", "title": "", "body_md": body}),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{saved}");
    }
    let edits: i64 =
        sqlx::query_scalar("SELECT count(*) FROM audit_events WHERE action = 'template.updated'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(edits, 1);

    let (status, template) = ada
        .put(
            &format!("{url}/tags"),
            json!({"names": ["Guides", "guides", "Ops"]}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{template}");
    assert_eq!(template["tags"].as_array().unwrap().len(), 2);
    let ops = topic(&mut ada, "Ops").await;
    let (_, template) = ada
        .put(&format!("{url}/topics"), json!({"topic_ids": [ops]}))
        .await;
    assert_eq!(template["topics"][0]["name"], "Ops");
    let (status, _) = ada
        .put(
            &format!("{url}/topics"),
            json!({"topic_ids": [uuid::Uuid::now_v7()]}),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Merging a tag carries templates along.
    let (_, tags) = ada.get("/api/orgs/acme/tags").await;
    let id_of = |name: &str| {
        tags.as_array()
            .unwrap()
            .iter()
            .find(|t| t["name"] == name)
            .unwrap()["id"]
            .clone()
    };
    let (status, _) = ada
        .post(
            &format!(
                "/api/orgs/acme/tags/{}/merge",
                id_of("Ops").as_str().unwrap()
            ),
            json!({"into_id": id_of("Guides")}),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, template) = ada.get(&url).await;
    let names: Vec<&Value> = template["tags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| &t["name"])
        .collect();
    assert_eq!(names, [&json!("Guides")]);

    let (status, _) = ada.post(&format!("{url}/archive"), json!({})).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(ada.get(&url).await.0, StatusCode::NOT_FOUND);
    let (_, list) = ada.get(TEMPLATES).await;
    assert_eq!(list, json!([]));
}
