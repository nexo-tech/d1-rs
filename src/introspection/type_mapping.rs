/// Cross-Database Type Mapping System
/// 
/// This module provides comprehensive type mapping capabilities across SQLite, PostgreSQL, 
/// and MySQL databases. It includes bidirectional type conversion, constraint mapping,
/// feature compatibility checks, and validation systems.
///
/// # Design Principles
/// 
/// - **Database Agnostic**: Works consistently across all supported databases
/// - **Type Safe**: Compile-time validation of type mappings
/// - **Bidirectional**: Convert between database-specific and unified types in both directions
/// - **Constraint Aware**: Preserves constraints and metadata during conversions
/// - **Validated**: Comprehensive compatibility and conversion validation
/// - **Extensible**: Easy to add new database types and mappings

use crate::dialects::DatabaseDialect;
use crate::introspection::schema::UnifiedColumnType;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use std::fmt;

/// Comprehensive type mapping information for cross-database conversion
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TypeMapping {
    /// Source database type
    pub source_type: String,
    /// Target unified type
    pub unified_type: UnifiedColumnType,
    /// Target database-specific type
    pub target_type: String,
    /// Type compatibility level
    pub compatibility: CompatibilityLevel,
    /// Precision information if applicable
    pub precision_info: Option<PrecisionInfo>,
    /// Type-specific constraints
    pub constraints: Vec<TypeConstraint>,
    /// Default value mapping information
    pub default_mapping: Option<DefaultMapping>,
    /// Type-specific notes and warnings
    pub notes: Vec<String>,
}

/// Type compatibility levels for cross-database conversions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CompatibilityLevel {
    /// Perfect compatibility - no data loss or functional differences
    Perfect,
    /// Compatible with minor differences (e.g., precision changes)
    Compatible,
    /// Compatible with potential data loss (e.g., size reduction)
    LossyCompatible,
    /// Compatible with functional differences (e.g., JSON vs TEXT)
    FunctionalDifferences,
    /// Incompatible - requires manual intervention
    Incompatible,
}

/// Precision and scale information for numeric types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrecisionInfo {
    /// Maximum precision (total digits)
    pub max_precision: Option<u32>,
    /// Maximum scale (decimal places)
    pub max_scale: Option<u32>,
    /// Minimum precision
    pub min_precision: Option<u32>,
    /// Default precision if not specified
    pub default_precision: Option<u32>,
    /// Whether precision is required
    pub precision_required: bool,
}

/// Type-specific constraints that affect mapping
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TypeConstraint {
    /// Maximum character length
    MaxLength(u32),
    /// Minimum character length
    MinLength(u32),
    /// Numeric range constraint
    NumericRange { min: Option<i64>, max: Option<i64> },
    /// Requires specific charset/collation
    CharsetRequired(String),
    /// Requires specific encoding
    EncodingRequired(String),
    /// Type-specific validation pattern
    ValidationPattern(String),
}

/// Default value mapping between databases
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DefaultMapping {
    /// Source default expression
    pub source_default: String,
    /// Target default expression
    pub target_default: String,
    /// Whether the mapping is lossy
    pub is_lossy: bool,
    /// Mapping notes
    pub notes: Option<String>,
}

/// Database feature support matrix
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FeatureSupportMatrix {
    /// Database dialect
    pub dialect: DatabaseDialect,
    /// Supported column types
    pub supported_types: Vec<UnifiedColumnType>,
    /// JSON support level
    pub json_support: JsonSupportLevel,
    /// UUID native support
    pub uuid_native_support: bool,
    /// Array type support
    pub array_support: bool,
    /// Full-text search support
    pub fulltext_support: bool,
    /// Spatial/GIS support
    pub spatial_support: bool,
    /// Maximum identifier length
    pub max_identifier_length: u32,
    /// Maximum index columns
    pub max_index_columns: u32,
    /// Supports partial indexes
    pub partial_indexes: bool,
    /// Supports functional indexes
    pub functional_indexes: bool,
    /// Check constraint support
    pub check_constraints: bool,
    /// Deferrable constraint support
    pub deferrable_constraints: bool,
}

/// Level of JSON support in the database
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JsonSupportLevel {
    /// No JSON support
    None,
    /// JSON stored as text with basic validation
    TextBased,
    /// Native JSON type with operators and functions
    Native,
    /// Binary JSON with advanced indexing
    Binary,
}

/// Comprehensive type mapping engine
pub struct CrossDatabaseTypeMapper {
    /// Type mappings for each source->target dialect pair
    mappings: HashMap<(DatabaseDialect, DatabaseDialect), Vec<TypeMapping>>,
    /// Feature support matrix for each database
    features: HashMap<DatabaseDialect, FeatureSupportMatrix>,
    /// Constraint mapping rules
    constraint_mappings: HashMap<DatabaseDialect, ConstraintMappings>,
}

/// Database-specific constraint mappings
#[derive(Debug, Clone)]
pub struct ConstraintMappings {
    /// Primary key syntax
    pub primary_key_syntax: String,
    /// Foreign key syntax template
    pub foreign_key_syntax: String,
    /// Unique constraint syntax
    pub unique_constraint_syntax: String,
    /// Check constraint syntax
    pub check_constraint_syntax: String,
    /// Default value syntax
    pub default_value_syntax: String,
    /// Auto increment syntax
    pub auto_increment_syntax: String,
}

impl CrossDatabaseTypeMapper {
    /// Create a new type mapper with comprehensive database support
    pub fn new() -> Self {
        let mut mapper = Self {
            mappings: HashMap::new(),
            features: HashMap::new(),
            constraint_mappings: HashMap::new(),
        };
        
        mapper.initialize_mappings();
        mapper.initialize_features();
        mapper.initialize_constraint_mappings();
        mapper
    }

    /// Initialize comprehensive type mappings between all database pairs
    fn initialize_mappings(&mut self) {
        // SQLite -> PostgreSQL mappings
        #[cfg(feature = "postgres")]
        self.add_mapping_set(DatabaseDialect::SQLite, DatabaseDialect::PostgreSQL, vec![
            self.create_mapping("INTEGER", UnifiedColumnType::Integer, "INTEGER", CompatibilityLevel::Perfect),
            self.create_mapping("TEXT", UnifiedColumnType::Text, "TEXT", CompatibilityLevel::Perfect),
            self.create_mapping("REAL", UnifiedColumnType::Real, "REAL", CompatibilityLevel::Perfect),
            self.create_mapping("BLOB", UnifiedColumnType::Blob, "BYTEA", CompatibilityLevel::Compatible),
            self.create_mapping("NUMERIC", UnifiedColumnType::Decimal, "NUMERIC", CompatibilityLevel::Perfect),
            self.create_boolean_mapping("BOOLEAN", DatabaseDialect::PostgreSQL),
            self.create_datetime_mapping("DATETIME", "TIMESTAMP", DatabaseDialect::PostgreSQL),
        ]);

        // SQLite -> MySQL mappings  
        #[cfg(feature = "mysql")]
        self.add_mapping_set(DatabaseDialect::SQLite, DatabaseDialect::MySQL, vec![
            self.create_mapping("INTEGER", UnifiedColumnType::Integer, "INT", CompatibilityLevel::Perfect),
            self.create_mapping("TEXT", UnifiedColumnType::Text, "TEXT", CompatibilityLevel::Perfect),
            self.create_mapping("REAL", UnifiedColumnType::Real, "DOUBLE", CompatibilityLevel::Compatible),
            self.create_mapping("BLOB", UnifiedColumnType::Blob, "BLOB", CompatibilityLevel::Perfect),
            self.create_mapping("NUMERIC", UnifiedColumnType::Decimal, "DECIMAL", CompatibilityLevel::Perfect),
            self.create_boolean_mapping("BOOLEAN", DatabaseDialect::MySQL),
            self.create_datetime_mapping("DATETIME", "DATETIME", DatabaseDialect::MySQL),
        ]);

        // PostgreSQL -> SQLite mappings
        #[cfg(feature = "postgres")]
        self.add_mapping_set(DatabaseDialect::PostgreSQL, DatabaseDialect::SQLite, vec![
            self.create_mapping("INTEGER", UnifiedColumnType::Integer, "INTEGER", CompatibilityLevel::Perfect),
            self.create_mapping("TEXT", UnifiedColumnType::Text, "TEXT", CompatibilityLevel::Perfect),
            self.create_mapping("REAL", UnifiedColumnType::Real, "REAL", CompatibilityLevel::Perfect),
            self.create_mapping("BYTEA", UnifiedColumnType::Blob, "BLOB", CompatibilityLevel::Compatible),
            self.create_mapping("BOOLEAN", UnifiedColumnType::Boolean, "INTEGER", CompatibilityLevel::Compatible),
            self.create_mapping("TIMESTAMP", UnifiedColumnType::DateTime, "TEXT", CompatibilityLevel::LossyCompatible),
            self.create_mapping("JSON", UnifiedColumnType::Json, "TEXT", CompatibilityLevel::FunctionalDifferences),
            self.create_mapping("UUID", UnifiedColumnType::Uuid, "TEXT", CompatibilityLevel::FunctionalDifferences),
        ]);

        // MySQL -> SQLite mappings
        #[cfg(feature = "mysql")]
        self.add_mapping_set(DatabaseDialect::MySQL, DatabaseDialect::SQLite, vec![
            self.create_mapping("INT", UnifiedColumnType::Integer, "INTEGER", CompatibilityLevel::Perfect),
            self.create_mapping("TEXT", UnifiedColumnType::Text, "TEXT", CompatibilityLevel::Perfect),
            self.create_mapping("DOUBLE", UnifiedColumnType::Double, "REAL", CompatibilityLevel::Compatible),
            self.create_mapping("BLOB", UnifiedColumnType::Blob, "BLOB", CompatibilityLevel::Perfect),
            self.create_mapping("TINYINT(1)", UnifiedColumnType::Boolean, "INTEGER", CompatibilityLevel::Compatible),
            self.create_mapping("DATETIME", UnifiedColumnType::DateTime, "TEXT", CompatibilityLevel::LossyCompatible),
            self.create_mapping("JSON", UnifiedColumnType::Json, "TEXT", CompatibilityLevel::FunctionalDifferences),
        ]);

        // PostgreSQL -> MySQL mappings
        #[cfg(all(feature = "postgres", feature = "mysql"))]
        self.add_mapping_set(DatabaseDialect::PostgreSQL, DatabaseDialect::MySQL, vec![
            self.create_mapping("INTEGER", UnifiedColumnType::Integer, "INT", CompatibilityLevel::Perfect),
            self.create_mapping("TEXT", UnifiedColumnType::Text, "TEXT", CompatibilityLevel::Perfect),
            self.create_mapping("REAL", UnifiedColumnType::Real, "FLOAT", CompatibilityLevel::Compatible),
            self.create_mapping("BYTEA", UnifiedColumnType::Blob, "BLOB", CompatibilityLevel::Compatible),
            self.create_mapping("BOOLEAN", UnifiedColumnType::Boolean, "TINYINT(1)", CompatibilityLevel::Compatible),
            self.create_mapping("TIMESTAMP", UnifiedColumnType::DateTime, "DATETIME", CompatibilityLevel::Compatible),
            self.create_mapping("JSON", UnifiedColumnType::Json, "JSON", CompatibilityLevel::Perfect),
            self.create_mapping("UUID", UnifiedColumnType::Uuid, "CHAR(36)", CompatibilityLevel::FunctionalDifferences),
        ]);

        // MySQL -> PostgreSQL mappings
        #[cfg(all(feature = "postgres", feature = "mysql"))]
        self.add_mapping_set(DatabaseDialect::MySQL, DatabaseDialect::PostgreSQL, vec![
            self.create_mapping("INT", UnifiedColumnType::Integer, "INTEGER", CompatibilityLevel::Perfect),
            self.create_mapping("TEXT", UnifiedColumnType::Text, "TEXT", CompatibilityLevel::Perfect),
            self.create_mapping("FLOAT", UnifiedColumnType::Real, "REAL", CompatibilityLevel::Compatible),
            self.create_mapping("BLOB", UnifiedColumnType::Blob, "BYTEA", CompatibilityLevel::Compatible),
            self.create_mapping("TINYINT(1)", UnifiedColumnType::Boolean, "BOOLEAN", CompatibilityLevel::Compatible),
            self.create_mapping("DATETIME", UnifiedColumnType::DateTime, "TIMESTAMP", CompatibilityLevel::Compatible),
            self.create_mapping("JSON", UnifiedColumnType::Json, "JSONB", CompatibilityLevel::Compatible),
        ]);
    }

    /// Initialize feature support matrices for all databases
    fn initialize_features(&mut self) {
        // SQLite feature support
        self.features.insert(DatabaseDialect::SQLite, FeatureSupportMatrix {
            dialect: DatabaseDialect::SQLite,
            supported_types: vec![
                UnifiedColumnType::Integer,
                UnifiedColumnType::Text,
                UnifiedColumnType::Real,
                UnifiedColumnType::Blob,
                UnifiedColumnType::Boolean, // Via INTEGER
            ],
            json_support: JsonSupportLevel::TextBased,
            uuid_native_support: false,
            array_support: false,
            fulltext_support: true,
            spatial_support: false,
            max_identifier_length: 255,
            max_index_columns: 16,
            partial_indexes: true,
            functional_indexes: false,
            check_constraints: true,
            deferrable_constraints: true,
        });

        // PostgreSQL feature support
        #[cfg(feature = "postgres")]
        self.features.insert(DatabaseDialect::PostgreSQL, FeatureSupportMatrix {
            dialect: DatabaseDialect::PostgreSQL,
            supported_types: vec![
                UnifiedColumnType::Boolean,
                UnifiedColumnType::SmallInt,
                UnifiedColumnType::Integer,
                UnifiedColumnType::BigInt,
                UnifiedColumnType::Real,
                UnifiedColumnType::Double,
                UnifiedColumnType::Decimal,
                UnifiedColumnType::Char,
                UnifiedColumnType::VarChar,
                UnifiedColumnType::Text,
                UnifiedColumnType::Blob,
                UnifiedColumnType::Json,
                UnifiedColumnType::Date,
                UnifiedColumnType::Time,
                UnifiedColumnType::DateTime,
                UnifiedColumnType::Timestamp,
                UnifiedColumnType::Uuid,
            ],
            json_support: JsonSupportLevel::Binary,
            uuid_native_support: true,
            array_support: true,
            fulltext_support: true,
            spatial_support: true,
            max_identifier_length: 63,
            max_index_columns: 32,
            partial_indexes: true,
            functional_indexes: true,
            check_constraints: true,
            deferrable_constraints: true,
        });

        // MySQL feature support
        #[cfg(feature = "mysql")]
        self.features.insert(DatabaseDialect::MySQL, FeatureSupportMatrix {
            dialect: DatabaseDialect::MySQL,
            supported_types: vec![
                UnifiedColumnType::Boolean, // Via TINYINT(1)
                UnifiedColumnType::SmallInt,
                UnifiedColumnType::Integer,
                UnifiedColumnType::BigInt,
                UnifiedColumnType::Real,
                UnifiedColumnType::Double,
                UnifiedColumnType::Decimal,
                UnifiedColumnType::Char,
                UnifiedColumnType::VarChar,
                UnifiedColumnType::Text,
                UnifiedColumnType::Blob,
                UnifiedColumnType::Json,
                UnifiedColumnType::Date,
                UnifiedColumnType::Time,
                UnifiedColumnType::DateTime,
                UnifiedColumnType::Timestamp,
            ],
            json_support: JsonSupportLevel::Native,
            uuid_native_support: false,
            array_support: false,
            fulltext_support: true,
            spatial_support: true,
            max_identifier_length: 64,
            max_index_columns: 16,
            partial_indexes: false,
            functional_indexes: true,
            check_constraints: true, // MySQL 8.0.16+
            deferrable_constraints: false,
        });
    }

    /// Initialize constraint mapping rules for each database
    fn initialize_constraint_mappings(&mut self) {
        // SQLite constraint mappings
        self.constraint_mappings.insert(DatabaseDialect::SQLite, ConstraintMappings {
            primary_key_syntax: "PRIMARY KEY".to_string(),
            foreign_key_syntax: "FOREIGN KEY ({columns}) REFERENCES {table}({ref_columns})".to_string(),
            unique_constraint_syntax: "UNIQUE".to_string(),
            check_constraint_syntax: "CHECK ({expression})".to_string(),
            default_value_syntax: "DEFAULT {value}".to_string(),
            auto_increment_syntax: "AUTOINCREMENT".to_string(),
        });

        // PostgreSQL constraint mappings
        #[cfg(feature = "postgres")]
        self.constraint_mappings.insert(DatabaseDialect::PostgreSQL, ConstraintMappings {
            primary_key_syntax: "PRIMARY KEY".to_string(),
            foreign_key_syntax: "FOREIGN KEY ({columns}) REFERENCES {table}({ref_columns})".to_string(),
            unique_constraint_syntax: "UNIQUE".to_string(),
            check_constraint_syntax: "CHECK ({expression})".to_string(),
            default_value_syntax: "DEFAULT {value}".to_string(),
            auto_increment_syntax: "GENERATED BY DEFAULT AS IDENTITY".to_string(),
        });

        // MySQL constraint mappings
        #[cfg(feature = "mysql")]
        self.constraint_mappings.insert(DatabaseDialect::MySQL, ConstraintMappings {
            primary_key_syntax: "PRIMARY KEY".to_string(),
            foreign_key_syntax: "FOREIGN KEY ({columns}) REFERENCES {table}({ref_columns})".to_string(),
            unique_constraint_syntax: "UNIQUE".to_string(),
            check_constraint_syntax: "CHECK ({expression})".to_string(),
            default_value_syntax: "DEFAULT {value}".to_string(),
            auto_increment_syntax: "AUTO_INCREMENT".to_string(),
        });
    }

    /// Helper method to add a set of mappings for a dialect pair
    #[allow(dead_code)]
    fn add_mapping_set(&mut self, source: DatabaseDialect, target: DatabaseDialect, mappings: Vec<TypeMapping>) {
        self.mappings.insert((source, target), mappings);
    }

    /// Create a basic type mapping
    #[allow(dead_code)]
    fn create_mapping(&self, source_type: &str, unified_type: UnifiedColumnType, target_type: &str, compatibility: CompatibilityLevel) -> TypeMapping {
        TypeMapping {
            source_type: source_type.to_string(),
            unified_type,
            target_type: target_type.to_string(),
            compatibility,
            precision_info: None,
            constraints: Vec::new(),
            default_mapping: None,
            notes: Vec::new(),
        }
    }

    /// Create a boolean type mapping with database-specific handling
    #[allow(dead_code)]
    fn create_boolean_mapping(&self, target_type: &str, target_dialect: DatabaseDialect) -> TypeMapping {
        let mut mapping = TypeMapping {
            source_type: "BOOLEAN".to_string(),
            unified_type: UnifiedColumnType::Boolean,
            target_type: target_type.to_string(),
            compatibility: CompatibilityLevel::Compatible,
            precision_info: None,
            constraints: Vec::new(),
            default_mapping: Some(DefaultMapping {
                source_default: "FALSE".to_string(),
                target_default: match target_dialect {
                    DatabaseDialect::SQLite => "0".to_string(),
                    #[cfg(feature = "postgres")]
                    DatabaseDialect::PostgreSQL => "FALSE".to_string(),
                    #[cfg(feature = "mysql")]
                    DatabaseDialect::MySQL => "0".to_string(),
                },
                is_lossy: false,
                notes: Some("Boolean value mapping".to_string()),
            }),
            notes: vec!["Database-specific boolean representation".to_string()],
        };

        if matches!(target_dialect, DatabaseDialect::SQLite) {
            mapping.notes.push("Stored as integer: 0=false, 1=true".to_string());
        }
        #[cfg(feature = "mysql")]
        if matches!(target_dialect, DatabaseDialect::MySQL) {
            mapping.notes.push("Stored as integer: 0=false, 1=true".to_string());
        }

        mapping
    }

    /// Create a datetime type mapping with timezone considerations
    #[allow(dead_code)]
    fn create_datetime_mapping(&self, source_type: &str, target_type: &str, target_dialect: DatabaseDialect) -> TypeMapping {
        TypeMapping {
            source_type: source_type.to_string(),
            unified_type: UnifiedColumnType::DateTime,
            target_type: target_type.to_string(),
            compatibility: match target_dialect {
                #[cfg(feature = "postgres")]
                DatabaseDialect::PostgreSQL => CompatibilityLevel::Compatible,
                #[cfg(feature = "mysql")]
                DatabaseDialect::MySQL => CompatibilityLevel::Compatible,
                DatabaseDialect::SQLite => CompatibilityLevel::LossyCompatible,
            },
            precision_info: None,
            constraints: Vec::new(),
            default_mapping: Some(DefaultMapping {
                source_default: "CURRENT_TIMESTAMP".to_string(),
                target_default: match target_dialect {
                    DatabaseDialect::SQLite => "CURRENT_TIMESTAMP".to_string(),
                    #[cfg(feature = "postgres")]
                    DatabaseDialect::PostgreSQL => "CURRENT_TIMESTAMP".to_string(),
                    #[cfg(feature = "mysql")]  
                    DatabaseDialect::MySQL => "CURRENT_TIMESTAMP".to_string(),
                },
                is_lossy: false,
                notes: Some("Timestamp default mapping".to_string()),
            }),
            notes: match target_dialect {
                DatabaseDialect::SQLite => vec!["Stored as text in ISO 8601 format".to_string()],
                #[cfg(feature = "postgres")]
                DatabaseDialect::PostgreSQL => vec!["Native datetime support".to_string()],
                #[cfg(feature = "mysql")]
                DatabaseDialect::MySQL => vec!["Native datetime support".to_string()],
            },
        }
    }

    /// Get type mappings between two databases
    pub fn get_mappings(&self, source: DatabaseDialect, target: DatabaseDialect) -> Option<&Vec<TypeMapping>> {
        self.mappings.get(&(source, target))
    }

    /// Find the best type mapping for a specific source type
    pub fn find_mapping(&self, source: DatabaseDialect, target: DatabaseDialect, source_type: &str) -> Option<&TypeMapping> {
        self.get_mappings(source, target)?
            .iter()
            .find(|mapping| mapping.source_type.eq_ignore_ascii_case(source_type))
    }

    /// Map a database-specific type to a unified type
    pub fn map_to_unified(&self, source_dialect: DatabaseDialect, database_type: &str) -> Option<UnifiedColumnType> {
        // Try direct mapping first
        for (_, mappings) in self.mappings.iter() {
            if let Some(mapping) = mappings.iter().find(|m| m.source_type.eq_ignore_ascii_case(database_type)) {
                return Some(mapping.unified_type.clone());
            }
        }

        // Fallback to pattern matching for complex types
        self.pattern_match_type(source_dialect, database_type)
    }

    /// Pattern match complex types with parameters
    fn pattern_match_type(&self, dialect: DatabaseDialect, database_type: &str) -> Option<UnifiedColumnType> {
        let upper_type = database_type.to_uppercase();
        
        match dialect {
            DatabaseDialect::SQLite => {
                if upper_type.contains("INT") { Some(UnifiedColumnType::Integer) }
                else if upper_type.contains("TEXT") || upper_type.contains("CHAR") { Some(UnifiedColumnType::Text) }
                else if upper_type.contains("REAL") || upper_type.contains("FLOA") || upper_type.contains("DOUB") { Some(UnifiedColumnType::Real) }
                else if upper_type.contains("BLOB") { Some(UnifiedColumnType::Blob) }
                else { None }
            },
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => {
                if upper_type.starts_with("VARCHAR") || upper_type.starts_with("CHARACTER VARYING") { Some(UnifiedColumnType::VarChar) }
                else if upper_type.starts_with("CHAR") && !upper_type.contains("VARYING") { Some(UnifiedColumnType::Char) }
                else if upper_type.starts_with("NUMERIC") || upper_type.starts_with("DECIMAL") { Some(UnifiedColumnType::Decimal) }
                else if upper_type == "TIMESTAMPTZ" { Some(UnifiedColumnType::Timestamp) }
                else { None }
            },
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => {
                if upper_type.starts_with("VARCHAR") { Some(UnifiedColumnType::VarChar) }
                else if upper_type.starts_with("CHAR") { Some(UnifiedColumnType::Char) }
                else if upper_type.starts_with("DECIMAL") || upper_type.starts_with("NUMERIC") { Some(UnifiedColumnType::Decimal) }
                else if upper_type == "TINYINT(1)" { Some(UnifiedColumnType::Boolean) }
                else if upper_type.starts_with("TINYINT") { Some(UnifiedColumnType::SmallInt) }
                else if upper_type.starts_with("MEDIUMINT") || upper_type.starts_with("INT") { Some(UnifiedColumnType::Integer) }
                else { None }
            },
        }
    }

    /// Map a unified type to a database-specific type
    pub fn map_from_unified(&self, target_dialect: DatabaseDialect, unified_type: &UnifiedColumnType) -> Option<String> {
        // Find the best mapping by looking through all source dialects
        #[allow(unused_mut)]
        let mut source_dialects = vec![DatabaseDialect::SQLite];
        
        #[cfg(feature = "postgres")]
        source_dialects.push(DatabaseDialect::PostgreSQL);
        
        #[cfg(feature = "mysql")]
        source_dialects.push(DatabaseDialect::MySQL);
        
        for source_dialect in source_dialects {
            if let Some(mappings) = self.get_mappings(source_dialect, target_dialect) {
                if let Some(mapping) = mappings.iter().find(|m| m.unified_type == *unified_type) {
                    return Some(mapping.target_type.clone());
                }
            }
        }

        // Fallback to standard type mapping
        self.fallback_unified_mapping(target_dialect, unified_type)
    }

    /// Fallback unified type mapping when no specific mapping exists
    fn fallback_unified_mapping(&self, dialect: DatabaseDialect, unified_type: &UnifiedColumnType) -> Option<String> {
        match dialect {
            DatabaseDialect::SQLite => match unified_type {
                UnifiedColumnType::Boolean => Some("INTEGER".to_string()),
                UnifiedColumnType::SmallInt | UnifiedColumnType::Integer | UnifiedColumnType::BigInt => Some("INTEGER".to_string()),
                UnifiedColumnType::Real | UnifiedColumnType::Double | UnifiedColumnType::Decimal => Some("REAL".to_string()),
                UnifiedColumnType::Char | UnifiedColumnType::VarChar | UnifiedColumnType::Text => Some("TEXT".to_string()),
                UnifiedColumnType::Blob => Some("BLOB".to_string()),
                UnifiedColumnType::Json => Some("TEXT".to_string()),
                UnifiedColumnType::Date | UnifiedColumnType::Time | UnifiedColumnType::DateTime | UnifiedColumnType::Timestamp => Some("TEXT".to_string()),
                UnifiedColumnType::Uuid => Some("TEXT".to_string()),
                UnifiedColumnType::Other(_) => None,
            },
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => match unified_type {
                UnifiedColumnType::Boolean => Some("BOOLEAN".to_string()),
                UnifiedColumnType::SmallInt => Some("SMALLINT".to_string()),
                UnifiedColumnType::Integer => Some("INTEGER".to_string()),
                UnifiedColumnType::BigInt => Some("BIGINT".to_string()),
                UnifiedColumnType::Real => Some("REAL".to_string()),
                UnifiedColumnType::Double => Some("DOUBLE PRECISION".to_string()),
                UnifiedColumnType::Decimal => Some("NUMERIC".to_string()),
                UnifiedColumnType::Char => Some("CHAR".to_string()),
                UnifiedColumnType::VarChar => Some("VARCHAR".to_string()),
                UnifiedColumnType::Text => Some("TEXT".to_string()),
                UnifiedColumnType::Blob => Some("BYTEA".to_string()),
                UnifiedColumnType::Json => Some("JSONB".to_string()),
                UnifiedColumnType::Date => Some("DATE".to_string()),
                UnifiedColumnType::Time => Some("TIME".to_string()),
                UnifiedColumnType::DateTime => Some("TIMESTAMP".to_string()),
                UnifiedColumnType::Timestamp => Some("TIMESTAMPTZ".to_string()),
                UnifiedColumnType::Uuid => Some("UUID".to_string()),
                UnifiedColumnType::Other(_) => None,
            },
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => match unified_type {
                UnifiedColumnType::Boolean => Some("TINYINT(1)".to_string()),
                UnifiedColumnType::SmallInt => Some("SMALLINT".to_string()),
                UnifiedColumnType::Integer => Some("INT".to_string()),
                UnifiedColumnType::BigInt => Some("BIGINT".to_string()),
                UnifiedColumnType::Real => Some("FLOAT".to_string()),
                UnifiedColumnType::Double => Some("DOUBLE".to_string()),
                UnifiedColumnType::Decimal => Some("DECIMAL".to_string()),
                UnifiedColumnType::Char => Some("CHAR".to_string()),
                UnifiedColumnType::VarChar => Some("VARCHAR".to_string()),
                UnifiedColumnType::Text => Some("TEXT".to_string()),
                UnifiedColumnType::Blob => Some("BLOB".to_string()),
                UnifiedColumnType::Json => Some("JSON".to_string()),
                UnifiedColumnType::Date => Some("DATE".to_string()),
                UnifiedColumnType::Time => Some("TIME".to_string()),
                UnifiedColumnType::DateTime => Some("DATETIME".to_string()),
                UnifiedColumnType::Timestamp => Some("TIMESTAMP".to_string()),
                UnifiedColumnType::Uuid => Some("CHAR(36)".to_string()),
                UnifiedColumnType::Other(_) => None,
            },
        }
    }

    /// Get feature support matrix for a database
    pub fn get_feature_support(&self, dialect: DatabaseDialect) -> Option<&FeatureSupportMatrix> {
        self.features.get(&dialect)
    }

    /// Check if a type is supported by a database
    pub fn is_type_supported(&self, dialect: DatabaseDialect, unified_type: &UnifiedColumnType) -> bool {
        self.get_feature_support(dialect)
            .map(|features| features.supported_types.contains(unified_type))
            .unwrap_or(false)
    }

    /// Get constraint mappings for a database
    pub fn get_constraint_mappings(&self, dialect: DatabaseDialect) -> Option<&ConstraintMappings> {
        self.constraint_mappings.get(&dialect)
    }

    /// Validate type conversion compatibility
    pub fn validate_conversion(&self, source: DatabaseDialect, target: DatabaseDialect, source_type: &str) -> ValidationResult {
        if let Some(mapping) = self.find_mapping(source, target, source_type) {
            ValidationResult {
                is_valid: true,
                compatibility: mapping.compatibility.clone(),
                warnings: self.generate_warnings(&mapping),
                recommendations: self.generate_recommendations(&mapping),
            }
        } else {
            ValidationResult {
                is_valid: false,
                compatibility: CompatibilityLevel::Incompatible,
                warnings: vec!["No direct mapping found".to_string()],
                recommendations: vec!["Manual conversion required".to_string()],
            }
        }
    }

    /// Generate warnings for a type mapping
    fn generate_warnings(&self, mapping: &TypeMapping) -> Vec<String> {
        let mut warnings = Vec::new();
        
        match mapping.compatibility {
            CompatibilityLevel::LossyCompatible => {
                warnings.push("Potential data loss during conversion".to_string());
            },
            CompatibilityLevel::FunctionalDifferences => {
                warnings.push("Functional differences may affect application behavior".to_string());
            },
            CompatibilityLevel::Incompatible => {
                warnings.push("Manual intervention required for conversion".to_string());
            },
            _ => {},
        }

        if let Some(default_mapping) = &mapping.default_mapping {
            if default_mapping.is_lossy {
                warnings.push("Default value conversion may be lossy".to_string());
            }
        }

        warnings.extend(mapping.notes.clone());
        warnings
    }

    /// Generate recommendations for a type mapping
    fn generate_recommendations(&self, mapping: &TypeMapping) -> Vec<String> {
        let mut recommendations = Vec::new();

        match mapping.compatibility {
            CompatibilityLevel::LossyCompatible => {
                recommendations.push("Validate data after conversion".to_string());
                recommendations.push("Consider increasing precision/size in target".to_string());
            },
            CompatibilityLevel::FunctionalDifferences => {
                recommendations.push("Test application functionality thoroughly".to_string());
                recommendations.push("Update queries to use target database features".to_string());
            },
            CompatibilityLevel::Incompatible => {
                recommendations.push("Use custom conversion logic".to_string());
                recommendations.push("Consider alternative type in target database".to_string());
            },
            _ => {
                recommendations.push("Conversion should be straightforward".to_string());
            },
        }

        recommendations
    }
}

impl Default for CrossDatabaseTypeMapper {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of type conversion validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the conversion is valid
    pub is_valid: bool,
    /// Compatibility level of the conversion
    pub compatibility: CompatibilityLevel,
    /// Warnings about the conversion
    pub warnings: Vec<String>,
    /// Recommendations for the conversion
    pub recommendations: Vec<String>,
}

/// Type mapping analysis result
#[derive(Debug, Clone)]
pub struct MappingAnalysis {
    /// Source database type
    pub source_type: String,
    /// Target database type
    pub target_type: Option<String>,
    /// Unified intermediate type
    pub unified_type: Option<UnifiedColumnType>,
    /// Conversion compatibility
    pub compatibility: CompatibilityLevel,
    /// Analysis notes and warnings
    pub notes: Vec<String>,
}

/// Comprehensive type mapping analyzer
pub struct TypeMappingAnalyzer {
    mapper: CrossDatabaseTypeMapper,
}

impl TypeMappingAnalyzer {
    /// Create a new type mapping analyzer
    pub fn new() -> Self {
        Self {
            mapper: CrossDatabaseTypeMapper::new(),
        }
    }

    /// Analyze type mapping between two databases
    pub fn analyze_mapping(&self, source: DatabaseDialect, target: DatabaseDialect, source_type: &str) -> MappingAnalysis {
        let unified_type = self.mapper.map_to_unified(source, source_type);
        let target_type = unified_type.as_ref().and_then(|ut| self.mapper.map_from_unified(target, ut));
        
        let mapping = self.mapper.find_mapping(source, target, source_type);
        let compatibility = mapping.map(|m| m.compatibility.clone()).unwrap_or(CompatibilityLevel::Incompatible);
        
        let mut notes = Vec::new();
        if let Some(mapping) = mapping {
            notes.extend(mapping.notes.clone());
        } else {
            notes.push("No direct mapping available - using fallback".to_string());
        }

        MappingAnalysis {
            source_type: source_type.to_string(),
            target_type,
            unified_type,
            compatibility,
            notes,
        }
    }

    /// Analyze all supported types for a database pair
    pub fn analyze_all_mappings(&self, source: DatabaseDialect, target: DatabaseDialect) -> Vec<MappingAnalysis> {
        let mut analyses = Vec::new();
        
        if let Some(mappings) = self.mapper.get_mappings(source, target) {
            for mapping in mappings {
                let analysis = MappingAnalysis {
                    source_type: mapping.source_type.clone(),
                    target_type: Some(mapping.target_type.clone()),
                    unified_type: Some(mapping.unified_type.clone()),
                    compatibility: mapping.compatibility.clone(),
                    notes: mapping.notes.clone(),
                };
                analyses.push(analysis);
            }
        }

        analyses
    }

    /// Get access to the underlying mapper
    pub fn mapper(&self) -> &CrossDatabaseTypeMapper {
        &self.mapper
    }
}

impl Default for TypeMappingAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for CompatibilityLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompatibilityLevel::Perfect => write!(f, "Perfect"),
            CompatibilityLevel::Compatible => write!(f, "Compatible"),
            CompatibilityLevel::LossyCompatible => write!(f, "Lossy Compatible"),
            CompatibilityLevel::FunctionalDifferences => write!(f, "Functional Differences"),
            CompatibilityLevel::Incompatible => write!(f, "Incompatible"),
        }
    }
}

impl fmt::Display for JsonSupportLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JsonSupportLevel::None => write!(f, "None"),
            JsonSupportLevel::TextBased => write!(f, "Text-based"),
            JsonSupportLevel::Native => write!(f, "Native"),
            JsonSupportLevel::Binary => write!(f, "Binary"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_mapper_creation() {
        let mapper = CrossDatabaseTypeMapper::new();
        
        // Test feature support matrices exist
        assert!(mapper.get_feature_support(DatabaseDialect::SQLite).is_some());
        
        // Test that basic mappings exist (only if features are enabled)
        #[cfg(feature = "postgres")]
        assert!(mapper.get_mappings(DatabaseDialect::SQLite, DatabaseDialect::PostgreSQL).is_some());
        
        #[cfg(feature = "mysql")]
        assert!(mapper.get_mappings(DatabaseDialect::SQLite, DatabaseDialect::MySQL).is_some());
    }

    #[cfg(feature = "postgres")]
    #[test]
    fn test_sqlite_to_postgres_mapping() {
        let mapper = CrossDatabaseTypeMapper::new();
        
        // Test INTEGER mapping
        let mapping = mapper.find_mapping(DatabaseDialect::SQLite, DatabaseDialect::PostgreSQL, "INTEGER");
        assert!(mapping.is_some());
        let mapping = mapping.unwrap();
        assert_eq!(mapping.target_type, "INTEGER");
        assert_eq!(mapping.compatibility, CompatibilityLevel::Perfect);
    }

    #[cfg(feature = "mysql")]
    #[test]
    fn test_sqlite_to_mysql_mapping() {
        let mapper = CrossDatabaseTypeMapper::new();
        
        // Test TEXT mapping
        let mapping = mapper.find_mapping(DatabaseDialect::SQLite, DatabaseDialect::MySQL, "TEXT");
        assert!(mapping.is_some());
        assert_eq!(mapping.unwrap().target_type, "TEXT");
    }

    #[test]
    fn test_unified_type_mapping() {
        let mapper = CrossDatabaseTypeMapper::new();
        
        // Test mapping to unified type
        let unified = mapper.map_to_unified(DatabaseDialect::SQLite, "INTEGER");
        assert_eq!(unified, Some(UnifiedColumnType::Integer));
        
        let unified = mapper.map_to_unified(DatabaseDialect::SQLite, "TEXT");
        assert_eq!(unified, Some(UnifiedColumnType::Text));
        
        let unified = mapper.map_to_unified(DatabaseDialect::SQLite, "BLOB");
        assert_eq!(unified, Some(UnifiedColumnType::Blob));
    }

    #[test]
    fn test_unified_to_database_mapping() {
        let mapper = CrossDatabaseTypeMapper::new();
        
        // Test mapping from unified type
        let db_type = mapper.map_from_unified(DatabaseDialect::SQLite, &UnifiedColumnType::Boolean);
        assert_eq!(db_type, Some("INTEGER".to_string()));
        
        let db_type = mapper.map_from_unified(DatabaseDialect::SQLite, &UnifiedColumnType::Json);
        assert_eq!(db_type, Some("TEXT".to_string()));
    }

    #[cfg(feature = "postgres")]
    #[test]
    fn test_postgres_feature_support() {
        let mapper = CrossDatabaseTypeMapper::new();
        let features = mapper.get_feature_support(DatabaseDialect::PostgreSQL).unwrap();
        
        assert_eq!(features.json_support, JsonSupportLevel::Binary);
        assert!(features.uuid_native_support);
        assert!(features.array_support);
        assert!(features.spatial_support);
        assert_eq!(features.max_identifier_length, 63);
    }

    #[cfg(feature = "mysql")]
    #[test]
    fn test_mysql_feature_support() {
        let mapper = CrossDatabaseTypeMapper::new();
        let features = mapper.get_feature_support(DatabaseDialect::MySQL).unwrap();
        
        assert_eq!(features.json_support, JsonSupportLevel::Native);
        assert!(!features.uuid_native_support);
        assert!(!features.array_support);
        assert!(features.spatial_support);
        assert_eq!(features.max_identifier_length, 64);
        assert!(!features.partial_indexes);
        assert!(!features.deferrable_constraints);
    }

    #[test]
    fn test_type_support_checking() {
        let mapper = CrossDatabaseTypeMapper::new();
        
        // SQLite supports basic types
        assert!(mapper.is_type_supported(DatabaseDialect::SQLite, &UnifiedColumnType::Integer));
        assert!(mapper.is_type_supported(DatabaseDialect::SQLite, &UnifiedColumnType::Text));
        
        // SQLite doesn't natively support UUID
        assert!(!mapper.is_type_supported(DatabaseDialect::SQLite, &UnifiedColumnType::Uuid));
    }

    #[cfg(feature = "postgres")]
    #[test]
    fn test_conversion_validation() {
        let mapper = CrossDatabaseTypeMapper::new();
        
        // Valid conversion
        let result = mapper.validate_conversion(DatabaseDialect::SQLite, DatabaseDialect::PostgreSQL, "INTEGER");
        assert!(result.is_valid);
        assert_eq!(result.compatibility, CompatibilityLevel::Perfect);
        
        // Invalid conversion (non-existent type)
        let result = mapper.validate_conversion(DatabaseDialect::SQLite, DatabaseDialect::PostgreSQL, "NONEXISTENT_TYPE");
        assert!(!result.is_valid);
        assert_eq!(result.compatibility, CompatibilityLevel::Incompatible);
    }

    #[cfg(feature = "postgres")]
    #[test]
    fn test_boolean_mapping_specifics() {
        let mapper = CrossDatabaseTypeMapper::new();
        
        // SQLite boolean mapping
        let mapping = mapper.find_mapping(DatabaseDialect::SQLite, DatabaseDialect::PostgreSQL, "BOOLEAN");
        assert!(mapping.is_some());
        let mapping = mapping.unwrap();
        assert_eq!(mapping.unified_type, UnifiedColumnType::Boolean);
        assert!(mapping.default_mapping.is_some());
        assert!(!mapping.notes.is_empty());
    }

    #[cfg(feature = "mysql")]
    #[test]
    fn test_datetime_mapping_specifics() {
        let mapper = CrossDatabaseTypeMapper::new();
        
        // DateTime mapping to MySQL
        let mapping = mapper.find_mapping(DatabaseDialect::SQLite, DatabaseDialect::MySQL, "DATETIME");
        assert!(mapping.is_some());
        let mapping = mapping.unwrap();
        assert_eq!(mapping.unified_type, UnifiedColumnType::DateTime);
        assert_eq!(mapping.target_type, "DATETIME");
        assert!(mapping.default_mapping.is_some());
    }

    #[test]
    fn test_constraint_mappings() {
        let mapper = CrossDatabaseTypeMapper::new();
        
        // SQLite constraints
        let constraints = mapper.get_constraint_mappings(DatabaseDialect::SQLite);
        assert!(constraints.is_some());
        let constraints = constraints.unwrap();
        assert_eq!(constraints.primary_key_syntax, "PRIMARY KEY");
        assert_eq!(constraints.auto_increment_syntax, "AUTOINCREMENT");
        
        // PostgreSQL constraints
        #[cfg(feature = "postgres")]
        {
            let constraints = mapper.get_constraint_mappings(DatabaseDialect::PostgreSQL);
            assert!(constraints.is_some());
            let constraints = constraints.unwrap();
            assert_eq!(constraints.auto_increment_syntax, "GENERATED BY DEFAULT AS IDENTITY");
        }
    }

    #[test]
    fn test_pattern_matching() {
        let mapper = CrossDatabaseTypeMapper::new();
        
        // Test pattern matching for complex types
        let unified = mapper.map_to_unified(DatabaseDialect::SQLite, "VARCHAR(255)");
        assert_eq!(unified, Some(UnifiedColumnType::Text));
        
        let unified = mapper.map_to_unified(DatabaseDialect::SQLite, "FLOAT");
        assert_eq!(unified, Some(UnifiedColumnType::Real));
    }

    #[cfg(feature = "postgres")]
    #[test]
    fn test_type_mapping_analyzer_postgres() {
        let analyzer = TypeMappingAnalyzer::new();
        
        // Analyze single mapping
        let analysis = analyzer.analyze_mapping(DatabaseDialect::SQLite, DatabaseDialect::PostgreSQL, "INTEGER");
        assert_eq!(analysis.target_type, Some("INTEGER".to_string()));
        assert_eq!(analysis.unified_type, Some(UnifiedColumnType::Integer));
        assert_eq!(analysis.compatibility, CompatibilityLevel::Perfect);
    }

    #[cfg(feature = "mysql")]
    #[test]
    fn test_type_mapping_analyzer_mysql() {
        let analyzer = TypeMappingAnalyzer::new();
        
        // Analyze all mappings
        let analyses = analyzer.analyze_all_mappings(DatabaseDialect::SQLite, DatabaseDialect::MySQL);
        assert!(!analyses.is_empty());
        assert!(analyses.iter().any(|a| a.source_type == "INTEGER"));
        assert!(analyses.iter().any(|a| a.source_type == "TEXT"));
    }

    #[test]
    fn test_fallback_mapping() {
        let mapper = CrossDatabaseTypeMapper::new();
        
        // Test fallback mapping for uncommon types
        let db_type = mapper.map_from_unified(DatabaseDialect::SQLite, &UnifiedColumnType::Other("CUSTOM".to_string()));
        assert_eq!(db_type, None);
        
        // Test known fallback mappings
        let db_type = mapper.map_from_unified(DatabaseDialect::SQLite, &UnifiedColumnType::Uuid);
        assert_eq!(db_type, Some("TEXT".to_string()));
    }

    #[test]
    fn test_compatibility_level_display() {
        assert_eq!(CompatibilityLevel::Perfect.to_string(), "Perfect");
        assert_eq!(CompatibilityLevel::Compatible.to_string(), "Compatible");
        assert_eq!(CompatibilityLevel::LossyCompatible.to_string(), "Lossy Compatible");
        assert_eq!(CompatibilityLevel::FunctionalDifferences.to_string(), "Functional Differences");
        assert_eq!(CompatibilityLevel::Incompatible.to_string(), "Incompatible");
    }

    #[test]
    fn test_json_support_level_display() {
        assert_eq!(JsonSupportLevel::None.to_string(), "None");
        assert_eq!(JsonSupportLevel::TextBased.to_string(), "Text-based");
        assert_eq!(JsonSupportLevel::Native.to_string(), "Native");
        assert_eq!(JsonSupportLevel::Binary.to_string(), "Binary");
    }

    #[test]
    fn test_precision_info_structure() {
        let precision = PrecisionInfo {
            max_precision: Some(10),
            max_scale: Some(2),
            min_precision: Some(1),
            default_precision: Some(5),
            precision_required: true,
        };
        
        assert_eq!(precision.max_precision, Some(10));
        assert_eq!(precision.max_scale, Some(2));
        assert!(precision.precision_required);
    }

    #[test]
    fn test_type_constraints() {
        let max_length = TypeConstraint::MaxLength(255);
        let numeric_range = TypeConstraint::NumericRange { min: Some(-100), max: Some(100) };
        let charset = TypeConstraint::CharsetRequired("utf8".to_string());
        
        assert!(matches!(max_length, TypeConstraint::MaxLength(255)));
        assert!(matches!(numeric_range, TypeConstraint::NumericRange { .. }));
        assert!(matches!(charset, TypeConstraint::CharsetRequired(_)));
    }

    #[test]
    fn test_default_mapping_structure() {
        let default_mapping = DefaultMapping {
            source_default: "NOW()".to_string(),
            target_default: "CURRENT_TIMESTAMP".to_string(),
            is_lossy: false,
            notes: Some("Function mapping".to_string()),
        };
        
        assert_eq!(default_mapping.source_default, "NOW()");
        assert_eq!(default_mapping.target_default, "CURRENT_TIMESTAMP");
        assert!(!default_mapping.is_lossy);
        assert!(default_mapping.notes.is_some());
    }

    #[test]
    fn test_validation_result_structure() {
        let result = ValidationResult {
            is_valid: true,
            compatibility: CompatibilityLevel::Compatible,
            warnings: vec!["Minor differences".to_string()],
            recommendations: vec!["Test thoroughly".to_string()],
        };
        
        assert!(result.is_valid);
        assert_eq!(result.compatibility, CompatibilityLevel::Compatible);
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.recommendations.len(), 1);
    }
}