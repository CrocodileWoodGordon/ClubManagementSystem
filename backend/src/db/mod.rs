use sqlx::PgPool;
use sqlx::migrate::{MigrateError, Migrator};
use sqlx::postgres::PgPoolOptions;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

pub mod models;

pub type DbPool = PgPool;

/// Initialize a connection pool consumed by repositories + services.
pub async fn connect(database_url: &str) -> Result<DbPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
}

/// Run pending migrations, tolerating databases that were pre-initialized
/// via docker/db/init.sh (tables exist but `_sqlx_migrations` is empty).
pub async fn run_migrations(pool: &DbPool) -> Result<(), MigrateError> {
    if needs_bootstrap_registration(pool)
        .await
        .map_err(MigrateError::from)?
    {
        seed_migration_history(pool)
            .await
            .map_err(MigrateError::from)?;
    }

    MIGRATOR.run(pool).await
}

async fn needs_bootstrap_registration(pool: &DbPool) -> Result<bool, sqlx::Error> {
    if has_table(pool, "_sqlx_migrations").await? {
        return Ok(false);
    }

    // 离线镜像预初始化时会直接建业务表，但不会写入 `_sqlx_migrations`，
    // 需要补录迁移记录以避免二次建表冲突。
    has_table(pool, "terms").await
}

async fn has_table(pool: &DbPool, name: &str) -> Result<bool, sqlx::Error> {
    let exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM information_schema.tables
            WHERE table_schema = 'public'
              AND table_name = $1
        )
        "#,
    )
    .bind(name)
    .fetch_one(pool)
    .await?;

    Ok(exists)
}

async fn seed_migration_history(pool: &DbPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS _sqlx_migrations (
            version BIGINT PRIMARY KEY,
            description TEXT NOT NULL,
            installed_on TIMESTAMPTZ NOT NULL DEFAULT now(),
            success BOOLEAN NOT NULL,
            checksum BYTEA NOT NULL,
            execution_time BIGINT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    for migration in MIGRATOR.iter() {
        sqlx::query(
            r#"
            INSERT INTO _sqlx_migrations (
                version,
                description,
                installed_on,
                success,
                checksum,
                execution_time
            ) VALUES ($1, $2, now(), TRUE, $3, 0)
            ON CONFLICT (version) DO NOTHING
            "#,
        )
        .bind(migration.version)
        .bind(&*migration.description)
        .bind(&*migration.checksum)
        .execute(pool)
        .await?;
    }

    Ok(())
}
