use std::time::Duration;

use sqlx::{PgPool, Postgres, Transaction, postgres::PgPoolOptions};
use uuid::Uuid;

use crate::Config;

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

pub async fn connect(config: &Config) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(config.database_max_connections)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&config.database_url)
        .await?;
    Ok(pool)
}

pub async fn migrate(pool: &PgPool) -> anyhow::Result<()> {
    MIGRATOR.run(pool).await?;
    Ok(())
}

/// Begin a transaction scoped to one organization.
///
/// The transaction runs as the `kenning_app` role with `app.org_id` set, so
/// Postgres row-level security hides every other organization's rows even if a
/// query forgets its `org_id` filter. Both settings end with the transaction.
pub async fn begin_org(
    pool: &PgPool,
    org_id: Uuid,
) -> sqlx::Result<Transaction<'static, Postgres>> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL ROLE kenning_app")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SELECT set_config('app.org_id', $1, true)")
        .bind(org_id.to_string())
        .execute(&mut *tx)
        .await?;
    Ok(tx)
}
