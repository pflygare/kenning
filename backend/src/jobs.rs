//! Background jobs stored in Postgres.
//!
//! Enqueue inside the transaction that needs the work done, so a job exists
//! only if that change committed. Workers claim jobs with
//! `FOR UPDATE SKIP LOCKED`, so any number of app servers can run them, and a
//! failed job is retried with backoff until it runs out of attempts.

use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc, time::Duration};

use serde_json::Value;
use sqlx::{FromRow, PgConnection, PgPool};
use tokio::sync::watch;
use uuid::Uuid;

/// How long a claimed job may run before another worker may take it over.
const LOCK_TIMEOUT_SECS: i64 = 300;

type HandlerFuture = Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send>>;
type Handler = Arc<dyn Fn(Value) -> HandlerFuture + Send + Sync>;

/// Queue `kind` with `payload` to run as soon as a worker is free.
pub async fn enqueue(conn: &mut PgConnection, kind: &str, payload: Value) -> sqlx::Result<Uuid> {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO jobs (id, kind, payload) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(kind)
        .bind(payload)
        .execute(conn)
        .await?;
    Ok(id)
}

#[derive(Debug, FromRow)]
struct ClaimedJob {
    id: Uuid,
    kind: String,
    payload: Value,
    attempts: i32,
    max_attempts: i32,
}

/// Runs registered handlers for queued jobs.
#[derive(Clone)]
pub struct Worker {
    pool: PgPool,
    handlers: HashMap<String, Handler>,
    poll_interval: Duration,
}

impl Worker {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            handlers: HashMap::new(),
            poll_interval: Duration::from_secs(1),
        }
    }

    pub fn poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// Register the handler for jobs of `kind`.
    pub fn handle<F, Fut>(mut self, kind: &str, handler: F) -> Self
    where
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        self.handlers.insert(
            kind.to_string(),
            Arc::new(move |payload| Box::pin(handler(payload))),
        );
        self
    }

    /// Process jobs until `shutdown` turns true.
    pub async fn run(self, mut shutdown: watch::Receiver<bool>) {
        tracing::info!(kinds = ?self.handlers.keys().collect::<Vec<_>>(), "job worker started");
        while !*shutdown.borrow() {
            match self.run_once().await {
                Ok(true) => continue,
                Ok(false) => {}
                Err(err) => tracing::error!(error = ?err, "job worker failed to poll"),
            }
            tokio::select! {
                _ = tokio::time::sleep(self.poll_interval) => {}
                _ = shutdown.changed() => {}
            }
        }
        tracing::info!("job worker stopped");
    }

    /// Claim and run one job. Returns `false` when nothing was ready.
    pub async fn run_once(&self) -> sqlx::Result<bool> {
        let kinds: Vec<String> = self.handlers.keys().cloned().collect();
        let job: Option<ClaimedJob> = sqlx::query_as(
            "UPDATE jobs SET locked_at = now(), attempts = attempts + 1
             WHERE id = (
                 SELECT id FROM jobs
                 WHERE done_at IS NULL
                   AND kind = ANY($1)
                   AND run_at <= now()
                   AND attempts < max_attempts
                   AND (locked_at IS NULL OR locked_at < now() - make_interval(secs => $2))
                 ORDER BY run_at
                 FOR UPDATE SKIP LOCKED
                 LIMIT 1
             )
             RETURNING id, kind, payload, attempts, max_attempts",
        )
        .bind(&kinds)
        .bind(LOCK_TIMEOUT_SECS as f64)
        .fetch_optional(&self.pool)
        .await?;

        let Some(job) = job else {
            return Ok(false);
        };
        let handler = self.handlers[&job.kind].clone();
        // Run on its own task so a panicking handler fails the job, not the worker.
        let outcome = tokio::spawn(handler(job.payload)).await;
        let result = match outcome {
            Ok(result) => result,
            Err(join_err) => Err(anyhow::anyhow!("handler panicked: {join_err}")),
        };

        match result {
            Ok(()) => {
                sqlx::query("UPDATE jobs SET done_at = now(), locked_at = NULL, last_error = NULL WHERE id = $1")
                    .bind(job.id)
                    .execute(&self.pool)
                    .await?;
            }
            Err(err) => {
                let gives_up = job.attempts >= job.max_attempts;
                tracing::warn!(job = %job.id, kind = %job.kind, attempt = job.attempts, gives_up, error = ?err, "job failed");
                sqlx::query(
                    "UPDATE jobs SET locked_at = NULL, last_error = $2,
                         run_at = now() + make_interval(secs => $3)
                     WHERE id = $1",
                )
                .bind(job.id)
                .bind(format!("{err:#}"))
                .bind(backoff_secs(job.attempts))
                .execute(&self.pool)
                .await?;
            }
        }
        Ok(true)
    }
}

/// 2, 4, 8 … seconds, capped at one hour.
fn backoff_secs(attempts: i32) -> f64 {
    2f64.powi(attempts.clamp(1, 12)).min(3600.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_grows_and_caps() {
        assert_eq!(backoff_secs(1), 2.0);
        assert_eq!(backoff_secs(3), 8.0);
        assert_eq!(backoff_secs(50), 3600.0);
    }
}
