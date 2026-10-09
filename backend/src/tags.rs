//! Tags: flat, colored labels on pages. Names are unique per organization
//! ignoring case and punctuation, so "Onboarding" and "onboarding" are one tag.

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
};

pub const MAX_NAME: usize = 50;
pub const MAX_PER_PAGE: usize = 20;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, TS)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[ts(export)]
pub enum TagColor {
    #[default]
    Gray,
    Blue,
    Green,
    Amber,
    Red,
    Purple,
    Teal,
    Pink,
}

/// A tag on a page.
#[derive(Clone, Debug, Serialize, Deserialize, FromRow, TS)]
#[ts(export)]
pub struct TagRef {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub color: TagColor,
}

/// A tag on the tags page.
#[derive(Debug, Serialize, FromRow, TS)]
#[ts(export)]
pub struct Tag {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub color: TagColor,
    #[ts(type = "number")]
    pub page_count: i64,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateTagRequest {
    pub name: String,
    #[serde(default)]
    #[ts(optional)]
    pub color: Option<TagColor>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateTagRequest {
    #[ts(optional)]
    pub name: Option<String>,
    #[ts(optional)]
    pub color: Option<TagColor>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct MergeTagRequest {
    /// The tag that stays; pages tagged with the merged tag get this one.
    pub into_id: Uuid,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct SetPageTagsRequest {
    /// Tag names; names that don't match a tag yet create one.
    pub names: Vec<String>,
}

/// The tag's display name and its slug, which decides whether two names are one tag.
pub fn normalize(name: &str) -> AppResult<(String, String)> {
    let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
    let slug = orgs::slugify(&name);
    if slug.is_empty() || name.chars().count() > MAX_NAME {
        return Err(AppError::coded(
            StatusCode::BAD_REQUEST,
            "invalid_tag",
            "A tag needs a letter or number and at most 50 characters.",
        ));
    }
    Ok((name, slug))
}

pub async fn list(conn: &mut PgConnection) -> sqlx::Result<Vec<Tag>> {
    sqlx::query_as(
        "SELECT t.id, t.name, t.slug, t.color,
                (SELECT count(*) FROM page_tags pt JOIN pages p ON p.id = pt.page_id
                 WHERE pt.tag_id = t.id AND p.archived_at IS NULL) AS page_count
         FROM tags t ORDER BY lower(t.name)",
    )
    .fetch_all(conn)
    .await
}

async fn find(conn: &mut PgConnection, id: Uuid) -> AppResult<TagRef> {
    sqlx::query_as("SELECT id, name, slug, color FROM tags WHERE id = $1")
        .bind(id)
        .fetch_optional(conn)
        .await?
        .ok_or(AppError::NotFound)
}

fn taken(name: &str) -> AppError {
    AppError::coded(
        StatusCode::CONFLICT,
        "tag_exists",
        format!("There is already a tag called {name}. Merge the two instead."),
    )
}

pub async fn create(
    conn: &mut PgConnection,
    org_id: Uuid,
    actor_id: Uuid,
    req: &CreateTagRequest,
) -> AppResult<TagRef> {
    let (name, slug) = normalize(&req.name)?;
    let tag: Option<TagRef> = sqlx::query_as(
        "INSERT INTO tags (id, org_id, name, slug, color, created_by) VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (org_id, slug) DO NOTHING
         RETURNING id, name, slug, color",
    )
    .bind(Uuid::now_v7())
    .bind(org_id)
    .bind(&name)
    .bind(&slug)
    .bind(req.color.unwrap_or_default())
    .bind(actor_id)
    .fetch_optional(&mut *conn)
    .await?;
    let tag = tag.ok_or_else(|| taken(&name))?;
    audit::record(
        conn,
        Event::new("tag.created")
            .org(org_id)
            .actor(actor_id)
            .object("tag", tag.id)
            .details(json!({"name": tag.name})),
    )
    .await?;
    Ok(tag)
}

pub async fn update(
    conn: &mut PgConnection,
    org_id: Uuid,
    actor_id: Uuid,
    id: Uuid,
    req: &UpdateTagRequest,
) -> AppResult<TagRef> {
    let tag = find(conn, id).await?;
    let (name, slug) = match &req.name {
        Some(name) => normalize(name)?,
        None => (tag.name.clone(), tag.slug.clone()),
    };
    let color = req.color.unwrap_or(tag.color);
    let clash: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM tags WHERE slug = $1 AND id <> $2)")
            .bind(&slug)
            .bind(id)
            .fetch_one(&mut *conn)
            .await?;
    if clash {
        return Err(taken(&name));
    }
    let updated: TagRef = sqlx::query_as(
        "UPDATE tags SET name = $2, slug = $3, color = $4 WHERE id = $1
         RETURNING id, name, slug, color",
    )
    .bind(id)
    .bind(&name)
    .bind(&slug)
    .bind(color)
    .fetch_one(&mut *conn)
    .await?;
    audit::record(
        conn,
        Event::new("tag.updated")
            .org(org_id)
            .actor(actor_id)
            .object("tag", id)
            .details(json!({"from": tag.name, "name": name, "color": color})),
    )
    .await?;
    Ok(updated)
}

/// Move every page from tag `id` to `into_id`, then delete tag `id`.
pub async fn merge(
    conn: &mut PgConnection,
    org_id: Uuid,
    actor_id: Uuid,
    id: Uuid,
    into_id: Uuid,
) -> AppResult<()> {
    if id == into_id {
        return Err(AppError::coded(
            StatusCode::BAD_REQUEST,
            "invalid_merge",
            "Pick a different tag to merge into.",
        ));
    }
    let tag = find(conn, id).await?;
    let into = find(conn, into_id).await?;
    sqlx::query(
        "INSERT INTO page_tags (org_id, page_id, tag_id, added_by)
         SELECT org_id, page_id, $2, $3 FROM page_tags WHERE tag_id = $1
         ON CONFLICT DO NOTHING",
    )
    .bind(id)
    .bind(into_id)
    .bind(actor_id)
    .execute(&mut *conn)
    .await?;
    sqlx::query("DELETE FROM tags WHERE id = $1")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    audit::record(
        conn,
        Event::new("tag.merged")
            .org(org_id)
            .actor(actor_id)
            .object("tag", into_id)
            .details(json!({"merged_id": id, "merged_name": tag.name, "into_name": into.name})),
    )
    .await?;
    Ok(())
}

pub async fn delete(
    conn: &mut PgConnection,
    org_id: Uuid,
    actor_id: Uuid,
    id: Uuid,
) -> AppResult<()> {
    let tag = find(conn, id).await?;
    sqlx::query("DELETE FROM tags WHERE id = $1")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    audit::record(
        conn,
        Event::new("tag.deleted")
            .org(org_id)
            .actor(actor_id)
            .object("tag", id)
            .details(json!({"name": tag.name})),
    )
    .await?;
    Ok(())
}

pub async fn for_page(conn: &mut PgConnection, page_id: Uuid) -> sqlx::Result<Vec<TagRef>> {
    sqlx::query_as(
        "SELECT t.id, t.name, t.slug, t.color FROM page_tags pt JOIN tags t ON t.id = pt.tag_id
         WHERE pt.page_id = $1 ORDER BY lower(t.name)",
    )
    .bind(page_id)
    .fetch_all(conn)
    .await
}

/// Make the page's tags exactly `names`, creating tags that don't exist yet.
pub async fn set_for_page(
    conn: &mut PgConnection,
    org_id: Uuid,
    actor_id: Uuid,
    page_id: Uuid,
    names: &[String],
) -> AppResult<Vec<TagRef>> {
    let mut wanted: Vec<(String, String)> = Vec::new();
    for name in names {
        let (name, slug) = normalize(name)?;
        if !wanted.iter().any(|(_, s)| *s == slug) {
            wanted.push((name, slug));
        }
    }
    if wanted.len() > MAX_PER_PAGE {
        return Err(AppError::coded(
            StatusCode::BAD_REQUEST,
            "too_many_tags",
            "A page can have at most 20 tags.",
        ));
    }
    let (names, slugs): (Vec<String>, Vec<String>) = wanted.into_iter().unzip();
    let new_ids: Vec<Uuid> = slugs.iter().map(|_| Uuid::now_v7()).collect();
    let created: Vec<TagRef> = sqlx::query_as(
        "INSERT INTO tags (id, org_id, name, slug, created_by)
         SELECT id, $1, name, slug, $2 FROM unnest($3::uuid[], $4::text[], $5::text[]) AS n(id, name, slug)
         ON CONFLICT (org_id, slug) DO NOTHING
         RETURNING id, name, slug, color",
    )
    .bind(org_id)
    .bind(actor_id)
    .bind(&new_ids)
    .bind(&names)
    .bind(&slugs)
    .fetch_all(&mut *conn)
    .await?;
    for tag in &created {
        audit::record(
            conn,
            Event::new("tag.created")
                .org(org_id)
                .actor(actor_id)
                .object("tag", tag.id)
                .details(json!({"name": tag.name})),
        )
        .await?;
    }
    let ids: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM tags WHERE slug = ANY($1)")
        .bind(&slugs)
        .fetch_all(&mut *conn)
        .await?;
    sqlx::query("DELETE FROM page_tags WHERE page_id = $1 AND NOT (tag_id = ANY($2))")
        .bind(page_id)
        .bind(&ids)
        .execute(&mut *conn)
        .await?;
    sqlx::query(
        "INSERT INTO page_tags (org_id, page_id, tag_id, added_by)
         SELECT $1, $2, id, $3 FROM unnest($4::uuid[]) AS id
         ON CONFLICT DO NOTHING",
    )
    .bind(org_id)
    .bind(page_id)
    .bind(actor_id)
    .bind(&ids)
    .execute(&mut *conn)
    .await?;
    audit::record(
        conn,
        Event::new("page.tags_changed")
            .org(org_id)
            .actor(actor_id)
            .object("page", page_id)
            .details(json!({"tags": names})),
    )
    .await?;
    for_page(conn, page_id).await.map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_collapse_spaces_and_share_slugs_across_case() {
        assert_eq!(
            normalize("  Getting   Started ").unwrap(),
            ("Getting Started".to_string(), "getting-started".to_string())
        );
        assert_eq!(
            normalize("onboarding").unwrap().1,
            normalize("Onboarding!").unwrap().1
        );
        assert!(normalize("!!!").is_err());
        assert!(normalize(&"x".repeat(51)).is_err());
    }
}
