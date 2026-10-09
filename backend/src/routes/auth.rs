use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use chrono::Duration;
use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;

use crate::{
    AppError, AppResult, AppState,
    accounts::{self, EmailTokenKind, User},
    audit::{self, Event},
    auth::{self, CurrentUser, MaybeUser},
    invites,
    orgs::{self, Membership},
};

const OAUTH_COOKIE: &str = "kenning_oauth";
const OAUTH_FLOW_TTL: Duration = Duration::minutes(10);

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/config", get(config))
        .route("/auth/me", get(me))
        .route("/auth/signup", post(signup))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/verify-email", post(verify_email))
        .route("/auth/resend-verification", post(resend_verification))
        .route("/auth/forgot-password", post(forgot_password))
        .route("/auth/reset-password", post(reset_password))
        .route("/auth/google/start", get(google_start))
        .route("/auth/google/callback", get(google_callback))
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct AuthConfig {
    /// Whether "Sign in with Google" is available.
    pub google: bool,
    /// Whether the testing page at `/dev` is on.
    pub dev_tools: bool,
}

async fn config(State(state): State<AppState>) -> Json<AuthConfig> {
    Json(AuthConfig {
        google: state.google.is_some(),
        dev_tools: state.config.dev_tools,
    })
}

/// The signed-in user and the organizations they belong to.
#[derive(Serialize, TS)]
#[ts(export)]
pub struct Me {
    pub user: User,
    pub orgs: Vec<Membership>,
}

async fn me_for(state: &AppState, user: User) -> AppResult<Json<Me>> {
    let mut conn = state.pool.acquire().await?;
    let orgs = orgs::memberships_of(&mut conn, user.id).await?;
    Ok(Json(Me { user, orgs }))
}

async fn me(State(state): State<AppState>, CurrentUser(user): CurrentUser) -> AppResult<Json<Me>> {
    me_for(&state, user).await
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct SignupRequest {
    pub name: String,
    pub email: String,
    pub password: String,
    /// Signing up from an invitation sent to this same address proves the
    /// address, so no confirmation email is needed.
    #[ts(optional)]
    pub invite_token: Option<String>,
}

async fn signup(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<SignupRequest>,
) -> AppResult<(CookieJar, Json<Me>)> {
    let name = accounts::validate_name(&req.name)?;
    let email = accounts::normalize_email(&req.email)?;
    accounts::validate_password(&req.password)?;
    let password_hash = auth::hash_password(req.password).await?;

    let mut tx = state.pool.begin().await?;
    let verified_by_invite = match &req.invite_token {
        Some(token) => {
            invites::open_invite_email(&mut tx, token).await?.as_deref() == Some(email.as_str())
        }
        None => false,
    };
    let user = accounts::create_user(&mut tx, &email, &name, None, verified_by_invite).await?;
    accounts::add_identity(
        &mut tx,
        user.id,
        "password",
        &user.id.to_string(),
        Some(&password_hash),
    )
    .await?;
    if !verified_by_invite {
        accounts::send_email_token(&mut tx, &state.config, &user, EmailTokenKind::VerifyEmail)
            .await?;
    }
    audit::record(
        &mut tx,
        Event::new("user.signed_up")
            .actor(user.id)
            .object("user", user.id)
            .details(json!({"provider": "password"})),
    )
    .await?;
    let cookie = auth::create_session(&mut tx, &state.config, user.id).await?;
    tx.commit().await?;

    let me = me_for(&state, user).await?;
    Ok((jar.add(cookie), me))
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<LoginRequest>,
) -> AppResult<(CookieJar, Json<Me>)> {
    let invalid = || {
        AppError::coded(
            StatusCode::UNAUTHORIZED,
            "invalid_credentials",
            "Email or password is incorrect.",
        )
    };
    let email = accounts::normalize_email(&req.email).map_err(|_| invalid())?;

    let mut tx = state.pool.begin().await?;
    let user = accounts::find_by_email(&mut tx, &email).await?;
    let hash = match &user {
        Some(user) => accounts::password_hash(&mut tx, user.id).await?,
        None => None,
    };
    let ok = auth::verify_password(req.password, hash).await?;
    let user = match user {
        Some(user) if ok && !user.disabled => user,
        _ => return Err(invalid()),
    };
    audit::record(
        &mut tx,
        Event::new("user.signed_in")
            .actor(user.id)
            .object("user", user.id)
            .details(json!({"provider": "password"})),
    )
    .await?;
    let cookie = auth::create_session(&mut tx, &state.config, user.id).await?;
    tx.commit().await?;

    let me = me_for(&state, user).await?;
    Ok((jar.add(cookie), me))
}

async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
    MaybeUser(user): MaybeUser,
) -> AppResult<(CookieJar, StatusCode)> {
    let mut tx = state.pool.begin().await?;
    let cookie = auth::end_session(&mut tx, &state.config, &jar).await?;
    if let Some(user) = user {
        audit::record(
            &mut tx,
            Event::new("user.signed_out")
                .actor(user.id)
                .object("user", user.id),
        )
        .await?;
    }
    tx.commit().await?;
    Ok((jar.add(cookie), StatusCode::NO_CONTENT))
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct TokenRequest {
    pub token: String,
}

async fn verify_email(
    State(state): State<AppState>,
    Json(req): Json<TokenRequest>,
) -> AppResult<StatusCode> {
    let mut tx = state.pool.begin().await?;
    let user =
        accounts::consume_email_token(&mut tx, EmailTokenKind::VerifyEmail, &req.token).await?;
    accounts::mark_verified(&mut tx, user.id).await?;
    audit::record(
        &mut tx,
        Event::new("user.email_verified")
            .actor(user.id)
            .object("user", user.id),
    )
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn resend_verification(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<StatusCode> {
    if !user.email_verified {
        let mut tx = state.pool.begin().await?;
        accounts::send_email_token(&mut tx, &state.config, &user, EmailTokenKind::VerifyEmail)
            .await?;
        tx.commit().await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

/// Always succeeds, so the form cannot be used to find out who has an account.
async fn forgot_password(
    State(state): State<AppState>,
    Json(req): Json<ForgotPasswordRequest>,
) -> AppResult<StatusCode> {
    let Ok(email) = accounts::normalize_email(&req.email) else {
        return Ok(StatusCode::NO_CONTENT);
    };
    let mut tx = state.pool.begin().await?;
    if let Some(user) = accounts::find_by_email(&mut tx, &email)
        .await?
        .filter(|user| !user.disabled)
    {
        accounts::send_email_token(&mut tx, &state.config, &user, EmailTokenKind::ResetPassword)
            .await?;
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub password: String,
}

/// Set a new password, sign out every other session, and sign in here.
async fn reset_password(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<ResetPasswordRequest>,
) -> AppResult<(CookieJar, Json<Me>)> {
    accounts::validate_password(&req.password)?;
    let password_hash = auth::hash_password(req.password).await?;

    let mut tx = state.pool.begin().await?;
    let user =
        accounts::consume_email_token(&mut tx, EmailTokenKind::ResetPassword, &req.token).await?;
    accounts::set_password(&mut tx, &user, &password_hash).await?;
    // The link reached their inbox, so the address is theirs.
    accounts::mark_verified(&mut tx, user.id).await?;
    auth::end_all_sessions(&mut tx, user.id).await?;
    audit::record(
        &mut tx,
        Event::new("user.password_reset")
            .actor(user.id)
            .object("user", user.id),
    )
    .await?;
    let cookie = auth::create_session(&mut tx, &state.config, user.id).await?;
    tx.commit().await?;

    let user = {
        let mut conn = state.pool.acquire().await?;
        accounts::find_by_id(&mut conn, user.id)
            .await?
            .ok_or(AppError::NotFound)?
    };
    let me = me_for(&state, user).await?;
    Ok((jar.add(cookie), me))
}

#[derive(Deserialize)]
struct GoogleStartQuery {
    next: Option<String>,
}

/// Only same-site paths, so the redirect after sign-in can't send people elsewhere.
fn safe_next(next: Option<String>) -> String {
    match next {
        Some(path) if path.starts_with('/') && !path.starts_with("//") && !path.contains('\\') => {
            path
        }
        _ => "/".to_string(),
    }
}

async fn google_start(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<GoogleStartQuery>,
) -> AppResult<Response> {
    let google = state.google.as_ref().ok_or(AppError::NotFound)?;
    let started = google.start().await?;

    let mut tx = state.pool.begin().await?;
    sqlx::query("DELETE FROM oauth_flows WHERE created_at < now() - make_interval(mins => $1)")
        .bind(OAUTH_FLOW_TTL.num_minutes() as i32)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO oauth_flows (state, nonce, pkce_verifier, next_path) VALUES ($1, $2, $3, $4)",
    )
    .bind(&started.state)
    .bind(&started.nonce)
    .bind(&started.pkce_verifier)
    .bind(safe_next(query.next))
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    // Ties the callback to this browser, so nobody can sign you in to their account.
    let cookie = Cookie::build((OAUTH_COOKIE, started.state))
        .path("/api/auth/google")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(state.config.secure_cookies())
        .max_age(cookie::time::Duration::minutes(
            OAUTH_FLOW_TTL.num_minutes(),
        ))
        .build();
    Ok((jar.add(cookie), Redirect::to(&started.authorize_url)).into_response())
}

#[derive(Deserialize)]
struct GoogleCallbackQuery {
    code: Option<String>,
    state: Option<String>,
}

async fn google_callback(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<GoogleCallbackQuery>,
) -> Response {
    let browser_state = jar
        .get(OAUTH_COOKIE)
        .map(|cookie| cookie.value().to_string());
    let jar = jar.remove(Cookie::build(OAUTH_COOKIE).path("/api/auth/google"));
    match finish_google(&state, browser_state, query).await {
        Ok((cookie, next)) => (jar.add(cookie), Redirect::to(&next)).into_response(),
        Err(err) => {
            tracing::warn!(error = ?err, "Google sign-in failed");
            (jar, Redirect::to("/login?error=google")).into_response()
        }
    }
}

async fn finish_google(
    state: &AppState,
    browser_state: Option<String>,
    query: GoogleCallbackQuery,
) -> anyhow::Result<(Cookie<'static>, String)> {
    let google = state
        .google
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Google sign-in is off"))?;
    let (Some(code), Some(returned_state)) = (query.code, query.state) else {
        anyhow::bail!("callback without code or state (the user may have cancelled)");
    };
    if browser_state.as_deref() != Some(returned_state.as_str()) {
        anyhow::bail!("state does not match this browser");
    }

    let flow: Option<(String, String, String)> = sqlx::query_as(
        "DELETE FROM oauth_flows WHERE state = $1 AND created_at > now() - make_interval(mins => $2)
         RETURNING nonce, pkce_verifier, next_path",
    )
    .bind(&returned_state)
    .bind(OAUTH_FLOW_TTL.num_minutes() as i32)
    .fetch_optional(&state.pool)
    .await?;
    let (nonce, pkce_verifier, next) =
        flow.ok_or_else(|| anyhow::anyhow!("unknown or expired sign-in attempt"))?;

    let profile = google.finish(code, nonce, pkce_verifier).await?;
    let email = accounts::normalize_email(&profile.email)
        .map_err(|_| anyhow::anyhow!("unusable email from Google"))?;
    let name = accounts::validate_name(&profile.name).unwrap_or_else(|_| email.clone());

    let mut tx = state.pool.begin().await?;
    let user = accounts::sign_in_external(
        &mut tx,
        "google",
        &profile.subject,
        &email,
        &name,
        profile.picture.as_deref(),
    )
    .await
    .map_err(|err| anyhow::anyhow!("{err}"))?;
    if user.disabled {
        anyhow::bail!("account is disabled");
    }
    audit::record(
        &mut tx,
        Event::new("user.signed_in")
            .actor(user.id)
            .object("user", user.id)
            .details(json!({"provider": "google"})),
    )
    .await?;
    let cookie = auth::create_session(&mut tx, &state.config, user.id).await?;
    tx.commit().await?;
    Ok((cookie, next))
}

#[cfg(test)]
mod tests {
    use super::safe_next;

    #[test]
    fn only_same_site_redirects() {
        assert_eq!(safe_next(Some("/acme/settings".into())), "/acme/settings");
        assert_eq!(safe_next(Some("https://evil.example".into())), "/");
        assert_eq!(safe_next(Some("//evil.example".into())), "/");
        assert_eq!(safe_next(Some("/\\evil.example".into())), "/");
        assert_eq!(safe_next(None), "/");
    }
}
