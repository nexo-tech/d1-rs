use crate::{D1Client, Result, D1RsError};
use crate::backends::QueryResult;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

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
        for row in result.rows() {
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
        let constraints = self.introspect_table_constraints(table_name).await?;

        Ok(TableSchema {
            name: table_name.to_string(),
            columns,
            indexes,
            foreign_keys,
            constraints,
        })
    }

    /// Introspect table-level constraints from CREATE TABLE statement
    /// Extracts CHECK, UNIQUE, and PRIMARY KEY constraints not covered by column/FK introspection
    pub async fn introspect_table_constraints(&self, table_name: &str) -> Result<Vec<ConstraintSchema>> {
        // Get the CREATE TABLE statement from sqlite_master
        let sql = "SELECT sql FROM sqlite_master WHERE type='table' AND name=?";
        let params = vec![serde_json::json!(table_name)];
        let result = self.db.execute(sql, &params).await?;

        let create_sql = match result.rows().first() {
            Some(Value::Object(obj)) => {
                obj.get("sql")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| D1RsError::Database(format!("No CREATE statement found for table '{}'", table_name)))?
            }
            _ => return Ok(vec![]), // Table not found or no SQL
        };

        self.parse_table_constraints(create_sql, table_name)
    }

    /// Parse table-level constraints from CREATE TABLE SQL statement
    fn parse_table_constraints(&self, create_sql: &str, table_name: &str) -> Result<Vec<ConstraintSchema>> {
        let mut constraints = Vec::new();
        
        // Normalize the SQL for easier parsing
        let normalized_sql = create_sql
            .replace('\n', " ")
            .replace('\r', " ")
            .replace('\t', " ");
        
        // Find the content between the parentheses
        let table_def = self.extract_table_definition(&normalized_sql)?;
        
        // Split by commas but be careful of nested parentheses
        let parts = self.split_table_definition(&table_def);
        
        for (i, part) in parts.iter().enumerate() {
            let trimmed = part.trim();
            
            // Skip column definitions (they don't start with constraint keywords)
            if self.is_column_definition(trimmed) {
                continue;
            }
            
            // Parse different constraint types
            if let Some(constraint) = self.parse_check_constraint(trimmed, table_name, i)? {
                constraints.push(constraint);
            } else if let Some(constraint) = self.parse_unique_constraint(trimmed, table_name, i)? {
                constraints.push(constraint);
            } else if let Some(constraint) = self.parse_primary_key_constraint(trimmed, table_name, i)? {
                constraints.push(constraint);
            }
        }
        
        Ok(constraints)
    }

    /// Extract the table definition content between parentheses
    fn extract_table_definition(&self, create_sql: &str) -> Result<String> {
        // Find the opening parenthesis after CREATE TABLE
        let start = create_sql
            .find('(')
            .ok_or_else(|| D1RsError::Database("Invalid CREATE TABLE syntax: no opening parenthesis".to_string()))?;
        
        // Find the matching closing parenthesis
        let mut paren_count = 0;
        let mut end = start;
        
        for (i, ch) in create_sql.chars().enumerate().skip(start) {
            match ch {
                '(' => paren_count += 1,
                ')' => {
                    paren_count -= 1;
                    if paren_count == 0 {
                        end = i;
                        break;
                    }
                },
                _ => {}
            }
        }
        
        if paren_count != 0 {
            return Err(D1RsError::Database("Invalid CREATE TABLE syntax: unmatched parentheses".to_string()));
        }
        
        Ok(create_sql[start + 1..end].to_string())
    }

    /// Split table definition by commas while respecting nested parentheses
    fn split_table_definition(&self, table_def: &str) -> Vec<String> {
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut paren_count = 0;
        let mut in_quotes = false;
        let mut quote_char = ' ';
        
        for ch in table_def.chars() {
            match ch {
                '\'' | '"' if !in_quotes => {
                    in_quotes = true;
                    quote_char = ch;
                    current.push(ch);
                },
                c if in_quotes && c == quote_char => {
                    in_quotes = false;
                    current.push(ch);
                },
                '(' if !in_quotes => {
                    paren_count += 1;
                    current.push(ch);
                },
                ')' if !in_quotes => {
                    paren_count -= 1;
                    current.push(ch);
                },
                ',' if !in_quotes && paren_count == 0 => {
                    parts.push(current.trim().to_string());
                    current.clear();
                },
                _ => {
                    current.push(ch);
                }
            }
        }
        
        if !current.trim().is_empty() {
            parts.push(current.trim().to_string());
        }
        
        parts
    }

    /// Check if a definition part is a column definition (vs constraint)
    fn is_column_definition(&self, part: &str) -> bool {
        let part_upper = part.to_uppercase();
        
        // If it starts with constraint keywords, it's a constraint
        if part_upper.starts_with("CONSTRAINT ") ||
           part_upper.starts_with("PRIMARY KEY") ||
           part_upper.starts_with("UNIQUE") ||
           part_upper.starts_with("CHECK") ||
           part_upper.starts_with("FOREIGN KEY") {
            return false;
        }
        
        // If it contains common column type keywords, it's likely a column
        let column_keywords = ["INTEGER", "TEXT", "REAL", "BLOB", "BOOLEAN", "VARCHAR", "CHAR", "DECIMAL", "DATETIME"];
        for keyword in &column_keywords {
            if part_upper.contains(keyword) {
                return true;
            }
        }
        
        // Default: assume it's a column if we can't determine otherwise
        true
    }

    /// Parse CHECK constraint from table definition part
    fn parse_check_constraint(&self, part: &str, table_name: &str, index: usize) -> Result<Option<ConstraintSchema>> {
        let part_upper = part.to_uppercase();
        
        if part_upper.contains("CHECK") {
            let constraint_name = if part_upper.starts_with("CONSTRAINT ") {
                // Named constraint: CONSTRAINT name CHECK (expression)
                part.split_whitespace()
                    .nth(1)
                    .unwrap_or(&format!("check_constraint_{}", index))
                    .to_string()
            } else {
                // Unnamed constraint: CHECK (expression)
                format!("{}_check_{}", table_name, index)
            };
            
            Ok(Some(ConstraintSchema {
                name: constraint_name,
                constraint_type: ConstraintType::Check,
                definition: part.to_string(),
            }))
        } else {
            Ok(None)
        }
    }

    /// Parse UNIQUE constraint from table definition part
    fn parse_unique_constraint(&self, part: &str, table_name: &str, index: usize) -> Result<Option<ConstraintSchema>> {
        let part_upper = part.to_uppercase();
        
        if part_upper.starts_with("UNIQUE") || (part_upper.starts_with("CONSTRAINT ") && part_upper.contains("UNIQUE")) {
            let constraint_name = if part_upper.starts_with("CONSTRAINT ") {
                // Named constraint: CONSTRAINT name UNIQUE (columns)
                part.split_whitespace()
                    .nth(1)
                    .unwrap_or(&format!("unique_constraint_{}", index))
                    .to_string()
            } else {
                // Unnamed constraint: UNIQUE (columns)
                format!("{}_unique_{}", table_name, index)
            };
            
            Ok(Some(ConstraintSchema {
                name: constraint_name,
                constraint_type: ConstraintType::Unique,
                definition: part.to_string(),
            }))
        } else {
            Ok(None)
        }
    }

    /// Parse PRIMARY KEY constraint from table definition part
    fn parse_primary_key_constraint(&self, part: &str, table_name: &str, index: usize) -> Result<Option<ConstraintSchema>> {
        let part_upper = part.to_uppercase();
        
        if part_upper.starts_with("PRIMARY KEY") || (part_upper.starts_with("CONSTRAINT ") && part_upper.contains("PRIMARY KEY")) {
            let constraint_name = if part_upper.starts_with("CONSTRAINT ") {
                // Named constraint: CONSTRAINT name PRIMARY KEY (columns)
                part.split_whitespace()
                    .nth(1)
                    .unwrap_or(&format!("pk_constraint_{}", index))
                    .to_string()
            } else {
                // Unnamed constraint: PRIMARY KEY (columns)
                format!("{}_pk_{}", table_name, index)
            };
            
            Ok(Some(ConstraintSchema {
                name: constraint_name,
                constraint_type: ConstraintType::PrimaryKey,
                definition: part.to_string(),
            }))
        } else {
            Ok(None)
        }
    }

    /// Get column details with constraints using pragma_table_info function
    /// NOTE: This method does NOT detect boolean fields - use introspect_columns_with_entity() for accurate boolean detection
    pub async fn introspect_columns(&self, table_name: &str) -> Result<Vec<ColumnSchema>> {
        let sql = format!("SELECT * FROM pragma_table_info('{}')", table_name);
        let result = self.db.execute(&sql, &[]).await?;

        let mut columns = Vec::new();
        for row in result.rows() {
            if let Value::Object(obj) = row {
                let column = self.parse_column_info(obj.clone(), None)?;
                columns.push(column);
            }
        }

        Ok(columns)
    }

    /// REVOLUTIONARY: Entity-aware column introspection - NO HEURISTICS!
    /// Uses Entity::boolean_fields() for accurate boolean detection instead of name patterns
    pub async fn introspect_columns_with_entity<T: crate::Entity>(&self, table_name: &str) -> Result<Vec<ColumnSchema>> {
        let sql = format!("SELECT * FROM pragma_table_info('{}')", table_name);
        let result = self.db.execute(&sql, &[]).await?;

        // Get boolean field names from the entity trait - COMPILE-TIME SAFE!
        let boolean_fields: HashSet<String> = T::boolean_fields()
            .iter()
            .map(|s| s.to_string())
            .collect();

        let mut columns = Vec::new();
        for row in result.rows() {
            if let Value::Object(obj) = row {
                let column = self.parse_column_info(obj.clone(), Some(&boolean_fields))?;
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
        for row in result.rows() {
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
        for row in result.rows() {
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

        for row in result.rows() {
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
        for row in result.rows() {
            if let Value::Object(ref obj) = row {
                if let Some(Value::Number(id)) = obj.get("id") {
                    if let Some(id) = id.as_i64() {
                        fk_groups.entry(id).or_insert_with(Vec::new).push(row.clone());
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
    /// If boolean_fields is provided, uses trait-based detection instead of heuristics
    fn parse_column_info(&self, obj: serde_json::Map<String, Value>, boolean_fields: Option<&HashSet<String>>) -> Result<ColumnSchema> {
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

        // REVOLUTIONARY: Determine if this is a boolean column - NO HEURISTICS EVER!
        let is_boolean = column_type.to_uppercase() == "INTEGER" && 
            match boolean_fields {
                Some(boolean_set) => boolean_set.contains(&name), // Trait-based detection ONLY!
                None => false, // NO FALLBACK HEURISTICS! Use entity-aware introspection instead.
            };

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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum ColumnConstraint {
    Check { expression: String },
    References { table: String, column: String },
}

/// Index schema representation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct IndexSchema {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
    pub table_name: Option<String>,
}

/// Foreign key constraint representation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ForeignKeySchema {
    pub name: String,
    pub columns: Vec<String>,
    pub referenced_table: String,
    pub referenced_columns: Vec<String>,
    pub on_delete: Option<String>,
    pub on_update: Option<String>,
}

/// General constraint representation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ConstraintSchema {
    pub name: String,
    pub constraint_type: ConstraintType,
    pub definition: String,
}

/// Types of database constraints
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
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
    async fn test_no_heuristic_boolean_detection() {
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

        // REVOLUTIONARY: Generic introspection NO LONGER uses heuristics!
        // All INTEGER columns are reported as INTEGER - no guessing!
        let is_active = columns.iter().find(|c| c.name == "is_active").unwrap();
        assert_eq!(is_active.column_type, "INTEGER", "No heuristics: is_active stays INTEGER");

        let has_permission = columns.iter().find(|c| c.name == "has_permission").unwrap();
        assert_eq!(has_permission.column_type, "INTEGER", "No heuristics: has_permission stays INTEGER");

        let status_flag = columns.iter().find(|c| c.name == "status_flag").unwrap();
        assert_eq!(status_flag.column_type, "INTEGER", "No heuristics: status_flag stays INTEGER");

        let regular_int = columns.iter().find(|c| c.name == "regular_int").unwrap();
        assert_eq!(regular_int.column_type, "INTEGER", "No heuristics: regular_int stays INTEGER");

        println!("🚀 REVOLUTIONARY: No more boolean heuristics! Use entity-aware introspection for boolean detection.");
    }

    // Simple test entity for revolutionary entity-aware boolean detection testing
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    struct TestUserForIntrospection {
        id: i64,
        is_active: bool,
    }

    #[derive(Debug, Clone)]
    struct TestUserQueryBuilder;
    
    impl crate::QueryBuilder<TestUserForIntrospection> for TestUserQueryBuilder {
        async fn all(self, _db: &D1Client) -> Result<Vec<TestUserForIntrospection>> { unimplemented!() }
        async fn first(self, _db: &D1Client) -> Result<Option<TestUserForIntrospection>> { unimplemented!() }
        async fn count(self, _db: &D1Client) -> Result<i64> { unimplemented!() }
        fn apply_relation_constraint(self, _field: &str, _value: serde_json::Value) -> Self { self }
    }
    
    #[derive(Debug, Clone)]
    struct TestUserCreateBuilder;
    
    impl crate::CreateBuilder<TestUserForIntrospection> for TestUserCreateBuilder {
        async fn save(self, _db: &D1Client) -> Result<TestUserForIntrospection> { unimplemented!() }
    }
    
    #[derive(Debug, Clone)]
    struct TestUserUpdateBuilder;
    
    impl crate::UpdateBuilder<TestUserForIntrospection> for TestUserUpdateBuilder {
        async fn save(self, _db: &D1Client) -> Result<TestUserForIntrospection> { unimplemented!() }
    }

    impl crate::Entity for TestUserForIntrospection {
        type PrimaryKey = i64;
        type QueryBuilder = TestUserQueryBuilder;
        type CreateBuilder = TestUserCreateBuilder;
        type UpdateBuilder = TestUserUpdateBuilder;

        const TABLE_NAME: &'static str = "test_users";
        
        fn primary_key(&self) -> &Self::PrimaryKey {
            &self.id
        }

        fn boolean_fields() -> &'static [&'static str] {
            // REVOLUTIONARY: Explicitly specify boolean fields - NO GUESSING!
            &["is_active"]
        }

        fn field_definitions() -> Vec<crate::FieldDefinition> {
            vec![
                crate::FieldDefinition {
                    name: "id".to_string(),
                    field_type: crate::FieldType::Integer,
                    nullable: false,
                    primary_key: true,
                    auto_increment: true,
                    default_value: None,
                    foreign_key: None,
                },
                crate::FieldDefinition {
                    name: "is_active".to_string(),
                    field_type: crate::FieldType::Boolean,
                    nullable: false,
                    primary_key: false,
                    auto_increment: false,
                    default_value: None,
                    foreign_key: None,
                },
            ]
        }

        async fn find(_db: &D1Client, _key: Self::PrimaryKey) -> Result<Option<Self>> {
            unimplemented!()
        }

        async fn delete(_db: &D1Client, _key: Self::PrimaryKey) -> Result<()> {
            unimplemented!()
        }

        fn query() -> Self::QueryBuilder {
            TestUserQueryBuilder
        }
        
        fn create() -> Self::CreateBuilder {
            TestUserCreateBuilder
        }
        
        fn update(_key: Self::PrimaryKey) -> Self::UpdateBuilder {
            TestUserUpdateBuilder
        }
    }

    #[tokio::test]
    async fn test_revolutionary_entity_aware_boolean_detection() {
        let db = D1Client::new_in_memory().await.unwrap();
        
        // Create table that matches TestUser structure but also tests edge cases
        let sql = r#"
            CREATE TABLE test_users (
                id INTEGER PRIMARY KEY,
                is_active INTEGER DEFAULT 0,           -- Boolean field defined in TestUser::boolean_fields()
                normal_count INTEGER DEFAULT 0         -- INTEGER that would be detected as boolean by heuristics
            )
        "#;
        db.execute(sql, &[]).await.unwrap();

        let introspector = SchemaIntrospector::new(&db);
        
        // Test REVOLUTIONARY entity-aware introspection - NO HEURISTICS!
        let entity_aware_columns = introspector
            .introspect_columns_with_entity::<TestUserForIntrospection>("test_users")
            .await
            .unwrap();

        // REVOLUTIONARY: Accurate detection based on Entity::boolean_fields() trait!
        let is_active = entity_aware_columns.iter().find(|c| c.name == "is_active").unwrap();
        assert_eq!(is_active.column_type, "BOOLEAN", "Entity-aware: 'is_active' should be BOOLEAN (from Entity::boolean_fields())");

        let normal_count = entity_aware_columns.iter().find(|c| c.name == "normal_count").unwrap();
        assert_eq!(normal_count.column_type, "INTEGER", "Entity-aware: 'normal_count' should be INTEGER (not in boolean_fields)");

        // Compare with legacy generic introspection (NO HEURISTICS)
        let generic_columns = introspector
            .introspect_columns("test_users")
            .await
            .unwrap();

        // REVOLUTIONARY: Generic introspection no longer uses heuristics - no boolean detection
        let is_active_generic = generic_columns.iter().find(|c| c.name == "is_active").unwrap();
        assert_eq!(is_active_generic.column_type, "INTEGER", "Generic: 'is_active' stays INTEGER (no heuristics)");

        let normal_count_generic = generic_columns.iter().find(|c| c.name == "normal_count").unwrap();
        assert_eq!(normal_count_generic.column_type, "INTEGER", "Generic: 'normal_count' stays INTEGER");

        // This demonstrates the REVOLUTIONARY superiority of trait-based detection!
        println!("🚀 REVOLUTIONARY SUCCESS: Entity-aware detection uses trait information!");
        println!("🚀 Generic introspection eliminated heuristics completely!");
        println!("✅ Boolean detection now requires explicit entity context!");
    }

    #[tokio::test]
    async fn test_introspect_check_constraints() {
        let db = D1Client::new_in_memory().await.unwrap();
        
        // Create table with various table-level CHECK constraints
        // Note: Column-level CHECK constraints (like in column definitions) are handled by column introspection
        let sql = r#"
            CREATE TABLE test_constraints (
                id INTEGER PRIMARY KEY,
                email TEXT,
                status TEXT,
                age INTEGER,
                CONSTRAINT valid_email CHECK (email LIKE '%@%.%'),
                CHECK (status IN ('active', 'inactive', 'pending')),
                CHECK (age >= 0 AND age <= 150)
            )
        "#;
        db.execute(sql, &[]).await.unwrap();

        let introspector = SchemaIntrospector::new(&db);
        let constraints = introspector.introspect_table_constraints("test_constraints").await.unwrap();
        
        // Should find the 3 table-level CHECK constraints
        let check_constraints: Vec<_> = constraints.iter()
            .filter(|c| matches!(c.constraint_type, ConstraintType::Check))
            .collect();
        
        assert_eq!(check_constraints.len(), 3, "Should find 3 table-level CHECK constraints");
        
        // Verify named constraint
        let named_check = check_constraints.iter()
            .find(|c| c.name == "valid_email")
            .expect("Should find named CHECK constraint 'valid_email'");
        assert!(named_check.definition.contains("email LIKE '%@%.%'"));
        
        // Verify unnamed constraints get generated names
        let unnamed_checks: Vec<_> = check_constraints.iter()
            .filter(|c| c.name.contains("test_constraints_check_"))
            .collect();
        assert_eq!(unnamed_checks.len(), 2, "Should find 2 unnamed CHECK constraints with generated names");
    }

    #[tokio::test]
    async fn test_introspect_unique_constraints() {
        let db = D1Client::new_in_memory().await.unwrap();
        
        // Create table with table-level UNIQUE constraints
        let sql = r#"
            CREATE TABLE test_unique (
                id INTEGER PRIMARY KEY,
                first_name TEXT,
                last_name TEXT,
                email TEXT,
                phone TEXT,
                UNIQUE (first_name, last_name),
                CONSTRAINT unique_contact UNIQUE (email, phone)
            )
        "#;
        db.execute(sql, &[]).await.unwrap();

        let introspector = SchemaIntrospector::new(&db);
        let constraints = introspector.introspect_table_constraints("test_unique").await.unwrap();
        
        // Should find the 2 UNIQUE constraints
        let unique_constraints: Vec<_> = constraints.iter()
            .filter(|c| matches!(c.constraint_type, ConstraintType::Unique))
            .collect();
        
        assert_eq!(unique_constraints.len(), 2, "Should find 2 UNIQUE constraints");
        
        // Verify named constraint
        let named_unique = unique_constraints.iter()
            .find(|c| c.name == "unique_contact")
            .expect("Should find named UNIQUE constraint 'unique_contact'");
        assert!(named_unique.definition.contains("email, phone"));
        
        // Verify unnamed constraint gets generated name
        let unnamed_unique = unique_constraints.iter()
            .find(|c| c.name.contains("test_unique_unique_"))
            .expect("Should find unnamed UNIQUE constraint with generated name");
        assert!(unnamed_unique.definition.contains("first_name, last_name"));
    }

    #[tokio::test]
    async fn test_introspect_primary_key_constraints() {
        let db = D1Client::new_in_memory().await.unwrap();
        
        // Create table with composite primary key
        let sql = r#"
            CREATE TABLE test_composite_pk (
                tenant_id INTEGER,
                user_id INTEGER,
                name TEXT,
                PRIMARY KEY (tenant_id, user_id)
            )
        "#;
        db.execute(sql, &[]).await.unwrap();

        let introspector = SchemaIntrospector::new(&db);
        let constraints = introspector.introspect_table_constraints("test_composite_pk").await.unwrap();
        
        // Should find the composite PRIMARY KEY constraint
        let pk_constraints: Vec<_> = constraints.iter()
            .filter(|c| matches!(c.constraint_type, ConstraintType::PrimaryKey))
            .collect();
        
        assert_eq!(pk_constraints.len(), 1, "Should find 1 PRIMARY KEY constraint");
        
        let pk = &pk_constraints[0];
        assert!(pk.name.contains("test_composite_pk_pk_"));
        assert!(pk.definition.contains("tenant_id, user_id"));
    }

    #[tokio::test]
    async fn test_introspect_named_primary_key_constraint() {
        let db = D1Client::new_in_memory().await.unwrap();
        
        // Create table with named composite primary key
        let sql = r#"
            CREATE TABLE test_named_pk (
                region_id INTEGER,
                location_id INTEGER,
                name TEXT,
                CONSTRAINT pk_region_location PRIMARY KEY (region_id, location_id)
            )
        "#;
        db.execute(sql, &[]).await.unwrap();

        let introspector = SchemaIntrospector::new(&db);
        let constraints = introspector.introspect_table_constraints("test_named_pk").await.unwrap();
        
        // Should find the named PRIMARY KEY constraint
        let pk_constraints: Vec<_> = constraints.iter()
            .filter(|c| matches!(c.constraint_type, ConstraintType::PrimaryKey))
            .collect();
        
        assert_eq!(pk_constraints.len(), 1, "Should find 1 PRIMARY KEY constraint");
        
        let pk = &pk_constraints[0];
        assert_eq!(pk.name, "pk_region_location");
        assert!(pk.definition.contains("PRIMARY KEY (region_id, location_id)"));
    }

    #[tokio::test]
    async fn test_introspect_mixed_constraints() {
        let db = D1Client::new_in_memory().await.unwrap();
        
        // Create table with multiple table-level constraint types
        let sql = r#"
            CREATE TABLE test_mixed (
                id INTEGER,
                category_id INTEGER,
                name TEXT NOT NULL,
                price REAL,
                status TEXT DEFAULT 'active',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (id, category_id),
                UNIQUE (name, category_id),
                CHECK (price > 0),
                CONSTRAINT valid_status CHECK (status IN ('active', 'inactive')),
                CONSTRAINT unique_name_per_category UNIQUE (name, category_id)
            )
        "#;
        db.execute(sql, &[]).await.unwrap();

        let introspector = SchemaIntrospector::new(&db);
        let constraints = introspector.introspect_table_constraints("test_mixed").await.unwrap();
        
        // Count constraint types
        let check_count = constraints.iter().filter(|c| matches!(c.constraint_type, ConstraintType::Check)).count();
        let unique_count = constraints.iter().filter(|c| matches!(c.constraint_type, ConstraintType::Unique)).count();
        let pk_count = constraints.iter().filter(|c| matches!(c.constraint_type, ConstraintType::PrimaryKey)).count();
        
        assert_eq!(check_count, 2, "Should find 2 CHECK constraints");
        assert_eq!(unique_count, 2, "Should find 2 UNIQUE constraints (note: duplicate definition should still be parsed)");
        assert_eq!(pk_count, 1, "Should find 1 PRIMARY KEY constraint");
        
        // Verify named constraints are found
        let constraint_names: Vec<&String> = constraints.iter().map(|c| &c.name).collect();
        assert!(constraint_names.contains(&&"valid_status".to_string()));
        assert!(constraint_names.contains(&&"unique_name_per_category".to_string()));
    }

    #[tokio::test]
    async fn test_introspect_constraints_integration() {
        let db = D1Client::new_in_memory().await.unwrap();
        
        // Create table with table-level constraints
        let sql = r#"
            CREATE TABLE test_integration (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                age INTEGER,
                email TEXT,
                CHECK (age >= 0),
                UNIQUE (name, email)
            )
        "#;
        db.execute(sql, &[]).await.unwrap();

        let introspector = SchemaIntrospector::new(&db);
        
        // Test that introspect_table includes constraints
        let table_schema = introspector.introspect_table("test_integration").await.unwrap();
        
        assert!(!table_schema.constraints.is_empty(), "Table schema should include constraints");
        
        // Should have CHECK and UNIQUE constraints
        let has_check = table_schema.constraints.iter()
            .any(|c| matches!(c.constraint_type, ConstraintType::Check));
        let has_unique = table_schema.constraints.iter()
            .any(|c| matches!(c.constraint_type, ConstraintType::Unique));
        
        assert!(has_check, "Should find CHECK constraint in table schema");
        assert!(has_unique, "Should find UNIQUE constraint in table schema");
    }

    #[tokio::test]
    async fn test_introspect_constraints_empty_table() {
        let db = D1Client::new_in_memory().await.unwrap();
        
        // Create simple table with no table-level constraints
        let sql = r#"
            CREATE TABLE test_no_constraints (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL
            )
        "#;
        db.execute(sql, &[]).await.unwrap();

        let introspector = SchemaIntrospector::new(&db);
        let constraints = introspector.introspect_table_constraints("test_no_constraints").await.unwrap();
        
        assert!(constraints.is_empty(), "Should find no table-level constraints");
    }

    #[tokio::test]
    async fn test_introspect_constraints_nonexistent_table() {
        let db = D1Client::new_in_memory().await.unwrap();

        let introspector = SchemaIntrospector::new(&db);
        let constraints = introspector.introspect_table_constraints("nonexistent_table").await.unwrap();
        
        assert!(constraints.is_empty(), "Should return empty vector for nonexistent table");
    }

    #[tokio::test]
    async fn test_constraint_sql_parsing_edge_cases() {
        let db = D1Client::new_in_memory().await.unwrap();
        
        // Create table with complex constraint expressions
        let sql = r#"
            CREATE TABLE test_complex (
                id INTEGER PRIMARY KEY,
                data TEXT,
                metadata TEXT,
                CHECK (json_valid(data) AND length(data) > 0),
                CONSTRAINT meta_check CHECK (metadata IS NULL OR (json_valid(metadata) AND json_extract(metadata, '$.version') IS NOT NULL))
            )
        "#;
        db.execute(sql, &[]).await.unwrap();

        let introspector = SchemaIntrospector::new(&db);
        let constraints = introspector.introspect_table_constraints("test_complex").await.unwrap();
        
        let check_constraints: Vec<_> = constraints.iter()
            .filter(|c| matches!(c.constraint_type, ConstraintType::Check))
            .collect();
        
        assert_eq!(check_constraints.len(), 2, "Should parse complex CHECK constraints");
        
        // Verify complex expressions are preserved
        let complex_check = check_constraints.iter()
            .find(|c| c.definition.contains("json_extract"))
            .expect("Should find constraint with json_extract");
        assert_eq!(complex_check.name, "meta_check");
    }

    #[tokio::test]
    async fn test_constraint_parsing_with_quoted_identifiers() {
        let db = D1Client::new_in_memory().await.unwrap();
        
        // Create table with quoted identifiers in constraints
        let sql = r#"
            CREATE TABLE "test quoted" (
                "user id" INTEGER PRIMARY KEY,
                "user name" TEXT,
                "user email" TEXT,
                CHECK ("user name" IS NOT NULL AND length("user name") > 0),
                UNIQUE ("user name", "user email")
            )
        "#;
        db.execute(sql, &[]).await.unwrap();

        let introspector = SchemaIntrospector::new(&db);
        let constraints = introspector.introspect_table_constraints("test quoted").await.unwrap();
        
        assert!(!constraints.is_empty(), "Should parse constraints with quoted identifiers");
        
        // Verify quoted identifiers are preserved in constraint definitions
        let check_constraint = constraints.iter()
            .find(|c| matches!(c.constraint_type, ConstraintType::Check))
            .expect("Should find CHECK constraint");
        assert!(check_constraint.definition.contains("\"user name\""));
        
        let unique_constraint = constraints.iter()
            .find(|c| matches!(c.constraint_type, ConstraintType::Unique))
            .expect("Should find UNIQUE constraint");
        assert!(unique_constraint.definition.contains("\"user name\""));
        assert!(unique_constraint.definition.contains("\"user email\""));
    }
}