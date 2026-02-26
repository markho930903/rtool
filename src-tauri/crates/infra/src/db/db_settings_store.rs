use super::DbConn;
use crate::db_error::DbResult;
use libsql::{params, params_from_iter};
use std::collections::HashMap;

pub async fn get_app_setting(conn: &DbConn, key: &str) -> DbResult<Option<String>> {
    let mut rows = conn
        .query(
            "SELECT value FROM app_settings WHERE key = ?1 LIMIT 1",
            params![key],
        )
        .await?;

    if let Some(row) = rows.next().await? {
        return Ok(Some(row.get::<String>(0)?));
    }

    Ok(None)
}

pub async fn set_app_setting(conn: &DbConn, key: &str, value: &str) -> DbResult<()> {
    conn.execute(
        "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )
    .await?;
    Ok(())
}

pub async fn get_app_settings_batch(
    conn: &DbConn,
    keys: &[&str],
) -> DbResult<HashMap<String, String>> {
    if keys.is_empty() {
        return Ok(HashMap::new());
    }

    let placeholders = (1..=keys.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!("SELECT key, value FROM app_settings WHERE key IN ({placeholders})");
    let mut rows = conn
        .query(sql.as_str(), params_from_iter(keys.iter().copied()))
        .await?;

    let mut values = HashMap::with_capacity(keys.len());
    while let Some(row) = rows.next().await? {
        let key = row.get::<String>(0)?;
        let value = row.get::<String>(1)?;
        values.insert(key, value);
    }
    Ok(values)
}

pub async fn set_app_settings_batch(conn: &DbConn, entries: &[(&str, &str)]) -> DbResult<()> {
    if entries.is_empty() {
        return Ok(());
    }

    let tx = conn.transaction().await?;
    for (key, value) in entries {
        tx.execute(
            "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![*key, *value],
        )
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn delete_app_settings(conn: &DbConn, keys: &[&str]) -> DbResult<()> {
    if keys.is_empty() {
        return Ok(());
    }

    let placeholders = (1..=keys.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!("DELETE FROM app_settings WHERE key IN ({placeholders})");
    conn.execute(sql.as_str(), params_from_iter(keys.iter().copied()))
        .await?;
    Ok(())
}
