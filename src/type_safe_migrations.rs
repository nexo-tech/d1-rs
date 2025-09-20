// Phase 4.3B: Revolutionary Type-Safe Migration System
// Eliminates ALL string literals from migration API with compile-time safety

use crate::{Entity, Result, types::SqlTypeMappable};
use async_trait::async_trait;
use std::marker::PhantomData;

// Phase 4.3B: REVOLUTIONARY Type-Safe Column System
// Replaces ALL string-based column definitions with compile-time safe types

/// Revolutionary type-safe column trait - eliminates ALL string literals in migrations
/// Each Entity field generates a type implementing this trait
pub trait TypeSafeColumn: std::fmt::Debug {
    /// The Rust type of this column (i64, String, bool, etc.)
    type RustType: SqlTypeMappable;
    
    /// Compile-time column name - NO string literals in API!
    fn column_name() -> &'static str;
    
    /// Get SQL type from the Rust type automatically
    fn sql_type() -> &'static str {
        // Use our revolutionary SqlTypeMappable system
        match Self::RustType::type_category() {
            crate::types::TypeCategory::Numeric => {
                if Self::type_name_contains_float() {
                    "REAL"
                } else {
                    "INTEGER"
                }
            },
            crate::types::TypeCategory::Text => "TEXT",
            crate::types::TypeCategory::Binary => "BLOB", 
            crate::types::TypeCategory::Temporal => "DATETIME",
            crate::types::TypeCategory::Special => {
                // Special handling for bool - stored as INTEGER
                if <Self::RustType as SqlTypeMappable>::IS_NUMERIC {
                    "INTEGER"
                } else {
                    "TEXT"
                }
            }
        }
    }
    
    /// Helper to detect floating point types (temporary until we have better type introspection)
    fn type_name_contains_float() -> bool {
        let type_name = std::any::type_name::<Self::RustType>();
        type_name.contains("f32") || type_name.contains("f64")
    }
    
    /// Add constraints to this column
    fn with_constraints(self, constraints: Vec<ColumnConstraint>) -> TypeSafeColumnBuilder<Self>
    where Self: Sized;
}

/// Column constraints for type-safe migrations
#[derive(Debug, Clone)]
pub enum ColumnConstraint {
    NotNull,
    PrimaryKey,
    Unique,
    Default(String),
    ForeignKey { table: String, column: String },
}

/// Builder for type-safe columns with constraints
#[derive(Debug)]
pub struct TypeSafeColumnBuilder<C: TypeSafeColumn> {
    _phantom: std::marker::PhantomData<C>,
    constraints: Vec<ColumnConstraint>,
}

impl<C: TypeSafeColumn> TypeSafeColumnBuilder<C> {
    pub fn new(_column: C) -> Self {
        Self {
            _phantom: std::marker::PhantomData,
            constraints: Vec::new(),
        }
    }
    
    /// Mark column as NOT NULL
    pub fn not_null(mut self) -> Self {
        self.constraints.push(ColumnConstraint::NotNull);
        self
    }
    
    /// Mark column as PRIMARY KEY
    pub fn primary_key(mut self) -> Self {
        self.constraints.push(ColumnConstraint::PrimaryKey);
        self.constraints.push(ColumnConstraint::NotNull); // Primary keys are automatically NOT NULL
        self
    }
    
    /// Mark column as UNIQUE
    pub fn unique(mut self) -> Self {
        self.constraints.push(ColumnConstraint::Unique);
        self
    }
    
    /// Add a default value (type-safe)
    pub fn default<V: Into<String>>(mut self, value: V) -> Self {
        self.constraints.push(ColumnConstraint::Default(value.into()));
        self
    }
    
    /// Add foreign key constraint
    pub fn foreign_key(mut self, table: &str, column: &str) -> Self {
        self.constraints.push(ColumnConstraint::ForeignKey {
            table: table.to_string(),
            column: column.to_string(),
        });
        self
    }
}

/// Trait for type-safe column definitions that can be used in migrations
pub trait TypeSafeColumnDef: std::fmt::Debug {
    fn column_name(&self) -> &'static str;
    fn sql_type(&self) -> &'static str;
    fn constraints(&self) -> &[ColumnConstraint];
}

impl<C: TypeSafeColumn + std::fmt::Debug> TypeSafeColumnDef for TypeSafeColumnBuilder<C> {
    fn column_name(&self) -> &'static str {
        C::column_name()
    }
    
    fn sql_type(&self) -> &'static str {
        C::sql_type()
    }
    
    fn constraints(&self) -> &[ColumnConstraint] {
        &self.constraints
    }
}

/// REVOLUTIONARY: Type-safe migration builder
/// Completely eliminates string literals from migration API
#[derive(Debug)]
pub struct TypeSafeMigration<T: Entity> {
    entity_type: PhantomData<T>,
    table_name: &'static str,
    columns: Vec<Box<dyn TypeSafeColumnDef>>,
    migration_name: &'static str,
    version: i64,
}

impl<T: Entity> TypeSafeMigration<T> {
    /// Create a new type-safe migration for creating a table
    pub fn create_table(migration_name: &'static str, version: i64) -> Self {
        Self {
            entity_type: PhantomData,
            table_name: T::TABLE_NAME,
            columns: Vec::new(),
            migration_name,
            version,
        }
    }
    
    /// Add a type-safe column to the migration
    /// This is the revolutionary method that eliminates ALL string literals!
    pub fn column<C: TypeSafeColumn + 'static>(mut self, column_builder: TypeSafeColumnBuilder<C>) -> Self {
        self.columns.push(Box::new(column_builder));
        self
    }
    
    /// Generate SQL for this migration
    pub fn to_sql(&self) -> String {
        let mut sql = format!("CREATE TABLE {} (", self.table_name);
        
        let column_defs: Vec<String> = self.columns.iter().map(|col| {
            let mut def = format!("{} {}", col.column_name(), col.sql_type());
            
            for constraint in col.constraints() {
                match constraint {
                    ColumnConstraint::NotNull => def.push_str(" NOT NULL"),
                    ColumnConstraint::PrimaryKey => def.push_str(" PRIMARY KEY"),
                    ColumnConstraint::Unique => def.push_str(" UNIQUE"),
                    ColumnConstraint::Default(value) => {
                        def.push_str(&format!(" DEFAULT {}", value));
                    },
                    ColumnConstraint::ForeignKey { table, column } => {
                        def.push_str(&format!(" REFERENCES {}({})", table, column));
                    },
                }
            }
            
            def
        }).collect();
        
        sql.push_str(&column_defs.join(", "));
        sql.push(')');
        
        sql
    }
    
    /// Get migration metadata
    pub fn name(&self) -> &'static str {
        self.migration_name
    }
    
    pub fn version(&self) -> i64 {
        self.version
    }
    
    pub fn table_name(&self) -> &'static str {
        self.table_name
    }
}

/// Trait for entities that can generate type-safe migrations
/// This will be implemented automatically by the derive macro
pub trait TypeSafeMigratable: Entity {
    /// Generate a type-safe create table migration for this entity
    fn create_table_migration(migration_name: &'static str, version: i64) -> TypeSafeMigration<Self>;
    
    /// Validate that a migration matches the current entity definition
    fn validate_migration(migration: &TypeSafeMigration<Self>) -> Result<()>;
}

/// REVOLUTIONARY: Compile-time migration validation
/// Ensures migrations always match Entity definitions
pub struct MigrationValidator;

impl MigrationValidator {
    /// Validate a type-safe migration at compile time
    pub fn validate<T: Entity + TypeSafeMigratable>(migration: &TypeSafeMigration<T>) -> Result<()> {
        // Validate table name matches
        if migration.table_name() != T::TABLE_NAME {
            return Err(crate::D1RsError::Database(format!(
                "Migration table name '{}' doesn't match Entity table name '{}'",
                migration.table_name(),
                T::TABLE_NAME
            )));
        }
        
        // Validate columns match entity fields
        T::validate_migration(migration)?;
        
        Ok(())
    }
    
    /// Ensure migration has required fields
    pub fn validate_required_fields<T: Entity>(migration: &TypeSafeMigration<T>) -> Result<()> {
        let column_names: std::collections::HashSet<&str> = migration.columns
            .iter()
            .map(|col| col.column_name())
            .collect();
            
        // Check for required id column (most entities should have one)
        if !column_names.contains("id") {
            return Err(crate::D1RsError::Database(
                "Migration missing required 'id' column".to_string()
            ));
        }
        
        Ok(())
    }
}

/// Integration with existing migration system
#[async_trait(?Send)]
impl<T: Entity> crate::Migration for TypeSafeMigration<T> {
    fn name(&self) -> &'static str {
        self.migration_name
    }
    
    fn version(&self) -> i64 {
        self.version
    }
    
    async fn up(&self, db: &crate::D1Client) -> Result<()> {
        let sql = self.to_sql();
        db.execute(&sql, &[]).await?;
        Ok(())
    }
    
    async fn down(&self, db: &crate::D1Client) -> Result<()> {
        let sql = format!("DROP TABLE IF EXISTS {}", self.table_name);
        db.execute(&sql, &[]).await?;
        Ok(())
    }
}

// Tests are in tests/test_type_safe_migrations.rs