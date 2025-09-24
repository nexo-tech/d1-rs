use crate::{D1Client, Result};
use crate::backends::QueryResult;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[async_trait(?Send)]
pub trait Migration {
    fn name(&self) -> &'static str;
    fn version(&self) -> i64;
    async fn up(&self, db: &D1Client) -> Result<()>;
    async fn down(&self, db: &D1Client) -> Result<()>;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MigrationRecord {
    pub id: i64,
    pub version: i64,
    pub name: String,
    pub applied_at: DateTime<Utc>,
}

pub struct MigrationRunner {
    migrations: Vec<Box<dyn Migration>>,
}

impl MigrationRunner {
    pub fn new() -> Self {
        Self {
            migrations: Vec::new(),
        }
    }

    pub fn add_migration(&mut self, migration: Box<dyn Migration>) {
        self.migrations.push(migration);
    }

    pub async fn run_pending_migrations(&self, db: &D1Client) -> Result<Vec<String>> {
        // Ensure migration table exists first
        self.ensure_migration_table(db).await?;

        // Try to acquire distributed lock with timeout
        let lock_acquired = self.acquire_migration_lock(db).await?;
        if !lock_acquired {
            // Another worker is running migrations, return empty (no migrations applied by us)
            return Ok(Vec::new());
        }

        // We have the lock, proceed with migrations
        let result = self.run_migrations_with_lock(db).await;

        // Always release the lock, even if migrations failed
        let _ = self.release_migration_lock(db).await;

        result
    }

    /// Verifies if all expected migrations are applied by comparing the highest
    /// expected migration version with the highest applied migration version.
    /// Returns true if all migrations are up to date, false otherwise.
    pub async fn verify_migrations_up_to_date(&self, db: &D1Client) -> Result<bool> {
        // Ensure migration table exists first
        self.ensure_migration_table(db).await?;

        // Get the highest expected migration version
        let expected_max_version = self
            .migrations
            .iter()
            .map(|m| m.version())
            .max()
            .unwrap_or(0);

        // If no migrations are registered, consider it up to date
        if expected_max_version == 0 {
            return Ok(true);
        }

        // Get applied migration versions
        let applied_versions = self.get_applied_migrations(db).await?;
        let applied_max_version = applied_versions.iter().max().copied().unwrap_or(0);

        // Check if all expected migrations are applied
        for migration in &self.migrations {
            if !applied_versions.contains(&migration.version()) {
                return Ok(false);
            }
        }

        // All migrations are applied
        Ok(applied_max_version >= expected_max_version)
    }

    async fn run_migrations_with_lock(&self, db: &D1Client) -> Result<Vec<String>> {
        // Sort migrations by version
        let mut sorted_migrations = self.migrations.iter().collect::<Vec<_>>();
        sorted_migrations.sort_by_key(|m| m.version());

        let mut applied_migrations = Vec::new();

        // Run each migration if not already applied
        for migration in sorted_migrations {
            let is_applied = self.is_migration_applied(db, migration.version()).await?;

            if !is_applied {
                // Run the migration
                migration.up(db).await?;
                // Record that it was applied
                self.record_migration(db, migration.as_ref()).await?;
                applied_migrations.push(format!("{} (v{})", migration.name(), migration.version()));
            }
        }

        Ok(applied_migrations)
    }

    pub async fn rollback_to_version(&self, db: &D1Client, target_version: i64) -> Result<()> {
        let applied_migrations = self.get_applied_migration_records(db).await?;

        let mut to_rollback: Vec<_> = applied_migrations
            .iter()
            .filter(|m| m.version > target_version)
            .collect();

        to_rollback.sort_by(|a, b| b.version.cmp(&a.version));

        for record in to_rollback {
            if let Some(migration) = self
                .migrations
                .iter()
                .find(|m| m.version() == record.version)
            {
                migration.down(db).await?;
                self.remove_migration_record(db, record.version).await?;
            }
        }

        Ok(())
    }

    async fn ensure_migration_table(&self, db: &D1Client) -> Result<()> {
        // Create migrations table (using single underscore to match tests)
        let migrations_sql = r#"
            CREATE TABLE IF NOT EXISTS _migrations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                version INTEGER NOT NULL UNIQUE,
                name TEXT NOT NULL,
                applied_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        "#;

        db.execute(migrations_sql, &[]).await?;

        // Create migration locks table for distributed locking
        let locks_sql = r#"
            CREATE TABLE IF NOT EXISTS _migration_lock (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                locked INTEGER NOT NULL DEFAULT 0,
                locked_at DATETIME
            )
        "#;

        db.execute(locks_sql, &[]).await?;
        
        // Insert the lock record if it doesn't exist
        let insert_lock_sql = "INSERT OR IGNORE INTO _migration_lock (id, locked) VALUES (1, 0)";
        db.execute(insert_lock_sql, &[]).await?;
        
        Ok(())
    }

    async fn acquire_migration_lock(&self, db: &D1Client) -> Result<bool> {
        // Try to acquire the lock using atomic UPDATE
        let acquire_sql = r#"
            UPDATE _migration_lock 
            SET locked = 1, locked_at = CURRENT_TIMESTAMP 
            WHERE id = 1 AND locked = 0
        "#;

        let _result = db.execute(acquire_sql, &[]).await?;
        
        // Check if we successfully updated a row (acquired the lock)
        // For SQLite, this is a bit tricky - we'll check by querying the lock state
        let check_sql = "SELECT locked FROM _migration_lock WHERE id = 1";
        let lock_result = db.execute(check_sql, &[]).await?;
        
        if let Some(row) = lock_result.rows().first() {
            if let serde_json::Value::Object(obj) = row {
                if let Some(serde_json::Value::Number(locked)) = obj.get("locked") {
                    return Ok(locked.as_i64() == Some(1));
                }
            }
        }
        
        Ok(false)
    }

    async fn release_migration_lock(&self, db: &D1Client) -> Result<()> {
        let sql = "UPDATE _migration_lock SET locked = 0, locked_at = NULL WHERE id = 1";
        let _ = db.execute(sql, &[]).await;
        Ok(())
    }

    async fn is_migration_applied(&self, db: &D1Client, version: i64) -> Result<bool> {
        let sql = "SELECT COUNT(*) as count FROM _migrations WHERE version = ?";
        let params = vec![serde_json::json!(version as i32)];

        match db.execute_returning_count(sql, &params).await {
            Ok(count) => Ok(count > 0),
            Err(_) => {
                // If query fails (likely because table doesn't exist), assume migration not applied
                Ok(false)
            }
        }
    }

    async fn get_applied_migrations(&self, db: &D1Client) -> Result<Vec<i64>> {
        let sql = "SELECT version FROM _migrations ORDER BY version";
        let result = db.execute(sql, &[]).await?;

        let versions = result
            .into_rows()
            .into_iter()
            .filter_map(|row| {
                if let Value::Object(obj) = row {
                    obj.get("version").and_then(|v| v.as_i64())
                } else {
                    None
                }
            })
            .collect();

        Ok(versions)
    }

    async fn get_applied_migration_records(&self, db: &D1Client) -> Result<Vec<MigrationRecord>> {
        let sql = "SELECT * FROM _migrations ORDER BY version DESC";
        let result = db.execute(sql, &[]).await?;

        result.into_simple_entities()
    }

    async fn record_migration(&self, db: &D1Client, migration: &dyn Migration) -> Result<()> {
        let sql = "INSERT INTO _migrations (version, name) VALUES (?, ?)";
        let params = vec![
            serde_json::json!(migration.version() as i32),
            serde_json::json!(migration.name()),
        ];

        db.execute(sql, &params).await?;
        Ok(())
    }

    async fn remove_migration_record(&self, db: &D1Client, version: i64) -> Result<()> {
        let sql = "DELETE FROM _migrations WHERE version = ?";
        let params = vec![serde_json::json!(version as i32)];

        db.execute(sql, &params).await?;
        Ok(())
    }
}

pub struct CreateTableMigration {
    name: &'static str,
    version: i64,
    table_name: String,
    columns: Vec<ColumnDefinition>,
}

pub struct ColumnBuilder {
    migration: CreateTableMigration,
    current_column: ColumnDefinition,
}

impl CreateTableMigration {
    pub fn new(name: &'static str, version: i64, table_name: String) -> Self {
        Self {
            name,
            version,
            table_name,
            columns: Vec::new(),
        }
    }

    pub fn column(self, name: &str, column_type: &str) -> ColumnBuilder {
        let column = ColumnDefinition {
            name: name.to_string(),
            column_type: column_type.to_string(),
            nullable: true,
            primary_key: false,
            unique: false,
            default: None,
        };
        
        ColumnBuilder {
            migration: self,
            current_column: column,
        }
    }
}

impl ColumnBuilder {
    pub fn primary_key(mut self) -> Self {
        self.current_column.primary_key = true;
        self.current_column.nullable = false;
        self
    }

    pub fn unique(mut self) -> Self {
        self.current_column.unique = true;
        self
    }

    pub fn not_null(mut self) -> Self {
        self.current_column.nullable = false;
        self
    }

    pub fn default(mut self, value: &str) -> Self {
        self.current_column.default = Some(value.to_string());
        self
    }

    pub fn column(self, name: &str, column_type: &str) -> ColumnBuilder {
        // Finish the current column and add it to the migration
        let mut migration = self.migration;
        migration.columns.push(self.current_column);
        
        // Start a new column
        let column = ColumnDefinition {
            name: name.to_string(),
            column_type: column_type.to_string(),
            nullable: true,
            primary_key: false,
            unique: false,
            default: None,
        };
        
        ColumnBuilder {
            migration,
            current_column: column,
        }
    }

    // Finish the current column and return the migration
    pub fn finish(mut self) -> CreateTableMigration {
        self.migration.columns.push(self.current_column);
        self.migration
    }
}

// Implement the Migration trait for ColumnBuilder as well, to complete the chain
#[async_trait(?Send)]
impl Migration for ColumnBuilder {
    fn name(&self) -> &'static str {
        self.migration.name
    }

    fn version(&self) -> i64 {
        self.migration.version
    }

    async fn up(&self, db: &D1Client) -> Result<()> {
        // Create a copy with the current column added
        let mut migration = CreateTableMigration {
            name: self.migration.name,
            version: self.migration.version,
            table_name: self.migration.table_name.clone(),
            columns: self.migration.columns.clone(),
        };
        migration.columns.push(self.current_column.clone());
        migration.up(db).await
    }

    async fn down(&self, db: &D1Client) -> Result<()> {
        self.migration.down(db).await
    }
}

#[async_trait(?Send)]
impl Migration for CreateTableMigration {
    fn name(&self) -> &'static str {
        self.name
    }

    fn version(&self) -> i64 {
        self.version
    }

    async fn up(&self, db: &D1Client) -> Result<()> {
        let mut sql = format!("CREATE TABLE IF NOT EXISTS {} (", self.table_name);

        for (idx, column) in self.columns.iter().enumerate() {
            if idx > 0 {
                sql.push_str(", ");
            }

            sql.push_str(&format!("{} {}", column.name, column.column_type));

            if column.primary_key {
                sql.push_str(" PRIMARY KEY AUTOINCREMENT");
            }

            if !column.nullable && !column.primary_key {
                sql.push_str(" NOT NULL");
            }

            if column.unique && !column.primary_key {
                sql.push_str(" UNIQUE");
            }

            if let Some(ref default_val) = column.default {
                sql.push_str(&format!(" DEFAULT {}", default_val));
            }
        }

        sql.push(')');

        db.execute(&sql, &[]).await?;
        Ok(())
    }

    async fn down(&self, db: &D1Client) -> Result<()> {
        let sql = format!("DROP TABLE IF EXISTS {}", self.table_name);
        db.execute(&sql, &[]).await?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ColumnDefinition {
    pub name: String,
    pub column_type: String,
    pub nullable: bool,
    pub primary_key: bool,
    pub unique: bool,
    pub default: Option<String>,
}

impl Default for MigrationRunner {
    fn default() -> Self {
        Self::new()
    }
}

