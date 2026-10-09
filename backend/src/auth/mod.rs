//! Passwords, sessions, and the extractor that tells a handler who is signed in.

pub mod google;

use std::sync::LazyLock;

use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};
use axum::{extract::FromRequestParts, http::request::Parts};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use chrono::{Duration, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::{
    AppError, AppState, Config,
    accounts::{self, User},
    tokens,
};

pub const SESSION_COOKIE: &str = "kenning_session";
const SESSION_TTL: Duration = Duration::days(30);

/// Hash a password with Argon2id. Runs on a blocking thread: it is slow on purpose.
pub async fn hash_password(password: String) -> anyhow::Result<String> {
    tokio::task::spawn_blocking(move || {
        Argon2::default()
            .hash_password(password.as_bytes())
            .map(|hash| hash.to_string())
            .map_err(|err| anyhow::anyhow!("hash password: {err}"))
    })
    .await?
}

/// Check `password` against a stored hash. With no hash, checks against a
/// dummy one so unknown accounts take as long as wrong passwords.
pub async fn verify_password(password: String, hash: Option<String>) -> anyhow::Result<bool> {
    static DUMMY: LazyLock<String> = LazyLock::new(|| {
        Argon2::default()
            .hash_password(b"kenning-dummy-password")
            .expect("hash dummy password")
            .to_string()
    });
    tokio::task::spawn_blocking(move || {
        let known = hash.is_some();
        let hash = hash.unwrap_or_else(|| DUMMY.clone());
        let parsed =
            PasswordHash::new(&hash).map_err(|err| anyhow::anyhow!("parse hash: {err}"))?;
        Ok(known
            && Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok())
    })
    .await?
}

/// Start a session for `user_id` and return the cookie that carries it.
pub async fn create_session(
    conn: &mut PgConnection,
    config: &Config,
    user_id: Uuid,
) -> sqlx::Result<Cookie<'static>> {
    let token = tokens::generate();
    sqlx::query("INSERT INTO sessions (token_hash, user_id, expires_at) VALUES ($1, $2, $3)")
        .bind(tokens::hash(&token))
        .bind(user_id)
        .bind(Utc::now() + SESSION_TTL)
        .execute(conn)
        .await?;
    Ok(session_cookie(config, token, SESSION_TTL))
}

/// End the session in `jar`, if any, and return the cookie that clears it.
pub async fn end_session(
    conn: &mut PgConnection,
    config: &Config,
    jar: &CookieJar,
) -> sqlx::Result<Cookie<'static>> {
    if let Some(cookie) = jar.get(SESSION_COOKIE) {
        sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
            .bind(tokens::hash(cookie.value()))
            .execute(conn)
            .await?;
    }
    Ok(session_cookie(config, String::new(), Duration::zero()))
}

/// Sign the user out everywhere (after a password reset).
pub async fn end_all_sessions(conn: &mut PgConnection, user_id: Uuid) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM sessions WHERE user_id = $1")
        .bind(user_id)
        .execute(conn)
        .await?;
    Ok(())
}

fn session_cookie(config: &Config, value: String, max_age: Duration) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE, value))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(config.secure_cookies())
        .max_age(cookie::time::Duration::seconds(max_age.num_seconds()))
        .build()
}

/// The signed-in user. Rejects the request with 401 when there is none.
#[derive(Clone, Debug)]
pub struct CurrentUser(pub User);

/// The signed-in user, if any.
#[derive(Clone, Debug)]
pub struct MaybeUser(pub Option<User>);

impl FromRequestParts<AppState> for MaybeUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        let Some(cookie) = jar.get(SESSION_COOKIE) else {
            return Ok(Self(None));
        };
        let mut conn = state.pool.acquire().await?;
        // Slide the expiry forward, at most once an hour per session.
        let user_id: Option<Uuid> = sqlx::query_scalar(
            "WITH s AS (
                 SELECT token_hash, user_id FROM sessions WHERE token_hash = $1 AND expires_at > now()
             ), touched AS (
                 UPDATE sessions SET last_seen_at = now(), expires_at = now() + make_interval(days => $2)
                 WHERE token_hash IN (SELECT token_hash FROM s) AND last_seen_at < now() - interval '1 hour'
             )
             SELECT user_id FROM s",
        )
        .bind(tokens::hash(cookie.value()))
        .bind(SESSION_TTL.num_days() as i32)
        .fetch_optional(&mut *conn)
        .await?;
        let Some(user_id) = user_id else {
            return Ok(Self(None));
        };
        let user = accounts::find_by_id(&mut conn, user_id).await?;
        Ok(Self(user.filter(|user| !user.disabled)))
    }
}

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        match MaybeUser::from_request_parts(parts, state).await? {
            MaybeUser(Some(user)) => Ok(Self(user)),
            MaybeUser(None) => Err(AppError::Unauthorized),
        }
    }
}
