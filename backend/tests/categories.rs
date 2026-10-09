mod common;

use axum::http::StatusCode;
use common::Browser;
use serde_json::{Value, json};
use sqlx::PgPool;

const MIGRATOR: &sqlx::migrate::Migrator = &kenning_server::db::MIGRATOR;

const CATEGORIES: &str = "/api/orgs/acme/categories";

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

/// Create a category and return it.
async fn category(browser: &mut Browser, body: Value) -> Value {
    let name = body["name"].as_str().unwrap().to_string();
    let (status, list) = browser.post(CATEGORIES, body).await;
    assert_eq!(status, StatusCode::CREATED, "{list}");
    find(&list, &name)
}

fn find(list: &Value, name: &str) -> Value {
    list.as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == name)
        .unwrap_or_else(|| panic!("no category {name} in {list}"))
        .clone()
}

fn value_id(category: &Value, name: &str) -> Value {
    category["values"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == name)
        .unwrap()["id"]
        .clone()
}

fn category_url(category: &Value) -> String {
    format!("{CATEGORIES}/{}", category["id"].as_str().unwrap())
}

/// Create a page and return its API path and draft revision.
async fn page(browser: &mut Browser, title: &str) -> (String, Value) {
    let (status, page) = browser
        .post("/api/orgs/acme/pages", json!({"title": title}))
        .await;
    assert_eq!(status, StatusCode::CREATED);
    (
        format!(
            "/api/orgs/acme/pages/{}",
            page["short_id"].as_str().unwrap()
        ),
        page["draft"]["revision_id"].clone(),
    )
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

fn page_category<'a>(detail: &'a Value, name: &str) -> &'a Value {
    detail["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == name)
        .unwrap()
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn categories_hold_ordered_values_that_admins_manage(pool: PgPool) {
    let mut ada = owner(&pool, "Ada", "ada@example.com", "acme").await;
    let class = category(
        &mut ada,
        json!({"name": "Information class", "values": ["Open", "Internal", "Restricted"]}),
    )
    .await;
    let names: Vec<&str> = class["values"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Open", "Internal", "Restricted"]);
    // Values start with different colors.
    assert_ne!(class["values"][0]["color"], class["values"][1]["color"]);
    assert_eq!(class["required_everywhere"], false);

    // Names are unique per organization, ignoring case; a category needs values.
    let (status, body) = ada
        .post(
            CATEGORIES,
            json!({"name": "information CLASS", "values": ["A"]}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"]["code"], "name_taken");
    let (status, _) = ada
        .post(CATEGORIES, json!({"name": "Empty", "values": []}))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Add, rename and recolor values.
    let (status, list) = ada
        .post(
            &format!("{}/values", category_url(&class)),
            json!({"name": "Secret"}),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED);
    let class = find(&list, "Information class");
    let secret = value_id(&class, "Secret");
    let (status, list) = ada
        .patch(
            &format!(
                "{}/values/{}",
                category_url(&class),
                secret.as_str().unwrap()
            ),
            json!({"name": "Top secret", "color": "red"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let class = find(&list, "Information class");
    assert_eq!(class["values"][3]["name"], "Top secret");
    assert_eq!(class["values"][3]["color"], "red");

    // A value in use can't be deleted; an unused one can.
    let (doc, _) = page(&mut ada, "Doc").await;
    let open = value_id(&class, "Open");
    let (status, _) = ada
        .put(
            &format!("{doc}/categories"),
            json!({"category_id": class["id"], "value_id": open}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = ada
        .delete(&format!(
            "{}/values/{}",
            category_url(&class),
            open.as_str().unwrap()
        ))
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"]["code"], "value_in_use");
    let (status, list) = ada
        .delete(&format!(
            "{}/values/{}",
            category_url(&class),
            secret.as_str().unwrap()
        ))
        .await;
    assert_eq!(status, StatusCode::OK);
    let class = find(&list, "Information class");
    assert_eq!(class["values"].as_array().unwrap().len(), 3);
    assert_eq!(class["values"][0]["page_count"], 1);

    // Members can read and set values, but not change categories.
    let mut bob = member(&pool, "Bob", "bob@example.com").await;
    let (status, list) = bob.get(CATEGORIES).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 1);
    let (status, body) = bob
        .patch(&category_url(&class), json!({"name": "Mine"}))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"]["code"], "forbidden");
    let (status, _) = bob
        .post(CATEGORIES, json!({"name": "Bob's", "values": ["x"]}))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let internal = value_id(&class, "Internal");
    let (status, detail) = bob
        .put(
            &format!("{doc}/categories"),
            json!({"category_id": class["id"], "value_id": internal}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        page_category(&detail, "Information class")["value"]["name"],
        "Internal"
    );

    // Clearing the value, then deleting the category.
    let (_, detail) = bob
        .put(
            &format!("{doc}/categories"),
            json!({"category_id": class["id"]}),
        )
        .await;
    assert_eq!(
        page_category(&detail, "Information class")["value"],
        Value::Null
    );
    let (status, list) = ada.delete(&category_url(&class)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list, json!([]));
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn a_page_missing_a_required_value_cannot_be_published(pool: PgPool) {
    let mut ada = owner(&pool, "Ada", "ada@example.com", "acme").await;
    let class = category(
        &mut ada,
        json!({"name": "Information class", "values": ["Open", "Internal"], "required_everywhere": true}),
    )
    .await;
    let (doc, revision) = page(&mut ada, "Doc").await;

    let (_, detail) = ada.get(&doc).await;
    assert_eq!(
        page_category(&detail, "Information class")["required"],
        true
    );
    let (_, list) = ada.get("/api/orgs/acme/pages").await;
    assert_eq!(list[0]["missing_required"], true);
    let (_, cats) = ada.get(CATEGORIES).await;
    assert_eq!(cats[0]["missing_count"], 1);

    let (status, body) = ada
        .post(&format!("{doc}/publish"), json!({"revision_id": revision}))
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"]["code"], "missing_categories");
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Information class")
    );

    ada.put(
        &format!("{doc}/categories"),
        json!({"category_id": class["id"], "value_id": value_id(&class, "Internal")}),
    )
    .await;
    let (status, _) = ada
        .post(&format!("{doc}/publish"), json!({"revision_id": revision}))
        .await;
    assert_eq!(status, StatusCode::OK);
    let (_, list) = ada.get("/api/orgs/acme/pages").await;
    assert_eq!(list[0]["missing_required"], false);
    assert_eq!(list[0]["values"][0]["name"], "Internal");
    assert_eq!(list[0]["values"][0]["category"], "Information class");
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn a_category_can_be_required_in_a_topic_and_its_subtopics(pool: PgPool) {
    let mut ada = owner(&pool, "Ada", "ada@example.com", "acme").await;
    let eng = topic(&mut ada, "Engineering", None).await;
    let backend = topic(&mut ada, "Backend", Some(&eng)).await;
    let people = topic(&mut ada, "People", None).await;
    let class = category(
        &mut ada,
        json!({"name": "Information class", "values": ["Open", "Internal"], "required_topic_ids": [eng["id"]]}),
    )
    .await;
    assert_eq!(class["required_topics"][0]["name"], "Engineering");

    let (in_sub, _) = page(&mut ada, "Deep in backend").await;
    let (elsewhere, _) = page(&mut ada, "Handbook").await;
    ada.put(
        &format!("{in_sub}/topics"),
        json!({"topic_ids": [backend["id"]]}),
    )
    .await;
    ada.put(
        &format!("{elsewhere}/topics"),
        json!({"topic_ids": [people["id"]]}),
    )
    .await;

    let (_, detail) = ada.get(&in_sub).await;
    assert_eq!(
        page_category(&detail, "Information class")["required"],
        true
    );
    let (_, detail) = ada.get(&elsewhere).await;
    assert_eq!(
        page_category(&detail, "Information class")["required"],
        false
    );

    // Moving the requirement to People flips both.
    let (status, list) = ada
        .patch(
            &category_url(&class),
            json!({"required_topic_ids": [people["id"]]}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(find(&list, "Information class")["missing_count"], 1);
    let (_, detail) = ada.get(&in_sub).await;
    assert_eq!(
        page_category(&detail, "Information class")["required"],
        false
    );
    let (_, detail) = ada.get(&elsewhere).await;
    assert_eq!(
        page_category(&detail, "Information class")["required"],
        true
    );
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn pages_can_be_listed_by_value(pool: PgPool) {
    let mut ada = owner(&pool, "Ada", "ada@example.com", "acme").await;
    let class = category(
        &mut ada,
        json!({"name": "Information class", "values": ["Open", "Restricted"]}),
    )
    .await;
    let (a, _) = page(&mut ada, "Alpha").await;
    page(&mut ada, "Beta").await;
    let restricted = value_id(&class, "Restricted");
    ada.put(
        &format!("{a}/categories"),
        json!({"category_id": class["id"], "value_id": restricted}),
    )
    .await;

    let (_, list) = ada
        .get(&format!(
            "/api/orgs/acme/pages?value={}",
            restricted.as_str().unwrap()
        ))
        .await;
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert_eq!(list[0]["title"], "Alpha");

    // A value from another category is refused.
    let other = category(&mut ada, json!({"name": "Status", "values": ["Draft"]})).await;
    let (status, body) = ada
        .put(
            &format!("{a}/categories"),
            json!({"category_id": class["id"], "value_id": value_id(&other, "Draft")}),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "unknown_value");
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn categories_stay_inside_their_organization(pool: PgPool) {
    let mut ada = owner(&pool, "Ada", "ada@example.com", "acme").await;
    let class = category(
        &mut ada,
        json!({"name": "Information class", "values": ["Open"]}),
    )
    .await;
    let mut eve = owner(&pool, "Eve", "eve@example.com", "evil").await;
    let (_, list) = eve.get("/api/orgs/evil/categories").await;
    assert_eq!(list, json!([]));
    let (status, _) = eve
        .patch(
            &format!(
                "/api/orgs/evil/categories/{}",
                class["id"].as_str().unwrap()
            ),
            json!({"name": "Mine"}),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = eve.get(CATEGORIES).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
