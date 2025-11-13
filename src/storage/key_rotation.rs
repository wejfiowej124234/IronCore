use anyhow::Result;
use sqlx::types::chrono::Utc;
use sqlx::{Row, SqlitePool};

#[derive(Debug, Clone)]
pub struct KeyLabelRecord {
    pub label: String,
    pub current_version: i64,
    pub current_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct KeyVersionRecord {
    pub label: String,
    pub version: i64,
    pub key_id: String,
    pub retired: bool,
    pub usage_count: i64,
    pub created_at: i64,
}

pub async fn init_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS key_labels (
            label TEXT PRIMARY KEY,
            current_version INTEGER NOT NULL,
            current_id TEXT
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS key_versions (
            label TEXT NOT NULL,
            version INTEGER NOT NULL,
            key_id TEXT NOT NULL,
            retired BOOLEAN NOT NULL DEFAULT 0,
            usage_count INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            PRIMARY KEY (label, version)
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn upsert_label(
    pool: &SqlitePool,
    label: &str,
    current_version: i64,
    current_id: Option<&str>,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO key_labels (label, current_version, current_id)
        VALUES (?1, ?2, ?3)
        ON CONFLICT(label) DO UPDATE SET current_version=excluded.current_version, current_id=excluded.current_id
        "#,
    )
    .bind(label)
    .bind(current_version)
    .bind(current_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn insert_version(
    pool: &SqlitePool,
    label: &str,
    version: i64,
    key_id: &str,
) -> Result<()> {
    let now = Utc::now().timestamp();
    sqlx::query(
        r#"
        INSERT INTO key_versions (label, version, key_id, retired, usage_count, created_at)
        VALUES (?1, ?2, ?3, 0, 0, ?4)
        "#,
    )
    .bind(label)
    .bind(version)
    .bind(key_id)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_retired(pool: &SqlitePool, label: &str, version: i64) -> Result<()> {
    sqlx::query("UPDATE key_versions SET retired=1 WHERE label=?1 AND version=?2")
        .bind(label)
        .bind(version)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn inc_usage(pool: &SqlitePool, label: &str, version: i64) -> Result<()> {
    sqlx::query(
        "UPDATE key_versions SET usage_count = usage_count + 1 WHERE label=?1 AND version=?2",
    )
    .bind(label)
    .bind(version)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_label(pool: &SqlitePool, label: &str) -> Result<Option<KeyLabelRecord>> {
    let row =
        sqlx::query("SELECT label, current_version, current_id FROM key_labels WHERE label=?1")
            .bind(label)
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|r| KeyLabelRecord {
        label: r.get("label"),
        current_version: r.get::<i64, _>("current_version"),
        current_id: r.get::<Option<String>, _>("current_id"),
    }))
}

pub async fn get_version(
    pool: &SqlitePool,
    label: &str,
    version: i64,
) -> Result<Option<KeyVersionRecord>> {
    let row = sqlx::query("SELECT label, version, key_id, retired, usage_count, created_at FROM key_versions WHERE label=?1 AND version=?2")
        .bind(label)
        .bind(version)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| KeyVersionRecord {
        label: r.get("label"),
        version: r.get("version"),
        key_id: r.get("key_id"),
        retired: r.get::<i64, _>("retired") != 0,
        usage_count: r.get("usage_count"),
        created_at: r.get("created_at"),
    }))
}
