mod common;

use axum::http::StatusCode;
use common::Browser;
use serde_json::{Value, json};
use sqlx::PgPool;

const MIGRATOR: &sqlx::migrate::Migrator = &kenning_server::db::MIGRATOR;

const PAGES: &str = "/api/orgs/acme/pages";

async fn owner(pool: &PgPool, name: &str, email: &str, org: &str) -> Browser {
    let mut browser = Browser::new(pool);
    browser.sign_up_verified(pool, name, email).await;
    let (status, _) = browser
        .post("/api/orgs", json!({"name": org, "slug": org}))
        .await;
    assert_eq!(status, StatusCode::CREATED);
    browser
}

async fn member(pool: &PgPool, name: &str, email: &str) -> Browser {
    let mut browser = Browser::new(pool);
    browser.sign_up_verified(pool, name, email).await;
    sqlx::query(
        "INSERT INTO memberships (org_id, user_id, role)
         SELECT o.id, u.id, 'member' FROM organizations o, users u
         WHERE o.slug = 'acme' AND u.email = $1",
    )
    .bind(email)
    .execute(pool)
    .await
    .unwrap();
    browser
}

/// Create and publish a page; returns its API path.
async fn published(browser: &mut Browser, title: &str, body: &str) -> String {
    let (status, page) = browser
        .post(PAGES, json!({"title": title, "body_md": body}))
        .await;
    assert_eq!(status, StatusCode::CREATED);
    let url = format!("{PAGES}/{}", page["short_id"].as_str().unwrap());
    let (status, _) = browser
        .post(
            &format!("{url}/publish"),
            json!({"revision_id": page["draft"]["revision_id"]}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    url
}

async fn search(browser: &mut Browser, query: &str) -> Value {
    let (status, results) = browser.get(&format!("/api/orgs/acme/search?{query}")).await;
    assert_eq!(status, StatusCode::OK, "{results}");
    results
}

fn plain(text: &Value) -> String {
    text.as_str().unwrap().replace(['\u{2}', '\u{3}'], "")
}

fn titles(results: &Value) -> Vec<String> {
    results["pages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|hit| plain(&hit["title"]))
        .collect()
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn finds_pages_by_words_in_titles_and_bodies(pool: PgPool) {
    let mut ada = owner(&pool, "Ada", "ada@example.com", "acme").await;
    published(
        &mut ada,
        "Deploy runbook",
        "# Steps\n\nRun the **migrations** first, then see [the dashboard](https://example.com/x).",
    )
    .await;
    published(
        &mut ada,
        "Onboarding",
        "Welcome! Ask about the deploy process.",
    )
    .await;
    published(&mut ada, "Lunch menu", "Soup on Fridays.").await;

    // A title match ranks above a body match; words match as prefixes.
    let results = search(&mut ada, "q=deplo").await;
    assert_eq!(titles(&results), ["Deploy runbook", "Onboarding"]);
    assert_eq!(results["pages"][0]["title"], "\u{2}Deploy\u{3} runbook");
    assert!(
        results["pages"][1]["snippet"]
            .as_str()
            .unwrap()
            .contains("\u{2}deploy\u{3}")
    );

    // Every word must match; markdown syntax and link targets aren't searched.
    let results = search(&mut ada, "q=run+migrations").await;
    assert_eq!(titles(&results), ["Deploy runbook"]);
    let snippet = plain(&results["pages"][0]["snippet"]);
    assert!(
        !snippet.contains("**") && !snippet.contains("example.com"),
        "{snippet}"
    );
    assert!(snippet.contains("the dashboard"), "{snippet}");
    assert_eq!(
        titles(&search(&mut ada, "q=example").await),
        Vec::<String>::new()
    );

    // Titles also match with a typo.
    assert_eq!(
        titles(&search(&mut ada, "q=Lunch+mneu").await),
        ["Lunch menu"]
    );

    // Nothing to search for gives nothing, not everything.
    let results = search(&mut ada, "q=+%21%3A*").await;
    assert_eq!(results["pages"], json!([]));
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn searches_what_readers_see(pool: PgPool) {
    let mut ada = owner(&pool, "Ada", "ada@example.com", "acme").await;
    let url = published(&mut ada, "Guide", "The old wording").await;
    let (_, page) = ada.get(&url).await;
    ada.put(
        &format!("{url}/draft"),
        json!({"base_revision_id": page["published"]["revision_id"], "title": "Guide", "body_md": "Brand new wording"}),
    )
    .await;
    // The unpublished draft isn't searched while a published version exists.
    assert_eq!(titles(&search(&mut ada, "q=old").await), ["Guide"]);
    assert_eq!(search(&mut ada, "q=brand").await["pages"], json!([]));

    // A page never published is found by its draft.
    ada.post(PAGES, json!({"title": "Ideas", "body_md": "Half-baked"}))
        .await;
    let results = search(&mut ada, "q=baked").await;
    assert_eq!(titles(&results), ["Ideas"]);
    assert_eq!(results["pages"][0]["published"], false);

    // Archived pages are gone from search.
    ada.post(&format!("{url}/archive"), json!({})).await;
    assert_eq!(search(&mut ada, "q=old").await["pages"], json!([]));
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn filters_by_topic_tag_and_value_and_finds_topics(pool: PgPool) {
    let mut ada = owner(&pool, "Ada", "ada@example.com", "acme").await;
    let (_, eng) = ada
        .post("/api/orgs/acme/topics", json!({"name": "Engineering"}))
        .await;
    let (_, backend) = ada
        .post(
            "/api/orgs/acme/topics",
            json!({"name": "Backend", "parent_id": eng["topic"]["id"]}),
        )
        .await;
    let api = published(&mut ada, "API guide", "How the service works").await;
    let people = published(&mut ada, "People guide", "How the team works").await;
    ada.put(
        &format!("{api}/topics"),
        json!({"topic_ids": [backend["topic"]["id"]]}),
    )
    .await;
    ada.put(&format!("{people}/tags"), json!({"names": ["Handbook"]}))
        .await;

    let results = search(&mut ada, "q=works").await;
    assert_eq!(results["pages"].as_array().unwrap().len(), 2);

    // A topic filter includes its sub-topics.
    let topic = eng["topic"]["id"].as_str().unwrap();
    let results = search(&mut ada, &format!("q=works&topic={topic}")).await;
    assert_eq!(titles(&results), ["API guide"]);
    assert_eq!(results["pages"][0]["topics"][0]["name"], "Backend");
    assert_eq!(
        titles(&search(&mut ada, "q=works&tag=handbook").await),
        ["People guide"]
    );

    // Tag names are searched too.
    let results = search(&mut ada, "q=handbook").await;
    assert_eq!(titles(&results), ["People guide"]);
    assert_eq!(results["pages"][0]["tags"][0]["name"], "Handbook");

    let (_, categories) = ada
        .post(
            "/api/orgs/acme/categories",
            json!({"name": "Class", "values": ["Open", "Internal"]}),
        )
        .await;
    let internal = categories[0]["values"][1]["id"].clone();
    ada.put(
        &format!("{people}/categories"),
        json!({"category_id": categories[0]["id"], "value_id": internal}),
    )
    .await;
    let results = search(
        &mut ada,
        &format!("q=works&value={}", internal.as_str().unwrap()),
    )
    .await;
    assert_eq!(titles(&results), ["People guide"]);
    assert_eq!(results["pages"][0]["values"][0]["name"], "Internal");

    // Topics come back by name, alongside pages.
    let results = search(&mut ada, "q=engin").await;
    assert_eq!(results["topics"][0]["name"], "Engineering");
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn quick_open_matches_titles_and_shows_recent_pages(pool: PgPool) {
    let mut ada = owner(&pool, "Ada", "ada@example.com", "acme").await;
    published(&mut ada, "Deploy runbook", "deploy deploy").await;
    published(&mut ada, "Release notes", "We deploy weekly").await;
    ada.post("/api/orgs/acme/topics", json!({"name": "Deployments"}))
        .await;

    let (_, quick) = ada.get("/api/orgs/acme/search/quick?q=depl").await;
    let names: Vec<&str> = quick["pages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["title"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Deploy runbook"], "titles only, not bodies");
    assert_eq!(quick["topics"][0]["name"], "Deployments");

    let (_, quick) = ada.get("/api/orgs/acme/search/quick?q=").await;
    assert_eq!(
        quick["pages"][0]["title"], "Release notes",
        "most recent first"
    );
    assert_eq!(quick["pages"].as_array().unwrap().len(), 2);
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn my_drafts_lists_only_my_unpublished_changes(pool: PgPool) {
    let mut ada = owner(&pool, "Ada", "ada@example.com", "acme").await;
    let mut bob = member(&pool, "Bob", "bob@example.com").await;
    published(&mut ada, "Done", "Finished").await;
    ada.post(PAGES, json!({"title": "Ada's idea", "body_md": ""}))
        .await;
    bob.post(PAGES, json!({"title": "Bob's idea", "body_md": ""}))
        .await;

    let (_, mine) = ada.get(&format!("{PAGES}?my_drafts=true")).await;
    let names: Vec<&str> = mine
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["title"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Ada's idea"]);
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn search_stays_inside_the_organization(pool: PgPool) {
    let mut ada = owner(&pool, "Ada", "ada@example.com", "acme").await;
    published(&mut ada, "Secret plan", "Only for Acme").await;
    let mut eve = owner(&pool, "Eve", "eve@example.com", "evil").await;
    let (_, results) = eve.get("/api/orgs/evil/search?q=secret").await;
    assert_eq!(results["pages"], json!([]));
    let (_, quick) = eve.get("/api/orgs/evil/search/quick?q=").await;
    assert_eq!(quick["pages"], json!([]));
    let (status, _) = eve.get("/api/orgs/acme/search?q=secret").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
