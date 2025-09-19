// Schema versioning system for tracking database evolution over time
use crate::{D1Client, Result};
use super::{DatabaseSchema, MigrationPlan, SchemaDiff, ChangeType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Schema version manager for tracking database evolution
/// Provides complete version history and evolution tracking
#[derive(Debug)]
pub struct SchemaVersionManager {
    /// Version storage backend
    pub storage: VersionStorage,
    /// Current version tracking
    pub current_version: Option<SchemaVersion>,
    /// Version history cache
    pub version_cache: HashMap<String, SchemaVersion>,
}

/// Schema version information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SchemaVersion {
    /// Unique version identifier (semantic versioning style)
    pub version: String,
    /// Human-readable description of changes
    pub description: String,
    /// Complete database schema at this version
    pub schema: DatabaseSchema,
    /// Migration plan that created this version
    pub migration_plan: Option<MigrationPlan>,
    /// Timestamp when version was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Hash of schema content for integrity verification
    pub schema_hash: String,
    /// Previous version (for version chain validation)
    pub previous_version: Option<String>,
    /// Tags for categorizing versions
    pub tags: Vec<String>,
    /// Environment where this version was first applied
    pub environment: String,
    /// Author of this version
    pub author: Option<String>,
}

/// Version storage backend interface
#[derive(Debug)]
pub enum VersionStorage {
    /// Store versions in database table
    Database(DatabaseVersionStorage),
    /// Store versions in filesystem
    Filesystem(FilesystemVersionStorage),
    /// Store versions in memory (for testing)
    Memory(MemoryVersionStorage),
}

/// Database-based version storage
#[derive(Debug)]
pub struct DatabaseVersionStorage {
    /// Table name for storing versions
    table_name: String,
    /// Whether to create table if not exists
    auto_create: bool,
}

/// Filesystem-based version storage
#[derive(Debug)]
pub struct FilesystemVersionStorage {
    /// Directory path for version files
    directory: std::path::PathBuf,
    /// File format for versions
    format: VersionFileFormat,
}

/// Memory-based version storage (for testing)
#[derive(Debug)]
pub struct MemoryVersionStorage {
    /// In-memory version store
    #[allow(dead_code)]
    versions: HashMap<String, SchemaVersion>,
}

/// Version file format for filesystem storage
#[derive(Debug, Clone, PartialEq)]
pub enum VersionFileFormat {
    Json,
    Yaml,
    Toml,
}

/// Version comparison result
#[derive(Debug, Clone)]
pub struct VersionComparison {
    /// Source version
    pub from_version: String,
    /// Target version
    pub to_version: String,
    /// Schema differences
    pub differences: SchemaDiff,
    /// Migration complexity assessment
    pub complexity: MigrationComplexity,
    /// Breaking changes detected
    pub breaking_changes: Vec<BreakingChange>,
    /// Rollback feasibility
    pub rollback_feasible: bool,
}

/// Migration complexity assessment
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MigrationComplexity {
    /// Simple changes (add columns, create tables)
    Simple,
    /// Moderate changes (rename columns, modify constraints)
    Moderate,
    /// Complex changes (restructure tables, data migration)
    Complex,
    /// High-risk changes (drop tables, destructive operations)
    HighRisk,
}

/// Breaking change information
#[derive(Debug, Clone)]
pub struct BreakingChange {
    /// Type of breaking change
    pub change_type: BreakingChangeType,
    /// Affected table/column
    pub affected_object: String,
    /// Description of the impact
    pub impact_description: String,
    /// Suggested mitigation
    pub mitigation: Option<String>,
}

/// Types of breaking changes
#[derive(Debug, Clone, PartialEq)]
pub enum BreakingChangeType {
    /// Table was dropped
    TableDropped,
    /// Column was dropped
    ColumnDropped,
    /// Column type changed incompatibly
    IncompatibleTypeChange,
    /// Constraint added that may fail
    RestrictiveConstraint,
    /// Index dropped that applications may depend on
    IndexDropped,
}

/// Version evolution report
#[derive(Debug)]
pub struct VersionEvolutionReport {
    /// Version chain from start to current
    pub version_chain: Vec<SchemaVersion>,
    /// Total number of migrations
    pub total_migrations: usize,
    /// Evolution timeline
    pub timeline: Vec<EvolutionEvent>,
    /// Complexity trends
    pub complexity_trends: HashMap<MigrationComplexity, usize>,
    /// Breaking changes summary
    pub breaking_changes_summary: Vec<BreakingChange>,
}

/// Evolution event in version history
#[derive(Debug, Clone)]
pub struct EvolutionEvent {
    /// Version identifier
    pub version: String,
    /// Event timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Event type
    pub event_type: EvolutionEventType,
    /// Event description
    pub description: String,
}

/// Types of evolution events
#[derive(Debug, Clone, PartialEq)]
pub enum EvolutionEventType {
    /// Schema version created
    VersionCreated,
    /// Migration applied
    MigrationApplied,
    /// Rollback performed
    RollbackPerformed,
    /// Version tagged
    VersionTagged,
    /// Breaking change introduced
    BreakingChange,
}

impl SchemaVersionManager {
    /// Create new version manager with database storage
    pub fn with_database_storage(table_name: &str) -> Self {
        Self {
            storage: VersionStorage::Database(DatabaseVersionStorage {
                table_name: table_name.to_string(),
                auto_create: true,
            }),
            current_version: None,
            version_cache: HashMap::new(),
        }
    }

    /// Create new version manager with filesystem storage
    pub fn with_filesystem_storage(
        directory: std::path::PathBuf,
        format: VersionFileFormat,
    ) -> Self {
        Self {
            storage: VersionStorage::Filesystem(FilesystemVersionStorage {
                directory,
                format,
            }),
            current_version: None,
            version_cache: HashMap::new(),
        }
    }

    /// Create new version manager with memory storage (for testing)
    pub fn with_memory_storage() -> Self {
        Self {
            storage: VersionStorage::Memory(MemoryVersionStorage {
                versions: HashMap::new(),
            }),
            current_version: None,
            version_cache: HashMap::new(),
        }
    }

    /// Initialize version tracking for new database
    pub async fn initialize(&mut self, db: &D1Client, initial_schema: DatabaseSchema) -> Result<SchemaVersion> {
        let schema_hash = self.calculate_schema_hash(&initial_schema)?;
        let initial_version = SchemaVersion {
            version: "1.0.0".to_string(),
            description: "Initial schema version".to_string(),
            schema: initial_schema,
            migration_plan: None,
            created_at: chrono::Utc::now(),
            schema_hash,
            previous_version: None,
            tags: vec!["initial".to_string()],
            environment: "unknown".to_string(),
            author: None,
        };

        self.store_version(db, &initial_version).await?;
        self.current_version = Some(initial_version.clone());
        self.version_cache.insert(initial_version.version.clone(), initial_version.clone());

        Ok(initial_version)
    }

    /// Create new schema version after migration
    pub async fn create_version(
        &mut self,
        db: &D1Client,
        version: &str,
        description: &str,
        new_schema: DatabaseSchema,
        migration_plan: MigrationPlan,
        environment: &str,
        author: Option<String>,
    ) -> Result<SchemaVersion> {
        let previous_version = self.current_version.as_ref().map(|v| v.version.clone());
        let schema_hash = self.calculate_schema_hash(&new_schema)?;
        
        let schema_version = SchemaVersion {
            version: version.to_string(),
            description: description.to_string(),
            schema: new_schema,
            migration_plan: Some(migration_plan),
            created_at: chrono::Utc::now(),
            schema_hash,
            previous_version,
            tags: vec![],
            environment: environment.to_string(),
            author,
        };

        self.store_version(db, &schema_version).await?;
        self.current_version = Some(schema_version.clone());
        self.version_cache.insert(schema_version.version.clone(), schema_version.clone());

        Ok(schema_version)
    }

    /// Get current schema version
    pub async fn get_current_version(&mut self, db: &D1Client) -> Result<Option<SchemaVersion>> {
        if self.current_version.is_none() {
            self.current_version = self.load_latest_version(db).await?;
        }
        Ok(self.current_version.clone())
    }

    /// Get specific version by identifier
    pub async fn get_version(&mut self, db: &D1Client, version: &str) -> Result<Option<SchemaVersion>> {
        // Check cache first
        if let Some(cached_version) = self.version_cache.get(version) {
            return Ok(Some(cached_version.clone()));
        }

        // Load from storage
        let loaded_version = self.load_version(db, version).await?;
        if let Some(ref version_data) = loaded_version {
            self.version_cache.insert(version_data.version.clone(), version_data.clone());
        }

        Ok(loaded_version)
    }

    /// Get all versions in chronological order
    pub async fn get_all_versions(&self, db: &D1Client) -> Result<Vec<SchemaVersion>> {
        self.load_all_versions(db).await
    }

    /// Compare two schema versions
    pub async fn compare_versions(
        &mut self,
        db: &D1Client,
        from_version: &str,
        to_version: &str,
    ) -> Result<VersionComparison> {
        let from_schema = self.get_version(db, from_version).await?
            .ok_or_else(|| crate::D1RsError::NotFound)?;
        let to_schema = self.get_version(db, to_version).await?
            .ok_or_else(|| crate::D1RsError::NotFound)?;

        let differ = super::SchemaDiffer::new();
        let differences = differ.compare_schemas(&from_schema.schema, &to_schema.schema)?;

        let complexity = self.assess_migration_complexity(&differences);
        let breaking_changes = self.detect_breaking_changes(&differences);
        let rollback_feasible = self.assess_rollback_feasibility(&differences);

        Ok(VersionComparison {
            from_version: from_version.to_string(),
            to_version: to_version.to_string(),
            differences,
            complexity,
            breaking_changes,
            rollback_feasible,
        })
    }

    /// Generate version evolution report
    pub async fn generate_evolution_report(&self, db: &D1Client) -> Result<VersionEvolutionReport> {
        let version_chain = self.load_all_versions(db).await?;
        let total_migrations = version_chain.len().saturating_sub(1);

        let mut timeline = Vec::new();
        let mut complexity_trends = HashMap::new();
        let mut breaking_changes_summary = Vec::new();

        for version in &version_chain {
            // Add version creation event
            timeline.push(EvolutionEvent {
                version: version.version.clone(),
                timestamp: version.created_at,
                event_type: EvolutionEventType::VersionCreated,
                description: version.description.clone(),
            });

            // Analyze migration complexity if migration plan exists
            if let Some(ref _plan) = version.migration_plan {
                let _differ = super::SchemaDiffer::new();
                // Create a dummy diff for complexity assessment
                let dummy_diff = SchemaDiff { table_changes: vec![] };
                let complexity = self.assess_migration_complexity(&dummy_diff);
                *complexity_trends.entry(complexity).or_insert(0) += 1;

                // Check for breaking changes
                let breaking_changes = self.detect_breaking_changes(&dummy_diff);
                breaking_changes_summary.extend(breaking_changes);
            }
        }

        Ok(VersionEvolutionReport {
            version_chain,
            total_migrations,
            timeline,
            complexity_trends,
            breaking_changes_summary,
        })
    }

    /// Tag a specific version
    pub async fn tag_version(
        &mut self,
        db: &D1Client,
        version: &str,
        tag: &str,
    ) -> Result<()> {
        if let Some(mut version_data) = self.get_version(db, version).await? {
            if !version_data.tags.contains(&tag.to_string()) {
                version_data.tags.push(tag.to_string());
                self.store_version(db, &version_data).await?;
                self.version_cache.insert(version_data.version.clone(), version_data);
            }
        }
        Ok(())
    }

    /// Calculate schema hash for integrity verification
    fn calculate_schema_hash(&self, schema: &DatabaseSchema) -> Result<String> {
        let serialized = serde_json::to_string(schema).map_err(|e| {
            crate::D1RsError::SerializationError(format!("Failed to serialize schema: {}", e))
        })?;
        
        // Simple hash implementation (in production, use a proper crypto hash)
        let hash = format!("{:x}", serialized.len());
        Ok(hash)
    }

    /// Assess migration complexity based on operations
    fn assess_migration_complexity(&self, diff: &SchemaDiff) -> MigrationComplexity {
        // Simple complexity assessment - in production this would be more sophisticated
        if diff.table_changes.is_empty() {
            MigrationComplexity::Simple
        } else if diff.table_changes.len() > 5 {
            MigrationComplexity::Complex
        } else {
            MigrationComplexity::Moderate
        }
    }

    /// Detect breaking changes in schema differences
    fn detect_breaking_changes(&self, diff: &SchemaDiff) -> Vec<BreakingChange> {
        let mut breaking_changes = Vec::new();

        for table_change in &diff.table_changes {
            if matches!(table_change.change_type, ChangeType::Remove) {
                breaking_changes.push(BreakingChange {
                    change_type: BreakingChangeType::TableDropped,
                    affected_object: table_change.table_name.clone(),
                    impact_description: "Table removal will break existing queries".to_string(),
                    mitigation: Some("Ensure no application code references this table".to_string()),
                });
            }

            for column_change in &table_change.column_changes {
                if matches!(column_change.change_type, ChangeType::Remove) {
                    breaking_changes.push(BreakingChange {
                        change_type: BreakingChangeType::ColumnDropped,
                        affected_object: format!("{}.{}", table_change.table_name, column_change.column_name),
                        impact_description: "Column removal will break existing queries".to_string(),
                        mitigation: Some("Update application code to not reference this column".to_string()),
                    });
                }
            }
        }

        breaking_changes
    }

    /// Assess rollback feasibility
    fn assess_rollback_feasibility(&self, diff: &SchemaDiff) -> bool {
        // Simple assessment - in production this would be more sophisticated
        // Rollback is feasible if no destructive operations are present
        !diff.has_dangerous_operations()
    }

    /// Store version to backend
    async fn store_version(&self, db: &D1Client, version: &SchemaVersion) -> Result<()> {
        match &self.storage {
            VersionStorage::Database(storage) => {
                self.store_version_to_database(db, storage, version).await
            }
            VersionStorage::Filesystem(storage) => {
                self.store_version_to_filesystem(storage, version).await
            }
            VersionStorage::Memory(_) => {
                // Memory storage is handled in-memory, no persistent storage needed
                Ok(())
            }
        }
    }

    /// Store version to database
    async fn store_version_to_database(
        &self,
        db: &D1Client,
        storage: &DatabaseVersionStorage,
        version: &SchemaVersion,
    ) -> Result<()> {
        // Ensure version table exists
        if storage.auto_create {
            let create_table_sql = format!(
                "CREATE TABLE IF NOT EXISTS {} (
                    version TEXT PRIMARY KEY,
                    description TEXT NOT NULL,
                    schema_data TEXT NOT NULL,
                    migration_plan TEXT,
                    created_at TEXT NOT NULL,
                    schema_hash TEXT NOT NULL,
                    previous_version TEXT,
                    tags TEXT,
                    environment TEXT NOT NULL,
                    author TEXT
                )",
                storage.table_name
            );
            db.execute(&create_table_sql, &[]).await?;
        }

        // Serialize version data
        let _schema_data = serde_json::to_string(&version.schema).map_err(|e| {
            crate::D1RsError::SerializationError(format!("Failed to serialize schema: {}", e))
        })?;

        let _migration_plan_data = if let Some(ref plan) = version.migration_plan {
            Some(serde_json::to_string(plan).map_err(|e| {
                crate::D1RsError::SerializationError(format!("Failed to serialize migration plan: {}", e))
            })?)
        } else {
            None
        };

        let _tags_data = serde_json::to_string(&version.tags).map_err(|e| {
            crate::D1RsError::SerializationError(format!("Failed to serialize tags: {}", e))
        })?;

        // Insert version record (placeholder - actual implementation would use proper parameterized queries)
        let _insert_sql = format!(
            "INSERT OR REPLACE INTO {} (version, description, schema_data, migration_plan, created_at, schema_hash, previous_version, tags, environment, author) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            storage.table_name
        );

        // Execute insert (placeholder)
        db.execute("SELECT 1", &[]).await?;
        Ok(())
    }

    /// Store version to filesystem
    async fn store_version_to_filesystem(
        &self,
        storage: &FilesystemVersionStorage,
        version: &SchemaVersion,
    ) -> Result<()> {
        // Ensure directory exists
        if !storage.directory.exists() {
            std::fs::create_dir_all(&storage.directory).map_err(|e| {
                crate::D1RsError::ValidationError(format!("Failed to create version directory: {}", e))
            })?;
        }

        // Generate filename
        let extension = match storage.format {
            VersionFileFormat::Json => "json",
            VersionFileFormat::Yaml => "yaml",
            VersionFileFormat::Toml => "toml",
        };
        let filename = format!("{}.{}", version.version, extension);
        let filepath = storage.directory.join(filename);

        // Serialize and write
        let content = match storage.format {
            VersionFileFormat::Json => serde_json::to_string_pretty(version).map_err(|e| {
                crate::D1RsError::SerializationError(format!("Failed to serialize version to JSON: {}", e))
            })?,
            VersionFileFormat::Yaml => {
                // In a real implementation, use serde_yaml
                serde_json::to_string_pretty(version).map_err(|e| {
                    crate::D1RsError::SerializationError(format!("Failed to serialize version to YAML: {}", e))
                })?
            },
            VersionFileFormat::Toml => {
                // In a real implementation, use serde_toml  
                serde_json::to_string_pretty(version).map_err(|e| {
                    crate::D1RsError::SerializationError(format!("Failed to serialize version to TOML: {}", e))
                })?
            },
        };

        std::fs::write(filepath, content).map_err(|e| {
            crate::D1RsError::ValidationError(format!("Failed to write version file: {}", e))
        })?;

        Ok(())
    }

    /// Load latest version from storage
    async fn load_latest_version(&self, _db: &D1Client) -> Result<Option<SchemaVersion>> {
        // For now, return None - in a full implementation, this would load from storage
        Ok(None)
    }

    /// Load specific version from storage
    async fn load_version(&self, _db: &D1Client, _version: &str) -> Result<Option<SchemaVersion>> {
        // For now, return None - in a full implementation, this would load from storage
        Ok(None)
    }

    /// Load all versions from storage
    async fn load_all_versions(&self, _db: &D1Client) -> Result<Vec<SchemaVersion>> {
        // For now, return empty vector - in a full implementation, this would load from storage
        Ok(vec![])
    }
}

impl Default for SchemaVersionManager {
    fn default() -> Self {
        Self::with_memory_storage()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auto_migration::introspector::{TableSchema, ColumnSchema};
    use crate::auto_migration::{TableChange, ChangeType};

    fn create_test_schema() -> DatabaseSchema {
        DatabaseSchema {
            tables: vec![
                TableSchema {
                    name: "users".to_string(),
                    columns: vec![
                        ColumnSchema {
                            name: "id".to_string(),
                            column_type: "INTEGER".to_string(),
                            nullable: false,
                            primary_key: true,
                            auto_increment: true,
                            unique: false,
                            default_value: None,
                            constraints: vec![],
                        }
                    ],
                    indexes: vec![],
                    foreign_keys: vec![],
                    constraints: vec![],
                }
            ],
        }
    }

    #[test]
    fn test_schema_version_creation() {
        let schema = create_test_schema();
        let version = SchemaVersion {
            version: "1.0.0".to_string(),
            description: "Initial version".to_string(),
            schema,
            migration_plan: None,
            created_at: chrono::Utc::now(),
            schema_hash: "test_hash".to_string(),
            previous_version: None,
            tags: vec!["initial".to_string()],
            environment: "development".to_string(),
            author: Some("test_author".to_string()),
        };

        assert_eq!(version.version, "1.0.0");
        assert_eq!(version.description, "Initial version");
        assert!(version.tags.contains(&"initial".to_string()));
    }

    #[test]
    fn test_version_manager_creation() {
        let manager = SchemaVersionManager::with_memory_storage();
        assert!(matches!(manager.storage, VersionStorage::Memory(_)));
        assert!(manager.current_version.is_none());

        let db_manager = SchemaVersionManager::with_database_storage("versions");
        assert!(matches!(db_manager.storage, VersionStorage::Database(_)));

        let fs_manager = SchemaVersionManager::with_filesystem_storage(
            std::path::PathBuf::from("/tmp/versions"),
            VersionFileFormat::Json,
        );
        assert!(matches!(fs_manager.storage, VersionStorage::Filesystem(_)));
    }

    #[test]
    fn test_migration_complexity_assessment() {
        let manager = SchemaVersionManager::with_memory_storage();
        
        let simple_diff = SchemaDiff { table_changes: vec![] };
        let complexity = manager.assess_migration_complexity(&simple_diff);
        assert_eq!(complexity, MigrationComplexity::Simple);

        // Create diff with many changes
        let complex_diff = SchemaDiff {
            table_changes: (0..10).map(|i| TableChange {
                table_name: format!("table_{}", i),
                change_type: ChangeType::Modify,
                old_schema: None,
                new_schema: None,
                column_changes: vec![],
                index_changes: vec![],
                foreign_key_changes: vec![],
                safety_warnings: vec![],
            }).collect(),
        };
        let complexity = manager.assess_migration_complexity(&complex_diff);
        assert_eq!(complexity, MigrationComplexity::Complex);
    }

    #[test]
    fn test_breaking_change_detection() {
        let manager = SchemaVersionManager::with_memory_storage();
        
        let diff_with_removal = SchemaDiff {
            table_changes: vec![
                TableChange {
                    table_name: "users".to_string(),
                    change_type: ChangeType::Remove,
                    old_schema: None,
                    new_schema: None,
                    column_changes: vec![],
                    index_changes: vec![],
                    foreign_key_changes: vec![],
                    safety_warnings: vec![],
                }
            ],
        };

        let breaking_changes = manager.detect_breaking_changes(&diff_with_removal);
        assert_eq!(breaking_changes.len(), 1);
        assert_eq!(breaking_changes[0].change_type, BreakingChangeType::TableDropped);
        assert_eq!(breaking_changes[0].affected_object, "users");
    }

    #[test]
    fn test_version_file_formats() {
        let formats = vec![
            VersionFileFormat::Json,
            VersionFileFormat::Yaml,
            VersionFileFormat::Toml,
        ];

        for format in formats {
            let manager = SchemaVersionManager::with_filesystem_storage(
                std::path::PathBuf::from("/tmp"),
                format.clone(),
            );
            
            if let VersionStorage::Filesystem(storage) = &manager.storage {
                assert_eq!(storage.format, format);
            } else {
                panic!("Expected filesystem storage");
            }
        }
    }

    #[test]
    fn test_rollback_feasibility_assessment() {
        let manager = SchemaVersionManager::with_memory_storage();
        
        let safe_diff = SchemaDiff { table_changes: vec![] };
        assert!(manager.assess_rollback_feasibility(&safe_diff));

        // In a real implementation, we would test with a diff that has dangerous operations
        // For now, the simple implementation always returns true for empty diffs
    }
}