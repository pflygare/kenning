//! Topics: a tree of collections that organize pages. A page can sit in
//! several topics; a topic lists its sub-topics and pages.

use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, PgConnection};
use ts_rs::TS;
use uuid::Uuid;

use crate::{
    AppError, AppResult,
    audit::{self, Event},
    orgs,
    pages::{self, ListFilter, PageSummary},
};

pub const MAX_NAME: usize = 100;
pub const MAX_DESCRIPTION: usize = 1000;

/// A topic in the tree. The whole tree is small enough to send at once.
#[derive(Clone, Debug, Serialize, FromRow, TS)]
#[ts(export)]
pub struct Topic {
    pub id: Uuid,
    pub short_id: String,
    pub slug: String,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub description: String,
    pub position: i32,
    /// Pages directly in this topic, not counting sub-topics.
    #[ts(type = "number")]
    pub page_count: i64,
}

/// Enough of a topic to link to it.
#[derive(Clone, Debug, Serialize, Deserialize, FromRow, TS)]
#[ts(export)]
pub struct TopicRef {
    pub id: Uuid,
    pub short_id: String,
    pub slug: String,
    pub name: String,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct TopicDetail {
    pub topic: Topic,
    /// Ancestors, the root first.
    pub path: Vec<TopicRef>,
    pub children: Vec<Topic>,
    pub pages: Vec<PageSummary>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateTopicRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Absent for a top-level topic.
    #[serde(default)]
    pub parent_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateTopicRequest {
    #[ts(optional)]
    pub name: Option<String>,
    #[ts(optional)]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct MoveTopicRequest {
    /// The new parent; absent to make it a top-level topic.
    #[serde(default)]
    pub parent_id: Option<Uuid>,
    /// Where among its new siblings it goes, 0 being first.
    pub index: u32,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct SetPageTopicsRequest {
    pub topic_ids: Vec<Uuid>,
}

fn validate_name(name: &str) -> AppResult<String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME {
        return Err(AppError::coded(
            StatusCode::BAD_REQUEST,
            "invalid_name",
            "Give the topic a name of up to 100 characters.",
        ));
    }
    Ok(name.to_string())
}

fn validate_description(description: &str) -> AppResult<String> {
    let description = description.trim();
    if description.chars().count() > MAX_DESCRIPTION {
        return Err(AppError::coded(
            StatusCode::BAD_REQUEST,
            "invalid_description",
            "Keep the description under 1,000 characters.",
        ));
    }
    Ok(description.to_string())
}

fn slug_for(name: &str) -> String {
    let slug = orgs::slugify(name);
    if slug.is_empty() {
        "topic".to_string()
    } else {
        slug
    }
}

const TOPIC_COLUMNS: &str =
    "t.id, t.short_id, t.slug, t.parent_id, t.name, t.description, t.position,
     (SELECT count(*) FROM page_topics pt JOIN pages p ON p.id = pt.page_id
      WHERE pt.topic_id = t.id AND p.archived_at IS NULL) AS page_count";

/// Every topic in the organization, siblings in order.
pub async fn list(conn: &mut PgConnection) -> sqlx::Result<Vec<Topic>> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {TOPIC_COLUMNS} FROM topics t WHERE t.archived_at IS NULL
         ORDER BY t.position, lower(t.name)"
    )))
    .fetch_all(conn)
    .await
}

async fn find(conn: &mut PgConnection, short_id: &str) -> AppResult<Topic> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {TOPIC_COLUMNS} FROM topics t WHERE t.short_id = $1 AND t.archived_at IS NULL"
    )))
    .bind(short_id)
    .fetch_optional(conn)
    .await?
    .ok_or(AppError::NotFound)
}

async fn find_by_id(conn: &mut PgConnection, id: Uuid) -> AppResult<Topic> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {TOPIC_COLUMNS} FROM topics t WHERE t.id = $1 AND t.archived_at IS NULL"
    )))
    .bind(id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| {
        AppError::coded(
            StatusCode::BAD_REQUEST,
            "unknown_topic",
            "That topic no longer exists.",
        )
    })
}

pub async fn detail(conn: &mut PgConnection, short_id: &str) -> AppResult<TopicDetail> {
    let topic = find(conn, short_id).await?;
    let path = sqlx::query_as(
        "WITH RECURSIVE up AS (
             SELECT id, parent_id, 0 AS depth FROM topics WHERE id = $1
             UNION ALL
             SELECT t.id, t.parent_id, up.depth + 1 FROM topics t JOIN up ON t.id = up.parent_id
         )
         SELECT t.id, t.short_id, t.slug, t.name FROM up JOIN topics t ON t.id = up.id
         WHERE up.depth > 0 ORDER BY up.depth DESC",
    )
    .bind(topic.id)
    .fetch_all(&mut *conn)
    .await?;
    let children = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {TOPIC_COLUMNS} FROM topics t WHERE t.parent_id = $1 AND t.archived_at IS NULL
         ORDER BY t.position, lower(t.name)"
    )))
    .bind(topic.id)
    .fetch_all(&mut *conn)
    .await?;
    let pages = pages::list(
        conn,
        &ListFilter {
            topic_id: Some(topic.id),
            ..ListFilter::default()
        },
    )
    .await?;
    Ok(TopicDetail {
        topic,
        path,
        children,
        pages,
    })
}

pub async fn create(
    conn: &mut PgConnection,
    org_id: Uuid,
    actor_id: Uuid,
    req: &CreateTopicRequest,
) -> AppResult<String> {
    let name = validate_name(&req.name)?;
    let description = validate_description(&req.description)?;
    if let Some(parent_id) = req.parent_id {
        find_by_id(conn, parent_id).await?;
    }
    let id = Uuid::now_v7();
    let short_id = pages::new_short_id();
    // New topics go last among their siblings.
    sqlx::query(
        "INSERT INTO topics (id, org_id, short_id, slug, parent_id, name, description, position, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7,
                 (SELECT coalesce(max(position) + 1, 0) FROM topics
                  WHERE parent_id IS NOT DISTINCT FROM $5 AND archived_at IS NULL),
                 $8)",
    )
    .bind(id)
    .bind(org_id)
    .bind(&short_id)
    .bind(slug_for(&name))
    .bind(req.parent_id)
    .bind(&name)
    .bind(&description)
    .bind(actor_id)
    .execute(&mut *conn)
    .await?;
    audit::record(
        conn,
        Event::new("topic.created")
            .org(org_id)
            .actor(actor_id)
            .object("topic", id)
            .details(json!({"name": name, "parent_id": req.parent_id})),
    )
    .await?;
    Ok(short_id)
}

pub async fn update(
    conn: &mut PgConnection,
    org_id: Uuid,
    actor_id: Uuid,
    short_id: &str,
    req: &UpdateTopicRequest,
) -> AppResult<()> {
    let topic = find(conn, short_id).await?;
    let name = match &req.name {
        Some(name) => validate_name(name)?,
        None => topic.name.clone(),
    };
    let description = match &req.description {
        Some(description) => validate_description(description)?,
        None => topic.description.clone(),
    };
    sqlx::query(
        "UPDATE topics SET name = $2, slug = $3, description = $4, updated_at = now() WHERE id = $1",
    )
    .bind(topic.id)
    .bind(&name)
    .bind(slug_for(&name))
    .bind(&description)
    .execute(&mut *conn)
    .await?;
    audit::record(
        conn,
        Event::new("topic.updated")
            .org(org_id)
            .actor(actor_id)
            .object("topic", topic.id)
            .details(json!({"name": name})),
    )
    .await?;
    Ok(())
}

/// Put the topic under `parent_id` (or at the top) at `index` among its siblings.
pub async fn move_to(
    conn: &mut PgConnection,
    org_id: Uuid,
    actor_id: Uuid,
    short_id: &str,
    req: &MoveTopicRequest,
) -> AppResult<()> {
    let topic = find(conn, short_id).await?;
    if let Some(parent_id) = req.parent_id {
        find_by_id(conn, parent_id).await?;
        // A topic cannot move into itself or anything below it.
        let inside: bool = sqlx::query_scalar(
            "WITH RECURSIVE up AS (
                 SELECT id, parent_id FROM topics WHERE id = $1
                 UNION ALL
                 SELECT t.id, t.parent_id FROM topics t JOIN up ON t.id = up.parent_id
             )
             SELECT EXISTS (SELECT 1 FROM up WHERE id = $2)",
        )
        .bind(parent_id)
        .bind(topic.id)
        .fetch_one(&mut *conn)
        .await?;
        if inside {
            return Err(AppError::coded(
                StatusCode::BAD_REQUEST,
                "invalid_move",
                "A topic can't move inside itself.",
            ));
        }
    }

    let mut siblings: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM topics
         WHERE parent_id IS NOT DISTINCT FROM $1 AND archived_at IS NULL AND id <> $2
         ORDER BY position, lower(name)",
    )
    .bind(req.parent_id)
    .bind(topic.id)
    .fetch_all(&mut *conn)
    .await?;
    let index = (req.index as usize).min(siblings.len());
    siblings.insert(index, topic.id);
    sqlx::query(
        "UPDATE topics t SET position = s.position - 1,
                parent_id = CASE WHEN t.id = $2 THEN $3 ELSE t.parent_id END,
                updated_at = CASE WHEN t.id = $2 THEN now() ELSE t.updated_at END
         FROM unnest($1::uuid[]) WITH ORDINALITY AS s(id, position)
         WHERE t.id = s.id",
    )
    .bind(&siblings)
    .bind(topic.id)
    .bind(req.parent_id)
    .execute(&mut *conn)
    .await?;
    audit::record(
        conn,
        Event::new("topic.moved")
            .org(org_id)
            .actor(actor_id)
            .object("topic", topic.id)
            .details(json!({"parent_id": req.parent_id, "index": index})),
    )
    .await?;
    Ok(())
}

pub async fn archive(
    conn: &mut PgConnection,
    org_id: Uuid,
    actor_id: Uuid,
    short_id: &str,
) -> AppResult<()> {
    let topic = find(conn, short_id).await?;
    let has_children: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM topics WHERE parent_id = $1 AND archived_at IS NULL)",
    )
    .bind(topic.id)
    .fetch_one(&mut *conn)
    .await?;
    if has_children {
        return Err(AppError::coded(
            StatusCode::CONFLICT,
            "topic_has_subtopics",
            "Move or archive its sub-topics first.",
        ));
    }
    sqlx::query("UPDATE topics SET archived_at = now() WHERE id = $1")
        .bind(topic.id)
        .execute(&mut *conn)
        .await?;
    audit::record(
        conn,
        Event::new("topic.archived")
            .org(org_id)
            .actor(actor_id)
            .object("topic", topic.id),
    )
    .await?;
    Ok(())
}

/// Topics the page is in, by name.
pub async fn for_page(conn: &mut PgConnection, page_id: Uuid) -> sqlx::Result<Vec<TopicRef>> {
    sqlx::query_as(
        "SELECT t.id, t.short_id, t.slug, t.name FROM page_topics pt
         JOIN topics t ON t.id = pt.topic_id
         WHERE pt.page_id = $1 AND t.archived_at IS NULL
         ORDER BY lower(t.name)",
    )
    .bind(page_id)
    .fetch_all(conn)
    .await
}

/// Make the page's topics exactly `topic_ids`.
pub async fn set_for_page(
    conn: &mut PgConnection,
    org_id: Uuid,
    actor_id: Uuid,
    page_id: Uuid,
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
    sqlx::query("DELETE FROM page_topics WHERE page_id = $1 AND NOT (topic_id = ANY($2))")
        .bind(page_id)
        .bind(&unique)
        .execute(&mut *conn)
        .await?;
    sqlx::query(
        "INSERT INTO page_topics (org_id, page_id, topic_id, added_by)
         SELECT $1, $2, id, $3 FROM unnest($4::uuid[]) AS id
         ON CONFLICT DO NOTHING",
    )
    .bind(org_id)
    .bind(page_id)
    .bind(actor_id)
    .bind(&unique)
    .execute(&mut *conn)
    .await?;
    audit::record(
        conn,
        Event::new("page.topics_changed")
            .org(org_id)
            .actor(actor_id)
            .object("page", page_id)
            .details(json!({"topic_ids": unique})),
    )
    .await?;
    Ok(())
}
