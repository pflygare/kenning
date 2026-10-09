//! Append-only, hash-chained audit log.
//!
//! Each organization's events (and system events, with no organization) form
//! their own chain: every event stores the hash of the one before it, so an
//! edited or deleted row breaks [`verify_chain`]. The table also refuses
//! UPDATE and DELETE outright.

use chrono::{DateTime, SubsecRound, Utc};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

/// An event to record. Build one with [`Event::new`] and the setters.
#[derive(Clone, Debug, Default)]
pub struct Event {
    pub org_id: Option<Uuid>,
    pub actor_id: Option<Uuid>,
    pub action: String,
    pub object_kind: Option<String>,
    pub object_id: Option<Uuid>,
    pub revision_id: Option<Uuid>,
    pub details: Value,
    pub ip: Option<std::net::IpAddr>,
}

impl Event {
    /// `action` is a dotted verb such as `page.published`.
    pub fn new(action: impl Into<String>) -> Self {
        Self {
            action: action.into(),
            details: json!({}),
            ..Default::default()
        }
    }

    pub fn org(mut self, org_id: Uuid) -> Self {
        self.org_id = Some(org_id);
        self
    }

    pub fn actor(mut self, actor_id: Uuid) -> Self {
        self.actor_id = Some(actor_id);
        self
    }

    pub fn object(mut self, kind: impl Into<String>, id: Uuid) -> Self {
        self.object_kind = Some(kind.into());
        self.object_id = Some(id);
        self
    }

    pub fn revision(mut self, revision_id: Uuid) -> Self {
        self.revision_id = Some(revision_id);
        self
    }

    pub fn details(mut self, details: Value) -> Self {
        self.details = details;
        self
    }

    pub fn ip(mut self, ip: std::net::IpAddr) -> Self {
        self.ip = Some(ip);
        self
    }
}

/// A stored event.
#[derive(Clone, Debug, FromRow)]
pub struct StoredEvent {
    pub seq: i64,
    pub id: Uuid,
    pub org_id: Option<Uuid>,
    pub actor_id: Option<Uuid>,
    pub action: String,
    pub object_kind: Option<String>,
    pub object_id: Option<Uuid>,
    pub revision_id: Option<Uuid>,
    pub details: Value,
    pub created_at: DateTime<Utc>,
    pub prev_hash: Option<Vec<u8>>,
    pub hash: Vec<u8>,
}

/// Append an event. Call it inside the transaction that makes the change, so
/// the change and its record commit or roll back together. For an
/// organization's event, use a transaction from [`crate::db::begin_org`].
pub async fn record(conn: &mut PgConnection, event: Event) -> sqlx::Result<StoredEvent> {
    // Serialize writers per chain so two events never claim the same parent.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(chain_key(event.org_id))
        .execute(&mut *conn)
        .await?;

    let prev_hash: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT hash FROM audit_events WHERE org_id IS NOT DISTINCT FROM $1 ORDER BY seq DESC LIMIT 1",
    )
    .bind(event.org_id)
    .fetch_optional(&mut *conn)
    .await?;

    let id = Uuid::now_v7();
    // Postgres stores microseconds; truncate so the hash input survives a round trip.
    let created_at = Utc::now().trunc_subsecs(6);
    let hash = compute_hash(
        prev_hash.as_deref(),
        &HashInput {
            id,
            org_id: event.org_id,
            actor_id: event.actor_id,
            action: &event.action,
            object_kind: event.object_kind.as_deref(),
            object_id: event.object_id,
            revision_id: event.revision_id,
            details: &event.details,
            created_at,
        },
    );

    sqlx::query_as(
        "INSERT INTO audit_events
            (id, org_id, actor_id, action, object_kind, object_id, revision_id, details, ip, created_at, prev_hash, hash)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::inet, $10, $11, $12)
         RETURNING seq, id, org_id, actor_id, action, object_kind, object_id, revision_id, details, created_at, prev_hash, hash",
    )
    .bind(id)
    .bind(event.org_id)
    .bind(event.actor_id)
    .bind(&event.action)
    .bind(&event.object_kind)
    .bind(event.object_id)
    .bind(event.revision_id)
    .bind(&event.details)
    .bind(event.ip.map(|ip| ip.to_string()))
    .bind(created_at)
    .bind(&prev_hash)
    .bind(&hash)
    .fetch_one(&mut *conn)
    .await
}

/// A break found by [`verify_chain`].
#[derive(Debug, PartialEq, Eq)]
pub struct ChainBreak {
    pub seq: i64,
    pub reason: &'static str,
}

/// Recompute one chain (`None` = system events) and report the first event
/// whose link or hash does not match.
pub async fn verify_chain(
    conn: &mut PgConnection,
    org_id: Option<Uuid>,
) -> sqlx::Result<Result<usize, ChainBreak>> {
    let events: Vec<StoredEvent> = sqlx::query_as(
        "SELECT seq, id, org_id, actor_id, action, object_kind, object_id, revision_id, details, created_at, prev_hash, hash
         FROM audit_events WHERE org_id IS NOT DISTINCT FROM $1 ORDER BY seq",
    )
    .bind(org_id)
    .fetch_all(&mut *conn)
    .await?;

    let mut expected_prev: Option<Vec<u8>> = None;
    for event in &events {
        if event.prev_hash != expected_prev {
            return Ok(Err(ChainBreak {
                seq: event.seq,
                reason: "previous hash does not match",
            }));
        }
        let hash = compute_hash(
            event.prev_hash.as_deref(),
            &HashInput {
                id: event.id,
                org_id: event.org_id,
                actor_id: event.actor_id,
                action: &event.action,
                object_kind: event.object_kind.as_deref(),
                object_id: event.object_id,
                revision_id: event.revision_id,
                details: &event.details,
                created_at: event.created_at,
            },
        );
        if hash != event.hash {
            return Ok(Err(ChainBreak {
                seq: event.seq,
                reason: "event content does not match its hash",
            }));
        }
        expected_prev = Some(hash);
    }
    Ok(Ok(events.len()))
}

fn chain_key(org_id: Option<Uuid>) -> String {
    match org_id {
        Some(id) => format!("audit:{id}"),
        None => "audit:system".to_string(),
    }
}

struct HashInput<'a> {
    id: Uuid,
    org_id: Option<Uuid>,
    actor_id: Option<Uuid>,
    action: &'a str,
    object_kind: Option<&'a str>,
    object_id: Option<Uuid>,
    revision_id: Option<Uuid>,
    details: &'a Value,
    created_at: DateTime<Utc>,
}

fn compute_hash(prev_hash: Option<&[u8]>, input: &HashInput) -> Vec<u8> {
    // serde_json sorts object keys (no preserve_order feature), and jsonb
    // round-trips keep the same values, so this encoding is stable.
    let canonical = json!({
        "id": input.id,
        "org_id": input.org_id,
        "actor_id": input.actor_id,
        "action": input.action,
        "object_kind": input.object_kind,
        "object_id": input.object_id,
        "revision_id": input.revision_id,
        "details": input.details,
        "created_at": input.created_at.to_rfc3339_opts(chrono::SecondsFormat::Micros, true),
    });
    let mut hasher = Sha256::new();
    hasher.update(prev_hash.unwrap_or_default());
    hasher.update(canonical.to_string().as_bytes());
    hasher.finalize().to_vec()
}
