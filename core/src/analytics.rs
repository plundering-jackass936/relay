//! Local SQLite analytics — tracks every handoff for insights and stats.

use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;

/// Open (or create) the analytics database.
pub fn open_db() -> Result<Connection> {
    let db_path = crate::data_dir().join("analytics.db");
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(&db_path)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS handoffs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            agent TEXT NOT NULL,
            success INTEGER NOT NULL DEFAULT 0,
            duration_ms INTEGER,
            context_bytes INTEGER,
            tokens_estimated INTEGER,
            template TEXT,
            project_dir TEXT,
            task TEXT,
            error_message TEXT,
            chain_depth INTEGER DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS agent_stats (
            agent TEXT PRIMARY KEY,
            total_handoffs INTEGER DEFAULT 0,
            successful INTEGER DEFAULT 0,
            failed INTEGER DEFAULT 0,
            total_duration_ms INTEGER DEFAULT 0,
            last_used TEXT
        );",
    )?;
    Ok(conn)
}

/// Record a handoff event.
pub fn record_handoff(
    conn: &Connection,
    agent: &str,
    success: bool,
    duration_ms: u128,
    context_bytes: usize,
    tokens_estimated: usize,
    template: &str,
    project_dir: &str,
    task: &str,
    error_message: Option<&str>,
    chain_depth: u32,
) -> Result<()> {
    conn.execute(
        "INSERT INTO handoffs (agent, success, duration_ms, context_bytes, tokens_estimated, template, project_dir, task, error_message, chain_depth)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        rusqlite::params![
            agent,
            success as i32,
            duration_ms as i64,
            context_bytes as i64,
            tokens_estimated as i64,
            template,
            project_dir,
            task,
            error_message.unwrap_or(""),
            chain_depth as i32,
        ],
    )?;

    // Update agent stats
    conn.execute(
        "INSERT INTO agent_stats (agent, total_handoffs, successful, failed, total_duration_ms, last_used)
         VALUES (?1, 1, ?2, ?3, ?4, datetime('now','localtime'))
         ON CONFLICT(agent) DO UPDATE SET
             total_handoffs = total_handoffs + 1,
             successful = successful + ?2,
             failed = failed + ?3,
             total_duration_ms = total_duration_ms + ?4,
             last_used = datetime('now','localtime')",
        rusqlite::params![
            agent,
            success as i32,
            (!success) as i32,
            duration_ms as i64,
        ],
    )?;

    Ok(())
}

/// Get summary stats for the dashboard.
#[derive(Debug, serde::Serialize)]
pub struct Stats {
    pub total_handoffs: u64,
    pub successful: u64,
    pub failed: u64,
    pub success_rate: f64,
    pub avg_duration_ms: u64,
    pub total_time_saved_est_min: f64,
    pub agents: Vec<AgentStat>,
    pub recent: Vec<RecentHandoff>,
}

#[derive(Debug, serde::Serialize)]
pub struct AgentStat {
    pub agent: String,
    pub total: u64,
    pub successful: u64,
    pub failed: u64,
    pub avg_duration_ms: u64,
    pub last_used: String,
}

#[derive(Debug, serde::Serialize)]
pub struct RecentHandoff {
    pub timestamp: String,
    pub agent: String,
    pub success: bool,
    pub duration_ms: u64,
    pub task: String,
}

pub fn get_stats(conn: &Connection) -> Result<Stats> {
    // Overall stats
    let (total, successful, failed, total_duration): (u64, u64, u64, u64) = conn.query_row(
        "SELECT COUNT(*), SUM(success), COUNT(*) - SUM(success), COALESCE(SUM(duration_ms), 0) FROM handoffs",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    ).unwrap_or((0, 0, 0, 0));

    let avg_duration_ms = if total > 0 { total_duration / total } else { 0 };
    let success_rate = if total > 0 { successful as f64 / total as f64 * 100.0 } else { 0.0 };
    // Estimate: each successful handoff saves ~15 min of re-explaining context
    let total_time_saved_est_min = successful as f64 * 15.0;

    // Per-agent stats
    let mut stmt = conn.prepare(
        "SELECT agent, total_handoffs, successful, failed,
                CASE WHEN total_handoffs > 0 THEN total_duration_ms / total_handoffs ELSE 0 END,
                COALESCE(last_used, '')
         FROM agent_stats ORDER BY total_handoffs DESC"
    )?;
    let agents: Vec<AgentStat> = stmt.query_map([], |row| {
        Ok(AgentStat {
            agent: row.get(0)?,
            total: row.get(1)?,
            successful: row.get(2)?,
            failed: row.get(3)?,
            avg_duration_ms: row.get(4)?,
            last_used: row.get(5)?,
        })
    })?.filter_map(|r| r.ok()).collect();

    // Recent handoffs
    let mut stmt = conn.prepare(
        "SELECT timestamp, agent, success, duration_ms, COALESCE(task, '')
         FROM handoffs ORDER BY id DESC LIMIT 10"
    )?;
    let recent: Vec<RecentHandoff> = stmt.query_map([], |row| {
        Ok(RecentHandoff {
            timestamp: row.get(0)?,
            agent: row.get(1)?,
            success: row.get::<_, i32>(2)? != 0,
            duration_ms: row.get(3)?,
            task: row.get(4)?,
        })
    })?.filter_map(|r| r.ok()).collect();

    Ok(Stats {
        total_handoffs: total,
        successful,
        failed,
        success_rate,
        avg_duration_ms,
        total_time_saved_est_min,
        agents,
        recent,
    })
}

/// Get stats from a project-specific path (for testing).
pub fn open_db_at(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS handoffs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            agent TEXT NOT NULL,
            success INTEGER NOT NULL DEFAULT 0,
            duration_ms INTEGER,
            context_bytes INTEGER,
            tokens_estimated INTEGER,
            template TEXT,
            project_dir TEXT,
            task TEXT,
            error_message TEXT,
            chain_depth INTEGER DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS agent_stats (
            agent TEXT PRIMARY KEY,
            total_handoffs INTEGER DEFAULT 0,
            successful INTEGER DEFAULT 0,
            failed INTEGER DEFAULT 0,
            total_duration_ms INTEGER DEFAULT 0,
            last_used TEXT
        );",
    )?;
    Ok(conn)
}
