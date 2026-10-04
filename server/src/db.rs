use std::time::Duration;

use sqlx::{PgPool, postgres::PgPoolOptions};

pub async fn connect(database_url: &str) -> sqlx::Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await
}

pub async fn migrate(db: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    let mut migrator = sqlx::migrate!("./migrations");
    // Lets a rolled-back (older) binary start against a database that newer releases have
    // already migrated. Migrations must therefore stay backward compatible (additive).
    migrator.set_ignore_missing(true);
    migrator.run(db).await?;
    tracing::info!("database migrations applied");
    Ok(())
}
