use anyhow::Result;
use rusqlite::Connection;

/// Контекст диалога в SQLite
pub struct ContextManager {
    db: Connection,
}

impl ContextManager {
    pub fn new() -> Result<Self> {
        let db = Connection::open_in_memory()?;
        db.execute(
            "CREATE TABLE IF NOT EXISTS history (
                id INTEGER PRIMARY KEY,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        Ok(Self { db })
    }
}
