use crate::db::{setting_get, Db};
use crate::embeddings::{f32_to_blob, EmbeddingClient};
use crate::settings::SharedEmbeddings;
use chrono::{Duration, Utc};
use rusqlite::params;
use std::time::Duration as StdDuration;
use tracing::{info, warn};

pub fn spawn_workers(db: Db, embeddings: SharedEmbeddings) {
    let db_embed = db.clone();
    let emb = embeddings.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(StdDuration::from_secs(5));
        loop {
            interval.tick().await;
            let client = emb.read().clone();
            if let Some(client) = client.as_ref() {
                if let Err(err) = run_embed_batch(&db_embed, client).await {
                    warn!(error = %err, "embed worker error");
                }
            }
        }
    });

    let db_ret = db;
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(StdDuration::from_secs(60));
        loop {
            interval.tick().await;
            if let Err(err) = tokio::task::spawn_blocking({
                let db = db_ret.clone();
                move || run_retention(&db)
            })
            .await
            .unwrap_or_else(|e| Err(anyhow::anyhow!(e)))
            {
                warn!(error = %err, "retention worker error");
            }
        }
    });
}

async fn run_embed_batch(db: &Db, client: &EmbeddingClient) -> anyhow::Result<()> {
    let batch = {
        let conn = db.lock();
        let mut stmt = conn.prepare(
            "SELECT event_id, message FROM embed_queue ORDER BY enqueued_at ASC LIMIT 32",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        out
    };
    if batch.is_empty() {
        return Ok(());
    }

    let texts: Vec<String> = batch.iter().map(|(_, m)| m.clone()).collect();
    let vectors = client.embed(&texts).await?;
    let now = Utc::now().to_rfc3339();

    let conn = db.lock();
    for (i, (event_id, _)) in batch.iter().enumerate() {
        let Some(vec) = vectors.get(i) else { continue };
        if vec.is_empty() {
            conn.execute(
                "UPDATE embed_queue SET attempts = attempts + 1 WHERE event_id = ?1",
                params![event_id],
            )?;
            continue;
        }
        let blob = f32_to_blob(vec);
        conn.execute(
            "INSERT OR REPLACE INTO log_embeddings(event_id, dim, embedding, created_at)
             VALUES(?1, ?2, ?3, ?4)",
            params![event_id, vec.len() as i64, blob, now],
        )?;
        conn.execute("DELETE FROM embed_queue WHERE event_id = ?1", params![event_id])?;
    }
    Ok(())
}

fn run_retention(db: &Db) -> anyhow::Result<()> {
    let conn = db.lock();
    let days = setting_get(&conn, "retention_days")?
        .and_then(|s| s.parse().ok())
        .unwrap_or(14u32);
    let max_events = setting_get(&conn, "max_events")?.and_then(|s| s.parse::<u64>().ok());

    let cutoff = (Utc::now() - Duration::days(days as i64)).to_rfc3339();
    let deleted_age = conn.execute("DELETE FROM log_events WHERE ts < ?1", params![cutoff])?;
    if deleted_age > 0 {
        info!(deleted_age, cutoff = %cutoff, "retention deleted by age");
    }

    if let Some(max) = max_events {
        let total: i64 = conn.query_row("SELECT COUNT(*) FROM log_events", [], |r| r.get(0))?;
        if total > max as i64 {
            let overflow = total - max as i64;
            let batch = overflow.min(5000);
            conn.execute(
                "DELETE FROM log_events WHERE id IN (
                    SELECT id FROM log_events ORDER BY ts ASC, id ASC LIMIT ?1
                )",
                params![batch],
            )?;
            info!(batch, "retention deleted by max_events");
        }
    }

    let _ = conn.execute_batch("PRAGMA optimize;");
    Ok(())
}
