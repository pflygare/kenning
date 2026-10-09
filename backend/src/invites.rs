//! Invitations to an organization, by email, for people with or without an account.

use axum::http::StatusCode;
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use sqlx::PgConnection;
use ts_rs::TS;
use uuid::Uuid;

use crate::{
    AppError, AppResult, Config,
    accounts::{self, User},
    audit::{self, Event},
    db,
    mail::{self, Email},
    orgs::{Membership, Organization, PendingInvite, Role},
    tokens,
};

const INVITE_TTL: Duration = Duration::days(14);

/// The address an invitation was sent to, if the token is for an open invitation.
pub async fn open_invite_email(
    conn: &mut PgConnection,
    token: &str,
) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar(
        "SELECT email FROM invites
         WHERE token_hash = $1 AND accepted_at IS NULL AND revoked_at IS NULL AND expires_at > now()",
    )
    .bind(tokens::hash(token))
    .fetch_optional(conn)
    .await
}

/// Invite `email` to `org` with `role`, or refresh the open invitation to that
/// address with a new link. Call inside the organization's transaction.
pub async fn send(
    conn: &mut PgConnection,
    config: &Config,
    org: &Organization,
    inviter: &User,
    email: &str,
    role: Role,
) -> AppResult<PendingInvite> {
    let already_member: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM memberships m JOIN users u ON u.id = m.user_id WHERE m.org_id = $1 AND u.email = $2)",
    )
    .bind(org.id)
    .bind(email)
    .fetch_one(&mut *conn)
    .await?;
    if already_member {
        return Err(AppError::coded(
            StatusCode::CONFLICT,
            "already_member",
            format!("{email} is already a member."),
        ));
    }

    let token = tokens::generate();
    let expires_at = Utc::now() + INVITE_TTL;
    let (id, created_at): (Uuid, DateTime<Utc>) = sqlx::query_as(
        "INSERT INTO invites (id, org_id, token_hash, email, role, invited_by, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         ON CONFLICT (org_id, email) WHERE accepted_at IS NULL AND revoked_at IS NULL
         DO UPDATE SET token_hash = EXCLUDED.token_hash, role = EXCLUDED.role,
                       invited_by = EXCLUDED.invited_by, expires_at = EXCLUDED.expires_at, created_at = now()
         RETURNING id, created_at",
    )
    .bind(Uuid::now_v7())
    .bind(org.id)
    .bind(tokens::hash(&token))
    .bind(email)
    .bind(role)
    .bind(inviter.id)
    .bind(expires_at)
    .fetch_one(&mut *conn)
    .await?;

    mail::queue(
        conn,
        &Email {
            to: email.to_string(),
            subject: format!("{} invited you to {} on Kenning", inviter.name, org.name),
            body: format!(
                "{} invited you to join {} on Kenning.\n\nOpen this link to accept. You can create an account there if you don't have one:\n\n{}\n\nThe invitation expires in 14 days.\n",
                inviter.name,
                org.name,
                config.link(&format!("/invite/{token}"))
            ),
        },
    )
    .await?;
    audit::record(
        conn,
        Event::new("invite.sent")
            .org(org.id)
            .actor(inviter.id)
            .object("invite", id)
            .details(serde_json::json!({"email": email, "role": role})),
    )
    .await?;

    Ok(PendingInvite {
        id,
        email: email.to_string(),
        role,
        invited_by_name: inviter.name.clone(),
        created_at,
        expires_at,
    })
}

pub async fn pending(conn: &mut PgConnection, org_id: Uuid) -> sqlx::Result<Vec<PendingInvite>> {
    sqlx::query_as(
        "SELECT i.id, i.email, i.role, u.name AS invited_by_name, i.created_at, i.expires_at
         FROM invites i JOIN users u ON u.id = i.invited_by
         WHERE i.org_id = $1 AND i.accepted_at IS NULL AND i.revoked_at IS NULL
         ORDER BY i.created_at DESC",
    )
    .bind(org_id)
    .fetch_all(conn)
    .await
}

pub async fn revoke(
    conn: &mut PgConnection,
    org_id: Uuid,
    actor: &User,
    invite_id: Uuid,
) -> AppResult<()> {
    let revoked = sqlx::query(
        "UPDATE invites SET revoked_at = now()
         WHERE id = $1 AND org_id = $2 AND accepted_at IS NULL AND revoked_at IS NULL",
    )
    .bind(invite_id)
    .bind(org_id)
    .execute(&mut *conn)
    .await?;
    if revoked.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    audit::record(
        conn,
        Event::new("invite.revoked")
            .org(org_id)
            .actor(actor.id)
            .object("invite", invite_id),
    )
    .await?;
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, sqlx::Type, TS)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[ts(export)]
pub enum InviteStatus {
    Open,
    Accepted,
    Revoked,
    Expired,
}

/// What the invitation page shows before anyone signs in.
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct InvitePreview {
    pub org_name: String,
    pub email: String,
    pub role: Role,
    pub invited_by_name: String,
    pub status: InviteStatus,
    /// Whether an account with the invited address exists (sign in vs. sign up).
    pub account_exists: bool,
}

pub async fn preview(conn: &mut PgConnection, token: &str) -> AppResult<InvitePreview> {
    let row: Option<(String, String, Role, String, InviteStatus, bool)> = sqlx::query_as(
        "SELECT o.name, i.email, i.role, u.name,
                CASE WHEN i.accepted_at IS NOT NULL THEN 'accepted'
                     WHEN i.revoked_at IS NOT NULL THEN 'revoked'
                     WHEN i.expires_at <= now() THEN 'expired'
                     ELSE 'open' END,
                EXISTS (SELECT 1 FROM users x WHERE x.email = i.email)
         FROM invites i
         JOIN organizations o ON o.id = i.org_id
         JOIN users u ON u.id = i.invited_by
         WHERE i.token_hash = $1",
    )
    .bind(tokens::hash(token))
    .fetch_optional(conn)
    .await?;
    let (org_name, email, role, invited_by_name, status, account_exists) =
        row.ok_or(AppError::NotFound)?;
    Ok(InvitePreview {
        org_name,
        email,
        role,
        invited_by_name,
        status,
        account_exists,
    })
}

/// Join the organization as `user`. The invitation must be open and addressed
/// to the user's email; accepting it also proves that address.
pub async fn accept(conn: &mut PgConnection, token: &str, user: &User) -> AppResult<Membership> {
    let row: Option<(Uuid, Uuid, String, Role)> = sqlx::query_as(
        "SELECT id, org_id, email, role FROM invites
         WHERE token_hash = $1 AND accepted_at IS NULL AND revoked_at IS NULL AND expires_at > now()
         FOR UPDATE",
    )
    .bind(tokens::hash(token))
    .fetch_optional(&mut *conn)
    .await?;
    let (invite_id, org_id, email, role) = row.ok_or_else(|| {
        AppError::coded(
            StatusCode::GONE,
            "invite_unavailable",
            "This invitation has expired or was already used.",
        )
    })?;
    if email != user.email {
        return Err(AppError::coded(
            StatusCode::FORBIDDEN,
            "invite_email_mismatch",
            format!("This invitation is for {email}. Sign in with that address to accept it."),
        ));
    }

    db::set_org(conn, org_id).await?;
    sqlx::query("INSERT INTO memberships (org_id, user_id, role) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING")
        .bind(org_id)
        .bind(user.id)
        .bind(role)
        .execute(&mut *conn)
        .await?;
    sqlx::query("UPDATE invites SET accepted_at = now(), accepted_by = $2 WHERE id = $1")
        .bind(invite_id)
        .bind(user.id)
        .execute(&mut *conn)
        .await?;
    accounts::mark_verified(conn, user.id).await?;
    audit::record(
        conn,
        Event::new("invite.accepted")
            .org(org_id)
            .actor(user.id)
            .object("invite", invite_id),
    )
    .await?;

    sqlx::query_as(
        "SELECT o.id, o.name, o.slug, m.role FROM organizations o
         JOIN memberships m ON m.org_id = o.id AND m.user_id = $2 WHERE o.id = $1",
    )
    .bind(org_id)
    .bind(user.id)
    .fetch_one(conn)
    .await
    .map_err(Into::into)
}
