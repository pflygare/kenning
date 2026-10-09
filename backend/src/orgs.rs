//! Organizations, their members, and invitations.

use axum::{
    extract::{FromRequestParts, Path},
    http::{StatusCode, request::Parts},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgConnection};
use ts_rs::TS;
use uuid::Uuid;

use crate::{AppError, AppResult, AppState, accounts::User, auth::CurrentUser};

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, sqlx::Type, TS,
)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[ts(export)]
pub enum Role {
    Member,
    Admin,
    Owner,
}

impl Role {
    /// Owners and admins manage members and invitations.
    pub fn manages_members(self) -> bool {
        self >= Role::Admin
    }
}

#[derive(Clone, Debug, Serialize, FromRow, TS)]
#[ts(export)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
}

/// An organization as seen by one of its members.
#[derive(Clone, Debug, Serialize, FromRow, TS)]
#[ts(export)]
pub struct Membership {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub role: Role,
}

#[derive(Clone, Debug, Serialize, FromRow, TS)]
#[ts(export)]
pub struct Member {
    pub user_id: Uuid,
    pub name: String,
    pub email: String,
    pub avatar_url: Option<String>,
    pub role: Role,
    pub joined_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, FromRow, TS)]
#[ts(export)]
pub struct PendingInvite {
    pub id: Uuid,
    pub email: String,
    pub role: Role,
    pub invited_by_name: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Paths that would collide with the app's own pages if used as an organization slug.
const RESERVED_SLUGS: &[&str] = &[
    "account",
    "admin",
    "api",
    "app",
    "assets",
    "auth",
    "favicon-svg",
    "forgot-password",
    "help",
    "invite",
    "invites",
    "login",
    "logout",
    "new",
    "new-org",
    "orgs",
    "reset-password",
    "settings",
    "signup",
    "static",
    "verify-email",
];

/// Turn a name or requested slug into `lowercase-words-like-this`.
pub fn slugify(input: &str) -> String {
    let mut slug = String::new();
    for c in input.trim().chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }
    let slug = slug.trim_end_matches('-');
    slug.chars()
        .take(48)
        .collect::<String>()
        .trim_end_matches('-')
        .to_string()
}

pub fn is_reserved(slug: &str) -> bool {
    RESERVED_SLUGS.contains(&slug)
}

pub async fn memberships_of(
    conn: &mut PgConnection,
    user_id: Uuid,
) -> sqlx::Result<Vec<Membership>> {
    sqlx::query_as(
        "SELECT o.id, o.name, o.slug, m.role
         FROM memberships m JOIN organizations o ON o.id = m.org_id
         WHERE m.user_id = $1 ORDER BY o.name",
    )
    .bind(user_id)
    .fetch_all(conn)
    .await
}

/// A request inside an organization the signed-in user belongs to: the
/// `{org}` path segment, resolved. Non-members get 404 so they cannot probe
/// which organizations exist.
#[derive(Clone, Debug)]
pub struct OrgContext {
    pub user: User,
    pub org: Organization,
    pub role: Role,
}

#[derive(Deserialize)]
struct OrgPath {
    org: String,
}

impl FromRequestParts<AppState> for OrgContext {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let CurrentUser(user) = CurrentUser::from_request_parts(parts, state).await?;
        let Path(path) = Path::<OrgPath>::from_request_parts(parts, state)
            .await
            .map_err(|_| AppError::NotFound)?;
        let row: Option<(Uuid, String, String, Role)> = sqlx::query_as(
            "SELECT o.id, o.name, o.slug, m.role
             FROM organizations o JOIN memberships m ON m.org_id = o.id AND m.user_id = $2
             WHERE o.slug = $1",
        )
        .bind(path.org.to_lowercase())
        .bind(user.id)
        .fetch_optional(&state.pool)
        .await?;
        let (id, name, slug, role) = row.ok_or(AppError::NotFound)?;
        Ok(Self {
            user,
            org: Organization { id, name, slug },
            role,
        })
    }
}

impl OrgContext {
    pub fn require_manager(&self) -> AppResult<()> {
        if self.role.manages_members() {
            Ok(())
        } else {
            Err(AppError::coded(
                StatusCode::FORBIDDEN,
                "forbidden",
                "Only owners and admins can manage members.",
            ))
        }
    }
}

/// Whether `actor` may change `target`'s role from `from` to `to`.
pub fn can_change_role(actor: Role, from: Role, to: Role) -> bool {
    actor.manages_members() && (actor == Role::Owner || (from != Role::Owner && to != Role::Owner))
}

/// Whether `actor` may remove a member whose role is `target` (leaving yourself is always allowed).
pub fn can_remove(actor: Role, target: Role, is_self: bool) -> bool {
    is_self || (actor.manages_members() && (actor == Role::Owner || target != Role::Owner))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugifies_names() {
        assert_eq!(slugify("  Acme Corp!  "), "acme-corp");
        assert_eq!(slugify("Ünïcode & Co"), "n-code-co");
        assert_eq!(slugify("---"), "");
        assert_eq!(slugify(&"a".repeat(80)).len(), 48);
    }

    #[test]
    fn role_rules() {
        use Role::*;
        assert!(can_change_role(Owner, Member, Owner));
        assert!(can_change_role(Admin, Member, Admin));
        assert!(!can_change_role(Admin, Member, Owner));
        assert!(!can_change_role(Admin, Owner, Member));
        assert!(!can_change_role(Member, Member, Admin));

        assert!(can_remove(Admin, Member, false));
        assert!(!can_remove(Admin, Owner, false));
        assert!(can_remove(Owner, Owner, false));
        assert!(!can_remove(Member, Member, false));
        assert!(can_remove(Member, Member, true));
    }
}
