pub use d1_rs_derive::*;

pub mod db;
pub mod query;
pub mod query_builder;
pub mod entity;
pub mod migrations;
pub mod types;
pub mod schema;
pub mod relations;
pub mod edges;
pub mod schema_evolution;
pub mod auto_migration;
pub mod type_safe_migrations;
pub mod dialects;
pub mod backends;
pub mod introspection;
pub mod migration_engine;

pub use db::*;
pub use query::*;
pub use query_builder::{QueryRenderer, SeaQueryExecutor, json_to_sea_value, sea_value_to_json};
// Specific exports from migrations to avoid conflicts
pub use migrations::{MigrationRunner, CreateTableMigration, Migration};
pub use types::*;
pub use schema::*;
pub use dialects::DatabaseDialect;
pub use backends::{QueryResult, BackendError, DatabaseBackend};
// Introspection module for internal use - exported in Tasks 4.2-4.4 when implementations are ready
// pub use introspection::{SchemaIntrospector, IntrospectionError, IntrospectionErrorKind};

// Convenient type aliases for common database clients
pub type SQLiteClient = crate::db::DatabaseClient<crate::backends::SQLiteBackend>;

#[cfg(feature = "postgres")]
pub type PostgreSQLClient = crate::db::DatabaseClient<crate::backends::PostgreSQLBackend>;

#[cfg(feature = "mysql")]
pub type MySQLClient = crate::db::DatabaseClient<crate::backends::MySQLBackend>;

// Maintain compatibility during transition - D1Client now points to SQLiteClient
pub type D1Client = SQLiteClient;
// pub use relations::*; // Unused module
pub use edges::*;
// Phase 4.3C: Revolutionary schema evolution exports
pub use schema_evolution::{
    TypeSafeSchema, TypeSafeColumnSchema, AutoMigrationPlanner, 
    TypeSafeSchemaDiff, TypeSafeColumnModification, TypeSafeTableChange,
    ColumnChangeType, IndexOperation, TypeSafeSchemaDiffer
};
// pub use auto_migration::*; // Selective exports to avoid conflicts
// Phase 4.3B: Type-safe migrations exports
pub use type_safe_migrations::*;

// Phase 5.1: Database-agnostic migration engine exports
pub use migration_engine::{
    SchemaDiffer, MigrationPlan, MigrationOperation, SafetyLevel, 
    SchemaError, MigrationResult, TableOperation, ColumnOperation,
    IndexOperation as MigrationIndexOperation, ConstraintOperation
};

pub use async_trait::async_trait;
use std::fmt;

#[cfg(target_arch = "wasm32")]
use worker::d1::D1Database;

/// Revolutionary field definition for advanced EntityAnalyzer
#[derive(Debug, Clone)]
pub struct FieldDefinition {
    pub name: String,
    pub field_type: FieldType,
    pub nullable: bool,
    pub primary_key: bool,
    pub auto_increment: bool,
    pub default_value: Option<String>,
    pub foreign_key: Option<ForeignKeyDefinition>,
}

/// Comprehensive field type mapping for schema generation
#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    Integer,
    BigInteger,
    Text,
    Boolean,
    Real,
    DateTime,
    Date,
    Time,
    Json,
    Blob,
}

impl FieldType {
    /// Convert FieldType to SQL type string
    pub fn to_sql_type(&self) -> &'static str {
        match self {
            FieldType::Integer => "INTEGER",
            FieldType::BigInteger => "BIGINT", 
            FieldType::Text => "TEXT",
            FieldType::Boolean => "BOOLEAN",
            FieldType::Real => "REAL",
            FieldType::DateTime => "DATETIME",
            FieldType::Date => "DATE",
            FieldType::Time => "TIME",
            FieldType::Json => "JSON",
            FieldType::Blob => "BLOB",
        }
    }
}

/// Foreign key relationship definition
#[derive(Debug, Clone)]
pub struct ForeignKeyDefinition {
    pub name: String,
    pub local_column: String,
    pub referenced_table: String,
    pub referenced_column: String,
    pub on_delete: Option<String>,
    pub on_update: Option<String>,
}

/// Recursive relationship information
#[derive(Debug, Clone)]
pub struct RecursiveRelationship {
    pub foreign_key_column: String,
    pub delete_strategy: RecursiveDeletionStrategy,
    pub allows_cycles: bool,
}

/// NEW: Trait for providing compile-time field metadata
/// This replaces all heuristic-based field detection with type-safe trait implementations
pub trait FieldMetadata {
    /// Returns comprehensive field definitions generated at compile time
    fn field_definitions() -> Vec<FieldDefinition>;
    
    /// Returns foreign key relationships defined at compile time
    fn foreign_key_definitions() -> Vec<ForeignKeyDefinition>;
}

/// REVOLUTIONARY: Trait for user-defined custom type conversions
/// Allows users to extend the ORM with ANY custom type mappings
pub trait CustomTypeMapping {
    /// The Rust type this mapping handles
    type RustType;
    
    /// The SQL type string for schema generation
    const SQL_TYPE: &'static str;
    
    /// Convert from Rust type to SQLite value
    fn to_sql_value(value: &Self::RustType) -> serde_json::Value;
    
    /// Convert from SQLite value to Rust type
    fn from_sql_value(value: serde_json::Value) -> Result<Self::RustType>;
    
    /// Optional: Custom field type for schema introspection
    fn field_type() -> FieldType {
        FieldType::Text // Default fallback
    }
}

/// REVOLUTIONARY: Registry for user-defined type mappings
/// This enables complete extensibility without hardcoded limits
pub trait TypeMappingRegistry {
    /// Register a custom type mapping at compile time
    fn register_mapping<T: CustomTypeMapping>() -> FieldType {
        T::field_type()
    }
    
    /// Check if a type has custom mapping defined
    fn has_custom_mapping(type_name: &str) -> bool;
    
    /// Get the field type for a custom mapped type
    fn get_field_type(type_name: &str) -> Option<FieldType>;
}

/// REVOLUTIONARY: SqlTypeMapping trait for Phase 3.3 - Complete user control!
/// This allows users to override ANY aspect of type detection and conversion
pub trait SqlTypeMapping {
    /// The Rust type this mapping applies to
    type RustType: serde::Serialize + serde::de::DeserializeOwned;
    
    /// SQL type string for DDL generation
    const SQL_TYPE: &'static str;
    
    /// Field type for schema introspection
    const FIELD_TYPE: FieldType;
    
    /// Whether this type should be treated as nullable by default
    const NULLABLE_BY_DEFAULT: bool = false;
    
    /// Whether this type should be treated as a boolean field
    const IS_BOOLEAN_TYPE: bool = false;
    
    /// Custom conversion to SQL value
    fn to_sql_value(value: &Self::RustType) -> serde_json::Value {
        // Default implementation using serde
        serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
    }
    
    /// Custom conversion from SQL value
    fn from_sql_value(value: serde_json::Value) -> Result<Self::RustType> {
        // Default implementation using serde
        serde_json::from_value(value)
            .map_err(|e| D1RsError::SerializationError(e.to_string()))
    }
    
    /// Override default type detection completely
    fn override_type_detection() -> bool {
        false // Default: use standard detection
    }
}

/// REVOLUTIONARY: Custom boolean field marking - NO MORE HEURISTICS!
/// Users can mark ANY field as boolean regardless of Rust type
pub trait CustomBooleanFields {
    /// List of field names that should be treated as boolean in SQL
    const BOOLEAN_FIELD_NAMES: &'static [&'static str];
    
    /// Check if a field should be treated as boolean
    fn is_boolean_field(field_name: &str) -> bool {
        Self::BOOLEAN_FIELD_NAMES.contains(&field_name)
    }
}

/// REVOLUTIONARY: Default type detection override system
/// Allows users to completely replace the built-in type detection
pub trait DefaultTypeDetectionOverride {
    /// Override the default field type for any Rust type
    fn override_field_type(rust_type_name: &str) -> Option<FieldType>;
    
    /// Override the default SQL type for any Rust type
    fn override_sql_type(rust_type_name: &str) -> Option<&'static str>;
    
    /// Override the default nullability for any Rust type
    fn override_nullable(rust_type_name: &str) -> Option<bool>;
}

/// NEW: Trait for entities that provide compile-time schema information
/// This enables the EntityAnalyzer to extract complete schema without heuristics
pub trait EntitySchema: Entity {
    /// Get the field metadata provider for this entity
    type FieldMetadata: FieldMetadata;
    
    /// Access to compile-time field definitions
    fn schema() -> Self::FieldMetadata;
}

/// Strategy for handling deletion in recursive relationships
#[derive(Debug, Clone, PartialEq)]
pub enum RecursiveDeletionStrategy {
    SetNull,    // Set parent_id to NULL when parent is deleted
    Cascade,    // Delete all children when parent is deleted
    Restrict,   // Prevent deletion if children exist
}

/// REVOLUTIONARY: Trait for entities with recursive relationships
/// Uses Rust's type system to detect recursive capabilities at compile time
pub trait RecursiveEntity: Entity {
    /// The column that references the parent entity (e.g., "parent_id")
    const PARENT_COLUMN: &'static str;
    
    /// Strategy for handling deletion
    const DELETE_STRATEGY: RecursiveDeletionStrategy = RecursiveDeletionStrategy::SetNull;
    
    /// Whether cycles are allowed in the tree structure
    const ALLOWS_CYCLES: bool = false;
    
    /// Get recursive relationship information
    fn recursive_relationship() -> RecursiveRelationship {
        RecursiveRelationship {
            foreign_key_column: Self::PARENT_COLUMN.to_string(),
            delete_strategy: Self::DELETE_STRATEGY,
            allows_cycles: Self::ALLOWS_CYCLES,
        }
    }
}

/// REVOLUTIONARY: Phase 4.1 - Compile-time relation validation system
/// Ensures all relationships are properly configured and consistent
pub trait RelationValidator {
    /// Validate that all relations are properly defined
    fn validate_relations() -> Result<()>;
    
    /// Check for circular dependencies in relationships
    fn check_circular_dependencies() -> Result<()>;
    
    /// Verify foreign key constraints are consistent
    fn verify_foreign_key_consistency() -> Result<()>;
    
    /// Validate junction table configurations for M2M relations
    fn validate_junction_tables() -> Result<()>;
}

/// REVOLUTIONARY: Enhanced relation error reporting with suggestions
/// Provides detailed context and helpful fixes for relation configuration issues
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RelationValidationError {
    pub entity: String,
    pub relation: String,
    pub issue: RelationIssueType,
    pub suggestion: String,
    pub affected_entities: Vec<String>,
}

/// Types of relation configuration issues that can be detected
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum RelationIssueType {
    MissingInverseRelation,
    InconsistentForeignKey,
    InvalidJunctionTable,
    CircularDependency,
    MissingReferencedEntity,
    TypeMismatchInForeignKey,
    DuplicateRelationDefinition,
}

impl std::fmt::Display for RelationIssueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format_relation_issue(self))
    }
}

/// REVOLUTIONARY: Phase 4.2 - Unified migration error types
/// Replaces all custom migration error enums with this centralized system
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MigrationErrorType {
    // Data migration errors
    TypeConversionFailed,
    ForeignKeyViolation,
    DataIntegrityViolation,
    TransformationLogicError,
    DuplicateKeyError,
    ConstraintViolation,
    InsufficientPermissions,
    TimeoutError,
    
    // Schema change errors
    TableRestructuringFailed,
    ColumnModificationFailed,
    IndexCreationFailed,
    RelationshipEvolutionFailed,
    JunctionTableCreationFailed,
    
    // Validation errors
    SchemaValidationFailed,
    BackupCreationFailed,
    RollbackFailed,
    IntegrityCheckFailed,
    
    // Performance errors
    MemoryLimitExceeded,
    ProcessingTimeoutError,
    ResourceExhausted,
}

impl std::fmt::Display for MigrationErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let description = match self {
            MigrationErrorType::TypeConversionFailed => "Type conversion failed",
            MigrationErrorType::ForeignKeyViolation => "Foreign key constraint violation",
            MigrationErrorType::DataIntegrityViolation => "Data integrity violation",
            MigrationErrorType::TransformationLogicError => "Transformation logic error",
            MigrationErrorType::DuplicateKeyError => "Duplicate key error",
            MigrationErrorType::ConstraintViolation => "Database constraint violation",
            MigrationErrorType::InsufficientPermissions => "Insufficient permissions",
            MigrationErrorType::TimeoutError => "Operation timeout",
            MigrationErrorType::TableRestructuringFailed => "Table restructuring failed",
            MigrationErrorType::ColumnModificationFailed => "Column modification failed",
            MigrationErrorType::IndexCreationFailed => "Index creation failed",
            MigrationErrorType::RelationshipEvolutionFailed => "Relationship evolution failed",
            MigrationErrorType::JunctionTableCreationFailed => "Junction table creation failed",
            MigrationErrorType::SchemaValidationFailed => "Schema validation failed",
            MigrationErrorType::BackupCreationFailed => "Backup creation failed",
            MigrationErrorType::RollbackFailed => "Rollback operation failed",
            MigrationErrorType::IntegrityCheckFailed => "Integrity check failed",
            MigrationErrorType::MemoryLimitExceeded => "Memory limit exceeded",
            MigrationErrorType::ProcessingTimeoutError => "Processing timeout",
            MigrationErrorType::ResourceExhausted => "System resources exhausted",
        };
        write!(f, "{}", description)
    }
}

/// REVOLUTIONARY: Automatic relation validation at compile time
/// Detects and reports relation configuration issues before runtime
pub trait AutoRelationValidation: Entity {
    /// Perform comprehensive validation of all relations
    fn validate_all_relations() -> Vec<RelationValidationError> {
        let mut errors = Vec::new();
        
        // Validate that foreign keys reference existing entities
        errors.extend(Self::validate_foreign_key_references());
        
        // Check for missing inverse relations
        errors.extend(Self::validate_inverse_relations());
        
        // Validate junction table configurations
        errors.extend(Self::validate_junction_table_setup());
        
        errors
    }
    
    /// Check that all foreign keys reference valid entities and columns
    fn validate_foreign_key_references() -> Vec<RelationValidationError> {
        // Default implementation - override in derive macro
        Vec::new()
    }
    
    /// Ensure bidirectional relations have proper inverse definitions
    fn validate_inverse_relations() -> Vec<RelationValidationError> {
        // Default implementation - override in derive macro
        Vec::new()
    }
    
    /// Validate many-to-many junction table configurations
    fn validate_junction_table_setup() -> Vec<RelationValidationError> {
        // Default implementation - override in derive macro
        Vec::new()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum D1RsError {
    Database(String),
    NotFound,
    ValidationError(String),
    SerializationError(String),
    // Enhanced error types for better relation error handling
    RelationNotFound {
        entity: String,
        relation: String,
        available_relations: Vec<String>,
    },
    RelationConstraintViolation {
        entity: String,
        relation: String,
        constraint: String,
        suggestion: String,
    },
    JunctionTableMissing {
        relation: String,
        expected_table: String,
        suggestion: String,
    },
    InvalidForeignKey {
        relation: String,
        foreign_key: String,
        target_table: String,
        suggestion: String,
    },
    // Automatic migration system errors
    AutoMigration(String),
    // REVOLUTIONARY: Phase 4.1 - Enhanced relation validation errors
    RelationValidation {
        errors: Vec<RelationValidationError>,
        total_issues: usize,
        critical_issues: usize,
    },
    // REVOLUTIONARY: Phase 4.2 - Unified migration error handling
    DataMigration {
        operation: String,
        table: Option<String>,
        column: Option<String>,
        record_id: Option<String>,
        error_type: MigrationErrorType,
        suggestion: String,
        recovery_actions: Vec<String>,
    },
    SchemaChange {
        operation: String,
        table: String,
        reason: String,
        recovery_actions: Vec<String>,
        affected_tables: Vec<String>,
    },
    MigrationValidation {
        validation_type: String,
        issues: Vec<String>,
        suggestions: Vec<String>,
        critical: bool,
    },
}

/// Helper function to format relation issue types into readable messages
pub fn format_relation_issue(issue: &RelationIssueType) -> &'static str {
    match issue {
        RelationIssueType::MissingInverseRelation => "Missing inverse relation",
        RelationIssueType::InconsistentForeignKey => "Inconsistent foreign key",
        RelationIssueType::InvalidJunctionTable => "Invalid junction table configuration",
        RelationIssueType::CircularDependency => "Circular dependency detected",
        RelationIssueType::MissingReferencedEntity => "Referenced entity not found",
        RelationIssueType::TypeMismatchInForeignKey => "Foreign key type mismatch",
        RelationIssueType::DuplicateRelationDefinition => "Duplicate relation definition",
    }
}

impl fmt::Display for D1RsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            D1RsError::Database(msg) => write!(f, "Database error: {}", msg),
            D1RsError::NotFound => write!(f, "Entity not found"),
            D1RsError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            D1RsError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            
            // Enhanced error messages for relations - much better than Ent-Go!
            D1RsError::RelationNotFound { entity, relation, available_relations } => {
                if available_relations.is_empty() {
                    write!(f, "Relation '{}' not found on entity '{}'. This entity has no relations defined.", relation, entity)
                } else {
                    write!(f, "Relation '{}' not found on entity '{}'. Available relations: [{}]. Did you mean one of these?", 
                           relation, entity, available_relations.join(", "))
                }
            }
            
            D1RsError::RelationConstraintViolation { entity, relation, constraint, suggestion } => {
                write!(f, "Relation constraint violated on '{}.{}': {}. Suggestion: {}", 
                       entity, relation, constraint, suggestion)
            }
            
            D1RsError::JunctionTableMissing { relation, expected_table, suggestion } => {
                write!(f, "Many-to-many relation '{}' requires junction table '{}' but it was not found. Suggestion: {}", 
                       relation, expected_table, suggestion)
            }
            
            D1RsError::InvalidForeignKey { relation, foreign_key, target_table, suggestion } => {
                write!(f, "Invalid foreign key '{}' for relation '{}' (target table: '{}'). Suggestion: {}", 
                       foreign_key, relation, target_table, suggestion)
            }
            
            D1RsError::AutoMigration(msg) => {
                write!(f, "Automatic migration error: {}", msg)
            }
            
            // REVOLUTIONARY: Phase 4.1 - Detailed relation validation error reporting
            D1RsError::RelationValidation { errors, total_issues, critical_issues } => {
                writeln!(f, "🚨 Relation validation failed! Found {} issues ({} critical):", total_issues, critical_issues)?;
                writeln!(f)?;
                
                for (i, error) in errors.iter().enumerate() {
                    let severity = match error.issue {
                        RelationIssueType::CircularDependency | 
                        RelationIssueType::MissingReferencedEntity => "🔥 CRITICAL",
                        RelationIssueType::InconsistentForeignKey |
                        RelationIssueType::TypeMismatchInForeignKey => "⚠️  WARNING",
                        _ => "ℹ️  INFO",
                    };
                    
                    writeln!(f, "{}. {} - {} in '{}.{}'", 
                             i + 1, severity, 
                             format_relation_issue(&error.issue),
                             error.entity, error.relation)?;
                    writeln!(f, "   💡 Suggestion: {}", error.suggestion)?;
                    
                    if !error.affected_entities.is_empty() {
                        writeln!(f, "   🔗 Affects: {}", error.affected_entities.join(", "))?;
                    }
                    writeln!(f)?;
                }
                
                write!(f, "Fix these issues to ensure relation consistency and prevent runtime errors.")
            }
            
            // REVOLUTIONARY: Phase 4.2 - Enhanced migration error reporting
            D1RsError::DataMigration { operation, table, column, record_id, error_type, suggestion, recovery_actions } => {
                write!(f, "🚨 Data migration failed during '{}'", operation)?;
                
                if let Some(table) = table {
                    write!(f, " on table '{}'", table)?;
                    if let Some(column) = column {
                        write!(f, ", column '{}'", column)?;
                    }
                    if let Some(record_id) = record_id {
                        write!(f, ", record ID '{}'", record_id)?;
                    }
                }
                
                writeln!(f)?;
                writeln!(f, "❌ Error: {}", error_type)?;
                writeln!(f, "💡 Suggestion: {}", suggestion)?;
                
                if !recovery_actions.is_empty() {
                    writeln!(f, "🔧 Recovery actions:")?;
                    for (i, action) in recovery_actions.iter().enumerate() {
                        writeln!(f, "   {}. {}", i + 1, action)?;
                    }
                }
                
                Ok(())
            }
            
            D1RsError::SchemaChange { operation, table, reason, recovery_actions, affected_tables } => {
                writeln!(f, "🚨 Schema change failed during '{}'", operation)?;
                writeln!(f, "📋 Table: {}", table)?;
                writeln!(f, "❌ Reason: {}", reason)?;
                
                if !affected_tables.is_empty() {
                    writeln!(f, "🔗 Affected tables: {}", affected_tables.join(", "))?;
                }
                
                if !recovery_actions.is_empty() {
                    writeln!(f, "🔧 Recovery actions:")?;
                    for (i, action) in recovery_actions.iter().enumerate() {
                        writeln!(f, "   {}. {}", i + 1, action)?;
                    }
                }
                
                Ok(())
            }
            
            D1RsError::MigrationValidation { validation_type, issues, suggestions, critical } => {
                let severity = if *critical { "🔥 CRITICAL" } else { "⚠️  WARNING" };
                writeln!(f, "{} - Migration validation failed: {}", severity, validation_type)?;
                
                if !issues.is_empty() {
                    writeln!(f, "❌ Issues found:")?;
                    for (i, issue) in issues.iter().enumerate() {
                        writeln!(f, "   {}. {}", i + 1, issue)?;
                    }
                }
                
                if !suggestions.is_empty() {
                    writeln!(f, "💡 Suggestions:")?;
                    for (i, suggestion) in suggestions.iter().enumerate() {
                        writeln!(f, "   {}. {}", i + 1, suggestion)?;
                    }
                }
                
                Ok(())
            }
        }
    }
}

impl std::error::Error for D1RsError {}

pub type Result<T> = std::result::Result<T, D1RsError>;

/// REVOLUTIONARY: Phase 4.2 - Convenient error creation helpers
/// Makes it easy to create standardized, helpful error messages throughout the codebase
impl D1RsError {
    /// Create a data migration error with helpful context and suggestions
    pub fn data_migration(
        operation: impl Into<String>,
        error_type: MigrationErrorType,
        suggestion: impl Into<String>,
    ) -> Self {
        Self::DataMigration {
            operation: operation.into(),
            table: None,
            column: None,
            record_id: None,
            error_type,
            suggestion: suggestion.into(),
            recovery_actions: Vec::new(),
        }
    }
    
    /// Create a data migration error with table context
    pub fn data_migration_with_table(
        operation: impl Into<String>,
        table: impl Into<String>,
        error_type: MigrationErrorType,
        suggestion: impl Into<String>,
    ) -> Self {
        Self::DataMigration {
            operation: operation.into(),
            table: Some(table.into()),
            column: None,
            record_id: None,
            error_type,
            suggestion: suggestion.into(),
            recovery_actions: Vec::new(),
        }
    }
    
    /// Create a data migration error with full context
    pub fn data_migration_detailed(
        operation: impl Into<String>,
        table: Option<String>,
        column: Option<String>,
        record_id: Option<String>,
        error_type: MigrationErrorType,
        suggestion: impl Into<String>,
        recovery_actions: Vec<String>,
    ) -> Self {
        Self::DataMigration {
            operation: operation.into(),
            table,
            column,
            record_id,
            error_type,
            suggestion: suggestion.into(),
            recovery_actions,
        }
    }
    
    /// Create a schema change error
    pub fn schema_change(
        operation: impl Into<String>,
        table: impl Into<String>,
        reason: impl Into<String>,
        recovery_actions: Vec<String>,
    ) -> Self {
        Self::SchemaChange {
            operation: operation.into(),
            table: table.into(),
            reason: reason.into(),
            recovery_actions,
            affected_tables: Vec::new(),
        }
    }
    
    /// Create a schema change error with affected tables
    pub fn schema_change_with_affected(
        operation: impl Into<String>,
        table: impl Into<String>,
        reason: impl Into<String>,
        recovery_actions: Vec<String>,
        affected_tables: Vec<String>,
    ) -> Self {
        Self::SchemaChange {
            operation: operation.into(),
            table: table.into(),
            reason: reason.into(),
            recovery_actions,
            affected_tables,
        }
    }
    
    /// Create a migration validation error
    pub fn migration_validation(
        validation_type: impl Into<String>,
        issues: Vec<String>,
        suggestions: Vec<String>,
        critical: bool,
    ) -> Self {
        Self::MigrationValidation {
            validation_type: validation_type.into(),
            issues,
            suggestions,
            critical,
        }
    }
}

#[allow(async_fn_in_trait)]
pub trait Entity: Sized + serde::Serialize + serde::de::DeserializeOwned {
    type PrimaryKey: Clone + fmt::Debug + serde::Serialize + serde::de::DeserializeOwned;
    type QueryBuilder: QueryBuilder<Self>;
    type CreateBuilder: CreateBuilder<Self>;
    type UpdateBuilder: UpdateBuilder<Self>;

    const TABLE_NAME: &'static str;
    
    fn primary_key(&self) -> &Self::PrimaryKey;
    
    fn query() -> Self::QueryBuilder;
    fn create() -> Self::CreateBuilder;
    fn update(key: Self::PrimaryKey) -> Self::UpdateBuilder;
    
    /// Returns field names that are boolean types - generated by derive macro
    fn boolean_fields() -> &'static [&'static str];
    
    /// NEW: Type-safe field definitions using compile-time metadata
    /// This replaces ALL heuristic-based detection with trait-based information
    /// Generated automatically by the derive macro - NO RUNTIME DETECTION!
    fn field_definitions() -> Vec<FieldDefinition> {
        // Default implementation provides standard id field only
        // The derive macro will generate proper implementations for each entity
        vec![
            FieldDefinition {
                name: "id".to_string(),
                field_type: FieldType::Integer,
                nullable: false,
                primary_key: true,
                auto_increment: true,
                default_value: None,
                foreign_key: None,
            }
        ]
    }
    
    /// Returns foreign key relationships for this entity
    fn foreign_key_definitions() -> Vec<ForeignKeyDefinition> {
        // Default implementation - would be overridden by derive macro
        Vec::new()
    }
    
    /// REVOLUTIONARY: Extract recursive foreign key if this entity implements RecursiveEntity
    /// This method provides a type-safe way to detect recursive relationships
    fn recursive_foreign_key() -> Option<crate::auto_migration::introspector::ForeignKeySchema> {
        // Default implementation: no recursive relationship
        None
    }
    
    /// Convert boolean fields from SQLite integers to Rust booleans
    fn convert_from_sqlite(mut value: serde_json::Value) -> serde_json::Value {
        if let serde_json::Value::Object(ref mut map) = value {
            for field_name in Self::boolean_fields() {
                if let Some(serde_json::Value::Number(n)) = map.get(*field_name) {
                    if let Some(i) = n.as_i64() {
                        if i == 0 || i == 1 {
                            map.insert(field_name.to_string(), serde_json::Value::Bool(i != 0));
                        }
                    }
                }
            }
            
            // Also handle SQLite datetime strings -> RFC3339 format for DateTime<Utc>
            for (field_name, field_value) in map.iter_mut() {
                if let serde_json::Value::String(date_str) = field_value {
                    if field_name.ends_with("_at") || field_name.contains("time") {
                        // Try to parse and reformat SQLite datetime to RFC3339
                        if let Ok(naive_dt) = chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S") {
                            let utc_dt = naive_dt.and_utc();
                            *field_value = serde_json::Value::String(utc_dt.to_rfc3339());
                        }
                    }
                }
            }
        }
        value
    }
    
    /// Convert boolean fields from Rust booleans to SQLite integers
    fn convert_to_sqlite(mut value: serde_json::Value) -> serde_json::Value {
        if let serde_json::Value::Object(ref mut map) = value {
            for field_name in Self::boolean_fields() {
                if let Some(serde_json::Value::Bool(b)) = map.get(*field_name) {
                    map.insert(field_name.to_string(), serde_json::Value::Number((*b as i64).into()));
                }
            }
        }
        value
    }
    
    async fn find(db: &D1Client, key: Self::PrimaryKey) -> Result<Option<Self>>;
    async fn delete(db: &D1Client, key: Self::PrimaryKey) -> Result<()>;
}

#[allow(async_fn_in_trait)]
pub trait QueryBuilder<T: Entity> {
    async fn all(self, db: &D1Client) -> Result<Vec<T>>;
    async fn first(self, db: &D1Client) -> Result<Option<T>>;
    async fn count(self, db: &D1Client) -> Result<i64>;
    
    /// INTERNAL USE ONLY: Apply a relation constraint to the query builder
    /// This is used internally by the Association/edges system to pre-apply WHERE constraints
    /// 
    /// 🚨 WARNING: This method takes string parameters but is NOT part of the public API!
    /// Users should NEVER call this directly - use type-safe query methods instead
    /// Field names come from edge definitions which are generated by macros (compile-time safe)
    #[doc(hidden)]
    fn apply_relation_constraint(self, field: &str, value: serde_json::Value) -> Self;
}

#[allow(async_fn_in_trait)]
pub trait CreateBuilder<T: Entity> {
    async fn save(self, db: &D1Client) -> Result<T>;
}

#[allow(async_fn_in_trait)]
pub trait UpdateBuilder<T: Entity> {
    async fn save(self, db: &D1Client) -> Result<T>;
}

#[cfg(test)]
mod backend_exports_tests {
    use super::*;

    #[test]
    fn test_sqlite_client_type_alias() {
        // Test that SQLiteClient is properly defined as an alias
        let _type_name = std::any::type_name::<SQLiteClient>();
        assert_eq!(_type_name, "d1_rs::db::DatabaseClient<d1_rs::backends::sqlite::SQLiteBackend>");
    }

    #[test]
    fn test_d1_client_backward_compatibility() {
        // Test that D1Client is the same as SQLiteClient for backward compatibility
        let sqlite_type_name = std::any::type_name::<SQLiteClient>();
        let d1_type_name = std::any::type_name::<D1Client>();
        assert_eq!(sqlite_type_name, d1_type_name);
    }

    #[cfg(feature = "postgres")]
    #[test]
    fn test_postgresql_client_type_alias() {
        // Test that PostgreSQLClient is properly defined when postgres feature is enabled
        let _type_name = std::any::type_name::<PostgreSQLClient>();
        assert_eq!(_type_name, "d1_rs::db::DatabaseClient<d1_rs::backends::postgres::postgres_impl::PostgreSQLBackend>");
    }

    #[cfg(feature = "mysql")]
    #[test]
    fn test_mysql_client_type_alias() {
        // Test that MySQLClient is properly defined when mysql feature is enabled
        let _type_name = std::any::type_name::<MySQLClient>();
        assert_eq!(_type_name, "d1_rs::db::DatabaseClient<d1_rs::backends::mysql::mysql_impl::MySQLBackend>");
    }

    #[test]
    fn test_backend_module_exports() {
        // Test that backend module re-exports are accessible
        use crate::backends::{BackendError, SQLiteBackend};
        
        // Test that types can be used (DatabaseBackend is not dyn-compatible due to Clone requirement)
        let _backend: Option<SQLiteBackend> = None;
        let _error: Option<BackendError> = None;
    }

    #[test]
    fn test_backend_sqlite_exports() {
        // Test that SQLite backend exports are accessible
        use crate::backends::sqlite::*;
        
        // SQLite backend should always be available
        let _sqlite_backend: Option<SQLiteBackend> = None;
    }

    #[cfg(feature = "postgres")]
    #[test]
    fn test_backend_postgres_exports() {
        // Test that PostgreSQL backend exports are accessible when feature is enabled
        use crate::backends::postgres::*;
        
        let _postgres_backend: Option<PostgreSQLBackend> = None;
    }

    #[cfg(feature = "mysql")]
    #[test]
    fn test_backend_mysql_exports() {
        // Test that MySQL backend exports are accessible when feature is enabled
        use crate::backends::mysql::*;
        
        let _mysql_backend: Option<MySQLBackend> = None;
    }

    #[test]
    fn test_backend_config_exports() {
        // Test that config exports are accessible
        use crate::backends::{DatabaseConfig, BackendError};
        
        // Config types should be available
        let _config: Option<DatabaseConfig> = None;
        let _error: Option<BackendError> = None;
    }

    #[test]
    fn test_all_client_types_implement_send_sync() {
        // Ensure all client types are Send + Sync for async usage
        fn assert_send_sync<T: Send + Sync>() {}
        
        assert_send_sync::<SQLiteClient>();
        
        #[cfg(feature = "postgres")]
        assert_send_sync::<PostgreSQLClient>();
        
        #[cfg(feature = "mysql")]
        assert_send_sync::<MySQLClient>();
    }
}