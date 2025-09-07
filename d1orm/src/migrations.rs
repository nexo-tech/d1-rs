use crate::{Result, D1Client, D1OrmError};
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
            if let Some(migration) = self.migrations.iter().find(|m| m.version() == record.version) {
                migration.down(db).await?;
                self.remove_migration_record(db, record.version).await?;
            }
        }

        Ok(())
    }

    async fn ensure_migration_table(&self, db: &D1Client) -> Result<()> {
        let sql = r#"
            CREATE TABLE IF NOT EXISTS __migrations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                version INTEGER NOT NULL UNIQUE,
                name TEXT NOT NULL,
                applied_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        "#;

        db.execute(sql, &[]).await?;
        Ok(())
    }

    async fn is_migration_applied(&self, db: &D1Client, version: i64) -> Result<bool> {
        let sql = "SELECT COUNT(*) as count FROM __migrations WHERE version = ?";
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
        let sql = "SELECT version FROM __migrations ORDER BY version";
        let result = db.execute(sql, &[]).await?;
        
        let versions = result.rows
            .into_iter()
            .filter_map(|row| {
                if let Value::Object(obj) = row {
                    obj.get("version")
                        .and_then(|v| v.as_i64())
                } else {
                    None
                }
            })
            .collect();

        Ok(versions)
    }

    async fn get_applied_migration_records(&self, db: &D1Client) -> Result<Vec<MigrationRecord>> {
        let sql = "SELECT * FROM __migrations ORDER BY version DESC";
        let result = db.execute(sql, &[]).await?;
        
        result.into_entities()
    }

    async fn record_migration(&self, db: &D1Client, migration: &dyn Migration) -> Result<()> {
        let sql = "INSERT INTO __migrations (version, name) VALUES (?, ?)";
        let params = vec![
            serde_json::json!(migration.version() as i32),
            serde_json::json!(migration.name()),
        ];

        db.execute(sql, &params).await?;
        Ok(())
    }

    async fn remove_migration_record(&self, db: &D1Client, version: i64) -> Result<()> {
        let sql = "DELETE FROM __migrations WHERE version = ?";
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

impl CreateTableMigration {
    pub fn new(name: &'static str, version: i64, table_name: String) -> Self {
        Self {
            name,
            version,
            table_name,
            columns: Vec::new(),
        }
    }

    pub fn column(mut self, name: &str, column_type: &str) -> Self {
        self.columns.push(ColumnDefinition {
            name: name.to_string(),
            column_type: column_type.to_string(),
            nullable: true,
            primary_key: false,
            unique: false,
            default: None,
        });
        self
    }

    pub fn primary_key(mut self) -> Self {
        if let Some(last) = self.columns.last_mut() {
            last.primary_key = true;
            last.nullable = false;
        }
        self
    }

    pub fn unique(mut self) -> Self {
        if let Some(last) = self.columns.last_mut() {
            last.unique = true;
        }
        self
    }

    pub fn not_null(mut self) -> Self {
        if let Some(last) = self.columns.last_mut() {
            last.nullable = false;
        }
        self
    }

    pub fn default(mut self, value: &str) -> Self {
        if let Some(last) = self.columns.last_mut() {
            last.default = Some(value.to_string());
        }
        self
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


#[derive(Debug)]
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