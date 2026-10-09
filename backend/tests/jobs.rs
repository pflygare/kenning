use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use kenning_server::jobs::{self, Worker};
use serde_json::json;
use sqlx::PgPool;

#[sqlx::test(migrator = "kenning_server::db::MIGRATOR")]
async fn runs_a_job_once(pool: PgPool) {
    let seen = Arc::new(AtomicUsize::new(0));
    let counter = seen.clone();
    let worker = Worker::new(pool.clone()).handle("count", move |payload| {
        let counter = counter.clone();
        async move {
            counter.fetch_add(payload["by"].as_u64().unwrap() as usize, Ordering::SeqCst);
            Ok(())
        }
    });

    let mut conn = pool.acquire().await.unwrap();
    jobs::enqueue(&mut conn, "count", json!({"by": 3}))
        .await
        .unwrap();

    assert!(worker.run_once().await.unwrap());
    assert!(
        !worker.run_once().await.unwrap(),
        "a finished job is not run again"
    );
    assert_eq!(seen.load(Ordering::SeqCst), 3);
}

#[sqlx::test(migrator = "kenning_server::db::MIGRATOR")]
async fn failed_jobs_are_rescheduled_with_the_error(pool: PgPool) {
    let worker =
        Worker::new(pool.clone()).handle("flaky", |_| async { anyhow::bail!("smtp down") });
    let mut conn = pool.acquire().await.unwrap();
    let id = jobs::enqueue(&mut conn, "flaky", json!({})).await.unwrap();

    assert!(worker.run_once().await.unwrap());
    // Backoff pushed it into the future, so it is not ready yet.
    assert!(!worker.run_once().await.unwrap());

    let (attempts, last_error, done): (i32, Option<String>, bool) =
        sqlx::query_as("SELECT attempts, last_error, done_at IS NOT NULL FROM jobs WHERE id = $1")
            .bind(id)
            .fetch_one(&mut *conn)
            .await
            .unwrap();
    assert_eq!(
        (attempts, last_error.as_deref(), done),
        (1, Some("smtp down"), false)
    );
}

#[sqlx::test(migrator = "kenning_server::db::MIGRATOR")]
async fn ignores_jobs_it_has_no_handler_for(pool: PgPool) {
    let worker = Worker::new(pool.clone()).handle("known", |_| async { Ok(()) });
    let mut conn = pool.acquire().await.unwrap();
    jobs::enqueue(&mut conn, "unknown", json!({}))
        .await
        .unwrap();
    assert!(!worker.run_once().await.unwrap());
}
