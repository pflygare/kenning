use kenning_server::{
    audit::{self, ChainBreak, Event},
    db,
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(migrator = "kenning_server::db::MIGRATOR")]
async fn events_form_a_verifiable_chain(pool: PgPool) {
    let org = Uuid::now_v7();
    let mut tx = db::begin_org(&pool, org).await.unwrap();
    let first = audit::record(
        &mut tx,
        Event::new("page.created")
            .org(org)
            .details(json!({"title": "Hello"})),
    )
    .await
    .unwrap();
    let second = audit::record(
        &mut tx,
        Event::new("page.published").org(org).actor(Uuid::now_v7()),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(first.prev_hash, None);
    assert_eq!(second.prev_hash.as_deref(), Some(first.hash.as_slice()));

    let mut conn = pool.acquire().await.unwrap();
    assert_eq!(
        audit::verify_chain(&mut conn, Some(org)).await.unwrap(),
        Ok(2)
    );
}

#[sqlx::test(migrator = "kenning_server::db::MIGRATOR")]
async fn system_events_have_their_own_chain(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    audit::record(&mut conn, Event::new("user.signed_in"))
        .await
        .unwrap();
    audit::record(&mut conn, Event::new("user.signed_in"))
        .await
        .unwrap();
    assert_eq!(audit::verify_chain(&mut conn, None).await.unwrap(), Ok(2));
}

#[sqlx::test(migrator = "kenning_server::db::MIGRATOR")]
async fn audit_rows_cannot_be_changed(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    audit::record(&mut conn, Event::new("user.signed_in"))
        .await
        .unwrap();

    let update = sqlx::query("UPDATE audit_events SET action = 'tampered'")
        .execute(&mut *conn)
        .await;
    assert!(update.unwrap_err().to_string().contains("append-only"));

    let delete = sqlx::query("DELETE FROM audit_events")
        .execute(&mut *conn)
        .await;
    assert!(delete.unwrap_err().to_string().contains("append-only"));
}

#[sqlx::test(migrator = "kenning_server::db::MIGRATOR")]
async fn verify_detects_tampering(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    audit::record(&mut conn, Event::new("a")).await.unwrap();
    let second = audit::record(&mut conn, Event::new("b")).await.unwrap();
    audit::record(&mut conn, Event::new("c")).await.unwrap();

    // Simulate someone with database access bypassing the trigger.
    sqlx::query("ALTER TABLE audit_events DISABLE TRIGGER audit_events_no_update_or_delete")
        .execute(&mut *conn)
        .await
        .unwrap();
    sqlx::query("UPDATE audit_events SET action = 'tampered' WHERE seq = $1")
        .bind(second.seq)
        .execute(&mut *conn)
        .await
        .unwrap();

    assert_eq!(
        audit::verify_chain(&mut conn, None).await.unwrap(),
        Err(ChainBreak {
            seq: second.seq,
            reason: "event content does not match its hash"
        })
    );
}
