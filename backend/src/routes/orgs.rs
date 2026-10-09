use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, patch},
};
use serde::Deserialize;
use serde_json::json;
use ts_rs::TS;
use uuid::Uuid;

use crate::{
    AppError, AppResult, AppState, accounts,
    audit::{self, Event},
    auth::CurrentUser,
    db, invites,
    orgs::{self, Member, Membership, OrgContext, PendingInvite, Role},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/orgs", axum::routing::post(create))
        .route("/orgs/{org}", get(show))
        .route("/orgs/{org}/members", get(members))
        .route(
            "/orgs/{org}/members/{user_id}",
            patch(change_role).delete(remove_member),
        )
        .route("/orgs/{org}/invites", get(list_invites).post(invite))
        .route("/orgs/{org}/invites/{invite_id}", delete(revoke_invite))
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct CreateOrgRequest {
    pub name: String,
    /// Defaults to one made from the name.
    #[ts(optional)]
    pub slug: Option<String>,
}

async fn create(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(req): Json<CreateOrgRequest>,
) -> AppResult<(StatusCode, Json<Membership>)> {
    if !user.email_verified {
        return Err(AppError::coded(
            StatusCode::FORBIDDEN,
            "email_unverified",
            "Confirm your email address before creating an organization.",
        ));
    }
    let name = accounts::validate_name(&req.name)?;
    let requested = req.slug.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let base = orgs::slugify(requested.unwrap_or(&name));
    if base.is_empty() || orgs::is_reserved(&base) {
        return Err(AppError::coded(
            StatusCode::BAD_REQUEST,
            "invalid_slug",
            "Choose a different web address: use letters and numbers.",
        ));
    }

    // A chosen address must be free; one made from the name gets a number if taken.
    let candidates: Vec<String> = if requested.is_some() {
        vec![base.clone()]
    } else {
        std::iter::once(base.clone())
            .chain((2..=50).map(|n| format!("{base}-{n}")))
            .collect()
    };
    let org_id = Uuid::now_v7();
    let mut tx = state.pool.begin().await?;
    let mut slug = None;
    for candidate in candidates {
        let inserted = sqlx::query(
            "INSERT INTO organizations (id, name, slug, created_by) VALUES ($1, $2, $3, $4) ON CONFLICT (slug) DO NOTHING",
        )
        .bind(org_id)
        .bind(&name)
        .bind(&candidate)
        .bind(user.id)
        .execute(&mut *tx)
        .await?;
        if inserted.rows_affected() == 1 {
            slug = Some(candidate);
            break;
        }
    }
    let slug = slug.ok_or_else(|| {
        AppError::coded(
            StatusCode::CONFLICT,
            "slug_taken",
            "That web address is taken. Choose another.",
        )
    })?;
    db::set_org(&mut tx, org_id).await?;
    sqlx::query("INSERT INTO memberships (org_id, user_id, role) VALUES ($1, $2, 'owner')")
        .bind(org_id)
        .bind(user.id)
        .execute(&mut *tx)
        .await?;
    audit::record(
        &mut tx,
        Event::new("org.created")
            .org(org_id)
            .actor(user.id)
            .object("organization", org_id)
            .details(json!({"name": name, "slug": slug})),
    )
    .await?;
    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(Membership {
            id: org_id,
            name,
            slug,
            role: Role::Owner,
        }),
    ))
}

async fn show(ctx: OrgContext) -> Json<Membership> {
    Json(Membership {
        id: ctx.org.id,
        name: ctx.org.name,
        slug: ctx.org.slug,
        role: ctx.role,
    })
}

async fn members(State(state): State<AppState>, ctx: OrgContext) -> AppResult<Json<Vec<Member>>> {
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    let members = sqlx::query_as(
        "SELECT u.id AS user_id, u.name, u.email, u.avatar_url, m.role, m.created_at AS joined_at
         FROM memberships m JOIN users u ON u.id = m.user_id
         ORDER BY u.name",
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(members))
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct ChangeRoleRequest {
    pub role: Role,
}

/// The target's current role, locking the organization's owner rows so two
/// requests cannot both remove "the other" owner.
async fn lock_target(
    tx: &mut sqlx::PgConnection,
    org_id: Uuid,
    user_id: Uuid,
) -> AppResult<(Role, i64)> {
    let owners: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM (SELECT 1 FROM memberships WHERE org_id = $1 AND role = 'owner' FOR UPDATE) o",
    )
    .bind(org_id)
    .fetch_one(&mut *tx)
    .await?;
    let role: Role = sqlx::query_scalar(
        "SELECT role FROM memberships WHERE org_id = $1 AND user_id = $2 FOR UPDATE",
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok((role, owners))
}

fn last_owner() -> AppError {
    AppError::coded(
        StatusCode::CONFLICT,
        "last_owner",
        "An organization needs at least one owner. Make someone else an owner first.",
    )
}

async fn change_role(
    State(state): State<AppState>,
    ctx: OrgContext,
    Path((_, user_id)): Path<(String, Uuid)>,
    Json(req): Json<ChangeRoleRequest>,
) -> AppResult<StatusCode> {
    ctx.require_manager()?;
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    let (from, owners) = lock_target(&mut tx, ctx.org.id, user_id).await?;
    if !orgs::can_change_role(ctx.role, from, req.role) {
        return Err(AppError::coded(
            StatusCode::FORBIDDEN,
            "forbidden",
            "Only owners can grant or remove the owner role.",
        ));
    }
    if from == Role::Owner && req.role != Role::Owner && owners <= 1 {
        return Err(last_owner());
    }
    if from != req.role {
        sqlx::query("UPDATE memberships SET role = $3 WHERE org_id = $1 AND user_id = $2")
            .bind(ctx.org.id)
            .bind(user_id)
            .bind(req.role)
            .execute(&mut *tx)
            .await?;
        audit::record(
            &mut tx,
            Event::new("member.role_changed")
                .org(ctx.org.id)
                .actor(ctx.user.id)
                .object("user", user_id)
                .details(json!({"from": from, "to": req.role})),
        )
        .await?;
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn remove_member(
    State(state): State<AppState>,
    ctx: OrgContext,
    Path((_, user_id)): Path<(String, Uuid)>,
) -> AppResult<StatusCode> {
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    let (target, owners) = lock_target(&mut tx, ctx.org.id, user_id).await?;
    let is_self = user_id == ctx.user.id;
    if !orgs::can_remove(ctx.role, target, is_self) {
        return Err(AppError::coded(
            StatusCode::FORBIDDEN,
            "forbidden",
            "You can't remove this member.",
        ));
    }
    if target == Role::Owner && owners <= 1 {
        return Err(last_owner());
    }
    sqlx::query("DELETE FROM memberships WHERE org_id = $1 AND user_id = $2")
        .bind(ctx.org.id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    let action = if is_self {
        "member.left"
    } else {
        "member.removed"
    };
    audit::record(
        &mut tx,
        Event::new(action)
            .org(ctx.org.id)
            .actor(ctx.user.id)
            .object("user", user_id),
    )
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_invites(
    State(state): State<AppState>,
    ctx: OrgContext,
) -> AppResult<Json<Vec<PendingInvite>>> {
    ctx.require_manager()?;
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    let pending = invites::pending(&mut tx, ctx.org.id).await?;
    tx.commit().await?;
    Ok(Json(pending))
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct InviteRequest {
    pub email: String,
    pub role: Role,
}

async fn invite(
    State(state): State<AppState>,
    ctx: OrgContext,
    Json(req): Json<InviteRequest>,
) -> AppResult<(StatusCode, Json<PendingInvite>)> {
    ctx.require_manager()?;
    if req.role == Role::Owner && ctx.role != Role::Owner {
        return Err(AppError::coded(
            StatusCode::FORBIDDEN,
            "forbidden",
            "Only owners can invite owners.",
        ));
    }
    let email = accounts::normalize_email(&req.email)?;
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    let pending = invites::send(
        &mut tx,
        &state.config,
        &ctx.org,
        &ctx.user,
        &email,
        req.role,
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(pending)))
}

async fn revoke_invite(
    State(state): State<AppState>,
    ctx: OrgContext,
    Path((_, invite_id)): Path<(String, Uuid)>,
) -> AppResult<StatusCode> {
    ctx.require_manager()?;
    let mut tx = db::begin_org(&state.pool, ctx.org.id).await?;
    invites::revoke(&mut tx, ctx.org.id, &ctx.user, invite_id).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
