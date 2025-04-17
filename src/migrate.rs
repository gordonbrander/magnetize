pub use rusqlite::Connection;

/// Migrate the database schema to the latest version.
pub fn migrate(connection: &mut Connection, migrations: &[&str]) -> Result<(), MigrationError> {
    let mut user_version: i32 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|err| MigrationError::new("Unable to read user_version", None, Some(err)))?;

    for (i, migration) in migrations.iter().enumerate() {
        let migration_version = i as i32 + 1;

        if migration_version > user_version {
            // Transaction automatically rolls back when `tx` is dropped
            let tx = connection.transaction().map_err(|err| {
                MigrationError::new("Unable to start transaction", Some(user_version), Some(err))
            })?;

            // Execute the migration
            tx.execute_batch(migration).map_err(|err| {
                MigrationError::new(
                    format!("Unable to execute migration\n\n{}", migration),
                    Some(user_version),
                    Some(err),
                )
            })?;

            // Update the user_version after successful migration
            tx.execute(&format!("PRAGMA user_version = {}", migration_version), [])
                .map_err(|err| {
                    MigrationError::new(
                        "Unable to update user_version",
                        Some(user_version),
                        Some(err),
                    )
                })?;

            // Commit the transaction
            tx.commit().map_err(|err| {
                MigrationError::new(
                    "Unable to commit transaction",
                    Some(user_version),
                    Some(err),
                )
            })?;

            user_version = migration_version;
        }
    }

    Ok(())
}

#[derive(Debug)]
pub struct MigrationError {
    message: String,
    version: Option<i32>,
    source: Option<rusqlite::Error>,
}

impl MigrationError {
    pub fn new(
        message: impl Into<String>,
        version: Option<i32>,
        source: Option<rusqlite::Error>,
    ) -> Self {
        MigrationError {
            version,
            message: message.into(),
            source,
        }
    }
}

impl std::fmt::Display for MigrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Error migrating database to latest.\n\tMessage: {}\n\tRolled back to version: {}",
            self.message,
            self.version
                .map_or("unknown".to_string(), |v| v.to_string())
        )
    }
}

impl std::error::Error for MigrationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|e| e as &dyn std::error::Error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_connection() -> Connection {
        Connection::open_in_memory().unwrap()
    }

    #[test]
    fn test_migrate_empty_migrations() {
        let mut conn = create_test_connection();
        let migrations: Vec<&str> = vec![];

        let result = migrate(&mut conn, &migrations);
        assert!(result.is_ok());

        let user_version: i32 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(user_version, 0);
    }

    #[test]
    fn test_migrate_single_migration() {
        let mut conn = create_test_connection();
        let migrations = vec!["CREATE TABLE test (id INTEGER PRIMARY KEY)"];

        let result = migrate(&mut conn, &migrations);
        assert!(result.is_ok());

        // Check user_version was updated
        let user_version: i32 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(user_version, 1);

        // Check table was created
        let count: i32 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='test'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_migrate_multiple_migrations() {
        let mut conn = create_test_connection();
        let migrations = vec![
            "CREATE TABLE users (id INTEGER PRIMARY KEY)",
            "CREATE TABLE posts (id INTEGER PRIMARY KEY, user_id INTEGER, FOREIGN KEY(user_id) REFERENCES users(id))",
            "ALTER TABLE users ADD COLUMN name TEXT",
        ];

        let result = migrate(&mut conn, &migrations);
        assert!(result.is_ok());

        // Check user_version was updated
        let user_version: i32 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(user_version, 3);

        // Check both tables exist
        let count: i32 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name IN ('users', 'posts')",
                [],
                |row| row.get(0)
            )
            .unwrap();
        assert_eq!(count, 2);

        // Check column was added
        let has_name_column: bool = conn
            .query_row(
                "SELECT 1 FROM pragma_table_info('users') WHERE name='name'",
                [],
                |_| Ok(true),
            )
            .unwrap_or(false);
        assert!(has_name_column);
    }

    #[test]
    fn test_migrate_idempotent() {
        let mut conn = create_test_connection();
        let migrations = vec!["CREATE TABLE test (id INTEGER PRIMARY KEY)"];

        // Run migrations first time
        let result = migrate(&mut conn, &migrations);
        assert!(result.is_ok());

        // Run migrations second time
        let result = migrate(&mut conn, &migrations);
        assert!(result.is_ok());

        // Version should still be 1
        let user_version: i32 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(user_version, 1);
    }

    #[test]
    fn test_migrate_failed_migration() {
        let mut conn = create_test_connection();
        let migrations = vec![
            "CREATE TABLE test (id INTEGER PRIMARY KEY)",
            "THIS IS NOT VALID SQL", // This will fail
        ];

        let result = migrate(&mut conn, &migrations);
        assert!(result.is_err());

        if let Err(err) = result {
            assert_eq!(err.version, Some(1));
        }

        // User version should remain at 1
        let user_version: i32 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(user_version, 1);
    }
}
