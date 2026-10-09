mod common;

use axum::http::StatusCode;
use common::Browser;
use serde_json::{Value, json};
use sqlx::PgPool;

const MIGRATOR: &sqlx::migrate::Migrator = &kenning_server::db::MIGRATOR;

async fn owner(pool: &PgPool, name: &str, email: &str, org: &str) -> Browser {
    let mut browser = Browser::new(pool);
    browser.sign_up_verified(pool, name, email).await;
    let (status, _) = browser
        .post("/api/orgs", json!({"name": org, "slug": org}))
        .await;
    assert_eq!(status, StatusCode::CREATED);
    browser
}

async fn topic(browser: &mut Browser, name: &str, parent: Option<&Value>) -> Value {
    let (status, detail) = browser
        .post(
            "/api/orgs/acme/topics",
            json!({"name": name, "parent_id": parent.map(|p| p["id"].clone())}),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{detail}");
    detail["topic"].clone()
}

fn topic_url(topic: &Value) -> String {
    format!(
        "/api/orgs/acme/topics/{}",
        topic["short_id"].as_str().unwrap()
    )
}

async fn page(browser: &mut Browser, title: &str) -> String {
    let (status, page) = browser
        .post("/api/orgs/acme/pages", json!({"title": title}))
        .await;
    assert_eq!(status, StatusCode::CREATED);
    format!(
        "/api/orgs/acme/pages/{}",
        page["short_id"].as_str().unwrap()
    )
}

fn names(list: &Value, key: &str) -> Vec<String> {
    list.as_array()
        .unwrap()
        .iter()
        .map(|item| item[key].as_str().unwrap().to_string())
        .collect()
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn topics_form_a_tree_that_can_be_rearranged(pool: PgPool) {
    let mut ada = owner(&pool, "Ada", "ada@example.com", "acme").await;
    let eng = topic(&mut ada, "Engineering", None).await;
    let people = topic(&mut ada, "People", None).await;
    let backend = topic(&mut ada, "Backend", Some(&eng)).await;
    let db = topic(&mut ada, "Databases", Some(&backend)).await;

    let (_, detail) = ada.get(&topic_url(&db)).await;
    assert_eq!(names(&detail["path"], "name"), ["Engineering", "Backend"]);
    let (_, detail) = ada.get(&topic_url(&eng)).await;
    assert_eq!(names(&detail["children"], "name"), ["Backend"]);

    // Siblings keep the order they were made in, and can be reordered.
    let (_, all) = ada.get("/api/orgs/acme/topics").await;
    let roots: Vec<&Value> = all
        .as_array()
        .unwrap()
        .iter()
        .filter(|t| t["parent_id"].is_null())
        .collect();
    assert_eq!(roots[0]["name"], "Engineering");
    let (status, all) = ada
        .post(
            &format!("{}/move", topic_url(&people)),
            json!({"parent_id": null, "index": 0}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let roots: Vec<&str> = all
        .as_array()
        .unwrap()
        .iter()
        .filter(|t| t["parent_id"].is_null())
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(roots, ["People", "Engineering"]);

    // Moving a topic under its own descendant is refused.
    let (status, body) = ada
        .post(
            &format!("{}/move", topic_url(&eng)),
            json!({"parent_id": db["id"], "index": 0}),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "invalid_move");

    // Moving Databases to the top level takes its place there.
    let (status, _) = ada
        .post(
            &format!("{}/move", topic_url(&db)),
            json!({"parent_id": null, "index": 1}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (_, detail) = ada.get(&topic_url(&db)).await;
    assert_eq!(detail["path"], json!([]));

    // Renaming changes the slug; archiving needs the topic to be empty of sub-topics.
    let (status, detail) = ada
        .patch(
            &topic_url(&eng),
            json!({"name": "Eng & Product", "description": "Builders"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(detail["topic"]["slug"], "eng-product");
    assert_eq!(detail["topic"]["description"], "Builders");
    let (status, body) = ada
        .post(&format!("{}/archive", topic_url(&eng)), json!({}))
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"]["code"], "topic_has_subtopics");
    let (status, _) = ada
        .post(&format!("{}/archive", topic_url(&backend)), json!({}))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = ada.get(&topic_url(&backend)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn pages_can_sit_in_several_topics(pool: PgPool) {
    let mut ada = owner(&pool, "Ada", "ada@example.com", "acme").await;
    let eng = topic(&mut ada, "Engineering", None).await;
    let onboarding = topic(&mut ada, "Onboarding", None).await;
    let setup = page(&mut ada, "Laptop setup").await;
    let zebra = page(&mut ada, "Zebra crossing").await;

    let (status, detail) = ada
        .put(
            &format!("{setup}/topics"),
            json!({"topic_ids": [eng["id"], onboarding["id"]]}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        names(&detail["topics"], "name"),
        ["Engineering", "Onboarding"]
    );
    ada.put(
        &format!("{zebra}/topics"),
        json!({"topic_ids": [eng["id"]]}),
    )
    .await;

    // A topic lists its pages by title.
    let (_, detail) = ada.get(&topic_url(&eng)).await;
    assert_eq!(
        names(&detail["pages"], "title"),
        ["Laptop setup", "Zebra crossing"]
    );
    assert_eq!(detail["topic"]["page_count"], 2);

    // Setting the topics replaces them.
    let (_, detail) = ada
        .put(
            &format!("{setup}/topics"),
            json!({"topic_ids": [onboarding["id"]]}),
        )
        .await;
    assert_eq!(names(&detail["topics"], "name"), ["Onboarding"]);
    let (_, detail) = ada.get(&topic_url(&eng)).await;
    assert_eq!(names(&detail["pages"], "title"), ["Zebra crossing"]);

    // Archived pages leave their topics.
    ada.post(&format!("{zebra}/archive"), json!({})).await;
    let (_, detail) = ada.get(&topic_url(&eng)).await;
    assert_eq!(detail["pages"], json!([]));
    assert_eq!(detail["topic"]["page_count"], 0);
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn tags_are_created_from_pages_and_managed_on_their_own(pool: PgPool) {
    let mut ada = owner(&pool, "Ada", "ada@example.com", "acme").await;
    let guide = page(&mut ada, "Guide").await;
    let policy = page(&mut ada, "Policy").await;

    // Names are matched ignoring case; new ones become tags.
    let (status, detail) = ada
        .put(
            &format!("{guide}/tags"),
            json!({"names": ["Onboarding", " how  to ", "onboarding"]}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{detail}");
    assert_eq!(names(&detail["tags"], "name"), ["how to", "Onboarding"]);
    let (_, detail) = ada
        .put(
            &format!("{policy}/tags"),
            json!({"names": ["ONBOARDING", "HR"]}),
        )
        .await;
    assert_eq!(names(&detail["tags"], "name"), ["HR", "Onboarding"]);

    let (_, tags) = ada.get("/api/orgs/acme/tags").await;
    assert_eq!(names(&tags, "name"), ["how to", "HR", "Onboarding"]);
    assert_eq!(tags[2]["page_count"], 2);
    let tag = |name: &str| {
        tags.as_array()
            .unwrap()
            .iter()
            .find(|t| t["name"] == name)
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string()
    };

    // Lists filter by tag and show each page's tags.
    let (_, list) = ada.get("/api/orgs/acme/pages?tag=hr").await;
    assert_eq!(names(&list, "title"), ["Policy"]);
    assert_eq!(names(&list[0]["tags"], "color"), ["gray", "gray"]);

    // Rename and recolor; a name another tag already has is refused.
    let (status, renamed) = ada
        .patch(
            &format!("/api/orgs/acme/tags/{}", tag("how to")),
            json!({"name": "How-to", "color": "teal"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(renamed["color"], "teal");
    let (status, body) = ada
        .patch(
            &format!("/api/orgs/acme/tags/{}", tag("HR")),
            json!({"name": "onboarding"}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"]["code"], "tag_exists");

    // Merging moves pages to the kept tag; deleting removes the tag everywhere.
    let (status, _) = ada
        .post(
            &format!("/api/orgs/acme/tags/{}/merge", tag("HR")),
            json!({"into_id": tag("Onboarding")}),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, detail) = ada.get(&policy).await;
    assert_eq!(names(&detail["tags"], "name"), ["Onboarding"]);
    let (status, _) = ada
        .delete(&format!("/api/orgs/acme/tags/{}", tag("Onboarding")))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, detail) = ada.get(&guide).await;
    assert_eq!(names(&detail["tags"], "name"), ["How-to"]);
    let (_, tags) = ada.get("/api/orgs/acme/tags").await;
    assert_eq!(names(&tags, "name"), ["How-to"]);

    let actions: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT action FROM audit_events WHERE action LIKE 'tag.%' ORDER BY action",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        actions,
        ["tag.created", "tag.deleted", "tag.merged", "tag.updated"]
    );
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn topics_and_tags_stay_inside_their_organization(pool: PgPool) {
    let mut ada = owner(&pool, "Ada", "ada@example.com", "acme").await;
    let mut eve = owner(&pool, "Eve", "eve@example.com", "evil").await;
    let secret = topic(&mut ada, "Secret plans", None).await;
    let guide = page(&mut ada, "Guide").await;
    ada.put(&format!("{guide}/tags"), json!({"names": ["Confidential"]}))
        .await;

    let (_, topics) = eve.get("/api/orgs/evil/topics").await;
    assert_eq!(topics, json!([]));
    let (_, tags) = eve.get("/api/orgs/evil/tags").await;
    assert_eq!(tags, json!([]));
    let (status, _) = eve
        .get(&format!(
            "/api/orgs/evil/topics/{}",
            secret["short_id"].as_str().unwrap()
        ))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Eve can't file her page under Ada's topic.
    let (_, page) = eve
        .post("/api/orgs/evil/pages", json!({"title": "Mine"}))
        .await;
    let (status, body) = eve
        .put(
            &format!(
                "/api/orgs/evil/pages/{}/topics",
                page["short_id"].as_str().unwrap()
            ),
            json!({"topic_ids": [secret["id"]]}),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "unknown_topic");

    // The same tag name in another organization is a different tag.
    let (_, detail) = eve
        .put(
            &format!(
                "/api/orgs/evil/pages/{}/tags",
                page["short_id"].as_str().unwrap()
            ),
            json!({"names": ["confidential"]}),
        )
        .await;
    assert_eq!(names(&detail["tags"], "name"), ["confidential"]);
}
