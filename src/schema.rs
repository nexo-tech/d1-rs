use crate::{D1Client, Result};
use crate::migrations::Migration;
use async_trait::async_trait;

/// Modern, type-safe column types for D1 ORM schema definitions
#[derive(Debug, Clone, PartialEq)]
pub enum ColumnType {
    /// INTEGER - 64-bit signed integer
    Integer,
    /// TEXT - UTF-8 string
    Text,
    /// REAL - Floating point number
    Real,
    /// BLOB - Binary data
    Blob,
    /// BOOLEAN - Stored as INTEGER (0/1) but with proper type metadata
    Boolean,
    /// DATETIME - ISO 8601 datetime string
    DateTime,
    /// JSON - Text field optimized for JSON data
    Json,
}

impl ColumnType {
    /// Convert to SQL type string for database creation
    pub fn to_sql(&self) -> &'static str {
        match self {
            ColumnType::Integer => "INTEGER",
            ColumnType::Text => "TEXT", 
            ColumnType::Real => "REAL",
            ColumnType::Blob => "BLOB",
            ColumnType::Boolean => "INTEGER", // Boolean stored as INTEGER in SQLite/D1
            ColumnType::DateTime => "DATETIME",
            ColumnType::Json => "TEXT",
        }
    }
    
    /// Check if this column type represents a boolean field
    pub fn is_boolean(&self) -> bool {
        matches!(self, ColumnType::Boolean)
    }
}

/// Column constraints and modifiers
#[derive(Debug, Clone)]
pub struct ColumnConstraints {
    pub primary_key: bool,
    pub not_null: bool,
    pub unique: bool,
    pub autoincrement: bool,
    pub default: Option<DefaultValue>,
    pub check: Option<String>,
    pub references: Option<ForeignKey>,
}

impl Default for ColumnConstraints {
    fn default() -> Self {
        Self {
            primary_key: false,
            not_null: false,
            unique: false,
            autoincrement: false,
            default: None,
            check: None,
            references: None,
        }
    }
}

/// Type-safe default values
#[derive(Debug, Clone)]
pub enum DefaultValue {
    Integer(i64),
    Text(String),
    Real(f64),
    Boolean(bool),
    CurrentTimestamp,
    Null,
    Expression(String), // For custom SQL expressions
}

impl DefaultValue {
    pub fn to_sql(&self) -> String {
        match self {
            DefaultValue::Integer(i) => i.to_string(),
            DefaultValue::Text(s) => format!("'{}'", s.replace('\'', "''")),
            DefaultValue::Real(f) => f.to_string(),
            DefaultValue::Boolean(b) => if *b { "1" } else { "0" }.to_string(),
            DefaultValue::CurrentTimestamp => "CURRENT_TIMESTAMP".to_string(),
            DefaultValue::Null => "NULL".to_string(),
            DefaultValue::Expression(expr) => expr.clone(),
        }
    }
}

/// Foreign key constraint definition
#[derive(Debug, Clone)]
pub struct ForeignKey {
    pub table: String,
    pub column: String,
    pub on_delete: Option<ForeignKeyAction>,
    pub on_update: Option<ForeignKeyAction>,
}

#[derive(Debug, Clone)]
pub enum ForeignKeyAction {
    Cascade,
    SetNull,
    SetDefault,
    Restrict,
    NoAction,
}

impl ForeignKeyAction {
    pub fn to_sql(&self) -> &'static str {
        match self {
            ForeignKeyAction::Cascade => "CASCADE",
            ForeignKeyAction::SetNull => "SET NULL",
            ForeignKeyAction::SetDefault => "SET DEFAULT",
            ForeignKeyAction::Restrict => "RESTRICT",
            ForeignKeyAction::NoAction => "NO ACTION",
        }
    }
}

/// Modern column definition with type safety
#[derive(Debug, Clone)]
pub struct Column {
    pub name: String,
    pub column_type: ColumnType,
    pub constraints: ColumnConstraints,
}

impl Column {
    pub fn new(name: impl Into<String>, column_type: ColumnType) -> Self {
        Self {
            name: name.into(),
            column_type,
            constraints: ColumnConstraints::default(),
        }
    }

    /// Generate SQL definition for this column
    pub fn to_sql(&self) -> String {
        let mut sql = format!("{} {}", self.name, self.column_type.to_sql());

        if self.constraints.primary_key {
            sql.push_str(" PRIMARY KEY");
            if self.constraints.autoincrement {
                sql.push_str(" AUTOINCREMENT");
            }
        }

        if self.constraints.not_null && !self.constraints.primary_key {
            sql.push_str(" NOT NULL");
        }

        if self.constraints.unique && !self.constraints.primary_key {
            sql.push_str(" UNIQUE");
        }

        if let Some(ref default) = self.constraints.default {
            sql.push_str(&format!(" DEFAULT {}", default.to_sql()));
        }

        if let Some(ref check) = self.constraints.check {
            sql.push_str(&format!(" CHECK ({})", check));
        }

        if let Some(ref fk) = self.constraints.references {
            sql.push_str(&format!(" REFERENCES {}({})", fk.table, fk.column));
            
            if let Some(ref on_delete) = fk.on_delete {
                sql.push_str(&format!(" ON DELETE {}", on_delete.to_sql()));
            }
            
            if let Some(ref on_update) = fk.on_update {
                sql.push_str(&format!(" ON UPDATE {}", on_update.to_sql()));
            }
        }

        sql
    }
}

/// Modern table definition builder with fluent API
#[derive(Debug, Clone)]
pub struct TableDefinition {
    pub name: String,
    pub columns: Vec<Column>,
    pub indexes: Vec<Index>,
}

/// Index definition for performance optimization
#[derive(Debug, Clone)]
pub struct Index {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
}

impl TableDefinition {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            columns: Vec::new(),
            indexes: Vec::new(),
        }
    }

    /// Add a column with fluent builder pattern
    pub fn add_column(self, name: impl Into<String>, column_type: ColumnType) -> SchemaColumnBuilder {
        let column = Column::new(name, column_type);
        SchemaColumnBuilder::new(self, column)
    }
    
    /// Convenience method for integer column
    pub fn integer(self, name: impl Into<String>) -> SchemaColumnBuilder {
        self.add_column(name, ColumnType::Integer)
    }
    
    /// Convenience method for text column
    pub fn text(self, name: impl Into<String>) -> SchemaColumnBuilder {
        self.add_column(name, ColumnType::Text)
    }
    
    /// Convenience method for boolean column (creates INTEGER with metadata)
    pub fn boolean(self, name: impl Into<String>) -> SchemaColumnBuilder {
        self.add_column(name, ColumnType::Boolean)
    }
    
    /// Convenience method for datetime column
    pub fn datetime(self, name: impl Into<String>) -> SchemaColumnBuilder {
        self.add_column(name, ColumnType::DateTime)
    }
    
    /// Convenience method for JSON column
    pub fn json(self, name: impl Into<String>) -> SchemaColumnBuilder {
        self.add_column(name, ColumnType::Json)
    }

    /// Add an index to the table
    pub fn add_index(mut self, name: impl Into<String>, columns: Vec<String>, unique: bool) -> Self {
        self.indexes.push(Index {
            name: name.into(),
            columns,
            unique,
        });
        self
    }
    
    /// Create table definition from column definitions (for migrations)
    pub fn from_columns(name: String, columns: Vec<crate::migrations::ColumnDefinition>) -> Self {
        let mut table = Self::new(name);
        for col in columns {
            // Convert string column type to ColumnType enum
            let column_type = match col.column_type.to_uppercase().as_str() {
                "INTEGER" => ColumnType::Integer,
                "TEXT" => ColumnType::Text,
                "REAL" => ColumnType::Real,
                "BLOB" => ColumnType::Blob,
                "BOOLEAN" => ColumnType::Boolean,
                "DATETIME" => ColumnType::DateTime,
                "JSON" => ColumnType::Json,
                _ => ColumnType::Text, // Default fallback
            };
            
            let mut column = Column::new(col.name.clone(), column_type.clone());
            column.constraints.not_null = !col.nullable;
            
            // Convert string default to DefaultValue enum
            column.constraints.default = col.default.map(|default_str| {
                // Try to infer the type based on the column type
                match column_type {
                    ColumnType::Integer => {
                        if let Ok(int_val) = default_str.parse::<i64>() {
                            DefaultValue::Integer(int_val)
                        } else {
                            DefaultValue::Expression(default_str)
                        }
                    },
                    ColumnType::Real => {
                        if let Ok(float_val) = default_str.parse::<f64>() {
                            DefaultValue::Real(float_val)
                        } else {
                            DefaultValue::Expression(default_str)
                        }
                    },
                    ColumnType::Boolean => {
                        if let Ok(bool_val) = default_str.parse::<bool>() {
                            DefaultValue::Boolean(bool_val)
                        } else {
                            DefaultValue::Expression(default_str)
                        }
                    },
                    _ => DefaultValue::Text(default_str),
                }
            });
            
            column.constraints.unique = col.unique;
            column.constraints.primary_key = col.primary_key;
            // Note: ColumnDefinition doesn't have auto_increment field, so we skip it
            table.columns.push(column);
        }
        table
    }

    /// Generate CREATE TABLE SQL
    pub fn to_sql(&self) -> String {
        let mut sql = format!("CREATE TABLE {} (", self.name);

        for (idx, column) in self.columns.iter().enumerate() {
            if idx > 0 {
                sql.push_str(", ");
            }
            sql.push_str(&column.to_sql());
        }

        sql.push(')');
        sql
    }
    
    /// Get all boolean columns in this table for metadata
    pub fn boolean_columns(&self) -> Vec<&str> {
        self.columns
            .iter()
            .filter(|col| col.column_type.is_boolean())
            .map(|col| col.name.as_str())
            .collect()
    }
}

/// Fluent builder for column constraints
pub struct SchemaColumnBuilder {
    table: TableDefinition,
    column: Column,
}

impl SchemaColumnBuilder {
    fn new(table: TableDefinition, column: Column) -> Self {
        Self { table, column }
    }

    /// Make this column a primary key
    pub fn primary_key(mut self) -> Self {
        self.column.constraints.primary_key = true;
        self.column.constraints.not_null = true; // Primary keys are always NOT NULL
        self
    }

    /// Make this column auto-increment (only valid for INTEGER primary keys)
    pub fn auto_increment(mut self) -> Self {
        self.column.constraints.autoincrement = true;
        self
    }

    /// Make this column NOT NULL
    pub fn not_null(mut self) -> Self {
        self.column.constraints.not_null = true;
        self
    }

    /// Make this column UNIQUE
    pub fn unique(mut self) -> Self {
        self.column.constraints.unique = true;
        self
    }

    /// Set a default value for this column
    pub fn default(mut self, value: DefaultValue) -> Self {
        self.column.constraints.default = Some(value);
        self
    }
    
    /// Convenience method for boolean default
    pub fn default_false(self) -> Self {
        self.default(DefaultValue::Boolean(false))
    }
    
    /// Convenience method for boolean default
    pub fn default_true(self) -> Self {
        self.default(DefaultValue::Boolean(true))
    }

    /// Add a CHECK constraint
    pub fn check(mut self, constraint: impl Into<String>) -> Self {
        self.column.constraints.check = Some(constraint.into());
        self
    }

    /// Add a foreign key reference
    pub fn references(mut self, table: impl Into<String>, column: impl Into<String>) -> Self {
        self.column.constraints.references = Some(ForeignKey {
            table: table.into(),
            column: column.into(),
            on_delete: None,
            on_update: None,
        });
        self
    }

    /// Set ON DELETE action for foreign key
    pub fn on_delete(mut self, action: ForeignKeyAction) -> Self {
        if let Some(ref mut fk) = self.column.constraints.references {
            fk.on_delete = Some(action);
        }
        self
    }

    /// Set ON UPDATE action for foreign key
    pub fn on_update(mut self, action: ForeignKeyAction) -> Self {
        if let Some(ref mut fk) = self.column.constraints.references {
            fk.on_update = Some(action);
        }
        self
    }

    /// Add another column to the table
    pub fn add_column(mut self, name: impl Into<String>, column_type: ColumnType) -> SchemaColumnBuilder {
        self.table.columns.push(self.column);
        let column = Column::new(name, column_type);
        SchemaColumnBuilder::new(self.table, column)
    }
    
    /// Convenience methods for chaining more columns
    pub fn integer(self, name: impl Into<String>) -> SchemaColumnBuilder {
        self.add_column(name, ColumnType::Integer)
    }
    
    pub fn text(self, name: impl Into<String>) -> SchemaColumnBuilder {
        self.add_column(name, ColumnType::Text)
    }
    
    pub fn boolean(self, name: impl Into<String>) -> SchemaColumnBuilder {
        self.add_column(name, ColumnType::Boolean)
    }
    
    pub fn datetime(self, name: impl Into<String>) -> SchemaColumnBuilder {
        self.add_column(name, ColumnType::DateTime)
    }
    
    pub fn json(self, name: impl Into<String>) -> SchemaColumnBuilder {
        self.add_column(name, ColumnType::Json)
    }

    /// Finish building and return the complete table definition
    pub fn build(mut self) -> TableDefinition {
        self.table.columns.push(self.column);
        self.table
    }
}

/// Modern migration with type-safe schema builder
pub struct SchemaMigration {
    name: &'static str,
    version: i64,
    operation: MigrationOperation,
}

/// Builder for creating multiple table migrations in a single batch
pub struct MigrationBuilder {
    name: String,
    base_version: i64,
    current_table: Option<TableDefinition>,
    migrations: Vec<SchemaMigration>,
}

impl MigrationBuilder {
    pub fn new(name: String) -> Self {
        Self {
            name,
            base_version: 1,
            current_table: None,
            migrations: Vec::new(),
        }
    }
    
    /// Start creating a new table
    pub fn create_table(mut self, table_name: &str) -> TableDefinitionBuilder {
        // If we have a current table being built, finish it first
        if let Some(table) = self.current_table.take() {
            let migration_name = Box::leak(format!("create_{}_table", table.name).into_boxed_str());
            let migration = SchemaMigration::create_table(migration_name, self.base_version, table);
            self.migrations.push(migration);
            self.base_version += 1;
        }
        
        TableDefinitionBuilder::new(self, table_name)
    }
    
    /// Auto-generate table for an Entity type (backward compatibility)
    pub fn auto_generate_for<T: crate::Entity>(self) -> Self {
        // This is a no-op for backward compatibility
        // The actual table creation is handled by the explicit create_table calls
        self
    }
    
    /// Add edge relationship (backward compatibility)
    pub fn add_edge<Parent: crate::Entity, Child: crate::Entity>(self) -> Self {
        // This is a no-op for backward compatibility
        // The actual relationships are defined in the Entity structs
        self
    }
    
    /// Specify one-to-many relationship (backward compatibility)
    pub fn one_to_many(self) -> Self {
        // This is a no-op for backward compatibility
        // The actual relationships are defined in the Entity structs
        self
    }
    
    /// Specify many-to-one relationship (backward compatibility)
    pub fn many_to_one(self) -> Self {
        // This is a no-op for backward compatibility
        self
    }
    
    /// Specify one-to-one relationship (backward compatibility)
    pub fn one_to_one(self) -> Self {
        // This is a no-op for backward compatibility
        self
    }
    
    /// Specify many-to-many relationship (backward compatibility)
    pub fn many_to_many(self) -> Self {
        // This is a no-op for backward compatibility
        self
    }
    
    /// Build and return the migration (backward compatibility)
    pub fn build(self) -> Self {
        // Just return self - the actual execution happens with execute()
        self
    }
    
    /// Execute all migrations in sequence
    pub async fn execute(mut self, db: &crate::D1Client) -> crate::Result<()> {
        // Finish any pending table
        if let Some(table) = self.current_table.take() {
            let migration_name = Box::leak(format!("create_{}_table", table.name).into_boxed_str());
            let migration = SchemaMigration::create_table(migration_name, self.base_version, table);
            self.migrations.push(migration);
        }
        
        // Execute all migrations
        for migration in self.migrations {
            migration.execute(db).await?;
        }
        
        Ok(())
    }
}

/// Builder for table definitions within a migration
pub struct TableDefinitionBuilder {
    migration_builder: MigrationBuilder,
    table_definition: TableDefinition,
    current_column: Option<Column>,
}

impl TableDefinitionBuilder {
    fn new(migration_builder: MigrationBuilder, table_name: &str) -> Self {
        Self {
            migration_builder,
            table_definition: TableDefinition::new(table_name),
            current_column: None,
        }
    }
    
    /// Add an integer column
    pub fn integer(mut self, name: &str) -> ColumnBuilder {
        self.finish_current_column();
        ColumnBuilder::new(self, Column::new(name, ColumnType::Integer))
    }
    
    /// Add a text column
    pub fn text(mut self, name: &str) -> ColumnBuilder {
        self.finish_current_column();
        ColumnBuilder::new(self, Column::new(name, ColumnType::Text))
    }
    
    /// Add a boolean column
    pub fn boolean(mut self, name: &str) -> ColumnBuilder {
        self.finish_current_column();
        ColumnBuilder::new(self, Column::new(name, ColumnType::Boolean))
    }
    
    /// Add a datetime column
    pub fn datetime(mut self, name: &str) -> ColumnBuilder {
        self.finish_current_column();
        ColumnBuilder::new(self, Column::new(name, ColumnType::DateTime))
    }
    
    fn finish_current_column(&mut self) {
        if let Some(column) = self.current_column.take() {
            self.table_definition.columns.push(column);
        }
    }
    
    /// Finish building this table and return to migration builder
    pub fn build(mut self) -> MigrationBuilder {
        self.finish_current_column();
        self.migration_builder.current_table = Some(self.table_definition);
        self.migration_builder
    }
}

/// Builder for individual columns within a table
pub struct ColumnBuilder {
    table_builder: TableDefinitionBuilder,
    column: Column,
}

impl ColumnBuilder {
    fn new(table_builder: TableDefinitionBuilder, column: Column) -> Self {
        Self { table_builder, column }
    }
    
    /// Make this column a primary key
    pub fn primary_key(mut self) -> Self {
        self.column.constraints.primary_key = true;
        self.column.constraints.not_null = true;
        self
    }
    
    /// Make this column auto increment
    pub fn auto_increment(mut self) -> Self {
        self.column.constraints.autoincrement = true;
        self
    }
    
    /// Make this column not null
    pub fn not_null(mut self) -> Self {
        self.column.constraints.not_null = true;
        self
    }
    
    /// Make this column unique
    pub fn unique(mut self) -> Self {
        self.column.constraints.unique = true;
        self
    }
    
    /// Set default value
    pub fn default_value(mut self, value: DefaultValue) -> Self {
        self.column.constraints.default = Some(value);
        self
    }
    
    /// Finish building this column
    pub fn build(mut self) -> TableDefinitionBuilder {
        self.table_builder.current_column = Some(self.column);
        self.table_builder
    }
}

#[derive(Debug)]
pub enum MigrationOperation {
    CreateTable(TableDefinition),
    DropTable(String),
    AddColumn { table: String, column: Column },
    DropColumn { table: String, column: String },
    CreateIndex(String, Index), // table_name, index
    DropIndex(String), // index_name
}

impl SchemaMigration {
    /// Create a new migration builder for chaining multiple operations
    pub fn new(name: String) -> MigrationBuilder {
        MigrationBuilder::new(name)
    }
    
    pub fn create_table(name: &'static str, version: i64, table_definition: TableDefinition) -> Self {
        Self {
            name,
            version,
            operation: MigrationOperation::CreateTable(table_definition),
        }
    }

    pub fn drop_table(name: &'static str, version: i64, table_name: String) -> Self {
        Self {
            name,
            version,
            operation: MigrationOperation::DropTable(table_name),
        }
    }

    pub fn add_column(name: &'static str, version: i64, table: String, column: Column) -> Self {
        Self {
            name,
            version,
            operation: MigrationOperation::AddColumn { table, column },
        }
    }
    
    /// Execute this single migration
    pub async fn execute(&self, db: &crate::D1Client) -> crate::Result<()> {
        self.up(db).await
    }
}

#[async_trait(?Send)]
impl crate::migrations::Migration for SchemaMigration {
    fn name(&self) -> &'static str {
        self.name
    }

    fn version(&self) -> i64 {
        self.version
    }

    async fn up(&self, db: &D1Client) -> Result<()> {
        match &self.operation {
            MigrationOperation::CreateTable(table) => {
                let sql = table.to_sql();
                db.execute(&sql, &[]).await?;
                
                // Create indexes
                for index in &table.indexes {
                    let index_sql = if index.unique {
                        format!(
                            "CREATE UNIQUE INDEX {} ON {} ({})",
                            index.name,
                            table.name,
                            index.columns.join(", ")
                        )
                    } else {
                        format!(
                            "CREATE INDEX {} ON {} ({})",
                            index.name,
                            table.name,
                            index.columns.join(", ")
                        )
                    };
                    db.execute(&index_sql, &[]).await?;
                }
            }
            MigrationOperation::DropTable(table_name) => {
                let sql = format!("DROP TABLE IF EXISTS {}", table_name);
                db.execute(&sql, &[]).await?;
            }
            MigrationOperation::AddColumn { table, column } => {
                let sql = format!("ALTER TABLE {} ADD COLUMN {}", table, column.to_sql());
                db.execute(&sql, &[]).await?;
            }
            MigrationOperation::DropColumn { table: _, column: _ } => {
                // SQLite doesn't support DROP COLUMN directly, need to recreate table
                // For now, just error - this is a complex operation
                return Err(crate::D1RsError::Database(
                    "DROP COLUMN not supported yet - requires table recreation".to_string()
                ));
            }
            MigrationOperation::CreateIndex(table_name, index) => {
                let sql = if index.unique {
                    format!(
                        "CREATE UNIQUE INDEX {} ON {} ({})",
                        index.name, table_name, index.columns.join(", ")
                    )
                } else {
                    format!(
                        "CREATE INDEX {} ON {} ({})",
                        index.name, table_name, index.columns.join(", ")
                    )
                };
                db.execute(&sql, &[]).await?;
            }
            MigrationOperation::DropIndex(index_name) => {
                let sql = format!("DROP INDEX IF EXISTS {}", index_name);
                db.execute(&sql, &[]).await?;
            }
        }
        Ok(())
    }

    async fn down(&self, db: &D1Client) -> Result<()> {
        match &self.operation {
            MigrationOperation::CreateTable(table) => {
                let sql = format!("DROP TABLE IF EXISTS {}", table.name);
                db.execute(&sql, &[]).await?;
            }
            MigrationOperation::DropTable(_table_name) => {
                // Can't easily reverse a DROP TABLE without knowing the schema
                return Err(crate::D1RsError::Database(
                    "Cannot reverse DROP TABLE - original schema unknown".to_string()
                ));
            }
            MigrationOperation::AddColumn { table: _, column: _ } => {
                // SQLite doesn't support DROP COLUMN directly
                return Err(crate::D1RsError::Database(
                    "Cannot reverse ADD COLUMN - requires table recreation".to_string()
                ));
            }
            MigrationOperation::DropColumn { table: _, column } => {
                // Can't reverse without knowing original column definition
                return Err(crate::D1RsError::Database(
                    format!("Cannot reverse DROP COLUMN {} - original definition unknown", column)
                ));
            }
            MigrationOperation::CreateIndex(_table_name, index) => {
                let sql = format!("DROP INDEX IF EXISTS {}", index.name);
                db.execute(&sql, &[]).await?;
            }
            MigrationOperation::DropIndex(index_name) => {
                // Can't reverse without knowing original index definition
                return Err(crate::D1RsError::Database(
                    format!("Cannot reverse DROP INDEX {} - original definition unknown", index_name)
                ));
            }
        }
        Ok(())
    }
}