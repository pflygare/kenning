mod common;

use axum::http::StatusCode;
use common::{Browser, emails_to, link_token};
use serde_json::json;
use sqlx::PgPool;

const MIGRATOR: &sqlx::migrate::Migrator = &kenning_server::db::MIGRATOR;

async fn owner_with_org(pool: &PgPool) -> Browser {
    let mut owner = Browser::new(pool);
    owner.sign_up_verified(pool, "Ada", "ada@example.com").await;
    let (status, org) = owner.post("/api/orgs", json!({"name": "Acme Corp"})).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(org["slug"], "acme-corp");
    assert_eq!(org["role"], "owner");
    owner
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn creating_an_org_needs_a_confirmed_email(pool: PgPool) {
    let mut browser = Browser::new(&pool);
    browser
        .post(
            "/api/auth/signup",
            json!({"name": "Ada", "email": "ada@example.com", "password": "correct horse"}),
        )
        .await;
    let (status, body) = browser.post("/api/orgs", json!({"name": "Acme"})).await;
    assert_eq!(
        (status, body["error"]["code"].as_str()),
        (StatusCode::FORBIDDEN, Some("email_unverified"))
    );
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn org_slugs_are_unique_and_not_reserved(pool: PgPool) {
    let mut owner = owner_with_org(&pool).await;
    let (_, second) = owner.post("/api/orgs", json!({"name": "Acme Corp"})).await;
    assert_eq!(second["slug"], "acme-corp-2");

    let (status, body) = owner
        .post("/api/orgs", json!({"name": "Other", "slug": "acme-corp"}))
        .await;
    assert_eq!(
        (status, body["error"]["code"].as_str()),
        (StatusCode::CONFLICT, Some("slug_taken"))
    );

    let (status, body) = owner.post("/api/orgs", json!({"name": "Login"})).await;
    assert_eq!(
        (status, body["error"]["code"].as_str()),
        (StatusCode::BAD_REQUEST, Some("invalid_slug"))
    );

    let (_, me) = owner.get("/api/auth/me").await;
    assert_eq!(me["orgs"].as_array().unwrap().len(), 2);
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn non_members_cannot_see_an_org(pool: PgPool) {
    owner_with_org(&pool).await;
    let mut stranger = Browser::new(&pool);
    stranger
        .sign_up_verified(&pool, "Eve", "eve@example.com")
        .await;
    assert_eq!(
        stranger.get("/api/orgs/acme-corp").await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        stranger.get("/api/orgs/acme-corp/members").await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        stranger.get("/api/orgs/no-such-org").await.0,
        StatusCode::NOT_FOUND
    );
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn invite_someone_without_an_account(pool: PgPool) {
    let mut owner = owner_with_org(&pool).await;
    let (status, invite) = owner
        .post(
            "/api/orgs/acme-corp/invites",
            json!({"email": "Grace@Example.com", "role": "admin"}),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(invite["email"], "grace@example.com");

    let emails = emails_to(&pool, "grace@example.com").await;
    assert!(emails[0].0.contains("Ada invited you to Acme Corp"));
    let token = link_token(&pool, "grace@example.com", "/invite/").await;

    let mut grace = Browser::new(&pool);
    let (status, preview) = grace.get(&format!("/api/invites/{token}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(preview["org_name"], "Acme Corp");
    assert_eq!(preview["status"], "open");
    assert_eq!(preview["account_exists"], false);

    // Signing up from the invitation proves the address: no confirmation email.
    let (status, me) = grace
        .post("/api/auth/signup", json!({"name": "Grace", "email": "grace@example.com", "password": "correct horse", "invite_token": token}))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["user"]["email_verified"], true);
    assert_eq!(emails_to(&pool, "grace@example.com").await.len(), 1);

    let (status, membership) = grace
        .post(&format!("/api/invites/{token}/accept"), json!({}))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(membership["slug"], "acme-corp");
    assert_eq!(membership["role"], "admin");

    let (_, members) = owner.get("/api/orgs/acme-corp/members").await;
    let emails: Vec<_> = members
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["email"].as_str().unwrap())
        .collect();
    assert_eq!(emails, ["ada@example.com", "grace@example.com"]);

    let (_, preview) = grace.get(&format!("/api/invites/{token}")).await;
    assert_eq!(preview["status"], "accepted");
    assert_eq!(
        grace
            .post(&format!("/api/invites/{token}/accept"), json!({}))
            .await
            .0,
        StatusCode::GONE
    );
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn invites_only_work_for_the_invited_address(pool: PgPool) {
    let mut owner = owner_with_org(&pool).await;
    owner
        .post(
            "/api/orgs/acme-corp/invites",
            json!({"email": "grace@example.com", "role": "member"}),
        )
        .await;
    let token = link_token(&pool, "grace@example.com", "/invite/").await;

    let mut eve = Browser::new(&pool);
    eve.sign_up_verified(&pool, "Eve", "eve@example.com").await;
    let (status, body) = eve
        .post(&format!("/api/invites/{token}/accept"), json!({}))
        .await;
    assert_eq!(
        (status, body["error"]["code"].as_str()),
        (StatusCode::FORBIDDEN, Some("invite_email_mismatch"))
    );
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn reinviting_refreshes_the_link_and_revoking_kills_it(pool: PgPool) {
    let mut owner = owner_with_org(&pool).await;
    owner
        .post(
            "/api/orgs/acme-corp/invites",
            json!({"email": "grace@example.com", "role": "member"}),
        )
        .await;
    let first = link_token(&pool, "grace@example.com", "/invite/").await;
    let (_, invite) = owner
        .post(
            "/api/orgs/acme-corp/invites",
            json!({"email": "grace@example.com", "role": "admin"}),
        )
        .await;
    let second = link_token(&pool, "grace@example.com", "/invite/").await;
    assert_ne!(first, second);

    let (_, pending) = owner.get("/api/orgs/acme-corp/invites").await;
    assert_eq!(
        pending.as_array().unwrap().len(),
        1,
        "one open invite per address"
    );
    assert_eq!(pending[0]["role"], "admin");
    assert_eq!(
        owner.get(&format!("/api/invites/{first}")).await.0,
        StatusCode::NOT_FOUND
    );

    let id = invite["id"].as_str().unwrap();
    assert_eq!(
        owner
            .delete(&format!("/api/orgs/acme-corp/invites/{id}"))
            .await
            .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        owner.get(&format!("/api/invites/{second}")).await.1["status"],
        "revoked"
    );
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn existing_members_cannot_be_invited(pool: PgPool) {
    let mut owner = owner_with_org(&pool).await;
    let (status, body) = owner
        .post(
            "/api/orgs/acme-corp/invites",
            json!({"email": "ada@example.com", "role": "member"}),
        )
        .await;
    assert_eq!(
        (status, body["error"]["code"].as_str()),
        (StatusCode::CONFLICT, Some("already_member"))
    );
}

/// Owner Ada; Bob joins as `role`. Returns (Ada, Bob, Bob's user id).
async fn org_with_second_member(pool: &PgPool, role: &str) -> (Browser, Browser, String) {
    let mut owner = owner_with_org(pool).await;
    owner
        .post(
            "/api/orgs/acme-corp/invites",
            json!({"email": "bob@example.com", "role": role}),
        )
        .await;
    let token = link_token(pool, "bob@example.com", "/invite/").await;
    let mut bob = Browser::new(pool);
    let (_, me) = bob
        .post("/api/auth/signup", json!({"name": "Bob", "email": "bob@example.com", "password": "correct horse", "invite_token": token}))
        .await;
    bob.post(&format!("/api/invites/{token}/accept"), json!({}))
        .await;
    let bob_id = me["user"]["id"].as_str().unwrap().to_string();
    (owner, bob, bob_id)
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn members_cannot_manage_members(pool: PgPool) {
    let (_, mut bob, _) = org_with_second_member(&pool, "member").await;
    assert_eq!(
        bob.get("/api/orgs/acme-corp/members").await.0,
        StatusCode::OK,
        "members can see who is in the org"
    );
    assert_eq!(
        bob.get("/api/orgs/acme-corp/invites").await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        bob.post(
            "/api/orgs/acme-corp/invites",
            json!({"email": "x@example.com", "role": "member"})
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn admins_cannot_touch_owners(pool: PgPool) {
    let (mut ada, mut bob, _) = org_with_second_member(&pool, "admin").await;
    let (_, me) = ada.get("/api/auth/me").await;
    let ada_id = me["user"]["id"].as_str().unwrap();

    assert_eq!(
        bob.patch(
            &format!("/api/orgs/acme-corp/members/{ada_id}"),
            json!({"role": "member"})
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        bob.delete(&format!("/api/orgs/acme-corp/members/{ada_id}"))
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        bob.post(
            "/api/orgs/acme-corp/invites",
            json!({"email": "x@example.com", "role": "owner"})
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn the_last_owner_stays(pool: PgPool) {
    let (mut ada, mut bob, bob_id) = org_with_second_member(&pool, "member").await;
    let (_, me) = ada.get("/api/auth/me").await;
    let ada_id = me["user"]["id"].as_str().unwrap().to_string();

    let (status, body) = ada
        .delete(&format!("/api/orgs/acme-corp/members/{ada_id}"))
        .await;
    assert_eq!(
        (status, body["error"]["code"].as_str()),
        (StatusCode::CONFLICT, Some("last_owner"))
    );
    let (status, _) = ada
        .patch(
            &format!("/api/orgs/acme-corp/members/{ada_id}"),
            json!({"role": "admin"}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // Hand over ownership, then leave.
    assert_eq!(
        ada.patch(
            &format!("/api/orgs/acme-corp/members/{bob_id}"),
            json!({"role": "owner"})
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        ada.delete(&format!("/api/orgs/acme-corp/members/{ada_id}"))
            .await
            .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        ada.get("/api/orgs/acme-corp").await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(bob.get("/api/orgs/acme-corp").await.1["role"], "owner");
}

#[sqlx::test(migrator = "MIGRATOR")]
async fn org_actions_are_audited(pool: PgPool) {
    let (_, _, _) = org_with_second_member(&pool, "member").await;
    let actions: Vec<String> = sqlx::query_scalar(
        "SELECT a.action FROM audit_events a JOIN organizations o ON o.id = a.org_id WHERE o.slug = 'acme-corp' ORDER BY a.seq",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(actions, ["org.created", "invite.sent", "invite.accepted"]);
}
