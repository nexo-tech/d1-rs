// Phase 3.2.4 - Migration Snapshots - Save schema states for rollback

use crate::Result;
use crate::auto_migration::{DatabaseSchema, MigrationPlan};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Complete snapshot of database state for rollback purposes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MigrationSnapshot {
    /// Unique identifier for this snapshot
    pub id: String,
    
    /// Human-readable description of the snapshot
    pub description: String,
    
    /// Complete database schema at time of snapshot
    pub schema: DatabaseSchema,
    
    /// Migration plan that was applied to reach this state
    pub applied_migration: Option<MigrationPlan>,
    
    /// Timestamp when snapshot was created
    pub created_at: DateTime<Utc>,
    
    /// Environment this snapshot was created in
    pub environment: SnapshotEnvironment,
    
    /// Type of snapshot (automatic, manual, pre-migration, etc.)
    pub snapshot_type: SnapshotType,
    
    /// Data snapshots for small reference tables
    pub data_snapshots: HashMap<String, Vec<SnapshotRow>>,
    
    /// Metadata about the snapshot
    pub metadata: SnapshotMetadata,
    
    /// Tags for organizing snapshots
    pub tags: Vec<String>,
    
    /// Size of the snapshot in bytes
    pub size_bytes: u64,
    
    /// Checksum for integrity verification
    pub checksum: String,
}

/// Environment where snapshot was created
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SnapshotEnvironment {
    Development,
    Testing,
    Staging,
    Production,
    Custom(String),
}

/// Type of snapshot
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SnapshotType {
    /// Automatically created before migration
    PreMigration,
    /// Automatically created after successful migration
    PostMigration,
    /// Manually created by user
    Manual,
    /// Created during rollback operation
    Rollback,
    /// Periodic backup snapshot
    Scheduled,
    /// Created before risky operation
    Safety,
}

/// Data row in a snapshot
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SnapshotRow {
    pub columns: HashMap<String, SnapshotValue>,
}

/// Value in a snapshot row
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SnapshotValue {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

/// Metadata about the snapshot
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SnapshotMetadata {
    /// Version of the migration system that created this snapshot
    pub migration_system_version: String,
    
    /// Database engine version
    pub database_version: String,
    
    /// Number of tables in the snapshot
    pub table_count: usize,
    
    /// Total number of data rows included
    pub total_rows: u64,
    
    /// Compression used (if any)
    pub compression: Option<String>,
    
    /// Additional custom metadata
    pub custom: HashMap<String, String>,
}

/// Configuration for snapshot creation
#[derive(Debug, Clone)]
pub struct SnapshotConfig {
    /// Maximum size of tables to include data snapshots for
    pub max_data_table_rows: u64,
    
    /// Tables to always include data for (regardless of size)
    pub always_include_data: Vec<String>,
    
    /// Tables to never include data for
    pub exclude_data: Vec<String>,
    
    /// Enable compression for snapshots
    pub enable_compression: bool,
    
    /// Retention policy for automatic snapshots
    pub retention_policy: RetentionPolicy,
    
    /// Storage backend configuration
    pub storage_config: StorageConfig,
}

/// Retention policy for snapshots
#[derive(Debug, Clone)]
pub struct RetentionPolicy {
    /// Maximum number of snapshots to keep
    pub max_snapshots: Option<usize>,
    
    /// Maximum age of snapshots to keep
    pub max_age_days: Option<u64>,
    
    /// Always keep snapshots with these tags
    pub preserve_tags: Vec<String>,
    
    /// Special retention for production snapshots
    pub production_retention_days: Option<u64>,
}

/// Storage backend configuration
#[derive(Debug, Clone)]
pub enum StorageConfig {
    /// Store snapshots in filesystem
    Filesystem { 
        base_path: PathBuf,
        create_subdirs: bool,
    },
    /// Store snapshots in database
    Database {
        table_name: String,
        compress_large_snapshots: bool,
    },
    /// Store snapshots in cloud storage (future)
    Cloud {
        provider: String,
        bucket: String,
        prefix: String,
    },
}

/// Result of rollback operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackResult {
    /// Snapshot that was rolled back to
    pub target_snapshot: String,
    
    /// Number of schema changes applied during rollback
    pub schema_changes_applied: usize,
    
    /// Number of data rows restored
    pub data_rows_restored: u64,
    
    /// Tables that had data restored
    pub restored_tables: Vec<String>,
    
    /// Time taken for rollback operation
    pub rollback_duration_ms: u64,
    
    /// Any warnings or non-critical issues during rollback
    pub warnings: Vec<String>,
    
    /// Snapshot created as backup before rollback
    pub backup_snapshot_id: Option<String>,
}

/// Migration snapshot manager
pub struct MigrationSnapshotManager {
    config: SnapshotConfig,
    snapshots: std::cell::RefCell<HashMap<String, MigrationSnapshot>>,
}

impl MigrationSnapshotManager {
    /// Create a new snapshot manager with configuration
    pub fn new(config: SnapshotConfig) -> Self {
        Self {
            config,
            snapshots: std::cell::RefCell::new(HashMap::new()),
        }
    }
    
    /// Create a snapshot of current database state
    pub async fn create_snapshot(
        &self,
        id: String,
        description: String,
        schema: DatabaseSchema,
        snapshot_type: SnapshotType,
        tags: Vec<String>,
    ) -> Result<MigrationSnapshot> {
        let snapshot = MigrationSnapshot {
            id: id.clone(),
            description,
            schema,
            applied_migration: None,
            created_at: Utc::now(),
            environment: SnapshotEnvironment::Development, // TODO: Detect from config
            snapshot_type,
            data_snapshots: HashMap::new(), // TODO: Implement data capture
            metadata: SnapshotMetadata {
                migration_system_version: "0.1.0".to_string(),
                database_version: "sqlite-3.x".to_string(),
                table_count: 0, // TODO: Calculate from schema
                total_rows: 0,
                compression: None,
                custom: HashMap::new(),
            },
            tags,
            size_bytes: 0, // TODO: Calculate actual size
            checksum: "placeholder".to_string(), // TODO: Calculate checksum
        };
        
        // Store the snapshot
        self.snapshots.borrow_mut().insert(id.clone(), snapshot.clone());
        
        // Apply retention policy
        self.apply_retention_policy().await?;
        
        Ok(snapshot)
    }
    
    /// Create automatic snapshot before migration
    pub async fn create_pre_migration_snapshot(
        &self,
        migration_id: &str,
        schema: DatabaseSchema,
    ) -> Result<MigrationSnapshot> {
        let snapshot_id = format!("pre_migration_{}", migration_id);
        let description = format!("Automatic snapshot before migration {}", migration_id);
        
        self.create_snapshot(
            snapshot_id,
            description,
            schema,
            SnapshotType::PreMigration,
            vec!["auto".to_string(), "pre-migration".to_string()],
        ).await
    }
    
    /// Create automatic snapshot after migration
    pub async fn create_post_migration_snapshot(
        &self,
        migration_id: &str,
        schema: DatabaseSchema,
        migration_plan: MigrationPlan,
    ) -> Result<MigrationSnapshot> {
        let snapshot_id = format!("post_migration_{}", migration_id);
        let description = format!("Automatic snapshot after migration {}", migration_id);
        
        let mut snapshot = self.create_snapshot(
            snapshot_id,
            description,
            schema,
            SnapshotType::PostMigration,
            vec!["auto".to_string(), "post-migration".to_string()],
        ).await?;
        
        snapshot.applied_migration = Some(migration_plan);
        
        // Update stored snapshot
        self.snapshots.borrow_mut().insert(snapshot.id.clone(), snapshot.clone());
        
        Ok(snapshot)
    }
    
    /// List all available snapshots
    pub fn list_snapshots(&self) -> Vec<MigrationSnapshot> {
        self.snapshots.borrow().values().cloned().collect()
    }
    
    /// Get a specific snapshot by ID
    pub fn get_snapshot(&self, id: &str) -> Option<MigrationSnapshot> {
        self.snapshots.borrow().get(id).cloned()
    }
    
    /// Delete a snapshot
    pub async fn delete_snapshot(&self, id: &str) -> Result<bool> {
        let removed = self.snapshots.borrow_mut().remove(id).is_some();
        Ok(removed)
    }
    
    /// Validate that rollback to snapshot is safe
    pub async fn validate_rollback(
        &self,
        snapshot_id: &str,
        current_schema: &DatabaseSchema,
    ) -> Result<RollbackValidation> {
        let snapshot = self.get_snapshot(snapshot_id)
            .ok_or_else(|| crate::D1RsError::AutoMigration(format!("Snapshot not found: {}", snapshot_id)))?;
        
        let mut validation = RollbackValidation {
            is_safe: true,
            warnings: Vec::new(),
            data_loss_risk: DataLossRisk::None,
            schema_changes_required: Vec::new(),
            estimated_duration_ms: 1000, // Placeholder
        };
        
        // Check for potential data loss
        for current_table in &current_schema.tables {
            if !snapshot.schema.tables.iter().any(|t| t.name == current_table.name) {
                validation.warnings.push(format!("Table '{}' will be lost in rollback", current_table.name));
                validation.data_loss_risk = DataLossRisk::High;
            }
        }
        
        // Check for column changes that might cause data loss
        for snapshot_table in &snapshot.schema.tables {
            if let Some(current_table) = current_schema.tables.iter().find(|t| t.name == snapshot_table.name) {
                for current_column in &current_table.columns {
                    if !snapshot_table.columns.iter().any(|c| c.name == current_column.name) {
                        validation.warnings.push(format!("Column '{}' in table '{}' will be lost", current_column.name, current_table.name));
                        if validation.data_loss_risk == DataLossRisk::None {
                            validation.data_loss_risk = DataLossRisk::Medium;
                        }
                    }
                }
            }
        }
        
        // If there are warnings, rollback might not be safe
        if !validation.warnings.is_empty() {
            validation.is_safe = false;
        }
        
        Ok(validation)
    }
    
    /// Perform rollback to a specific snapshot
    pub async fn rollback_to_snapshot(
        &self,
        snapshot_id: &str,
        current_schema: &DatabaseSchema,
        create_backup: bool,
    ) -> Result<RollbackResult> {
        // Validate rollback safety first
        let validation = self.validate_rollback(snapshot_id, current_schema).await?;
        if !validation.is_safe {
            return Err(crate::D1RsError::AutoMigration(
                format!("Rollback to {} is not safe: {:?}", snapshot_id, validation.warnings)
            ));
        }
        
        let _snapshot = self.get_snapshot(snapshot_id)
            .ok_or_else(|| crate::D1RsError::AutoMigration(format!("Snapshot not found: {}", snapshot_id)))?;
        
        let mut result = RollbackResult {
            target_snapshot: snapshot_id.to_string(),
            schema_changes_applied: 0,
            data_rows_restored: 0,
            restored_tables: Vec::new(),
            rollback_duration_ms: 0,
            warnings: validation.warnings,
            backup_snapshot_id: None,
        };
        
        // Create backup snapshot if requested
        if create_backup {
            let backup_id = format!("rollback_backup_{}", chrono::Utc::now().timestamp());
            let backup = self.create_snapshot(
                backup_id.clone(),
                format!("Backup before rollback to {}", snapshot_id),
                current_schema.clone(),
                SnapshotType::Rollback,
                vec!["backup".to_string(), "rollback".to_string()],
            ).await?;
            result.backup_snapshot_id = Some(backup.id);
        }
        
        // TODO: Implement actual rollback logic
        // This would involve:
        // 1. Generate migration plan from current schema to snapshot schema
        // 2. Execute schema changes (DROP tables, ALTER tables, etc.)
        // 3. Restore data from snapshot where available
        // 4. Validate final state matches snapshot
        
        Ok(result)
    }
    
    /// Apply retention policy to clean up old snapshots
    async fn apply_retention_policy(&self) -> Result<()> {
        let mut snapshots = self.snapshots.borrow_mut();
        let policy = &self.config.retention_policy;
        
        // Remove snapshots that exceed max count
        if let Some(max_count) = policy.max_snapshots {
            if snapshots.len() > max_count {
                // Keep the most recent snapshots
                let mut snapshot_vec: Vec<_> = snapshots.values().cloned().collect();
                snapshot_vec.sort_by(|a, b| b.created_at.cmp(&a.created_at));
                
                // Remove oldest snapshots beyond max_count
                for snapshot in snapshot_vec.iter().skip(max_count) {
                    snapshots.remove(&snapshot.id);
                }
            }
        }
        
        // Remove snapshots that exceed max age
        if let Some(max_age_days) = policy.max_age_days {
            let cutoff_date = Utc::now() - chrono::Duration::days(max_age_days as i64);
            snapshots.retain(|_, snapshot| {
                snapshot.created_at > cutoff_date || 
                snapshot.tags.iter().any(|tag| policy.preserve_tags.contains(tag))
            });
        }
        
        Ok(())
    }
    
    /// Get storage usage statistics
    pub fn get_storage_stats(&self) -> SnapshotStorageStats {
        let snapshots = self.snapshots.borrow();
        
        SnapshotStorageStats {
            total_snapshots: snapshots.len(),
            total_size_bytes: snapshots.values().map(|s| s.size_bytes).sum(),
            snapshots_by_type: {
                let mut by_type = HashMap::new();
                for snapshot in snapshots.values() {
                    *by_type.entry(format!("{:?}", snapshot.snapshot_type)).or_insert(0) += 1;
                }
                by_type
            },
            oldest_snapshot: snapshots.values().map(|s| s.created_at).min(),
            newest_snapshot: snapshots.values().map(|s| s.created_at).max(),
        }
    }
}

/// Validation result for rollback operation
#[derive(Debug, Clone)]
pub struct RollbackValidation {
    /// Whether rollback is safe to perform
    pub is_safe: bool,
    
    /// Warnings about potential issues
    pub warnings: Vec<String>,
    
    /// Assessment of data loss risk
    pub data_loss_risk: DataLossRisk,
    
    /// Schema changes that will be required
    pub schema_changes_required: Vec<String>,
    
    /// Estimated time for rollback in milliseconds
    pub estimated_duration_ms: u64,
}

/// Risk level for data loss during rollback
#[derive(Debug, Clone, PartialEq)]
pub enum DataLossRisk {
    None,
    Low,
    Medium,
    High,
}

/// Storage usage statistics
#[derive(Debug, Clone)]
pub struct SnapshotStorageStats {
    pub total_snapshots: usize,
    pub total_size_bytes: u64,
    pub snapshots_by_type: HashMap<String, usize>,
    pub oldest_snapshot: Option<DateTime<Utc>>,
    pub newest_snapshot: Option<DateTime<Utc>>,
}

impl Default for SnapshotConfig {
    fn default() -> Self {
        Self {
            max_data_table_rows: 1000,
            always_include_data: vec!["schema_versions".to_string(), "migration_history".to_string()],
            exclude_data: vec!["logs".to_string(), "temporary_data".to_string()],
            enable_compression: true,
            retention_policy: RetentionPolicy {
                max_snapshots: Some(50),
                max_age_days: Some(90),
                preserve_tags: vec!["important".to_string(), "release".to_string()],
                production_retention_days: Some(365),
            },
            storage_config: StorageConfig::Filesystem {
                base_path: PathBuf::from("./snapshots"),
                create_subdirs: true,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auto_migration::{TableSchema, ColumnSchema};

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
                            default_value: None,
                            primary_key: true,
                            auto_increment: true,
                            unique: false,
                            constraints: Vec::new(),
                        },
                        ColumnSchema {
                            name: "name".to_string(),
                            column_type: "TEXT".to_string(),
                            nullable: false,
                            default_value: None,
                            primary_key: false,
                            auto_increment: false,
                            unique: false,
                            constraints: Vec::new(),
                        },
                    ],
                    indexes: Vec::new(),
                    foreign_keys: Vec::new(),
                    constraints: Vec::new(),
                },
            ],
        }
    }

    #[tokio::test]
    async fn test_create_snapshot() {
        let config = SnapshotConfig::default();
        let manager = MigrationSnapshotManager::new(config);
        let schema = create_test_schema();
        
        let snapshot = manager.create_snapshot(
            "test_snapshot".to_string(),
            "Test snapshot creation".to_string(),
            schema.clone(),
            SnapshotType::Manual,
            vec!["test".to_string()],
        ).await.unwrap();
        
        assert_eq!(snapshot.id, "test_snapshot");
        assert_eq!(snapshot.description, "Test snapshot creation");
        assert_eq!(snapshot.schema, schema);
        assert_eq!(snapshot.snapshot_type, SnapshotType::Manual);
        assert!(snapshot.tags.contains(&"test".to_string()));
    }

    #[tokio::test]
    async fn test_pre_migration_snapshot() {
        let config = SnapshotConfig::default();
        let manager = MigrationSnapshotManager::new(config);
        let schema = create_test_schema();
        
        let snapshot = manager.create_pre_migration_snapshot("mig_001", schema).await.unwrap();
        
        assert_eq!(snapshot.id, "pre_migration_mig_001");
        assert!(snapshot.description.contains("mig_001"));
        assert_eq!(snapshot.snapshot_type, SnapshotType::PreMigration);
        assert!(snapshot.tags.contains(&"auto".to_string()));
        assert!(snapshot.tags.contains(&"pre-migration".to_string()));
    }

    #[tokio::test]
    async fn test_list_snapshots() {
        let config = SnapshotConfig::default();
        let manager = MigrationSnapshotManager::new(config);
        let schema = create_test_schema();
        
        // Create multiple snapshots
        manager.create_snapshot(
            "snap1".to_string(),
            "First snapshot".to_string(),
            schema.clone(),
            SnapshotType::Manual,
            vec!["test".to_string()],
        ).await.unwrap();
        
        manager.create_snapshot(
            "snap2".to_string(),
            "Second snapshot".to_string(),
            schema,
            SnapshotType::PreMigration,
            vec!["auto".to_string()],
        ).await.unwrap();
        
        let snapshots = manager.list_snapshots();
        assert_eq!(snapshots.len(), 2);
        
        let snap1 = snapshots.iter().find(|s| s.id == "snap1").unwrap();
        let snap2 = snapshots.iter().find(|s| s.id == "snap2").unwrap();
        
        assert_eq!(snap1.snapshot_type, SnapshotType::Manual);
        assert_eq!(snap2.snapshot_type, SnapshotType::PreMigration);
    }

    #[tokio::test]
    async fn test_get_snapshot() {
        let config = SnapshotConfig::default();
        let manager = MigrationSnapshotManager::new(config);
        let schema = create_test_schema();
        
        manager.create_snapshot(
            "test_get".to_string(),
            "Test get snapshot".to_string(),
            schema,
            SnapshotType::Manual,
            vec!["test".to_string()],
        ).await.unwrap();
        
        let retrieved = manager.get_snapshot("test_get").unwrap();
        assert_eq!(retrieved.id, "test_get");
        assert_eq!(retrieved.description, "Test get snapshot");
        
        let not_found = manager.get_snapshot("nonexistent");
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_delete_snapshot() {
        let config = SnapshotConfig::default();
        let manager = MigrationSnapshotManager::new(config);
        let schema = create_test_schema();
        
        manager.create_snapshot(
            "test_delete".to_string(),
            "Test delete snapshot".to_string(),
            schema,
            SnapshotType::Manual,
            vec!["test".to_string()],
        ).await.unwrap();
        
        // Verify snapshot exists
        assert!(manager.get_snapshot("test_delete").is_some());
        
        // Delete snapshot
        let deleted = manager.delete_snapshot("test_delete").await.unwrap();
        assert!(deleted);
        
        // Verify snapshot is gone
        assert!(manager.get_snapshot("test_delete").is_none());
        
        // Try to delete non-existent snapshot
        let not_deleted = manager.delete_snapshot("nonexistent").await.unwrap();
        assert!(!not_deleted);
    }

    #[tokio::test]
    async fn test_rollback_validation_safe() {
        let config = SnapshotConfig::default();
        let manager = MigrationSnapshotManager::new(config);
        let schema = create_test_schema();
        
        // Create snapshot
        manager.create_snapshot(
            "test_rollback".to_string(),
            "Test rollback validation".to_string(),
            schema.clone(),
            SnapshotType::Manual,
            vec!["test".to_string()],
        ).await.unwrap();
        
        // Validate rollback to same schema (should be safe)
        let validation = manager.validate_rollback("test_rollback", &schema).await.unwrap();
        assert!(validation.is_safe);
        assert_eq!(validation.data_loss_risk, DataLossRisk::None);
        assert!(validation.warnings.is_empty());
    }

    #[tokio::test]
    async fn test_rollback_validation_unsafe() {
        let config = SnapshotConfig::default();
        let manager = MigrationSnapshotManager::new(config);
        let old_schema = create_test_schema();
        
        // Create snapshot with old schema
        manager.create_snapshot(
            "test_unsafe".to_string(),
            "Test unsafe rollback".to_string(),
            old_schema,
            SnapshotType::Manual,
            vec!["test".to_string()],
        ).await.unwrap();
        
        // Create new schema with additional table
        let mut new_schema = create_test_schema();
        new_schema.tables.push(TableSchema {
            name: "posts".to_string(),
            columns: vec![
                ColumnSchema {
                    name: "id".to_string(),
                    column_type: "INTEGER".to_string(),
                    nullable: false,
                    default_value: None,
                    primary_key: true,
                    auto_increment: true,
                    unique: false,
                    constraints: Vec::new(),
                },
            ],
            indexes: Vec::new(),
            foreign_keys: Vec::new(),
            constraints: Vec::new(),
        });
        
        // Validate rollback (should be unsafe due to data loss)
        let validation = manager.validate_rollback("test_unsafe", &new_schema).await.unwrap();
        assert!(!validation.is_safe);
        assert_eq!(validation.data_loss_risk, DataLossRisk::High);
        assert!(!validation.warnings.is_empty());
        assert!(validation.warnings.iter().any(|w| w.contains("posts")));
    }

    #[tokio::test]
    async fn test_storage_stats() {
        let config = SnapshotConfig::default();
        let manager = MigrationSnapshotManager::new(config);
        let schema = create_test_schema();
        
        // Create snapshots of different types
        manager.create_snapshot(
            "manual1".to_string(),
            "Manual snapshot".to_string(),
            schema.clone(),
            SnapshotType::Manual,
            vec!["test".to_string()],
        ).await.unwrap();
        
        manager.create_snapshot(
            "auto1".to_string(),
            "Auto snapshot".to_string(),
            schema,
            SnapshotType::PreMigration,
            vec!["auto".to_string()],
        ).await.unwrap();
        
        let stats = manager.get_storage_stats();
        assert_eq!(stats.total_snapshots, 2);
        assert!(stats.snapshots_by_type.contains_key("Manual"));
        assert!(stats.snapshots_by_type.contains_key("PreMigration"));
        assert!(stats.oldest_snapshot.is_some());
        assert!(stats.newest_snapshot.is_some());
    }
}