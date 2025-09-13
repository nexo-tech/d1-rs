use crate::{D1Client, Result, D1RsError};
use serde_json::Value;
use std::collections::HashMap;

/// Revolutionary schema introspection engine - reads current database schema
/// Supports both SQLite and D1 backends with SQLite PRAGMA expertise
pub struct SchemaIntrospector<'a> {
    db: &'a D1Client,
}

impl<'a> SchemaIntrospector<'a> {
    pub fn new(db: &'a D1Client) -> Self {
        Self { db }
    }

    /// Introspect the entire database schema
    pub async fn introspect_database(&self) -> Result<DatabaseSchema> {
        let tables = self.introspect_tables().await?;
        let mut table_schemas = Vec::new();

        for table_name in tables {
            let table_schema = self.introspect_table(&table_name).await?;
            table_schemas.push(table_schema);
        }

        Ok(DatabaseSchema {
            tables: table_schemas,
        })
    }

    /// Get all table names in the database
    pub async fn introspect_tables(&self) -> Result<Vec<String>> {
        // Use SQLite master table to get all tables (excluding system tables but not migration tables)
        let sql = "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'";
        let result = self.db.execute(sql, &[]).await?;

        let mut tables = Vec::new();
        for row in result.rows {
            if let Value::Object(obj) = row {
                if let Some(Value::String(name)) = obj.get("name") {
                    tables.push(name.clone());
                }
            }
        }

        Ok(tables)
    }

    /// Introspect a specific table's schema
    pub async fn introspect_table(&self, table_name: &str) -> Result<TableSchema> {
        let columns = self.introspect_columns(table_name).await?;
        let indexes = self.introspect_indexes(table_name).await?;
        let foreign_keys = self.introspect_foreign_keys(table_name).await?;

        Ok(TableSchema {
            name: table_name.to_string(),
            columns,
            indexes,
            foreign_keys,
            constraints: vec![], // TODO: introspect other constraints
        })
    }

    /// Get column details with constraints using pragma_table_info function
    pub async fn introspect_columns(&self, table_name: &str) -> Result<Vec<ColumnSchema>> {
        let sql = format!("SELECT * FROM pragma_table_info('{}')", table_name);
        let result = self.db.execute(&sql, &[]).await?;

        let mut columns = Vec::new();
        for row in result.rows {
            if let Value::Object(obj) = row {
                let column = self.parse_column_info(obj)?;
                columns.push(column);
            }
        }

        Ok(columns)
    }

    /// Get index information using pragma_index_list and pragma_index_info functions
    pub async fn introspect_indexes(&self, table_name: &str) -> Result<Vec<IndexSchema>> {
        // Get list of indexes for the table
        let sql = format!("SELECT * FROM pragma_index_list('{}')", table_name);
        let result = self.db.execute(&sql, &[]).await?;

        let mut indexes = Vec::new();
        for row in result.rows {
            if let Value::Object(obj) = row {
                if let Some(Value::String(index_name)) = obj.get("name") {
                    // Skip auto-created indexes for primary keys and unique constraints
                    if index_name.starts_with("sqlite_autoindex_") {
                        continue;
                    }

                    let index_schema = self.introspect_index_details(index_name).await?;
                    indexes.push(index_schema);
                }
            }
        }

        Ok(indexes)
    }

    /// Get detailed information about a specific index
    async fn introspect_index_details(&self, index_name: &str) -> Result<IndexSchema> {
        let sql = format!("SELECT * FROM pragma_index_info('{}')", index_name);
        let result = self.db.execute(&sql, &[]).await?;

        let mut columns = Vec::new();
        for row in result.rows {
            if let Value::Object(obj) = row {
                if let Some(Value::String(column_name)) = obj.get("name") {
                    columns.push(column_name.clone());
                }
            }
        }

        // Check if index is unique by examining the index list
        let unique = self.is_index_unique(index_name).await?;

        Ok(IndexSchema {
            name: index_name.to_string(),
            columns,
            unique,
            table_name: None, // Will be set by the caller
        })
    }

    /// Check if an index is unique
    async fn is_index_unique(&self, index_name: &str) -> Result<bool> {
        let sql = "SELECT * FROM sqlite_master WHERE type='index' AND name=?";
        let params = vec![serde_json::json!(index_name)];
        let result = self.db.execute(sql, &params).await?;

        for row in result.rows {
            if let Value::Object(obj) = row {
                if let Some(Value::String(sql_text)) = obj.get("sql") {
                    // Check if the CREATE INDEX statement contains UNIQUE
                    return Ok(sql_text.to_uppercase().contains("UNIQUE"));
                }
            }
        }

        Ok(false)
    }

    /// Get foreign key relationships using pragma_foreign_key_list function
    pub async fn introspect_foreign_keys(&self, table_name: &str) -> Result<Vec<ForeignKeySchema>> {
        let sql = format!("SELECT * FROM pragma_foreign_key_list('{}')", table_name);
        let result = self.db.execute(&sql, &[]).await?;

        let mut foreign_keys = Vec::new();
        let mut fk_groups: HashMap<i64, Vec<Value>> = HashMap::new();

        // Group foreign key parts by their ID (for composite keys)
        for row in result.rows {
            if let Value::Object(ref obj) = row {
                if let Some(Value::Number(id)) = obj.get("id") {
                    if let Some(id) = id.as_i64() {
                        fk_groups.entry(id).or_insert_with(Vec::new).push(row);
                    }
                }
            }
        }

        // Process each foreign key group
        for (_, fk_rows) in fk_groups {
            if let Some(fk_schema) = self.parse_foreign_key_group(fk_rows)? {
                foreign_keys.push(fk_schema);
            }
        }

        Ok(foreign_keys)
    }

    /// Parse column information from PRAGMA table_info result
    fn parse_column_info(&self, obj: serde_json::Map<String, Value>) -> Result<ColumnSchema> {
        let name = obj.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| D1RsError::Database("Missing column name".to_string()))?
            .to_string();

        let column_type = obj.get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| D1RsError::Database("Missing column type".to_string()))?
            .to_string();

        let not_null = obj.get("notnull")
            .and_then(|v| v.as_i64())
            .map(|v| v != 0)
            .unwrap_or(false);

        let default_value = obj.get("dflt_value")
            .and_then(|v| match v {
                Value::Null => None,
                _ => v.as_str().map(|s| s.to_string()),
            });

        let primary_key = obj.get("pk")
            .and_then(|v| v.as_i64())
            .map(|v| v != 0)
            .unwrap_or(false);

        // Determine if this is a boolean column (stored as INTEGER in SQLite)
        let is_boolean = column_type.to_uppercase() == "INTEGER" && 
            (name.starts_with("is_") || name.starts_with("has_") || name.ends_with("_flag"));

        let final_column_type = if is_boolean { "BOOLEAN".to_string() } else { column_type.clone() };

        Ok(ColumnSchema {
            name,
            column_type: final_column_type,
            nullable: !not_null,
            default_value,
            primary_key,
            auto_increment: primary_key && column_type.to_uppercase() == "INTEGER", // SQLite auto-increment detection
            unique: false, // Will be detected from index information
            constraints: vec![],
        })
    }

    /// Parse foreign key information from grouped rows
    fn parse_foreign_key_group(&self, fk_rows: Vec<Value>) -> Result<Option<ForeignKeySchema>> {
        if fk_rows.is_empty() {
            return Ok(None);
        }

        let first_row = match &fk_rows[0] {
            Value::Object(obj) => obj,
            _ => return Ok(None),
        };

        let table = first_row.get("table")
            .and_then(|v| v.as_str())
            .ok_or_else(|| D1RsError::Database("Missing foreign key table".to_string()))?
            .to_string();

        let on_delete = first_row.get("on_delete")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let on_update = first_row.get("on_update")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Collect column mappings
        let mut columns = Vec::new();
        let mut referenced_columns = Vec::new();

        for row in fk_rows {
            if let Value::Object(obj) = row {
                if let Some(Value::String(from_col)) = obj.get("from") {
                    columns.push(from_col.clone());
                }
                if let Some(Value::String(to_col)) = obj.get("to") {
                    referenced_columns.push(to_col.clone());
                }
            }
        }

        Ok(Some(ForeignKeySchema {
            name: format!("fk_{}_{}", columns.join("_"), table), // Generate name
            columns,
            referenced_table: table,
            referenced_columns,
            on_delete,
            on_update,
        }))
    }
}

/// Complete database schema representation
#[derive(Debug, Clone)]
pub struct DatabaseSchema {
    pub tables: Vec<TableSchema>,
}

impl DatabaseSchema {
    pub fn get_table(&self, name: &str) -> Option<&TableSchema> {
        self.tables.iter().find(|t| t.name == name)
    }

    pub fn table_names(&self) -> Vec<&str> {
        self.tables.iter().map(|t| t.name.as_str()).collect()
    }
}

/// Table schema representation
#[derive(Debug, Clone)]
pub struct TableSchema {
    pub name: String,
    pub columns: Vec<ColumnSchema>,
    pub indexes: Vec<IndexSchema>,
    pub foreign_keys: Vec<ForeignKeySchema>,
    pub constraints: Vec<ConstraintSchema>,
}

impl TableSchema {
    pub fn get_column(&self, name: &str) -> Option<&ColumnSchema> {
        self.columns.iter().find(|c| c.name == name)
    }

    pub fn column_names(&self) -> Vec<&str> {
        self.columns.iter().map(|c| c.name.as_str()).collect()
    }

    pub fn primary_key_columns(&self) -> Vec<&ColumnSchema> {
        self.columns.iter().filter(|c| c.primary_key).collect()
    }
}

/// Column schema representation
#[derive(Debug, Clone)]
pub struct ColumnSchema {
    pub name: String,
    pub column_type: String,
    pub nullable: bool,
    pub default_value: Option<String>,
    pub primary_key: bool,
    pub auto_increment: bool,
    pub unique: bool,
    pub constraints: Vec<ColumnConstraint>,
}

/// Column-level constraints
#[derive(Debug, Clone)]
pub enum ColumnConstraint {
    Check { expression: String },
    References { table: String, column: String },
}

/// Index schema representation
#[derive(Debug, Clone)]
pub struct IndexSchema {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
    pub table_name: Option<String>,
}

/// Foreign key constraint representation
#[derive(Debug, Clone)]
pub struct ForeignKeySchema {
    pub name: String,
    pub columns: Vec<String>,
    pub referenced_table: String,
    pub referenced_columns: Vec<String>,
    pub on_delete: Option<String>,
    pub on_update: Option<String>,
}

/// General constraint representation
#[derive(Debug, Clone)]
pub struct ConstraintSchema {
    pub name: String,
    pub constraint_type: ConstraintType,
    pub definition: String,
}

/// Types of database constraints
#[derive(Debug, Clone)]
pub enum ConstraintType {
    PrimaryKey,
    Unique,
    ForeignKey,
    Check,
    NotNull,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::D1Client;

    async fn create_test_db() -> D1Client {
        let db = D1Client::new_in_memory().await.unwrap();
        
        // Create a test table with various column types and constraints
        let sql = r#"
            CREATE TABLE test_users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                email TEXT UNIQUE NOT NULL,
                age INTEGER,
                is_active BOOLEAN DEFAULT 1,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        "#;
        db.execute(sql, &[]).await.unwrap();

        // Create an index
        let index_sql = "CREATE INDEX idx_users_email ON test_users(email)";
        db.execute(index_sql, &[]).await.unwrap();

        // Create a table with foreign keys
        let posts_sql = r#"
            CREATE TABLE test_posts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                user_id INTEGER NOT NULL,
                FOREIGN KEY (user_id) REFERENCES test_users(id) ON DELETE CASCADE
            )
        "#;
        db.execute(posts_sql, &[]).await.unwrap();

        db
    }

    #[tokio::test]
    async fn test_introspect_tables() {
        let db = create_test_db().await;
        let introspector = SchemaIntrospector::new(&db);

        let tables = introspector.introspect_tables().await.unwrap();
        
        assert!(tables.contains(&"test_users".to_string()));
        assert!(tables.contains(&"test_posts".to_string()));
        assert!(!tables.is_empty());
    }

    #[tokio::test]
    async fn test_introspect_columns() {
        let db = create_test_db().await;
        let introspector = SchemaIntrospector::new(&db);

        let columns = introspector.introspect_columns("test_users").await.unwrap();
        
        // Check we have all expected columns
        assert_eq!(columns.len(), 6);
        
        // Check specific column properties
        let id_col = columns.iter().find(|c| c.name == "id").unwrap();
        assert!(id_col.primary_key);
        assert!(id_col.auto_increment);
        assert_eq!(id_col.column_type, "INTEGER");

        let email_col = columns.iter().find(|c| c.name == "email").unwrap();
        assert!(!email_col.nullable);
        assert_eq!(email_col.column_type, "TEXT");

        let is_active_col = columns.iter().find(|c| c.name == "is_active").unwrap();
        assert_eq!(is_active_col.column_type, "BOOLEAN"); // Should be detected as boolean
        assert_eq!(is_active_col.default_value, Some("1".to_string()));
    }

    #[tokio::test]
    async fn test_introspect_indexes() {
        let db = create_test_db().await;
        let introspector = SchemaIntrospector::new(&db);

        let indexes = introspector.introspect_indexes("test_users").await.unwrap();
        
        // Should have our created index (auto-indexes are filtered out)
        let email_index = indexes.iter().find(|i| i.name == "idx_users_email").unwrap();
        assert_eq!(email_index.columns, vec!["email"]);
        assert!(!email_index.unique); // It's a regular index, not unique
    }

    #[tokio::test]
    async fn test_introspect_foreign_keys() {
        let db = create_test_db().await;
        let introspector = SchemaIntrospector::new(&db);

        let foreign_keys = introspector.introspect_foreign_keys("test_posts").await.unwrap();
        
        assert_eq!(foreign_keys.len(), 1);
        
        let fk = &foreign_keys[0];
        assert_eq!(fk.columns, vec!["user_id"]);
        assert_eq!(fk.referenced_table, "test_users");
        assert_eq!(fk.referenced_columns, vec!["id"]);
        assert_eq!(fk.on_delete, Some("CASCADE".to_string()));
    }

    #[tokio::test]
    async fn test_introspect_database() {
        let db = create_test_db().await;
        let introspector = SchemaIntrospector::new(&db);

        let schema = introspector.introspect_database().await.unwrap();
        
        assert_eq!(schema.tables.len(), 2);
        
        let users_table = schema.get_table("test_users").unwrap();
        assert_eq!(users_table.columns.len(), 6);
        assert!(!users_table.indexes.is_empty());

        let posts_table = schema.get_table("test_posts").unwrap();
        assert_eq!(posts_table.columns.len(), 3);
        assert_eq!(posts_table.foreign_keys.len(), 1);
    }

    #[tokio::test]
    async fn test_boolean_column_detection() {
        let db = D1Client::new_in_memory().await.unwrap();
        
        // Create table with various boolean-like columns
        let sql = r#"
            CREATE TABLE test_booleans (
                id INTEGER PRIMARY KEY,
                is_active INTEGER DEFAULT 0,
                has_permission INTEGER,
                status_flag INTEGER,
                regular_int INTEGER
            )
        "#;
        db.execute(sql, &[]).await.unwrap();

        let introspector = SchemaIntrospector::new(&db);
        let columns = introspector.introspect_columns("test_booleans").await.unwrap();

        // Check boolean detection
        let is_active = columns.iter().find(|c| c.name == "is_active").unwrap();
        assert_eq!(is_active.column_type, "BOOLEAN");

        let has_permission = columns.iter().find(|c| c.name == "has_permission").unwrap();
        assert_eq!(has_permission.column_type, "BOOLEAN");

        let status_flag = columns.iter().find(|c| c.name == "status_flag").unwrap();
        assert_eq!(status_flag.column_type, "BOOLEAN");

        let regular_int = columns.iter().find(|c| c.name == "regular_int").unwrap();
        assert_eq!(regular_int.column_type, "INTEGER"); // Should not be detected as boolean
    }
}