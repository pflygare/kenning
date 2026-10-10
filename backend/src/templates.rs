//! Templates: starting points for new pages, with a title, text, topics and
//! tags. A page started from one gets a copy with placeholders filled in, so
//! editing a template later never changes pages already made from it.

use axum::http::StatusCode;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, PgConnection, types::Json};
use ts_rs::TS;
use uuid::Uuid;

use crate::{
    AppError, AppResult,
    audit::{self, Event},
    pages,
    tags::{self, TagRef},
    topics::TopicRef,
};

pub const MAX_NAME: usize = 100;
pub const MAX_DESCRIPTION: usize = 500;

/// A template in a list or the New page picker.
#[derive(Debug, Serialize, FromRow, TS)]
#[ts(export)]
pub struct TemplateSummary {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    /// The new page's title, placeholders unfilled; empty to let the writer choose.
    pub title: String,
    /// Whether the text uses {{title}}, which needs the title before the page starts.
    pub uses_title: bool,
    pub updated_at: DateTime<Utc>,
    pub updated_by_name: String,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct TemplateDetail {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub title: String,
    pub body_md: String,
    pub topics: Vec<TopicRef>,
    pub tags: Vec<TagRef>,
    pub updated_at: DateTime<Utc>,
    pub updated_by_name: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateTemplateRequest {
    pub name: String,
    #[serde(default)]
    #[ts(optional)]
    pub description: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub title: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub body_md: Option<String>,
    /// Preset topics, e.g. those of the page being saved as a template.
    #[serde(default)]
    #[ts(optional)]
    pub topic_ids: Option<Vec<Uuid>>,
    #[serde(default)]
    #[ts(optional)]
    pub tag_names: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateTemplateRequest {
    pub name: String,
    pub description: String,
    pub title: String,
    pub body_md: String,
}

/// What a page started from a template begins with.
pub struct Start {
    pub template_id: Uuid,
    pub title: String,
    pub body_md: String,
    pub topic_ids: Vec<Uuid>,
    pub tag_names: Vec<String>,
}

fn validate(name: &str, description: &str, title: &str, body_md: &str) -> AppResult<()> {
    if name.trim().is_empty() || name.trim().chars().count() > MAX_NAME {
        return Err(AppError::coded(
            StatusCode::BAD_REQUEST,
            "invalid_name",
            "Give the template a name of up to 100 characters.",
        ));
    }
    if description.trim().chars().count() > MAX_DESCRIPTION {
        return Err(AppError::coded(
            StatusCode::BAD_REQUEST,
            "invalid_description",
            "Keep the description under 500 characters.",
        ));
    }
    if title.trim().chars().count() > pages::MAX_TITLE {
        return Err(AppError::coded(
            StatusCode::BAD_REQUEST,
            "invalid_title",
            "Keep the page title under 200 characters.",
        ));
    }
    pages::validate_body(body_md)
}

/// Fill {{date}}, {{author}} and {{title}}. Unknown placeholders stay as typed.
pub fn fill(text: &str, date: NaiveDate, author: &str, title: &str) -> String {
    text.replace("{{date}}", &date.format("%Y-%m-%d").to_string())
        .replace("{{author}}", author)
        .replace("{{title}}", title)
}

pub async fn list(conn: &mut PgConnection) -> sqlx::Result<Vec<TemplateSummary>> {
    sqlx::query_as(
        "SELECT t.id, t.name, t.description, t.title,
                position('{{title}}' IN t.body_md) > 0 AS uses_title,
                t.updated_at, u.name AS updated_by_name
         FROM templates t JOIN users u ON u.id = t.updated_by
         WHERE t.archived_at IS NULL
         ORDER BY lower(t.name)",
    )
    .fetch_all(conn)
    .await
}

#[derive(FromRow)]
struct TemplateRow {
    id: Uuid,
    name: String,
    description: String,
    title: String,
    body_md: String,
    updated_at: DateTime<Utc>,
    updated_by_name: String,
    topics: Json<Vec<TopicRef>>,
    tags: Json<Vec<TagRef>>,
}

pub async fn detail(conn: &mut PgConnection, id: Uuid) -> AppResult<TemplateDetail> {
    let row: TemplateRow = sqlx::query_as(
        "SELECT t.id, t.name, t.description, t.title, t.body_md, t.updated_at,
                u.name AS updated_by_name,
                coalesce((SELECT json_agg(json_build_object('id', tp.id, 'short_id', tp.short_id,
                                                            'slug', tp.slug, 'name', tp.name)
                                          ORDER BY lower(tp.name))
                          FROM template_topics tt JOIN topics tp ON tp.id = tt.topic_id
                          WHERE tt.template_id = t.id AND tp.archived_at IS NULL), '[]') AS topics,
                coalesce((SELECT json_agg(json_build_object('id', g.id, 'name', g.name,
                                                            'slug', g.slug, 'color', g.color)
                                          ORDER BY lower(g.name))
                          FROM template_tags tg JOIN tags g ON g.id = tg.tag_id
                          WHERE tg.template_id = t.id), '[]') AS tags
         FROM templates t JOIN users u ON u.id = t.updated_by
         WHERE t.id = $1 AND t.archived_at IS NULL",
    )
    .bind(id)
    .fetch_optional(conn)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(TemplateDetail {
        id: row.id,
        name: row.name,
        description: row.description,
        title: row.title,
        body_md: row.body_md,
        topics: row.topics.0,
        tags: row.tags.0,
        updated_at: row.updated_at,
        updated_by_name: row.updated_by_name,
    })
}

pub async fn create(
    conn: &mut PgConnection,
    org_id: Uuid,
    actor_id: Uuid,
    req: &CreateTemplateRequest,
) -> AppResult<Uuid> {
    let description = req.description.as_deref().unwrap_or("");
    let title = req.title.as_deref().unwrap_or("");
    let body_md = req.body_md.as_deref().unwrap_or("");
    validate(&req.name, description, title, body_md)?;
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO templates (id, org_id, name, description, title, body_md, created_by, updated_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $7)",
    )
    .bind(id)
    .bind(org_id)
    .bind(req.name.trim())
    .bind(description.trim())
    .bind(title.trim())
    .bind(body_md)
    .bind(actor_id)
    .execute(&mut *conn)
    .await?;
    if let Some(topic_ids) = &req.topic_ids {
        set_topics(conn, org_id, id, topic_ids).await?;
    }
    if let Some(names) = &req.tag_names {
        set_tags(conn, org_id, actor_id, id, names).await?;
    }
    audit::record(
        conn,
        Event::new("template.created")
            .org(org_id)
            .actor(actor_id)
            .object("template", id)
            .details(json!({"name": req.name.trim()})),
    )
    .await?;
    Ok(id)
}

pub async fn update(
    conn: &mut PgConnection,
    org_id: Uuid,
    actor_id: Uuid,
    id: Uuid,
    req: &UpdateTemplateRequest,
) -> AppResult<()> {
    validate(&req.name, &req.description, &req.title, &req.body_md)?;
    let updated = sqlx::query(
        "UPDATE templates SET name = $2, description = $3, title = $4, body_md = $5,
                              updated_by = $6, updated_at = now()
         WHERE id = $1 AND archived_at IS NULL",
    )
    .bind(id)
    .bind(req.name.trim())
    .bind(req.description.trim())
    .bind(req.title.trim())
    .bind(&req.body_md)
    .bind(actor_id)
    .execute(&mut *conn)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    // Templates autosave, so record one event per editing session, not per keystroke.
    let recent: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM audit_events
                        WHERE object_kind = 'template' AND object_id = $1 AND actor_id = $2
                          AND action = 'template.updated'
                          AND created_at > now() - $3::interval)",
    )
    .bind(id)
    .bind(actor_id)
    .bind(format!("{} minutes", pages::COLLAPSE_WINDOW.num_minutes()))
    .fetch_one(&mut *conn)
    .await?;
    if !recent {
        audit::record(
            conn,
            Event::new("template.updated")
                .org(org_id)
                .actor(actor_id)
                .object("template", id)
                .details(json!({"name": req.name.trim()})),
        )
        .await?;
    }
    Ok(())
}

async fn exists(conn: &mut PgConnection, id: Uuid) -> AppResult<()> {
    sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM templates WHERE id = $1 AND archived_at IS NULL FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(conn)
    .await?
    .map(|_| ())
    .ok_or(AppError::NotFound)
}

/// Make the template's topics exactly `topic_ids`.
pub async fn set_topics(
    conn: &mut PgConnection,
    org_id: Uuid,
    id: Uuid,
    topic_ids: &[Uuid],
) -> AppResult<()> {
    exists(conn, id).await?;
    let mut unique = topic_ids.to_vec();
    unique.sort();
    unique.dedup();
    let known: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM topics WHERE id = ANY($1) AND archived_at IS NULL",
    )
    .bind(&unique)
    .fetch_one(&mut *conn)
    .await?;
    if known as usize != unique.len() {
        return Err(AppError::coded(
            StatusCode::BAD_REQUEST,
            "unknown_topic",
            "One of those topics no longer exists.",
        ));
    }
    sqlx::query("DELETE FROM template_topics WHERE template_id = $1")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    sqlx::query(
        "INSERT INTO template_topics (org_id, template_id, topic_id)
         SELECT $1, $2, id FROM unnest($3::uuid[]) AS id",
    )
    .bind(org_id)
    .bind(id)
    .bind(&unique)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Make the template's tags exactly `names`, creating tags that don't exist yet.
pub async fn set_tags(
    conn: &mut PgConnection,
    org_id: Uuid,
    actor_id: Uuid,
    id: Uuid,
    names: &[String],
) -> AppResult<()> {
    exists(conn, id).await?;
    let ids = tags::ensure(conn, org_id, actor_id, names).await?;
    sqlx::query("DELETE FROM template_tags WHERE template_id = $1")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    sqlx::query(
        "INSERT INTO template_tags (org_id, template_id, tag_id)
         SELECT $1, $2, id FROM unnest($3::uuid[]) AS id",
    )
    .bind(org_id)
    .bind(id)
    .bind(&ids)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn archive(
    conn: &mut PgConnection,
    org_id: Uuid,
    actor_id: Uuid,
    id: Uuid,
) -> AppResult<()> {
    exists(conn, id).await?;
    sqlx::query("UPDATE templates SET archived_at = now() WHERE id = $1")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    audit::record(
        conn,
        Event::new("template.archived")
            .org(org_id)
            .actor(actor_id)
            .object("template", id),
    )
    .await?;
    Ok(())
}

/// What a new page from template `id` starts with. `title` is the writer's
/// choice, used when the template doesn't set one.
pub async fn start(
    conn: &mut PgConnection,
    id: Uuid,
    date: NaiveDate,
    author: &str,
    title: &str,
) -> AppResult<Start> {
    let template = detail(conn, id).await?;
    let title = if template.title.is_empty() {
        title.to_string()
    } else {
        fill(&template.title, date, author, title)
    };
    Ok(Start {
        template_id: id,
        body_md: fill(&template.body_md, date, author, &title),
        title,
        topic_ids: template.topics.into_iter().map(|t| t.id).collect(),
        tag_names: template.tags.into_iter().map(|t| t.name).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_are_filled() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 9).unwrap();
        assert_eq!(
            fill(
                "{{title}}: notes by {{author}} on {{date}}, {{date}} {{other}}",
                date,
                "Ada",
                "Standup"
            ),
            "Standup: notes by Ada on 2026-10-09, 2026-10-09 {{other}}"
        );
    }
}
