//! Users, their sign-in identities, and the emailed links that verify them.

use axum::http::StatusCode;
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use sqlx::{FromRow, PgConnection};
use ts_rs::TS;
use uuid::Uuid;

use crate::{
    AppError, AppResult, Config,
    audit::{self, Event},
    mail::{self, Email},
    tokens,
};

pub const MIN_PASSWORD_LEN: usize = 8;
const MAX_PASSWORD_LEN: usize = 256;
const VERIFY_EMAIL_TTL: Duration = Duration::days(7);
const RESET_PASSWORD_TTL: Duration = Duration::hours(1);

#[derive(Clone, Debug, Serialize, FromRow, TS)]
#[ts(export)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub email_verified: bool,
    #[serde(skip)]
    #[ts(skip)]
    pub disabled: bool,
}

const USER_COLUMNS: &str = "id, email, name, avatar_url, email_verified_at IS NOT NULL AS email_verified, disabled_at IS NOT NULL AS disabled";

/// Trim, lowercase and sanity-check an email address.
pub fn normalize_email(raw: &str) -> AppResult<String> {
    let email = raw.trim().to_lowercase();
    let valid = email.len() <= 254
        && !email.contains(char::is_whitespace)
        && matches!(email.split_once('@'), Some((local, domain))
            if !local.is_empty() && domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.'));
    if valid {
        Ok(email)
    } else {
        Err(AppError::coded(
            StatusCode::BAD_REQUEST,
            "invalid_email",
            "Enter a valid email address.",
        ))
    }
}

pub fn validate_name(raw: &str) -> AppResult<String> {
    let name = raw.trim();
    if name.is_empty() || name.chars().count() > 100 {
        return Err(AppError::coded(
            StatusCode::BAD_REQUEST,
            "invalid_name",
            "Enter a name of up to 100 characters.",
        ));
    }
    Ok(name.to_string())
}

pub fn validate_password(password: &str) -> AppResult<()> {
    let len = password.chars().count();
    if !(MIN_PASSWORD_LEN..=MAX_PASSWORD_LEN).contains(&len) {
        return Err(AppError::coded(
            StatusCode::BAD_REQUEST,
            "weak_password",
            format!("Use a password of at least {MIN_PASSWORD_LEN} characters."),
        ));
    }
    Ok(())
}

pub async fn find_by_id(conn: &mut PgConnection, id: Uuid) -> sqlx::Result<Option<User>> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {USER_COLUMNS} FROM users WHERE id = $1"
    )))
    .bind(id)
    .fetch_optional(conn)
    .await
}

pub async fn find_by_email(conn: &mut PgConnection, email: &str) -> sqlx::Result<Option<User>> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {USER_COLUMNS} FROM users WHERE email = $1"
    )))
    .bind(email)
    .fetch_optional(conn)
    .await
}

/// Create a user. `verified` marks the email as proven (Google, or an invite link).
pub async fn create_user(
    conn: &mut PgConnection,
    email: &str,
    name: &str,
    avatar_url: Option<&str>,
    verified: bool,
) -> AppResult<User> {
    let result = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "INSERT INTO users (id, email, name, avatar_url, email_verified_at)
         VALUES ($1, $2, $3, $4, CASE WHEN $5 THEN now() END)
         RETURNING {USER_COLUMNS}"
    )))
    .bind(Uuid::now_v7())
    .bind(email)
    .bind(name)
    .bind(avatar_url)
    .bind(verified)
    .fetch_one(&mut *conn)
    .await;
    match result {
        Ok(user) => Ok(user),
        Err(sqlx::Error::Database(err)) if err.is_unique_violation() => Err(AppError::coded(
            StatusCode::CONFLICT,
            "email_taken",
            "An account with this email already exists. Sign in instead.",
        )),
        Err(err) => Err(err.into()),
    }
}

pub async fn mark_verified(conn: &mut PgConnection, user_id: Uuid) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE users SET email_verified_at = coalesce(email_verified_at, now()) WHERE id = $1",
    )
    .bind(user_id)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn add_identity(
    conn: &mut PgConnection,
    user_id: Uuid,
    provider: &str,
    subject: &str,
    password_hash: Option<&str>,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO identities (id, user_id, provider, subject, password_hash) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(Uuid::now_v7())
    .bind(user_id)
    .bind(provider)
    .bind(subject)
    .bind(password_hash)
    .execute(conn)
    .await?;
    Ok(())
}

/// The user's password hash, if they have a password.
pub async fn password_hash(conn: &mut PgConnection, user_id: Uuid) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar(
        "SELECT password_hash FROM identities WHERE user_id = $1 AND provider = 'password'",
    )
    .bind(user_id)
    .fetch_optional(conn)
    .await
}

/// Set (or first create) the user's password.
pub async fn set_password(
    conn: &mut PgConnection,
    user: &User,
    password_hash: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO identities (id, user_id, provider, subject, password_hash)
         VALUES ($1, $2, 'password', $3, $4)
         ON CONFLICT (user_id) WHERE provider = 'password' DO UPDATE SET password_hash = EXCLUDED.password_hash",
    )
    .bind(Uuid::now_v7())
    .bind(user.id)
    .bind(user.id.to_string())
    .bind(password_hash)
    .execute(conn)
    .await?;
    Ok(())
}

/// The user signed in with `provider` as `subject`, if any.
pub async fn find_by_identity(
    conn: &mut PgConnection,
    provider: &str,
    subject: &str,
) -> sqlx::Result<Option<User>> {
    sqlx::query_as(
        "SELECT u.id, u.email, u.name, u.avatar_url,
                u.email_verified_at IS NOT NULL AS email_verified, u.disabled_at IS NOT NULL AS disabled
         FROM users u JOIN identities i ON i.user_id = u.id
         WHERE i.provider = $1 AND i.subject = $2",
    )
    .bind(provider)
    .bind(subject)
    .fetch_optional(conn)
    .await
}

/// Sign in through an external provider that has verified `email`: reuse the
/// linked account, else link the account with that email, else create one.
pub async fn sign_in_external(
    conn: &mut PgConnection,
    provider: &str,
    subject: &str,
    email: &str,
    name: &str,
    avatar_url: Option<&str>,
) -> AppResult<User> {
    if let Some(user) = find_by_identity(conn, provider, subject).await? {
        return Ok(user);
    }
    let user = match find_by_email(conn, email).await? {
        Some(user) => {
            mark_verified(conn, user.id).await?;
            find_by_id(conn, user.id).await?.ok_or(AppError::NotFound)?
        }
        None => {
            let user = create_user(conn, email, name, avatar_url, true).await?;
            audit::record(
                conn,
                Event::new("user.signed_up")
                    .actor(user.id)
                    .object("user", user.id)
                    .details(serde_json::json!({"provider": provider})),
            )
            .await?;
            user
        }
    };
    add_identity(conn, user.id, provider, subject, None).await?;
    audit::record(
        conn,
        Event::new("user.identity_linked")
            .actor(user.id)
            .object("user", user.id)
            .details(serde_json::json!({"provider": provider})),
    )
    .await?;
    Ok(user)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmailTokenKind {
    VerifyEmail,
    ResetPassword,
}

impl EmailTokenKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::VerifyEmail => "verify_email",
            Self::ResetPassword => "reset_password",
        }
    }
}

/// Create a single-use link token and queue the email that carries it.
pub async fn send_email_token(
    conn: &mut PgConnection,
    config: &Config,
    user: &User,
    kind: EmailTokenKind,
) -> sqlx::Result<()> {
    let token = tokens::generate();
    let ttl = match kind {
        EmailTokenKind::VerifyEmail => VERIFY_EMAIL_TTL,
        EmailTokenKind::ResetPassword => RESET_PASSWORD_TTL,
    };
    sqlx::query(
        "INSERT INTO email_tokens (token_hash, kind, user_id, email, expires_at) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(tokens::hash(&token))
    .bind(kind.as_str())
    .bind(user.id)
    .bind(&user.email)
    .bind(Utc::now() + ttl)
    .execute(&mut *conn)
    .await?;

    let email = match kind {
        EmailTokenKind::VerifyEmail => Email {
            to: user.email.clone(),
            subject: "Confirm your email for Kenning".to_string(),
            body: format!(
                "Hi {},\n\nConfirm your email address by opening this link:\n\n{}\n\nThe link works for 7 days. If you did not sign up for Kenning, ignore this email.\n",
                user.name,
                config.link(&format!("/verify-email?token={token}"))
            ),
        },
        EmailTokenKind::ResetPassword => Email {
            to: user.email.clone(),
            subject: "Reset your Kenning password".to_string(),
            body: format!(
                "Hi {},\n\nSomeone asked to reset the password for this account. To choose a new password, open this link:\n\n{}\n\nThe link works for 1 hour. If it wasn't you, ignore this email; your password stays the same.\n",
                user.name,
                config.link(&format!("/reset-password?token={token}"))
            ),
        },
    };
    mail::queue(conn, &email).await
}

/// Use up a link token. Returns the user it belongs to, if the token is valid,
/// unused, unexpired, and the account's email has not changed since.
pub async fn consume_email_token(
    conn: &mut PgConnection,
    kind: EmailTokenKind,
    token: &str,
) -> AppResult<User> {
    let row: Option<(Uuid, String, DateTime<Utc>)> = sqlx::query_as(
        "UPDATE email_tokens SET used_at = now()
         WHERE token_hash = $1 AND kind = $2 AND used_at IS NULL
         RETURNING user_id, email, expires_at",
    )
    .bind(tokens::hash(token))
    .bind(kind.as_str())
    .fetch_optional(&mut *conn)
    .await?;

    let invalid = || {
        AppError::coded(
            StatusCode::BAD_REQUEST,
            "invalid_token",
            "This link is invalid or has already been used.",
        )
    };
    let (user_id, email, expires_at) = row.ok_or_else(invalid)?;
    if expires_at < Utc::now() {
        return Err(AppError::coded(
            StatusCode::BAD_REQUEST,
            "expired_token",
            "This link has expired. Ask for a new one.",
        ));
    }
    let user = find_by_id(conn, user_id).await?.ok_or_else(invalid)?;
    if user.email != email || user.disabled {
        return Err(invalid());
    }
    Ok(user)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_emails() {
        assert_eq!(
            normalize_email("  Ada@Example.COM ").unwrap(),
            "ada@example.com"
        );
        for bad in [
            "",
            "ada",
            "@example.com",
            "ada@example",
            "ada@.com",
            "a da@example.com",
        ] {
            assert!(normalize_email(bad).is_err(), "{bad} should be rejected");
        }
    }

    #[test]
    fn checks_password_length() {
        assert!(validate_password("short").is_err());
        assert!(validate_password("long enough").is_ok());
    }
}
