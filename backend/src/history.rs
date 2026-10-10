//! Page history: every revision a page has had, any of which can be viewed and
//! brought back as a new draft, and archived pages, which can be restored.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, PgConnection};
use ts_rs::TS;
use uuid::Uuid;

use crate::{
    AppError, AppResult,
    audit::{self, Event},
    pages::{self, Version},
};

/// One revision in a page's history.
#[derive(Debug, Serialize, FromRow, TS)]
#[ts(export)]
pub struct RevisionSummary {
    pub revision_id: Uuid,
    pub title: String,
    pub author_name: String,
    pub created_at: DateTime<Utc>,
    /// The last save into this revision.
    pub updated_at: DateTime<Utc>,
    /// When it went live, even if a later version has replaced it since.
    pub published_at: Option<DateTime<Utc>>,
    pub published_by_name: Option<String>,
    /// When the draft it belonged to was discarded.
    pub discarded_at: Option<DateTime<Utc>>,
    /// The earlier revision it was restored from, and that revision's last save.
    pub restored_from: Option<Uuid>,
    pub restored_from_at: Option<DateTime<Utc>>,
    /// What readers see now.
    pub is_live: bool,
    /// What editors work on now: the draft, or the live version when there is no draft.
    pub is_current: bool,
}

/// A revision with its content, and the one before it to compare with.
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct RevisionDetail {
    pub revision: RevisionSummary,
    pub body_md: String,
    pub previous: Option<Version>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct RestoreRevisionRequest {
    /// The page's current revision when the person chose to restore; a newer
    /// one means someone saved in between, and the restore is refused.
    pub base_revision_id: Uuid,
}

/// An archived page.
#[derive(Debug, Serialize, FromRow, TS)]
#[ts(export)]
pub struct ArchivedPage {
    pub short_id: String,
    pub slug: String,
    pub title: String,
    pub archived_at: DateTime<Utc>,
    pub archived_by_name: Option<String>,
}

/// The page's revisions newest first, or just `only` when given.
async fn summaries(
    conn: &mut PgConnection,
    page_id: Uuid,
    only: Option<Uuid>,
) -> sqlx::Result<Vec<RevisionSummary>> {
    sqlx::query_as(
        "SELECT r.id AS revision_id, r.title, u.name AS author_name, r.created_at, r.updated_at,
                r.published_at, pu.name AS published_by_name, r.discarded_at, r.restored_from,
                src.updated_at AS restored_from_at,
                r.id IS NOT DISTINCT FROM p.published_revision_id AS is_live,
                r.id = p.current_revision_id AS is_current
         FROM page_revisions r
         JOIN pages p ON p.id = r.page_id
         JOIN users u ON u.id = r.author_id
         LEFT JOIN users pu ON pu.id = r.published_by
         LEFT JOIN page_revisions src ON src.id = r.restored_from
         WHERE r.page_id = $1 AND ($2::uuid IS NULL OR r.id = $2)
         ORDER BY r.created_at DESC, r.id DESC
         LIMIT 500",
    )
    .bind(page_id)
    .bind(only)
    .fetch_all(conn)
    .await
}

/// The page's revisions, newest first.
pub async fn list(conn: &mut PgConnection, short_id: &str) -> AppResult<Vec<RevisionSummary>> {
    let page = pages::find(conn, short_id, false).await?;
    Ok(summaries(conn, page.id, None).await?)
}

pub async fn revision(
    conn: &mut PgConnection,
    short_id: &str,
    revision_id: Uuid,
) -> AppResult<RevisionDetail> {
    let page = pages::find(conn, short_id, false).await?;
    let revision = summaries(conn, page.id, Some(revision_id))
        .await?
        .pop()
        .ok_or(AppError::NotFound)?;
    let body_md: String = sqlx::query_scalar("SELECT body_md FROM page_revisions WHERE id = $1")
        .bind(revision_id)
        .fetch_one(&mut *conn)
        .await?;
    let previous: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM page_revisions
         WHERE page_id = $1 AND (created_at, id) < ($2, $3)
         ORDER BY created_at DESC, id DESC LIMIT 1",
    )
    .bind(page.id)
    .bind(revision.created_at)
    .bind(revision_id)
    .fetch_optional(&mut *conn)
    .await?;
    let previous = match previous {
        Some(id) => Some(pages::version(conn, id).await?),
        None => None,
    };
    Ok(RevisionDetail {
        revision,
        body_md,
        previous,
    })
}

/// Copy an earlier revision into a new draft. Readers keep seeing the live
/// version until someone publishes it.
pub async fn restore(
    conn: &mut PgConnection,
    org_id: Uuid,
    actor_id: Uuid,
    short_id: &str,
    revision_id: Uuid,
    base_revision_id: Uuid,
) -> AppResult<()> {
    let page = pages::find(conn, short_id, true).await?;
    if base_revision_id != page.current_revision_id {
        return Err(pages::stale());
    }
    if revision_id == page.current_revision_id {
        return Ok(());
    }
    let new_id = Uuid::now_v7();
    let restored = sqlx::query(
        "INSERT INTO page_revisions (id, org_id, page_id, title, body_md, author_id, restored_from)
         SELECT $1, org_id, page_id, title, body_md, $2, id FROM page_revisions
         WHERE id = $3 AND page_id = $4",
    )
    .bind(new_id)
    .bind(actor_id)
    .bind(revision_id)
    .bind(page.id)
    .execute(&mut *conn)
    .await?;
    if restored.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    sqlx::query("UPDATE pages SET current_revision_id = $2, updated_at = now() WHERE id = $1")
        .bind(page.id)
        .bind(new_id)
        .execute(&mut *conn)
        .await?;
    audit::record(
        conn,
        Event::new("page.revision_restored")
            .org(org_id)
            .actor(actor_id)
            .object("page", page.id)
            .revision(new_id)
            .details(json!({"restored_from": revision_id})),
    )
    .await?;
    Ok(())
}

/// Archived pages, most recently archived first.
pub async fn archived(conn: &mut PgConnection) -> sqlx::Result<Vec<ArchivedPage>> {
    sqlx::query_as(
        "SELECT p.short_id, p.slug, coalesce(pub.title, cur.title) AS title, p.archived_at,
                u.name AS archived_by_name
         FROM pages p
         JOIN page_revisions cur ON cur.id = p.current_revision_id
         LEFT JOIN page_revisions pub ON pub.id = p.published_revision_id
         LEFT JOIN users u ON u.id = p.archived_by
         WHERE p.archived_at IS NOT NULL
         ORDER BY p.archived_at DESC
         LIMIT 500",
    )
    .fetch_all(conn)
    .await
}

/// Bring an archived page back, as it was.
pub async fn unarchive(
    conn: &mut PgConnection,
    org_id: Uuid,
    actor_id: Uuid,
    short_id: &str,
) -> AppResult<()> {
    let page_id: Uuid = sqlx::query_scalar(
        "UPDATE pages SET archived_at = NULL, archived_by = NULL, updated_at = now()
         WHERE short_id = $1 AND archived_at IS NOT NULL
         RETURNING id",
    )
    .bind(short_id)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or(AppError::NotFound)?;
    audit::record(
        conn,
        Event::new("page.unarchived")
            .org(org_id)
            .actor(actor_id)
            .object("page", page_id),
    )
    .await?;
    Ok(())
}
