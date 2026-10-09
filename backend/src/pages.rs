//! Pages, drafts and publishing.
//!
//! A page's content lives in `page_revisions`. Editors always work on the
//! page's current revision; readers see its published revision. Saving while
//! the page has no draft starts one; publishing points the published revision
//! at the draft; discarding points the draft back at what is published.

use axum::http::StatusCode;
use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, PgConnection, types::Json};
use ts_rs::TS;
use uuid::Uuid;

use crate::{
    AppError, AppResult,
    audit::{self, Event},
    orgs,
    tags::{self, TagRef},
    topics::{self, TopicRef},
};

/// Saves by the same author within this long update one revision instead of
/// adding another, so history holds editing sessions rather than keystrokes.
pub const COLLAPSE_WINDOW: Duration = Duration::minutes(10);

pub const MAX_TITLE: usize = 200;
pub const MAX_BODY: usize = 1_000_000;

/// A page in a list.
#[derive(Debug, Serialize, FromRow, TS)]
#[ts(export)]
pub struct PageSummary {
    pub short_id: String,
    pub slug: String,
    /// The published title, or the draft's for a page never published.
    pub title: String,
    pub published: bool,
    /// Whether there are unpublished changes.
    pub has_draft: bool,
    pub updated_at: DateTime<Utc>,
    pub updated_by_name: String,
    #[ts(as = "Vec<TagRef>")]
    pub tags: Json<Vec<TagRef>>,
}

/// Which pages a list shows; every filter given must match.
#[derive(Debug, Default, Deserialize)]
pub struct ListFilter {
    /// A tag's slug.
    pub tag: Option<String>,
    pub topic_id: Option<Uuid>,
}

/// One version of a page's content.
#[derive(Clone, Debug, Serialize, FromRow, TS)]
#[ts(export)]
pub struct Version {
    pub revision_id: Uuid,
    pub title: String,
    pub body_md: String,
    pub author_name: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct PageDetail {
    pub id: Uuid,
    pub short_id: String,
    pub slug: String,
    /// What readers see; absent until the page is first published.
    pub published: Option<Version>,
    pub published_at: Option<DateTime<Utc>>,
    pub published_by_name: Option<String>,
    /// Unpublished changes, when there are any.
    pub draft: Option<Version>,
    pub created_at: DateTime<Utc>,
    pub topics: Vec<TopicRef>,
    pub tags: Vec<TagRef>,
}

/// The result of saving a draft.
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct SavedDraft {
    /// Send this as `base_revision_id` with the next save.
    pub revision_id: Uuid,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct SaveDraftRequest {
    /// The revision the editor started from; a different current revision
    /// means someone else saved in between, and the save is refused.
    pub base_revision_id: Uuid,
    pub title: String,
    pub body_md: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreatePageRequest {
    pub title: String,
    #[serde(default)]
    pub body_md: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct PublishRequest {
    /// The draft revision the person reviewed; publishing fails if it changed.
    pub revision_id: Uuid,
}

pub fn validate_title(title: &str) -> AppResult<String> {
    let title = title.trim();
    if title.is_empty() || title.chars().count() > MAX_TITLE {
        return Err(AppError::coded(
            StatusCode::BAD_REQUEST,
            "invalid_title",
            "Give the page a title of up to 200 characters.",
        ));
    }
    Ok(title.to_string())
}

pub fn validate_body(body: &str) -> AppResult<()> {
    if body.len() > MAX_BODY {
        return Err(AppError::coded(
            StatusCode::PAYLOAD_TOO_LARGE,
            "page_too_large",
            "This page is too long to save. Split it into several pages.",
        ));
    }
    Ok(())
}

/// The URL slug for a title; never empty.
pub fn slug_for(title: &str) -> String {
    let slug = orgs::slugify(title);
    if slug.is_empty() {
        "untitled".to_string()
    } else {
        slug
    }
}

pub fn new_short_id() -> String {
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::rng();
    (0..10)
        .map(|_| ALPHABET[rng.random_range(0..ALPHABET.len())] as char)
        .collect()
}

fn stale() -> AppError {
    AppError::coded(
        StatusCode::CONFLICT,
        "stale_draft",
        "Someone else changed this page while you were editing. Reload to see their changes.",
    )
}

/// Pages in the organization: in a topic by title, otherwise most recently changed first.
pub async fn list(conn: &mut PgConnection, filter: &ListFilter) -> sqlx::Result<Vec<PageSummary>> {
    sqlx::query_as(
        "SELECT p.short_id, p.slug,
                coalesce(pub.title, cur.title) AS title,
                p.published_revision_id IS NOT NULL AS published,
                p.published_revision_id IS DISTINCT FROM p.current_revision_id AS has_draft,
                p.updated_at, u.name AS updated_by_name,
                coalesce((SELECT json_agg(json_build_object('id', t.id, 'name', t.name, 'slug', t.slug,
                                                            'color', t.color) ORDER BY lower(t.name))
                          FROM page_tags pt JOIN tags t ON t.id = pt.tag_id WHERE pt.page_id = p.id),
                         '[]') AS tags
         FROM pages p
         JOIN page_revisions cur ON cur.id = p.current_revision_id
         LEFT JOIN page_revisions pub ON pub.id = p.published_revision_id
         JOIN users u ON u.id = cur.author_id
         WHERE p.archived_at IS NULL
           AND ($1::text IS NULL OR EXISTS (
                 SELECT 1 FROM page_tags pt JOIN tags t ON t.id = pt.tag_id
                 WHERE pt.page_id = p.id AND t.slug = $1))
           AND ($2::uuid IS NULL OR EXISTS (
                 SELECT 1 FROM page_topics pt WHERE pt.page_id = p.id AND pt.topic_id = $2))
         ORDER BY CASE WHEN $2::uuid IS NOT NULL THEN lower(coalesce(pub.title, cur.title)) END,
                  p.updated_at DESC
         LIMIT 200",
    )
    .bind(&filter.tag)
    .bind(filter.topic_id)
    .fetch_all(conn)
    .await
}

pub async fn create(
    conn: &mut PgConnection,
    org_id: Uuid,
    author_id: Uuid,
    req: &CreatePageRequest,
) -> AppResult<String> {
    let title = validate_title(&req.title)?;
    validate_body(&req.body_md)?;
    let page_id = Uuid::now_v7();
    let revision_id = Uuid::now_v7();
    let short_id = new_short_id();
    sqlx::query(
        "INSERT INTO pages (id, org_id, short_id, slug, current_revision_id, created_by)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(page_id)
    .bind(org_id)
    .bind(&short_id)
    .bind(slug_for(&title))
    .bind(revision_id)
    .bind(author_id)
    .execute(&mut *conn)
    .await?;
    sqlx::query(
        "INSERT INTO page_revisions (id, org_id, page_id, title, body_md, author_id)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(revision_id)
    .bind(org_id)
    .bind(page_id)
    .bind(&title)
    .bind(&req.body_md)
    .bind(author_id)
    .execute(&mut *conn)
    .await?;
    audit::record(
        conn,
        Event::new("page.created")
            .org(org_id)
            .actor(author_id)
            .object("page", page_id)
            .revision(revision_id)
            .details(json!({"title": title})),
    )
    .await?;
    Ok(short_id)
}

#[derive(FromRow)]
struct PageRow {
    id: Uuid,
    short_id: String,
    slug: String,
    current_revision_id: Uuid,
    published_revision_id: Option<Uuid>,
    published_at: Option<DateTime<Utc>>,
    published_by_name: Option<String>,
    created_at: DateTime<Utc>,
}

/// The page's id, for changes that don't touch its content.
pub async fn id_of(conn: &mut PgConnection, short_id: &str) -> AppResult<Uuid> {
    Ok(find(conn, short_id, true).await?.id)
}

async fn find(conn: &mut PgConnection, short_id: &str, lock: bool) -> AppResult<PageRow> {
    let sql = if lock {
        "SELECT p.id, p.short_id, p.slug, p.current_revision_id, p.published_revision_id,
                p.published_at, NULL::text AS published_by_name, p.created_at
         FROM pages p WHERE p.short_id = $1 AND p.archived_at IS NULL FOR UPDATE"
    } else {
        "SELECT p.id, p.short_id, p.slug, p.current_revision_id, p.published_revision_id,
                p.published_at, u.name AS published_by_name, p.created_at
         FROM pages p LEFT JOIN users u ON u.id = p.published_by
         WHERE p.short_id = $1 AND p.archived_at IS NULL"
    };
    sqlx::query_as(sql)
        .bind(short_id)
        .fetch_optional(conn)
        .await?
        .ok_or(AppError::NotFound)
}

async fn version(conn: &mut PgConnection, revision_id: Uuid) -> sqlx::Result<Version> {
    sqlx::query_as(
        "SELECT r.id AS revision_id, r.title, r.body_md, u.name AS author_name, r.updated_at
         FROM page_revisions r JOIN users u ON u.id = r.author_id
         WHERE r.id = $1",
    )
    .bind(revision_id)
    .fetch_one(conn)
    .await
}

pub async fn detail(conn: &mut PgConnection, short_id: &str) -> AppResult<PageDetail> {
    let page = find(conn, short_id, false).await?;
    let published = match page.published_revision_id {
        Some(id) => Some(version(conn, id).await?),
        None => None,
    };
    let draft = if Some(page.current_revision_id) == page.published_revision_id {
        None
    } else {
        Some(version(conn, page.current_revision_id).await?)
    };
    Ok(PageDetail {
        id: page.id,
        short_id: page.short_id,
        slug: page.slug,
        published,
        published_at: page.published_at,
        published_by_name: page.published_by_name,
        draft,
        created_at: page.created_at,
        topics: topics::for_page(conn, page.id).await?,
        tags: tags::for_page(conn, page.id).await?,
    })
}

pub async fn save_draft(
    conn: &mut PgConnection,
    org_id: Uuid,
    author_id: Uuid,
    short_id: &str,
    req: &SaveDraftRequest,
) -> AppResult<SavedDraft> {
    let title = validate_title(&req.title)?;
    validate_body(&req.body_md)?;
    let page = find(conn, short_id, true).await?;
    if req.base_revision_id != page.current_revision_id {
        return Err(stale());
    }

    // Keep typing into the draft you started a moment ago; anything else (no
    // draft yet, someone else's draft, an old session) starts a new revision.
    let has_draft = Some(page.current_revision_id) != page.published_revision_id;
    let updated: Option<(Uuid, DateTime<Utc>)> = if has_draft {
        sqlx::query_as(
            "UPDATE page_revisions SET title = $2, body_md = $3, updated_at = now()
             WHERE id = $1 AND author_id = $4 AND updated_at > now() - $5::interval
             RETURNING id, updated_at",
        )
        .bind(page.current_revision_id)
        .bind(&title)
        .bind(&req.body_md)
        .bind(author_id)
        .bind(format!("{} minutes", COLLAPSE_WINDOW.num_minutes()))
        .fetch_optional(&mut *conn)
        .await?
    } else {
        None
    };
    let (revision_id, updated_at) = match updated {
        Some(row) => row,
        None => {
            let revision_id = Uuid::now_v7();
            let row: (Uuid, DateTime<Utc>) = sqlx::query_as(
                "INSERT INTO page_revisions (id, org_id, page_id, title, body_md, author_id)
                 VALUES ($1, $2, $3, $4, $5, $6) RETURNING id, updated_at",
            )
            .bind(revision_id)
            .bind(org_id)
            .bind(page.id)
            .bind(&title)
            .bind(&req.body_md)
            .bind(author_id)
            .fetch_one(&mut *conn)
            .await?;
            row
        }
    };
    sqlx::query("UPDATE pages SET current_revision_id = $2, updated_at = now() WHERE id = $1")
        .bind(page.id)
        .bind(revision_id)
        .execute(&mut *conn)
        .await?;
    Ok(SavedDraft {
        revision_id,
        updated_at,
    })
}

pub async fn publish(
    conn: &mut PgConnection,
    org_id: Uuid,
    actor_id: Uuid,
    short_id: &str,
    revision_id: Uuid,
) -> AppResult<()> {
    let page = find(conn, short_id, true).await?;
    if revision_id != page.current_revision_id {
        return Err(stale());
    }
    if page.published_revision_id == Some(revision_id) {
        return Ok(());
    }
    let title: String = sqlx::query_scalar("SELECT title FROM page_revisions WHERE id = $1")
        .bind(revision_id)
        .fetch_one(&mut *conn)
        .await?;
    sqlx::query(
        "UPDATE pages SET published_revision_id = $2, published_at = now(), published_by = $3,
                          slug = $4, updated_at = now()
         WHERE id = $1",
    )
    .bind(page.id)
    .bind(revision_id)
    .bind(actor_id)
    .bind(slug_for(&title))
    .execute(&mut *conn)
    .await?;
    audit::record(
        conn,
        Event::new("page.published")
            .org(org_id)
            .actor(actor_id)
            .object("page", page.id)
            .revision(revision_id)
            .details(json!({"title": title})),
    )
    .await?;
    Ok(())
}

pub async fn discard_draft(
    conn: &mut PgConnection,
    org_id: Uuid,
    actor_id: Uuid,
    short_id: &str,
) -> AppResult<()> {
    let page = find(conn, short_id, true).await?;
    let Some(published) = page.published_revision_id else {
        return Err(AppError::coded(
            StatusCode::CONFLICT,
            "never_published",
            "This page has never been published, so there is nothing to go back to. Archive it instead.",
        ));
    };
    if published == page.current_revision_id {
        return Ok(());
    }
    sqlx::query("UPDATE pages SET current_revision_id = $2, updated_at = now() WHERE id = $1")
        .bind(page.id)
        .bind(published)
        .execute(&mut *conn)
        .await?;
    audit::record(
        conn,
        Event::new("page.draft_discarded")
            .org(org_id)
            .actor(actor_id)
            .object("page", page.id)
            .revision(page.current_revision_id),
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
    let page = find(conn, short_id, true).await?;
    sqlx::query("UPDATE pages SET archived_at = now() WHERE id = $1")
        .bind(page.id)
        .execute(&mut *conn)
        .await?;
    audit::record(
        conn,
        Event::new("page.archived")
            .org(org_id)
            .actor(actor_id)
            .object("page", page.id),
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_never_empty() {
        assert_eq!(slug_for("Onboarding Guide"), "onboarding-guide");
        assert_eq!(slug_for("!!!"), "untitled");
    }

    #[test]
    fn short_ids_match_the_column_check() {
        for _ in 0..100 {
            let id = new_short_id();
            assert_eq!(id.len(), 10);
            assert!(
                id.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
            );
        }
    }

    #[test]
    fn titles_are_trimmed_and_bounded() {
        assert_eq!(validate_title("  Hello ").unwrap(), "Hello");
        assert!(validate_title("   ").is_err());
        assert!(validate_title(&"x".repeat(201)).is_err());
    }
}
