use crate::migrate::{self, migrate};
use crate::url::Url;
use rusqlite::{self, OptionalExtension, named_params};

/// Interface for interacting with SQLite database.
/// Database is used ot keep track of basic server state, such as URLs that should be notified, allowed, denied.
pub struct Database {
    connection: rusqlite::Connection,
}

impl Database {
    /// Open a database connection at the given path.
    /// Path is a path to a SQLite database file, or a SQLite connection string.
    pub fn open(path: &str) -> Result<Self, DbError> {
        let connection = rusqlite::Connection::open(path)?;
        Ok(Database { connection })
    }

    /// Create a new database instance from an existing connection.
    pub fn new(connection: rusqlite::Connection) -> Self {
        Database { connection }
    }

    /// Get the version of the database.
    pub fn version(&self) -> Result<i32, DbError> {
        let mut stmt = self.connection.prepare("PRAGMA user_version")?;
        let version: i32 = stmt.query_row([], |row| row.get(0))?;
        Ok(version)
    }

    /// Migrate the database schema
    pub fn migrate(&mut self) -> Result<(), DbError> {
        migrate(
            &mut self.connection,
            &["
                CREATE TABLE IF NOT EXISTS notify (url TEXT PRIMARY KEY);
                CREATE TABLE IF NOT EXISTS origin (
                    url TEXT PRIMARY KEY,
                    deny BOOLEAN NOT NULL DEFAULT 0
                );
                "],
        )?;
        Ok(())
    }

    /// Insert or update a URL in the notify table.
    pub fn upsert_notify(&mut self, url: &Url) -> Result<(), DbError> {
        self.connection.execute(
            "INSERT OR REPLACE INTO notify (url) VALUES (:url)",
            named_params! {
                ":url": url.to_string(),
            },
        )?;
        Ok(())
    }

    /// Read a URL from the notify table.
    pub fn read_notify(&mut self, url: &Url) -> Result<Option<Url>, DbError> {
        let row: Option<String> = self
            .connection
            .query_row(
                "SELECT url FROM notify WHERE url = :url",
                named_params! {
                    ":url": url.to_string(),
                },
                |row| row.get(0),
            )
            .optional()?;

        match row {
            Some(string) => Ok(Some(Url::parse(&string)?)),
            None => Ok(None),
        }
    }

    pub fn delete_notify(&mut self, url: &Url) -> Result<(), DbError> {
        self.connection.execute(
            "DELETE FROM notify WHERE url = :url",
            named_params! {
                ":url": url.to_string(),
            },
        )?;
        Ok(())
    }

    /// Choose up to `limit` notification URLs.
    pub fn choose_random_notify(&mut self, limit: usize) -> Result<Vec<Url>, DbError> {
        let mut stmt = self
            .connection
            .prepare("SELECT url FROM notify ORDER BY RANDOM() LIMIT :limit")?;

        let mut rows = stmt.query(named_params! {
            ":limit": limit,
        })?;

        let mut urls = Vec::new();
        while let Some(row) = rows.next()? {
            let url_string: String = row.get(0)?;
            urls.push(Url::parse(&url_string)?);
        }
        Ok(urls)
    }

    /// Insert or update an origin, marking it as allowed.
    pub fn upsert_allow(&mut self, url: &Url) -> Result<(), DbError> {
        self.connection.execute(
            "INSERT OR REPLACE INTO origin (url, deny) VALUES (:url, 0)",
            named_params! {
                ":url": url.origin().ascii_serialization(),
            },
        )?;
        Ok(())
    }

    /// Insert or update an origin, marking it as denied.
    pub fn upsert_deny(&mut self, url: &Url) -> Result<(), DbError> {
        self.connection.execute(
            "INSERT OR REPLACE INTO origin (url, deny) VALUES (:url, 1)",
            named_params! {
                ":url": url.origin().ascii_serialization(),
            },
        )?;
        Ok(())
    }

    /// Read the allow/deny status of an origin.
    pub fn read_origin_status(&self, url: &Url) -> Result<OriginStatus, DbError> {
        let deny: &Option<bool> = &self
            .connection
            .query_row(
                "SELECT deny FROM origin WHERE url = :url",
                named_params! {
                    ":url": url.origin().ascii_serialization(),
                },
                |row| row.get(0),
            )
            .optional()?;

        Ok(match deny {
            Some(true) => OriginStatus::Deny,
            Some(false) => OriginStatus::Allow,
            None => OriginStatus::Unknown,
        })
    }

    /// Delete an origin from the database.
    pub fn delete_origin(&mut self, url: &Url) -> Result<(), DbError> {
        self.connection.execute(
            "DELETE FROM origin WHERE url = :url",
            named_params! {
                ":url": url.origin().ascii_serialization(),
            },
        )?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum OriginStatus {
    /// The origin is allow-listed.
    Allow,
    /// The origin is deny-listed.
    Deny,
    /// The origin is not one we know about.
    Unknown,
}

#[derive(Debug)]
pub enum DbError {
    Migration(migrate::MigrationError),
    Database(rusqlite::Error),
    Url(url::ParseError),
}

impl std::error::Error for DbError {}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DbError::Migration(e) => write!(f, "Migration error: {}", e),
            DbError::Database(e) => write!(f, "Database error: {}", e),
            DbError::Url(e) => write!(f, "URL error: {}", e),
        }
    }
}

impl From<rusqlite::Error> for DbError {
    fn from(error: rusqlite::Error) -> Self {
        DbError::Database(error)
    }
}

impl From<migrate::MigrationError> for DbError {
    fn from(error: migrate::MigrationError) -> Self {
        DbError::Migration(error)
    }
}

impl From<url::ParseError> for DbError {
    fn from(error: url::ParseError) -> Self {
        DbError::Url(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_db() -> Database {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        let mut db = Database::new(connection);
        db.migrate().unwrap();
        db
    }

    #[test]
    fn test_upsert_and_delete_notify() {
        let mut db = create_test_db();
        let url = Url::parse("https://example.com/notify").unwrap();

        // Test upsert
        db.upsert_notify(&url).unwrap();

        let url2 = db.read_notify(&url).unwrap();
        assert_eq!(url2, Some(url.clone()));

        // Test delete
        db.delete_notify(&url).unwrap();
        let url2 = db.read_notify(&url).unwrap();
        assert_eq!(url2, None);
    }

    #[test]
    fn test_choose_random_notify() {
        let mut db = create_test_db();

        // Add multiple URLs
        let url1 = Url::parse("https://example.com/1").unwrap();
        let url2 = Url::parse("https://example.com/2").unwrap();
        let url3 = Url::parse("https://example.com/3").unwrap();

        db.upsert_notify(&url1).unwrap();
        db.upsert_notify(&url2).unwrap();
        db.upsert_notify(&url3).unwrap();

        // Test limit works
        let urls = db.choose_random_notify(2).unwrap();
        assert_eq!(urls.len(), 2);

        // Test all URLs are returned with large limit
        let urls = db.choose_random_notify(10).unwrap();
        assert_eq!(urls.len(), 3);
    }

    #[test]
    fn test_origin_allow_deny() {
        let mut db = create_test_db();
        let url = Url::parse("https://example.com/page").unwrap();

        // Test default status
        let status = db.read_origin_status(&url).unwrap();
        assert_eq!(status, OriginStatus::Unknown);

        // Test allow
        db.upsert_allow(&url).unwrap();
        let status = db.read_origin_status(&url).unwrap();
        assert_eq!(status, OriginStatus::Allow);

        // Test deny
        db.upsert_deny(&url).unwrap();
        let status = db.read_origin_status(&url).unwrap();
        assert_eq!(status, OriginStatus::Deny);

        // Test delete
        db.delete_origin(&url).unwrap();
        let status = db.read_origin_status(&url).unwrap();
        assert_eq!(status, OriginStatus::Unknown);
    }

    #[test]
    fn test_multiple_origins() {
        let mut db = create_test_db();

        let url1 = Url::parse("https://example1.com/page").unwrap();
        let url2 = Url::parse("https://example2.com/page").unwrap();

        db.upsert_allow(&url1).unwrap();
        db.upsert_deny(&url2).unwrap();

        let status1 = db.read_origin_status(&url1).unwrap();
        let status2 = db.read_origin_status(&url2).unwrap();

        assert!(matches!(status1, OriginStatus::Allow));
        assert!(matches!(status2, OriginStatus::Deny));
    }
}
