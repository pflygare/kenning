//! Categories: named sets of fixed values ("Information class": open,
//! internal, restricted). A page has at most one value per category.
//! Owners and admins define categories and where they are required; any
//! member sets a page's values. A page missing a required value can't be
//! published.

use std::collections::HashMap;

use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, PgConnection};
use ts_rs::TS;
use uuid::Uuid;

use crate::{
    AppError, AppResult,
    audit::{self, Event},
    tags::TagColor,
    topics::TopicRef,
};

pub const MAX_NAME: usize = 50;
pub const MAX_DESCRIPTION: usize = 500;
pub const MAX_VALUES: usize = 30;

/// Colors handed to a new category's values in order, so they start distinct.
const VALUE_COLORS: [TagColor; 8] = [
    TagColor::Green,
    TagColor::Blue,
    TagColor::Amber,
    TagColor::Red,
    TagColor::Purple,
    TagColor::Teal,
    TagColor::Pink,
    TagColor::Gray,
];

#[derive(Clone, Debug, Serialize, Deserialize, FromRow, TS)]
#[ts(export)]
pub struct CategoryValue {
    pub id: Uuid,
    pub name: String,
    pub color: TagColor,
    /// Pages with this value.
    #[ts(type = "number")]
    pub page_count: i64,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct Category {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub required_everywhere: bool,
    /// Pages in these topics, or their sub-topics, need a value.
    pub required_topics: Vec<TopicRef>,
    /// In the order people pick from.
    pub values: Vec<CategoryValue>,
    /// Pages that need a value here and don't have one.
    #[ts(type = "number")]
    pub missing_count: i64,
}

/// A value as shown on a page.
#[derive(Clone, Debug, Serialize, Deserialize, FromRow, TS)]
#[ts(export)]
pub struct ValueRef {
    pub id: Uuid,
    pub name: String,
    pub color: TagColor,
}

/// One category as it applies to a page.
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct PageCategory {
    pub category_id: Uuid,
    pub name: String,
    /// Whether the page must have a value before it can be published.
    pub required: bool,
    pub value: Option<ValueRef>,
    /// The values to choose from.
    pub options: Vec<ValueRef>,
}

/// A page's value in a list.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PageValue {
    pub category: String,
    pub id: Uuid,
    pub name: String,
    pub color: TagColor,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateCategoryRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Value names in order.
    pub values: Vec<String>,
    #[serde(default)]
    pub required_everywhere: bool,
    #[serde(default)]
    pub required_topic_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateCategoryRequest {
    #[ts(optional)]
    pub name: Option<String>,
    #[ts(optional)]
    pub description: Option<String>,
    #[ts(optional)]
    pub required_everywhere: Option<bool>,
    #[ts(optional)]
    pub required_topic_ids: Option<Vec<Uuid>>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateValueRequest {
    pub name: String,
    #[serde(default)]
    #[ts(optional)]
    pub color: Option<TagColor>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateValueRequest {
    #[ts(optional)]
    pub name: Option<String>,
    #[ts(optional)]
    pub color: Option<TagColor>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct SetPageCategoryRequest {
    pub category_id: Uuid,
    /// Absent to clear the page's value.
    #[serde(default)]
    pub value_id: Option<Uuid>,
}

fn validate_name(name: &str, what: &str) -> AppResult<String> {
    let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.is_empty() || name.chars().count() > MAX_NAME {
        return Err(AppError::coded(
            StatusCode::BAD_REQUEST,
            "invalid_name",
            format!("Give the {what} a name of up to 50 characters."),
        ));
    }
    Ok(name)
}

fn validate_description(description: &str) -> AppResult<String> {
    let description = description.trim();
    if description.chars().count() > MAX_DESCRIPTION {
        return Err(AppError::coded(
            StatusCode::BAD_REQUEST,
            "invalid_description",
            "Keep the description under 500 characters.",
        ));
    }
    Ok(description.to_string())
}

/// Turn a duplicate-name insert or update into a clear conflict.
fn name_taken(err: sqlx::Error, message: String) -> AppError {
    match err {
        sqlx::Error::Database(db) if db.is_unique_violation() => {
            AppError::coded(StatusCode::CONFLICT, "name_taken", message)
        }
        err => err.into(),
    }
}

pub async fn list(conn: &mut PgConnection) -> sqlx::Result<Vec<Category>> {
    let rows: Vec<(Uuid, String, String, bool, i64)> = sqlx::query_as(
        "SELECT c.id, c.name, c.description, c.required_everywhere,
                (SELECT count(*) FROM pages p
                 WHERE p.archived_at IS NULL
                   AND c.id IN (SELECT category_id FROM kenning_required_categories(p.id))
                   AND NOT EXISTS (SELECT 1 FROM page_categories pc
                                   WHERE pc.page_id = p.id AND pc.category_id = c.id))
         FROM categories c ORDER BY lower(c.name)",
    )
    .fetch_all(&mut *conn)
    .await?;
    let values: Vec<(Uuid, Uuid, String, TagColor, i64)> = sqlx::query_as(
        "SELECT v.category_id, v.id, v.name, v.color,
                (SELECT count(*) FROM page_categories pc JOIN pages p ON p.id = pc.page_id
                 WHERE pc.value_id = v.id AND p.archived_at IS NULL)
         FROM category_values v ORDER BY v.position, v.created_at",
    )
    .fetch_all(&mut *conn)
    .await?;
    let topics: Vec<(Uuid, Uuid, String, String, String)> = sqlx::query_as(
        "SELECT ct.category_id, t.id, t.short_id, t.slug, t.name
         FROM category_topics ct JOIN topics t ON t.id = ct.topic_id
         WHERE t.archived_at IS NULL ORDER BY lower(t.name)",
    )
    .fetch_all(&mut *conn)
    .await?;

    let mut by_category: HashMap<Uuid, Vec<CategoryValue>> = HashMap::new();
    for (category_id, id, name, color, page_count) in values {
        by_category
            .entry(category_id)
            .or_default()
            .push(CategoryValue {
                id,
                name,
                color,
                page_count,
            });
    }
    let mut topics_by_category: HashMap<Uuid, Vec<TopicRef>> = HashMap::new();
    for (category_id, id, short_id, slug, name) in topics {
        topics_by_category
            .entry(category_id)
            .or_default()
            .push(TopicRef {
                id,
                short_id,
                slug,
                name,
            });
    }
    Ok(rows
        .into_iter()
        .map(
            |(id, name, description, required_everywhere, missing_count)| Category {
                id,
                name,
                description,
                required_everywhere,
                required_topics: topics_by_category.remove(&id).unwrap_or_default(),
                values: by_category.remove(&id).unwrap_or_default(),
                missing_count,
            },
        )
        .collect())
}

async fn exists(conn: &mut PgConnection, id: Uuid) -> AppResult<String> {
    sqlx::query_scalar("SELECT name FROM categories WHERE id = $1")
        .bind(id)
        .fetch_optional(conn)
        .await?
        .ok_or(AppError::NotFound)
}

async fn set_required_topics(
    conn: &mut PgConnection,
    org_id: Uuid,
    category_id: Uuid,
    topic_ids: &[Uuid],
) -> AppResult<()> {
    let known: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM topics WHERE id = ANY($1) AND archived_at IS NULL",
    )
    .bind(topic_ids)
    .fetch_one(&mut *conn)
    .await?;
    let mut unique = topic_ids.to_vec();
    unique.sort();
    unique.dedup();
    if known as usize != unique.len() {
        return Err(AppError::coded(
            StatusCode::BAD_REQUEST,
            "unknown_topic",
            "One of those topics no longer exists.",
        ));
    }
    sqlx::query("DELETE FROM category_topics WHERE category_id = $1")
        .bind(category_id)
        .execute(&mut *conn)
        .await?;
    sqlx::query(
        "INSERT INTO category_topics (org_id, category_id, topic_id)
         SELECT $1, $2, id FROM unnest($3::uuid[]) AS id",
    )
    .bind(org_id)
    .bind(category_id)
    .bind(&unique)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn create(
    conn: &mut PgConnection,
    org_id: Uuid,
    actor_id: Uuid,
    req: &CreateCategoryRequest,
) -> AppResult<Uuid> {
    let name = validate_name(&req.name, "category")?;
    let description = validate_description(&req.description)?;
    let mut values: Vec<String> = Vec::new();
    for value in &req.values {
        let value = validate_name(value, "value")?;
        if !values.iter().any(|v| v.eq_ignore_ascii_case(&value)) {
            values.push(value);
        }
    }
    if values.is_empty() || values.len() > MAX_VALUES {
        return Err(AppError::coded(
            StatusCode::BAD_REQUEST,
            "invalid_values",
            "Give the category between 1 and 30 values.",
        ));
    }
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO categories (id, org_id, name, description, required_everywhere, created_by)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id)
    .bind(org_id)
    .bind(&name)
    .bind(&description)
    .bind(req.required_everywhere)
    .bind(actor_id)
    .execute(&mut *conn)
    .await
    .map_err(|err| name_taken(err, format!("There is already a category called {name}.")))?;
    for (position, value) in values.iter().enumerate() {
        sqlx::query(
            "INSERT INTO category_values (id, org_id, category_id, name, color, position)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(Uuid::now_v7())
        .bind(org_id)
        .bind(id)
        .bind(value)
        .bind(VALUE_COLORS[position % VALUE_COLORS.len()])
        .bind(position as i32)
        .execute(&mut *conn)
        .await?;
    }
    set_required_topics(conn, org_id, id, &req.required_topic_ids).await?;
    audit::record(
        conn,
        Event::new("category.created")
            .org(org_id)
            .actor(actor_id)
            .object("category", id)
            .details(json!({
                "name": name,
                "values": values,
                "required_everywhere": req.required_everywhere,
                "required_topic_ids": req.required_topic_ids,
            })),
    )
    .await?;
    Ok(id)
}

pub async fn update(
    conn: &mut PgConnection,
    org_id: Uuid,
    actor_id: Uuid,
    id: Uuid,
    req: &UpdateCategoryRequest,
) -> AppResult<()> {
    exists(conn, id).await?;
    let name = req
        .name
        .as_deref()
        .map(|n| validate_name(n, "category"))
        .transpose()?;
    let description = req
        .description
        .as_deref()
        .map(validate_description)
        .transpose()?;
    sqlx::query(
        "UPDATE categories SET name = coalesce($2, name), description = coalesce($3, description),
                required_everywhere = coalesce($4, required_everywhere), updated_at = now()
         WHERE id = $1",
    )
    .bind(id)
    .bind(&name)
    .bind(&description)
    .bind(req.required_everywhere)
    .execute(&mut *conn)
    .await
    .map_err(|err| {
        name_taken(
            err,
            format!(
                "There is already a category called {}.",
                name.as_deref().unwrap_or_default()
            ),
        )
    })?;
    if let Some(topic_ids) = &req.required_topic_ids {
        set_required_topics(conn, org_id, id, topic_ids).await?;
    }
    audit::record(
        conn,
        Event::new("category.updated")
            .org(org_id)
            .actor(actor_id)
            .object("category", id)
            .details(json!({
                "name": name,
                "required_everywhere": req.required_everywhere,
                "required_topic_ids": req.required_topic_ids,
            })),
    )
    .await?;
    Ok(())
}

/// Delete the category and every page's value in it.
pub async fn delete(
    conn: &mut PgConnection,
    org_id: Uuid,
    actor_id: Uuid,
    id: Uuid,
) -> AppResult<()> {
    let name = exists(conn, id).await?;
    sqlx::query("DELETE FROM page_categories WHERE category_id = $1")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    sqlx::query("DELETE FROM categories WHERE id = $1")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    audit::record(
        conn,
        Event::new("category.deleted")
            .org(org_id)
            .actor(actor_id)
            .object("category", id)
            .details(json!({"name": name})),
    )
    .await?;
    Ok(())
}

pub async fn add_value(
    conn: &mut PgConnection,
    org_id: Uuid,
    actor_id: Uuid,
    category_id: Uuid,
    req: &CreateValueRequest,
) -> AppResult<()> {
    exists(conn, category_id).await?;
    let name = validate_name(&req.name, "value")?;
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM category_values WHERE category_id = $1")
            .bind(category_id)
            .fetch_one(&mut *conn)
            .await?;
    if count as usize >= MAX_VALUES {
        return Err(AppError::coded(
            StatusCode::BAD_REQUEST,
            "invalid_values",
            "A category can have at most 30 values.",
        ));
    }
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO category_values (id, org_id, category_id, name, color, position)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id)
    .bind(org_id)
    .bind(category_id)
    .bind(&name)
    .bind(
        req.color
            .unwrap_or(VALUE_COLORS[count as usize % VALUE_COLORS.len()]),
    )
    .bind(count as i32)
    .execute(&mut *conn)
    .await
    .map_err(|err| name_taken(err, format!("This category already has {name}.")))?;
    audit::record(
        conn,
        Event::new("category.value_added")
            .org(org_id)
            .actor(actor_id)
            .object("category", category_id)
            .details(json!({"value_id": id, "name": name})),
    )
    .await?;
    Ok(())
}

pub async fn update_value(
    conn: &mut PgConnection,
    org_id: Uuid,
    actor_id: Uuid,
    category_id: Uuid,
    value_id: Uuid,
    req: &UpdateValueRequest,
) -> AppResult<()> {
    let name = req
        .name
        .as_deref()
        .map(|n| validate_name(n, "value"))
        .transpose()?;
    let updated = sqlx::query(
        "UPDATE category_values SET name = coalesce($3, name), color = coalesce($4, color)
         WHERE id = $2 AND category_id = $1",
    )
    .bind(category_id)
    .bind(value_id)
    .bind(&name)
    .bind(req.color)
    .execute(&mut *conn)
    .await
    .map_err(|err| {
        name_taken(
            err,
            format!(
                "This category already has {}.",
                name.as_deref().unwrap_or_default()
            ),
        )
    })?;
    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    audit::record(
        conn,
        Event::new("category.value_updated")
            .org(org_id)
            .actor(actor_id)
            .object("category", category_id)
            .details(json!({"value_id": value_id, "name": name, "color": req.color})),
    )
    .await?;
    Ok(())
}

/// Delete a value no page uses. Values in use stay, so no page silently loses
/// its classification; rename the value instead.
pub async fn delete_value(
    conn: &mut PgConnection,
    org_id: Uuid,
    actor_id: Uuid,
    category_id: Uuid,
    value_id: Uuid,
) -> AppResult<()> {
    let in_use: i64 =
        sqlx::query_scalar("SELECT count(*) FROM page_categories WHERE value_id = $1")
            .bind(value_id)
            .fetch_one(&mut *conn)
            .await?;
    if in_use > 0 {
        return Err(AppError::coded(
            StatusCode::CONFLICT,
            "value_in_use",
            format!(
                "{in_use} {} use this value. Change them first, or rename the value.",
                if in_use == 1 { "page" } else { "pages" }
            ),
        ));
    }
    let values: Vec<Uuid> =
        sqlx::query_scalar("SELECT id FROM category_values WHERE category_id = $1")
            .bind(category_id)
            .fetch_all(&mut *conn)
            .await?;
    if !values.contains(&value_id) {
        return Err(AppError::NotFound);
    }
    if values.len() == 1 {
        return Err(AppError::coded(
            StatusCode::CONFLICT,
            "last_value",
            "A category needs at least one value. Delete the category instead.",
        ));
    }
    sqlx::query("DELETE FROM category_values WHERE id = $1")
        .bind(value_id)
        .execute(&mut *conn)
        .await?;
    audit::record(
        conn,
        Event::new("category.value_deleted")
            .org(org_id)
            .actor(actor_id)
            .object("category", category_id)
            .details(json!({"value_id": value_id})),
    )
    .await?;
    Ok(())
}

/// Every category as it applies to the page, with the values to pick from.
pub async fn for_page(conn: &mut PgConnection, page_id: Uuid) -> sqlx::Result<Vec<PageCategory>> {
    let rows: Vec<(Uuid, String, bool, Option<Uuid>)> = sqlx::query_as(
        "SELECT c.id, c.name,
                c.id IN (SELECT category_id FROM kenning_required_categories($1)),
                pc.value_id
         FROM categories c
         LEFT JOIN page_categories pc ON pc.category_id = c.id AND pc.page_id = $1
         ORDER BY lower(c.name)",
    )
    .bind(page_id)
    .fetch_all(&mut *conn)
    .await?;
    let values: Vec<(Uuid, Uuid, String, TagColor)> = sqlx::query_as(
        "SELECT category_id, id, name, color FROM category_values ORDER BY position, created_at",
    )
    .fetch_all(&mut *conn)
    .await?;
    let mut options: HashMap<Uuid, Vec<ValueRef>> = HashMap::new();
    for (category_id, id, name, color) in values {
        options
            .entry(category_id)
            .or_default()
            .push(ValueRef { id, name, color });
    }
    Ok(rows
        .into_iter()
        .map(|(category_id, name, required, value_id)| {
            let options = options.remove(&category_id).unwrap_or_default();
            PageCategory {
                category_id,
                name,
                required,
                value: value_id.and_then(|v| options.iter().find(|o| o.id == v).cloned()),
                options,
            }
        })
        .collect())
}

pub async fn set_for_page(
    conn: &mut PgConnection,
    org_id: Uuid,
    actor_id: Uuid,
    page_id: Uuid,
    req: &SetPageCategoryRequest,
) -> AppResult<()> {
    let category = exists(conn, req.category_id).await.map_err(|_| {
        AppError::coded(
            StatusCode::BAD_REQUEST,
            "unknown_category",
            "That category no longer exists.",
        )
    })?;
    match req.value_id {
        Some(value_id) => {
            let valid: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM category_values WHERE id = $1 AND category_id = $2)",
            )
            .bind(value_id)
            .bind(req.category_id)
            .fetch_one(&mut *conn)
            .await?;
            if !valid {
                return Err(AppError::coded(
                    StatusCode::BAD_REQUEST,
                    "unknown_value",
                    format!("That isn't one of the values in {category}."),
                ));
            }
            sqlx::query(
                "INSERT INTO page_categories (org_id, page_id, category_id, value_id, set_by)
                 VALUES ($1, $2, $3, $4, $5)
                 ON CONFLICT (page_id, category_id)
                 DO UPDATE SET value_id = EXCLUDED.value_id, set_by = EXCLUDED.set_by, updated_at = now()",
            )
            .bind(org_id)
            .bind(page_id)
            .bind(req.category_id)
            .bind(value_id)
            .bind(actor_id)
            .execute(&mut *conn)
            .await?;
        }
        None => {
            sqlx::query("DELETE FROM page_categories WHERE page_id = $1 AND category_id = $2")
                .bind(page_id)
                .bind(req.category_id)
                .execute(&mut *conn)
                .await?;
        }
    }
    audit::record(
        conn,
        Event::new("page.category_set")
            .org(org_id)
            .actor(actor_id)
            .object("page", page_id)
            .details(json!({"category_id": req.category_id, "value_id": req.value_id})),
    )
    .await?;
    Ok(())
}

/// Names of the required categories the page has no value in, by name.
pub async fn missing_for_page(conn: &mut PgConnection, page_id: Uuid) -> sqlx::Result<Vec<String>> {
    sqlx::query_scalar(
        "SELECT c.name FROM kenning_required_categories($1) r JOIN categories c ON c.id = r.category_id
         WHERE NOT EXISTS (SELECT 1 FROM page_categories pc
                           WHERE pc.page_id = $1 AND pc.category_id = r.category_id)
         ORDER BY lower(c.name)",
    )
    .bind(page_id)
    .fetch_all(conn)
    .await
}
