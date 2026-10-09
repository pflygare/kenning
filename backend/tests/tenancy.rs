//! Row-level security keeps organizations apart even when a query forgets to filter.

use kenning_server::{
    audit::{self, Event},
    db,
};
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(migrator = "kenning_server::db::MIGRATOR")]
async fn org_transactions_only_see_their_own_rows(pool: PgPool) {
    let (acme, globex) = (Uuid::now_v7(), Uuid::now_v7());
    for org in [acme, globex] {
        let mut tx = db::begin_org(&pool, org).await.unwrap();
        audit::record(&mut tx, Event::new("page.created").org(org))
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }

    let mut tx = db::begin_org(&pool, acme).await.unwrap();
    // No WHERE clause on purpose.
    let visible: Vec<Uuid> = sqlx::query_scalar("SELECT org_id FROM audit_events")
        .fetch_all(&mut *tx)
        .await
        .unwrap();
    assert_eq!(visible, vec![acme]);
}

#[sqlx::test(migrator = "kenning_server::db::MIGRATOR")]
async fn org_transactions_cannot_write_into_another_org(pool: PgPool) {
    let (acme, globex) = (Uuid::now_v7(), Uuid::now_v7());
    let mut tx = db::begin_org(&pool, acme).await.unwrap();
    let result = audit::record(&mut tx, Event::new("page.created").org(globex)).await;
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("row-level security")
    );
}
